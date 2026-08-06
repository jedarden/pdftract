package codegen

// MethodMetadata defines the signature and behavior of a single SDK method.
type MethodMetadata struct {
	// Name is the snake_case method name (e.g., "extract_text")
	Name string
	// PascalName is the exported Go method name (e.g., "ExtractText")
	PascalName string
	// Description is a one-line description for godoc
	Description string
	// CLIFlag is the CLI subcommand flag (e.g., "extract", "grep", "hash")
	CLIFlag string
	// ReturnsString indicates whether the method returns a plain string (vs JSON struct)
	ReturnsString bool
	// OptionsType is the name of the options struct (empty if no options)
	OptionsType string
	// ReturnType is the Go return type (e.g., "*Document", "string", "<-chan PageResult")
	ReturnType string
	// IsChannel indicates whether the return type is a channel (streaming method)
	IsChannel bool
	// ChannelElementType is the element type for channel returns (e.g., "PageResult")
	ChannelElementType string
	// UsesStringParams indicates if method uses string params instead of Source
	UsesStringParams bool
	// StringParamCount is the number of string parameters (0 for Source-based methods)
	StringParamCount int
	// AdditionalCLIArgs are extra CLI flags to append (e.g., "--text", "--md")
	AdditionalCLIArgs []string
}

// MethodInfo provides computed values for template rendering.
type MethodInfo struct {
	MethodMetadata
	// HasOptions indicates if the method has an options parameter
	HasOptions bool
	// NeedsSource indicates if method takes a Source parameter
	NeedsSource bool
	// BufferedChannelSize is the buffer size for streaming channels
	BufferedChannelSize int
}

// AllMethods defines the complete method surface for the Go SDK.
// This is the source of truth for code generation.
var AllMethods = []MethodMetadata{
	{
		Name:                "extract",
		PascalName:          "Extract",
		Description:        "Extract structured data from a PDF",
		CLIFlag:             "extract",
		ReturnsString:       false,
		OptionsType:         "ExtractOptions",
		ReturnType:          "*Document",
		IsChannel:           false,
		UsesStringParams:    false,
		StringParamCount:    0,
		AdditionalCLIArgs:   []string{},
	},
	{
		Name:                "extract_text",
		PascalName:          "ExtractText",
		Description:        "Extract plain text from a PDF",
		CLIFlag:             "extract",
		ReturnsString:       true,
		OptionsType:         "ExtractOptions",
		ReturnType:          "string",
		IsChannel:           false,
		UsesStringParams:    false,
		StringParamCount:    0,
		AdditionalCLIArgs:   []string{"--text"},
	},
	{
		Name:                "extract_markdown",
		PascalName:          "ExtractMarkdown",
		Description:        "Extract Markdown-formatted text from a PDF",
		CLIFlag:             "extract",
		ReturnsString:       true,
		OptionsType:         "ExtractOptions",
		ReturnType:          "string",
		IsChannel:           false,
		UsesStringParams:    false,
		StringParamCount:    0,
		AdditionalCLIArgs:   []string{"--md"},
	},
	{
		Name:                "extract_stream",
		PascalName:          "ExtractStream",
		Description:        "Extract pages from a PDF as a stream",
		CLIFlag:             "extract",
		ReturnsString:       false,
		OptionsType:         "ExtractOptions",
		ReturnType:          "<-chan PageResult",
		IsChannel:           true,
		ChannelElementType:  "PageResult",
		UsesStringParams:    false,
		StringParamCount:    0,
		AdditionalCLIArgs:   []string{"--ndjson"},
	},
	{
		Name:                "search",
		PascalName:          "Search",
		Description:        "Search for text in a PDF",
		CLIFlag:             "grep",
		ReturnsString:       false,
		OptionsType:         "SearchOptions",
		ReturnType:          "<-chan MatchResult",
		IsChannel:           true,
		ChannelElementType:  "MatchResult",
		UsesStringParams:    false,
		StringParamCount:    0,
		AdditionalCLIArgs:   []string{},
	},
	{
		Name:                "get_metadata",
		PascalName:          "GetMetadata",
		Description:        "Get metadata from a PDF",
		CLIFlag:             "extract",
		ReturnsString:       false,
		OptionsType:         "BaseOptions",
		ReturnType:          "*Metadata",
		IsChannel:           false,
		UsesStringParams:    false,
		StringParamCount:    0,
		AdditionalCLIArgs:   []string{"--metadata-only"},
	},
	{
		Name:                "hash",
		PascalName:          "Hash",
		Description:        "Compute hash fingerprint of a PDF",
		CLIFlag:             "hash",
		ReturnsString:       false,
		OptionsType:         "HashOptions",
		ReturnType:          "*Fingerprint",
		IsChannel:           false,
		UsesStringParams:    false,
		StringParamCount:    0,
		AdditionalCLIArgs:   []string{},
	},
	{
		Name:                "classify",
		PascalName:          "Classify",
		Description:        "Classify a PDF document",
		CLIFlag:             "classify",
		ReturnsString:       false,
		OptionsType:         "", // No options for classify
		ReturnType:          "*Classification",
		IsChannel:           false,
		UsesStringParams:    false,
		StringParamCount:    0,
		AdditionalCLIArgs:   []string{},
	},
	{
		Name:                "verify_receipt",
		PascalName:          "VerifyReceipt",
		Description:        "Verify a citation receipt",
		CLIFlag:             "verify-receipt",
		ReturnsString:       false,
		OptionsType:         "", // No options for verify_receipt
		ReturnType:          "bool",
		IsChannel:           false,
		UsesStringParams:    true,
		StringParamCount:    2,
		AdditionalCLIArgs:   []string{},
	},
}

// ToMethodInfo computes derived values for template rendering.
func ToMethodInfo(m MethodMetadata) MethodInfo {
	return MethodInfo{
		MethodMetadata:      m,
		HasOptions:         m.OptionsType != "",
		NeedsSource:        !m.UsesStringParams,
		BufferedChannelSize: 16,
	}
}

// CLIFlagToGoField converts CLI kebab-case flags to Go PascalCase field names.
// Examples:
//   - "ocr-language" -> "OCRLanguage"
//   - "preserve-layout" -> "PreserveLayout"
//   - "case-insensitive" -> "CaseInsensitive"
func CLIFlagToGoField(flag string) string {
	var result []rune
	capitalizeNext := true

	for _, r := range flag {
		if r == '-' || r == '_' {
			capitalizeNext = true
			continue
		}
		if capitalizeNext {
			result = append(result, toUpper(r))
			capitalizeNext = false
		} else {
			result = append(result, r)
		}
	}

	return string(result)
}

func toUpper(r rune) rune {
	if r >= 'a' && r <= 'z' {
		return r - ('a' - 'A')
	}
	return r
}
