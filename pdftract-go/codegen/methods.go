package codegen

import (
	"bytes"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"text/template"
)

// Generator handles template-based Go code generation.
type Generator struct {
	templates *template.Template
	config    *GeneratorConfig
}

// GeneratorConfig configures the generation process.
type GeneratorConfig struct {
	// PackageName is the Go package name for generated files
	PackageName string
	// Version is the SDK version string
	Version string
	// OutputDir is where generated files will be written
	OutputDir string
}

// NewGenerator creates a new code generator.
func NewGenerator(config *GeneratorConfig) (*Generator, error) {
	tmpl := template.New("codegen").Funcs(template.FuncMap{
		"toPascal":       toPascalCase,
		"toCamel":        toCamelCase,
		"toSnake":        toSnakeCase,
		"cliFlagToField": CLIFlagToGoField,
		"hasOptions":     func(m MethodMetadata) bool { return m.OptionsType != "" },
		"isChannel":      func(m MethodMetadata) bool { return m.IsChannel },
		"needsSource":    func(m MethodMetadata) bool { return !m.UsesStringParams },
	})

	// Load all templates from the templates directory
	templatesDir := filepath.Join("codegen", "templates")
	if _, err := tmpl.ParseGlob(filepath.Join(templatesDir, "*.tmpl")); err != nil {
		return nil, fmt.Errorf("failed to parse templates: %w", err)
	}

	return &Generator{
		templates: tmpl,
		config:    config,
	}, nil
}

// GenerateMethods generates the methods.go file with all SDK methods.
func (g *Generator) GenerateMethods() error {
	methodsData := struct {
		PackageName string
		Version     string
		Methods     []MethodInfo
	}{
		PackageName: g.config.PackageName,
		Version:     g.config.Version,
		Methods:     methodsToInfo(AllMethods),
	}

	// Check if base template exists, otherwise use inline template
	var tmpl *template.Template
	if g.templates.Lookup("methods.go.tmpl") != nil {
		tmpl = g.templates.Lookup("methods.go.tmpl")
	} else {
		// Use default inline template
		var err error
		tmpl, err = g.templates.New("methods").Parse(defaultMethodsTemplate)
		if err != nil {
			return fmt.Errorf("failed to parse default template: %w", err)
		}
	}

	var buf bytes.Buffer
	if err := tmpl.Execute(&buf, methodsData); err != nil {
		return fmt.Errorf("failed to render template: %w", err)
	}

	// Write to output file
	outputPath := filepath.Join(g.config.OutputDir, "methods.go")
	if err := os.WriteFile(outputPath, buf.Bytes(), 0644); err != nil {
		return fmt.Errorf("failed to write output file: %w", err)
	}

	fmt.Printf("Generated: %s\n", outputPath)
	return nil
}

// GenerateMethodTemplates generates individual method templates for special cases.
func (g *Generator) GenerateMethodTemplates() error {
	for _, method := range AllMethods {
		// Only generate separate templates for methods with special cases
		// (streaming methods, string params, etc.)
		if needsSpecialTemplate(method) {
			if err := g.generateSingleMethod(method); err != nil {
				return fmt.Errorf("failed to generate method %s: %w", method.Name, err)
			}
		}
	}
	return nil
}

// generateSingleMethod generates a template for a single method.
func (g *Generator) generateSingleMethod(method MethodMetadata) error {
	methodInfo := ToMethodInfo(method)

	methodData := struct {
		PackageName string
		Version     string
		Method      MethodInfo
	}{
		PackageName: g.config.PackageName,
		Version:     g.config.Version,
		Method:      methodInfo,
	}

	tmplName := fmt.Sprintf("method_%s.tmpl", method.Name)
	tmpl := g.templates.Lookup(tmplName)
	if tmpl == nil {
		// Try the default streaming template
		if method.IsChannel {
			tmpl = g.templates.Lookup("streaming_method.tmpl")
		} else if method.UsesStringParams {
			tmpl = g.templates.Lookup("string_params_method.tmpl")
		}
	}

	if tmpl == nil {
		return fmt.Errorf("no template found for method %s", method.Name)
	}

	var buf bytes.Buffer
	if err := tmpl.Execute(&buf, methodData); err != nil {
		return fmt.Errorf("failed to render method template: %w", err)
	}

	// Write to a method-specific file
	outputPath := filepath.Join(g.config.OutputDir, fmt.Sprintf("method_%s.go", method.Name))
	if err := os.WriteFile(outputPath, buf.Bytes(), 0644); err != nil {
		return fmt.Errorf("failed to write method file: %w", err)
	}

	fmt.Printf("Generated: %s\n", outputPath)
	return nil
}

// GenerateAll generates all methods and method-specific templates.
func (g *Generator) GenerateAll() error {
	if err := g.GenerateMethods(); err != nil {
		return err
	}
	return g.GenerateMethodTemplates()
}

// methodsToInfo converts all MethodMetadata to MethodInfo.
func methodsToInfo(methods []MethodMetadata) []MethodInfo {
	result := make([]MethodInfo, len(methods))
	for i, m := range methods {
		result[i] = ToMethodInfo(m)
	}
	return result
}

// needsSpecialTemplate determines if a method needs its own template file.
func needsSpecialTemplate(method MethodMetadata) bool {
	return method.IsChannel || method.UsesStringParams
}

// Case conversion helpers for templates

func toPascalCase(s string) string {
	return toTitleCase(strings.ReplaceAll(s, "_", " "))
}

func toCamelCase(s string) string {
	pascal := toPascalCase(s)
	if len(pascal) == 0 {
		return pascal
	}
	return strings.ToLower(pascal[:1]) + pascal[1:]
}

func toSnakeCase(s string) string {
	var result []rune
	for i, r := range s {
		if i > 0 && r >= 'A' && r <= 'Z' {
			result = append(result, '_')
		}
		result = append(result, r)
	}
	return strings.ToLower(string(result))
}

func toTitleCase(s string) string {
	words := strings.Fields(s)
	for i, word := range words {
		if len(word) > 0 {
			words[i] = strings.ToUpper(word[:1]) + strings.ToLower(word[1:])
		}
	}
	return strings.Join(words, "")
}

// defaultMethodsTemplate is the fallback template when no template file is found.
const defaultMethodsTemplate = `// Code generated by pdftract-go/codegen. DO NOT EDIT.

package {{.PackageName}}

// This file was generated from method definitions.
// Generator version: {{.Version}}

{{range .Methods}}
// {{.Description}}
func (c *Client) {{.PascalName}}(
	{{- if .NeedsSource}}source Source{{end}}
	{{- if .HasOptions}}, options *{{.OptionsType}}{{end}}
	{{- if .UsesStringParams}}
		{{- if eq .StringParamCount 2}}path string, receipt string{{end -}}
	{{end}}
) ({{.ReturnType}}, error) {
	args := []string{"{{.CLIFlag}}"}
	{{- if .NeedsSource}}
	args = append(args, source.source()...)
	{{- else if .UsesStringParams}}
	args = append(args, path)
	{{- end}}
	{{- if .HasOptions}}
	if options != nil {
		args = append(args, options.toArgs()...)
	}
	{{- end}}
	{{- range .AdditionalCLIArgs}}
	args = append(args, "{{.}}")
	{{- end}}

	return c.invoke({{.PascalName}}Result, args)
}
{{end}}
`
