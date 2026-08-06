package codegen

import (
	"errors"
	"fmt"
)

// ErrKind represents the kind of pdftract error.
type ErrKind int

const (
	// ErrCorruptPDF indicates the PDF file is corrupted or malformed (exit code 2).
	ErrCorruptPDF ErrKind = 1 + iota

	// ErrEncryption indicates the PDF is encrypted and requires a password (exit code 3).
	ErrEncryption

	// ErrIO indicates an I/O error occurred (exit code 4).
	ErrIO

	// ErrInvalidSource indicates the source is invalid or cannot be accessed (exit code 5).
	ErrInvalidSource

	// ErrOCRFailed indicates OCR processing failed (exit code 6).
	ErrOCRFailed

	// ErrOutOfMemory indicates the process ran out of memory (exit code 7).
	ErrOutOfMemory

	// ErrPagination indicates an error occurred during pagination (exit code 8).
	ErrPagination

	// ErrUnknown indicates an unknown error occurred.
	ErrUnknown
)

// PdftractError is the base error type for all pdftract errors.
type PdftractError struct {
	Kind     ErrKind
	Message  string
	ExitCode int
}

// Error implements the error interface.
func (e *PdftractError) Error() string {
	kindDesc := e.kindDescription()
	if e.Message != "" {
		return fmt.Sprintf("pdftract error (exit %d): %s: %s", e.ExitCode, kindDesc, e.Message)
	}
	return fmt.Sprintf("pdftract error (exit %d): %s", e.ExitCode, kindDesc)
}

// Is allows errors.Is to match error kinds.
func (e *PdftractError) Is(target error) bool {
	if target == nil {
		return false
	}
	t, ok := target.(*PdftractError)
	if !ok || t == nil {
		return false
	}
	return e.Kind == t.Kind
}

// kindDescription returns a human-readable description of the error kind.
func (e *PdftractError) kindDescription() string {
	switch e.Kind {
	case ErrCorruptPDF:
		return "corrupt PDF"
	case ErrEncryption:
		return "encryption error"
	case ErrIO:
		return "I/O error"
	case ErrInvalidSource:
		return "invalid source"
	case ErrOCRFailed:
		return "OCR failed"
	case ErrOutOfMemory:
		return "out of memory"
	case ErrPagination:
		return "pagination error"
	case ErrUnknown:
		return "unknown error"
	default:
		return "undefined error"
	}
}

// Predefined error instances for use with errors.Is.

// ErrCorruptPDFInstance is the predefined corrupt PDF error.
var ErrCorruptPDFInstance = &PdftractError{
	Kind:     ErrCorruptPDF,
	Message:  "PDF file is corrupted or malformed",
	ExitCode: 2,
}

// ErrEncryptionInstance is the predefined encryption error.
var ErrEncryptionInstance = &PdftractError{
	Kind:     ErrEncryption,
	Message:  "PDF is encrypted and requires a password",
	ExitCode: 3,
}

// ErrIOInstance is the predefined I/O error.
var ErrIOInstance = &PdftractError{
	Kind:     ErrIO,
	Message:  "I/O error occurred",
	ExitCode: 4,
}

// ErrInvalidSourceInstance is the predefined invalid source error.
var ErrInvalidSourceInstance = &PdftractError{
	Kind:     ErrInvalidSource,
	Message:  "Source is invalid or cannot be accessed",
	ExitCode: 5,
}

// ErrOCRFailedInstance is the predefined OCR failed error.
var ErrOCRFailedInstance = &PdftractError{
	Kind:     ErrOCRFailed,
	Message:  "OCR processing failed",
	ExitCode: 6,
}

// ErrOutOfMemoryInstance is the predefined out of memory error.
var ErrOutOfMemoryInstance = &PdftractError{
	Kind:     ErrOutOfMemory,
	Message:  "Process ran out of memory",
	ExitCode: 7,
}

// ErrPaginationInstance is the predefined pagination error.
var ErrPaginationInstance = &PdftractError{
	Kind:     ErrPagination,
	Message:  "Error occurred during pagination",
	ExitCode: 8,
}

// ErrUnknownInstance is the predefined unknown error.
var ErrUnknownInstance = &PdftractError{
	Kind:     ErrUnknown,
	Message:  "Unknown error occurred",
	ExitCode: 1,
}

// mapExitCodeToErrKind converts CLI exit codes to ErrKind values.
func mapExitCodeToErrKind(exitCode int) ErrKind {
	switch exitCode {
	case 2:
		return ErrCorruptPDF
	case 3:
		return ErrEncryption
	case 4:
		return ErrIO
	case 5:
		return ErrInvalidSource
	case 6:
		return ErrOCRFailed
	case 7:
		return ErrOutOfMemory
	case 8:
		return ErrPagination
	default:
		return ErrUnknown
	}
}

// NewPdftractError creates a new PdftractError from an exit code and message.
func NewPdftractError(exitCode int, message string) *PdftractError {
	kind := mapExitCodeToErrKind(exitCode)
	return &PdftractError{
		Kind:     kind,
		Message:  message,
		ExitCode: exitCode,
	}
}

// AsPdftractError extracts the underlying *PdftractError if present.
// This is a convenience wrapper around errors.As.
func AsPdftractError(err error) (*PdftractError, bool) {
	var pdftractErr *PdftractError
	if errors.As(err, &pdftractErr) {
		return pdftractErr, true
	}
	return nil, false
}
