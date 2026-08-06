package codegen

import (
	"os"
	"path/filepath"
	"testing"
)

func TestGenerator_GenerateMethods(t *testing.T) {
	// Create a temporary output directory
	tmpDir := t.TempDir()

	config := &GeneratorConfig{
		PackageName: "pdftract",
		Version:     "0.1.0-test",
		OutputDir:   tmpDir,
	}

	gen, err := NewGenerator(config)
	if err != nil {
		t.Fatalf("Failed to create generator: %v", err)
	}

	if err := gen.GenerateMethods(); err != nil {
		t.Fatalf("Failed to generate methods: %v", err)
	}

	// Verify the output file exists
	outputPath := filepath.Join(tmpDir, "methods.go")
	if _, err := os.Stat(outputPath); os.IsNotExist(err) {
		t.Fatalf("Generated file does not exist: %s", outputPath)
	}

	// Read and verify the content contains expected methods
	content, err := os.ReadFile(outputPath)
	if err != nil {
		t.Fatalf("Failed to read generated file: %v", err)
	}

	contentStr := string(content)

	// Check for key method signatures
	expectedMethods := []string{
		"ExtractText",
		"ExtractMarkdown",
		"ExtractStream",
		"Search",
		"GetMetadata",
		"Hash",
		"Classify",
		"VerifyReceipt",
	}

	for _, method := range expectedMethods {
		if !contains(contentStr, method) {
			t.Errorf("Generated file missing method: %s", method)
		}
	}
}

func TestCLIFlagToGoField(t *testing.T) {
	tests := []struct {
		input    string
		expected string
	}{
		{"ocr-language", "OCRLanguage"},
		{"preserve-layout", "PreserveLayout"},
		{"case-insensitive", "CaseInsensitive"},
		{"extract-images", "ExtractImages"},
		{"min-image-size", "MinImageSize"},
		{"image-format", "ImageFormat"},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			result := CLIFlagToGoField(tt.input)
			if result != tt.expected {
				t.Errorf("CLIFlagToGoField(%q) = %q, want %q", tt.input, result, tt.expected)
			}
		})
	}
}

func TestAllMethods_Complete(t *testing.T) {
	// Verify we have all 9 expected methods
	expectedCount := 9
	if len(AllMethods) != expectedCount {
		t.Errorf("AllMethods has %d entries, want %d", len(AllMethods), expectedCount)
	}

	// Verify each method has required fields
	for i, method := range AllMethods {
		if method.Name == "" {
			t.Errorf("Method %d: Name is empty", i)
		}
		if method.PascalName == "" {
			t.Errorf("Method %d: PascalName is empty", i)
		}
		if method.CLIFlag == "" {
			t.Errorf("Method %d: CLIFlag is empty", i)
		}
		if method.ReturnType == "" {
			t.Errorf("Method %d: ReturnType is empty", i)
		}
	}
}

func TestToMethodInfo(t *testing.T) {
	tests := []struct {
		name     string
		method   MethodMetadata
		expected MethodInfo
	}{
		{
			name: "ExtractWithOptions",
			method: MethodMetadata{
				Name:        "extract",
				OptionsType: "ExtractOptions",
				IsChannel:   false,
			},
			expected: MethodInfo{
				MethodMetadata: MethodMetadata{
					Name:        "extract",
					OptionsType: "ExtractOptions",
					IsChannel:   false,
				},
				HasOptions: true,
				NeedsSource: true,
			},
		},
		{
			name: "ExtractStreamChannel",
			method: MethodMetadata{
				Name:        "extract_stream",
				OptionsType: "ExtractOptions",
				IsChannel:   true,
			},
			expected: MethodInfo{
				MethodMetadata: MethodMetadata{
					Name:        "extract_stream",
					OptionsType: "ExtractOptions",
					IsChannel:   true,
				},
				HasOptions: true,
				NeedsSource: true,
			},
		},
		{
			name: "ClassifyNoOptions",
			method: MethodMetadata{
				Name:        "classify",
				OptionsType: "",
				IsChannel:   false,
			},
			expected: MethodInfo{
				MethodMetadata: MethodMetadata{
					Name:        "classify",
					OptionsType: "",
					IsChannel:   false,
				},
				HasOptions: false,
				NeedsSource: true,
			},
		},
		{
			name: "VerifyReceiptStringParams",
			method: MethodMetadata{
				Name:             "verify_receipt",
				OptionsType:      "",
				IsChannel:        false,
				UsesStringParams: true,
				StringParamCount: 2,
			},
			expected: MethodInfo{
				MethodMetadata: MethodMetadata{
					Name:             "verify_receipt",
					OptionsType:      "",
					IsChannel:        false,
					UsesStringParams: true,
					StringParamCount: 2,
				},
				HasOptions:  false,
				NeedsSource: false,
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := ToMethodInfo(tt.method)
			if result.HasOptions != tt.expected.HasOptions {
				t.Errorf("HasOptions = %v, want %v", result.HasOptions, tt.expected.HasOptions)
			}
			if result.NeedsSource != tt.expected.NeedsSource {
				t.Errorf("NeedsSource = %v, want %v", result.NeedsSource, tt.expected.NeedsSource)
			}
		})
	}
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > len(substr) && containsSubstring(s, substr))
}

func containsSubstring(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
