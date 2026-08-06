package codegen

import (
	"errors"
	"testing"
)

func TestErrKindValues(t *testing.T) {
	tests := []struct {
		name     string
		kind     ErrKind
		expected int
	}{
		{"ErrCorruptPDF", ErrCorruptPDF, 1},
		{"ErrEncryption", ErrEncryption, 2},
		{"ErrIO", ErrIO, 3},
		{"ErrInvalidSource", ErrInvalidSource, 4},
		{"ErrOCRFailed", ErrOCRFailed, 5},
		{"ErrOutOfMemory", ErrOutOfMemory, 6},
		{"ErrPagination", ErrPagination, 7},
		{"ErrUnknown", ErrUnknown, 8},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if int(tt.kind) != tt.expected {
				t.Errorf("ErrKind %s = %d, want %d", tt.name, int(tt.kind), tt.expected)
			}
		})
	}
}

func TestPdftractErrorError(t *testing.T) {
	tests := []struct {
		name     string
		err      *PdftractError
		contains string
	}{
		{
			name:     "corrupt PDF with message",
			err:      &PdftractError{Kind: ErrCorruptPDF, Message: "file header is invalid", ExitCode: 2},
			contains: "corrupt PDF",
		},
		{
			name:     "encryption error with message",
			err:      &PdftractError{Kind: ErrEncryption, Message: "password required", ExitCode: 3},
			contains: "encryption error",
		},
		{
			name:     "I/O error",
			err:      &PdftractError{Kind: ErrIO, Message: "failed to read file", ExitCode: 4},
			contains: "I/O error",
		},
		{
			name:     "invalid source",
			err:      &PdftractError{Kind: ErrInvalidSource, Message: "file not found", ExitCode: 5},
			contains: "invalid source",
		},
		{
			name:     "OCR failed",
			err:      &PdftractError{Kind: ErrOCRFailed, Message: "tesseract not available", ExitCode: 6},
			contains: "OCR failed",
		},
		{
			name:     "out of memory",
			err:      &PdftractError{Kind: ErrOutOfMemory, Message: "cannot allocate 500MB buffer", ExitCode: 7},
			contains: "out of memory",
		},
		{
			name:     "pagination error",
			err:      &PdftractError{Kind: ErrPagination, Message: "page count mismatch", ExitCode: 8},
			contains: "pagination error",
		},
		{
			name:     "unknown error",
			err:      &PdftractError{Kind: ErrUnknown, Message: "unexpected failure", ExitCode: 1},
			contains: "unknown error",
		},
		{
			name:     "error without message",
			err:      &PdftractError{Kind: ErrCorruptPDF, ExitCode: 2},
			contains: "corrupt PDF",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			errMsg := tt.err.Error()
			if errMsg == "" {
				t.Errorf("Error() returned empty string")
			}
			// Check that it contains the kind description
			if !contains(errMsg, tt.contains) {
				t.Errorf("Error() = %q, want to contain %q", errMsg, tt.contains)
			}
			// Check that it contains the exit code
			if !contains(errMsg, "exit") {
				t.Errorf("Error() = %q, want to contain exit code", errMsg)
			}
		})
	}
}

func TestPdftractErrorIs(t *testing.T) {
	tests := []struct {
		name     string
		err      *PdftractError
		target   *PdftractError
		expected bool
	}{
		{
			name:     "same kind matches",
			err:      &PdftractError{Kind: ErrCorruptPDF, ExitCode: 2},
			target:   &PdftractError{Kind: ErrCorruptPDF, ExitCode: 2},
			expected: true,
		},
		{
			name:     "different kind does not match",
			err:      &PdftractError{Kind: ErrCorruptPDF, ExitCode: 2},
			target:   &PdftractError{Kind: ErrEncryption, ExitCode: 3},
			expected: false,
		},
		{
			name:     "nil target does not match",
			err:      &PdftractError{Kind: ErrCorruptPDF, ExitCode: 2},
			target:   nil,
			expected: false,
		},
		{
			name:     "matching predefined instance",
			err:      ErrCorruptPDFInstance,
			target:   &PdftractError{Kind: ErrCorruptPDF, ExitCode: 2},
			expected: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := tt.err.Is(tt.target)
			if result != tt.expected {
				t.Errorf("Is() = %v, want %v", result, tt.expected)
			}
		})
	}
}

func TestErrorsIs(t *testing.T) {
	tests := []struct {
		name     string
		err      error
		target   error
		expected bool
	}{
		{
			name:     "matching ErrCorruptPDF",
			err:      &PdftractError{Kind: ErrCorruptPDF, ExitCode: 2},
			target:   ErrCorruptPDFInstance,
			expected: true,
		},
		{
			name:     "matching ErrEncryption",
			err:      &PdftractError{Kind: ErrEncryption, ExitCode: 3},
			target:   ErrEncryptionInstance,
			expected: true,
		},
		{
			name:     "matching ErrIO",
			err:      &PdftractError{Kind: ErrIO, ExitCode: 4},
			target:   ErrIOInstance,
			expected: true,
		},
		{
			name:     "matching ErrInvalidSource",
			err:      &PdftractError{Kind: ErrInvalidSource, ExitCode: 5},
			target:   ErrInvalidSourceInstance,
			expected: true,
		},
		{
			name:     "matching ErrOCRFailed",
			err:      &PdftractError{Kind: ErrOCRFailed, ExitCode: 6},
			target:   ErrOCRFailedInstance,
			expected: true,
		},
		{
			name:     "matching ErrOutOfMemory",
			err:      &PdftractError{Kind: ErrOutOfMemory, ExitCode: 7},
			target:   ErrOutOfMemoryInstance,
			expected: true,
		},
		{
			name:     "matching ErrPagination",
			err:      &PdftractError{Kind: ErrPagination, ExitCode: 8},
			target:   ErrPaginationInstance,
			expected: true,
		},
		{
			name:     "matching ErrUnknown",
			err:      &PdftractError{Kind: ErrUnknown, ExitCode: 1},
			target:   ErrUnknownInstance,
			expected: true,
		},
		{
			name:     "non-matching kind",
			err:      &PdftractError{Kind: ErrCorruptPDF, ExitCode: 2},
			target:   ErrEncryptionInstance,
			expected: false,
		},
		{
			name:     "standard error does not match",
			err:      errors.New("standard error"),
			target:   ErrCorruptPDFInstance,
			expected: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := errors.Is(tt.err, tt.target)
			if result != tt.expected {
				t.Errorf("errors.Is() = %v, want %v", result, tt.expected)
			}
		})
	}
}

func TestErrorsAs(t *testing.T) {
	tests := []struct {
		name        string
		err         error
		expectFound bool
		expectKind  ErrKind
	}{
		{
			name:        "extract PdftractError",
			err:         &PdftractError{Kind: ErrCorruptPDF, Message: "test", ExitCode: 2},
			expectFound: true,
			expectKind:  ErrCorruptPDF,
		},
		{
			name:        "extract predefined instance",
			err:         ErrCorruptPDFInstance,
			expectFound: true,
			expectKind:  ErrCorruptPDF,
		},
		{
			name:        "standard error does not extract",
			err:         errors.New("standard error"),
			expectFound: false,
			expectKind:  0,
		},
		{
			name:        "nil error",
			err:         nil,
			expectFound: false,
			expectKind:  0,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			var pdftractErr *PdftractError
			found := errors.As(tt.err, &pdftractErr)

			if found != tt.expectFound {
				t.Errorf("errors.As() found = %v, want %v", found, tt.expectFound)
			}

			if tt.expectFound {
				if pdftractErr.Kind != tt.expectKind {
					t.Errorf("errors.As() kind = %v, want %v", pdftractErr.Kind, tt.expectKind)
				}
			} else {
				if pdftractErr != nil {
					t.Errorf("errors.As() returned non-nil when not found: %v", pdftractErr)
				}
			}
		})
	}
}

func TestAsPdftractError(t *testing.T) {
	tests := []struct {
		name        string
		err         error
		expectFound bool
		expectKind  ErrKind
	}{
		{
			name:        "extract PdftractError",
			err:         &PdftractError{Kind: ErrCorruptPDF, Message: "test", ExitCode: 2},
			expectFound: true,
			expectKind:  ErrCorruptPDF,
		},
		{
			name:        "extract predefined instance",
			err:         ErrCorruptPDFInstance,
			expectFound: true,
			expectKind:  ErrCorruptPDF,
		},
		{
			name:        "standard error does not extract",
			err:         errors.New("standard error"),
			expectFound: false,
			expectKind:  0,
		},
		{
			name:        "nil error",
			err:         nil,
			expectFound: false,
			expectKind:  0,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			pdftractErr, found := AsPdftractError(tt.err)

			if found != tt.expectFound {
				t.Errorf("AsPdftractError() found = %v, want %v", found, tt.expectFound)
			}

			if tt.expectFound {
				if pdftractErr.Kind != tt.expectKind {
					t.Errorf("AsPdftractError() kind = %v, want %v", pdftractErr.Kind, tt.expectKind)
				}
			} else {
				if pdftractErr != nil {
					t.Errorf("AsPdftractError() returned non-nil when not found: %v", pdftractErr)
				}
			}
		})
	}
}

func TestNewPdftractError(t *testing.T) {
	tests := []struct {
		name            string
		exitCode        int
		message         string
		expectedKind    ErrKind
		expectedMessage string
	}{
		{
			name:            "exit code 2 -> ErrCorruptPDF",
			exitCode:        2,
			message:         "corrupt file",
			expectedKind:    ErrCorruptPDF,
			expectedMessage: "corrupt file",
		},
		{
			name:            "exit code 3 -> ErrEncryption",
			exitCode:        3,
			message:         "need password",
			expectedKind:    ErrEncryption,
			expectedMessage: "need password",
		},
		{
			name:            "exit code 4 -> ErrIO",
			exitCode:        4,
			message:         "read failed",
			expectedKind:    ErrIO,
			expectedMessage: "read failed",
		},
		{
			name:            "exit code 5 -> ErrInvalidSource",
			exitCode:        5,
			message:         "not found",
			expectedKind:    ErrInvalidSource,
			expectedMessage: "not found",
		},
		{
			name:            "exit code 6 -> ErrOCRFailed",
			exitCode:        6,
			message:         "ocr error",
			expectedKind:    ErrOCRFailed,
			expectedMessage: "ocr error",
		},
		{
			name:            "exit code 7 -> ErrOutOfMemory",
			exitCode:        7,
			message:         "oom",
			expectedKind:    ErrOutOfMemory,
			expectedMessage: "oom",
		},
		{
			name:            "exit code 8 -> ErrPagination",
			exitCode:        8,
			message:         "page error",
			expectedKind:    ErrPagination,
			expectedMessage: "page error",
		},
		{
			name:            "unknown exit code -> ErrUnknown",
			exitCode:        99,
			message:         "weird error",
			expectedKind:    ErrUnknown,
			expectedMessage: "weird error",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := NewPdftractError(tt.exitCode, tt.message)

			if err.Kind != tt.expectedKind {
				t.Errorf("NewPdftractError() Kind = %v, want %v", err.Kind, tt.expectedKind)
			}

			if err.Message != tt.expectedMessage {
				t.Errorf("NewPdftractError() Message = %q, want %q", err.Message, tt.expectedMessage)
			}

			if err.ExitCode != tt.exitCode {
				t.Errorf("NewPdftractError() ExitCode = %d, want %d", err.ExitCode, tt.exitCode)
			}
		})
	}
}

func TestMapExitCodeToErrKind(t *testing.T) {
	tests := []struct {
		name            string
		exitCode        int
		expectedKind    ErrKind
	}{
		{
			name:         "exit code 2 -> ErrCorruptPDF",
			exitCode:     2,
			expectedKind: ErrCorruptPDF,
		},
		{
			name:         "exit code 3 -> ErrEncryption",
			exitCode:     3,
			expectedKind: ErrEncryption,
		},
		{
			name:         "exit code 4 -> ErrIO",
			exitCode:     4,
			expectedKind: ErrIO,
		},
		{
			name:         "exit code 5 -> ErrInvalidSource",
			exitCode:     5,
			expectedKind: ErrInvalidSource,
		},
		{
			name:         "exit code 6 -> ErrOCRFailed",
			exitCode:     6,
			expectedKind: ErrOCRFailed,
		},
		{
			name:         "exit code 7 -> ErrOutOfMemory",
			exitCode:     7,
			expectedKind: ErrOutOfMemory,
		},
		{
			name:         "exit code 8 -> ErrPagination",
			exitCode:     8,
			expectedKind: ErrPagination,
		},
		{
			name:         "unknown exit code -> ErrUnknown",
			exitCode:     99,
			expectedKind: ErrUnknown,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			kind := mapExitCodeToErrKind(tt.exitCode)

			if kind != tt.expectedKind {
				t.Errorf("mapExitCodeToErrKind() = %v, want %v", kind, tt.expectedKind)
			}
		})
	}
}

// Helper function to check if a string contains a substring
func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(substr) == 0 ||
		(len(s) > 0 && len(substr) > 0 && containsHelper(s, substr)))
}

func containsHelper(s, substr string) bool {
	for i := 0; i <= len(s)-len(substr); i++ {
		if s[i:i+len(substr)] == substr {
			return true
		}
	}
	return false
}
