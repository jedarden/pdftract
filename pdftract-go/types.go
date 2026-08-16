// Package pdftract provides Go types for the pdftract PDF processing library.
// These types correspond to the core data structures used across pdftract's SDKs.
package pdftract

// Page represents a single page in a PDF document.
// It contains the page's dimensions and rotation information.
type Page struct {
	Number   int // Page number (1-indexed)
	Width    int // Page width in points
	Height   int // Page height in points
	Rotation int // Page rotation in degrees (0, 90, 180, 270)
}

// Metadata contains document-level metadata extracted from a PDF.
// It includes standard PDF information fields and document properties.
type Metadata struct {
	Pages       int      // Total number of pages
	Title       string   // Document title
	Author      string   // Document author
	Subject     string   // Document subject
	Keywords    []string // Keywords/tags associated with the document
	Creator     string   // Application that created the PDF
	Producer    string   // Application that generated the PDF
	CreationDate string // Date the document was created
	ModDate     string   // Date the document was last modified
	Tagged      bool     // Whether the PDF is tagged (accessible)
	Form        bool     // Whether the PDF contains form fields
	Encrypted   bool     // Whether the PDF is encrypted
}

// Fingerprint provides a unique identifier for a PDF document.
// It enables deduplication and change detection across document sets.
type Fingerprint struct {
	Hash      string // Document hash (e.g., SHA-256)
	Algorithm string // Hash algorithm used (e.g., "SHA256")
	Pages     int    // Number of pages in the document
}

// Classification represents a machine learning classification result.
// It categorizes content with an associated confidence score.
type Classification struct {
	Type       string  // Classification category/type
	Confidence float64 // Confidence score (0.0 to 1.0)
	Label      string  // Human-readable label for the classification
}
