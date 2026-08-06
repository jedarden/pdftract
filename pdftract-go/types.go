package pdftract

// Document represents a PDF document with pages and metadata.
type Document struct {
	Path     string
	Pages    []Page
	Metadata Metadata
}

// Page represents a single page in the document.
type Page struct {
	Number   int
	Width    int
	Height   int
	Rotation int
}

// Metadata represents PDF document metadata.
type Metadata struct {
	Pages        int
	Title        string
	Author       string
	Subject      string
	Keywords     []string
	Creator      string
	Producer     string
	CreationDate string
	ModDate      string
	Tagged       bool
	Form         bool
	Encrypted    bool
}

// Fingerprint represents document hash information.
type Fingerprint struct {
	Hash      string
	Algorithm string
	Pages     int
}

// Classification represents document classification results.
type Classification struct {
	Type       string
	Confidence float64
	Label      string
}

// PageResult represents the result of a page extraction operation.
type PageResult struct {
	PageNum int
	Content string
	Err     error
}

// MatchResult represents a search match with position and scoring information.
type MatchResult struct {
	PageNum  int
	Position []int
	Snippet  string
	Score    float64
}
