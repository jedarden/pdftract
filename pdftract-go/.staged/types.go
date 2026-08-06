package pdftract

import "strconv"

// Document represents a PDF document with pages and metadata.
type Document struct {
	SchemaVersion string   `json:"schema_version"`
	Pages         []Page   `json:"pages"`
	Metadata      Metadata `json:"metadata"`
}

// Page represents a single page in the document.
type Page struct {
	Page     int     `json:"page"`
	Width    float64 `json:"width"`
	Height   float64 `json:"height"`
	Rotation int     `json:"rotation"`
	Spans    []Span  `json:"spans"`
	Blocks   []Block `json:"blocks"`
}

// Span represents a text span with font and position information.
type Span struct {
	Text       string   `json:"text"`
	Bbox       [4]float64 `json:"bbox"`
	Font       string   `json:"font"`
	Size       float64  `json:"size"`
	Confidence *float64 `json:"confidence"`
}

// Block represents a structural block (paragraph, heading, table, etc.).
type Block struct {
	Kind   string `json:"kind"`
	Text   string `json:"text"`
	Bbox   [4]float64 `json:"bbox"`
	Level  *int   `json:"level,omitempty"`
}

// Match represents a search match result.
type Match struct {
	Text    string       `json:"text"`
	Page    int          `json:"page"`
	Bbox    [4]float64   `json:"bbox"`
	Context MatchContext `json:"context"`
}

// MatchContext provides surrounding text for a match.
type MatchContext struct {
	Before string `json:"before"`
	After  string `json:"after"`
}

// Fingerprint represents document hash information.
type Fingerprint struct {
	Hash      string   `json:"hash"`
	PageCount int      `json:"page_count"`
	FastHash  string   `json:"fast_hash"`
	Metadata  Metadata `json:"metadata"`
}

// Classification represents document classification results.
type Classification struct {
	Category    string              `json:"category"`
	Confidence  float64             `json:"confidence"`
	Tags        []string            `json:"tags"`
	Heuristics  map[string]bool     `json:"heuristics"`
}

// Metadata represents document metadata.
type Metadata struct {
	Title     string   `json:"title,omitempty"`
	Author    string   `json:"author,omitempty"`
	Subject   string   `json:"subject,omitempty"`
	Keywords  []string `json:"keywords,omitempty"`
	Creator   string   `json:"creator,omitempty"`
	Producer  string   `json:"producer,omitempty"`
	Created   *string  `json:"created,omitempty"`
	Modified  *string  `json:"modified,omitempty"`
	PageCount int      `json:"page_count"`
}

// Receipt represents a cryptographic receipt for document verification.
type Receipt struct {
	Hash      string `json:"hash"`
	Signature string `json:"signature"`
	Timestamp string `json:"timestamp"`
}

// ExtractOptions controls extraction behavior.
type ExtractOptions struct {
	Password       string
	OCRLanguage    string
	OCRThreshold   float64
	PreserveLayout bool
	ExtractImages  bool
	ImageFormat    string
	MinImageSize   int
}

func (o *ExtractOptions) toArgs() []string {
	args := []string{}
	if o.Password != "" {
		args = append(args, "--password", o.Password)
	}
	if o.OCRLanguage != "" {
		args = append(args, "--ocr-language", o.OCRLanguage)
	}
	if o.OCRThreshold != 0 {
		args = append(args, "--ocr-threshold", strconv.FormatFloat(o.OCRThreshold, 'f', -1, 64))
	}
	if o.PreserveLayout {
		args = append(args, "--preserve-layout")
	}
	if o.ExtractImages {
		args = append(args, "--extract-images")
	}
	if o.ImageFormat != "" {
		args = append(args, "--image-format", o.ImageFormat)
	}
	if o.MinImageSize != 0 {
		args = append(args, "--min-image-size", strconv.Itoa(o.MinImageSize))
	}
	return args
}

// SearchOptions controls search behavior.
type SearchOptions struct {
	CaseInsensitive bool
	Regex           bool
	WholeWord       bool
	MaxResults      *int
}

func (o *SearchOptions) toArgs() []string {
	args := []string{}
	if o.CaseInsensitive {
		args = append(args, "--case-insensitive")
	}
	if o.Regex {
		args = append(args, "--regex")
	}
	if o.WholeWord {
		args = append(args, "--whole-word")
	}
	if o.MaxResults != nil {
		args = append(args, "--max-results", strconv.Itoa(*o.MaxResults))
	}
	return args
}

// HashOptions controls hash computation behavior.
type HashOptions struct {
	Password string
}

func (o *HashOptions) toArgs() []string {
	args := []string{}
	if o.Password != "" {
		args = append(args, "--password", o.Password)
	}
	return args
}
