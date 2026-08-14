package pdftract

import (
	"fmt"
)

// validatePage validates a Page struct and returns an error if any fields are missing or invalid.
func validatePage(page *Page) error {
	if page == nil {
		return &MalformedPageDataError{Message: "page is nil"}
	}

	// Validate Number field
	if page.Number <= 0 {
		return &InvalidPageFieldError{
			FieldName: "Number",
			FieldValue: page.Number,
			Reason: "page number must be positive (greater than 0)",
		}
	}

	// Validate Width field
	if page.Width <= 0 {
		return &InvalidPageFieldError{
			FieldName: "Width",
			FieldValue: page.Width,
			Reason: "page width must be positive (greater than 0)",
		}
	}

	// Validate Height field
	if page.Height <= 0 {
		return &InvalidPageFieldError{
			FieldName: "Height",
			FieldValue: page.Height,
			Reason: "page height must be positive (greater than 0)",
		}
	}

	// Validate Rotation field (common values: 0, 90, 180, 270, but allow any int)
	if page.Rotation < 0 || page.Rotation > 360 {
		// This is a warning rather than a hard error, as some PDFs may have unusual rotations
		// For now, we'll allow it but could log a warning in production code
	}

	return nil
}

// validateDocumentPages validates that a Document has valid Pages data.
func validateDocumentPages(doc *Document) error {
	if doc == nil {
		return &MalformedPageDataError{Message: "document is nil"}
	}

	if len(doc.Pages) == 0 {
		return &MissingPageDataError{Message: fmt.Sprintf("document contains no pages (metadata.page_count=%d)", doc.Metadata.Pages)}
	}

	// Check for page count mismatch
	if doc.Metadata.Pages > 0 && len(doc.Pages) != doc.Metadata.Pages {
		return &MalformedPageDataError{
			Message: fmt.Sprintf("page count mismatch: metadata claims %d pages but %d pages found in data", doc.Metadata.Pages, len(doc.Pages)),
		}
	}

	// Validate each page
	for i, page := range doc.Pages {
		if err := validatePage(&page); err != nil {
			// Add context about which page failed validation
			if malformedErr, ok := err.(*MalformedPageDataError); ok {
				malformedErr.Message = fmt.Sprintf("page %d: %s", i+1, malformedErr.Message)
			}
			return err
		}
	}

	return nil
}
