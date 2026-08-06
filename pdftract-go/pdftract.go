// Package pdftract provides a Go SDK for the pdftract PDF extraction library.
//
// The SDK accepts three types of PDF sources via the Source interface:
//   - PathSource: local filesystem path
//   - URLSource: remote URL
//   - BytesSource: in-memory PDF bytes
//
// Example usage:
//
//	import "github.com/jedarden/pdftract-go"
//
//	// From a local file
//	source := pdftract.PathSource("document.pdf")
//
//	// From a URL
//	source := pdftract.URLSource("https://example.com/doc.pdf")
//
//	// From raw bytes
//	data, _ := os.ReadFile("document.pdf")
//	source := pdftract.BytesSource(data)
package pdftract
