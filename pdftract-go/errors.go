package pdftract

import (
	"errors"
	"fmt"
)

// ErrKind represents the kind of pdftract error.
type ErrKind int

const (
	ErrKindUnknown ErrKind = iota
	ErrKindCorruptPdf
	ErrKindEncryption
	ErrKindSourceUnreachable
	ErrKindRemoteFetchInterrupted
	ErrKindTls
	ErrKindReceiptVerify
)

// PdftractError is the base error type for all pdftract errors.
type PdftractError struct {
	Kind     ErrKind
	Message  string
	ExitCode int
}

func (e *PdftractError) Error() string {
	if e.Message != "" {
		return fmt.Sprintf("pdftract error (exit %d): %s", e.ExitCode, e.Message)
	}
	return fmt.Sprintf("pdftract error (exit %d)", e.ExitCode)
}

// Is allows errors.Is to match error kinds.
func (e *PdftractError) Is(target error) bool {
	t, ok := target.(*PdftractError)
	if !ok {
		return false
	}
	return e.Kind == t.Kind
}

// CorruptPdfError represents a corrupt PDF error (exit code 2).
type CorruptPdfError struct {
	Message  string
	ExitCode int
}

func (e *CorruptPdfError) Error() string {
	if e.Message != "" {
		return fmt.Sprintf("corrupt PDF (exit %d): %s", e.ExitCode, e.Message)
	}
	return "corrupt PDF"
}

func (e *CorruptPdfError) Is(target error) bool {
	t, ok := target.(*CorruptPdfError)
	return ok && e.ExitCode == t.ExitCode
}

// EncryptionError represents an encryption error (exit code 3).
type EncryptionError struct {
	Message  string
	ExitCode int
}

func (e *EncryptionError) Error() string {
	if e.Message != "" {
		return fmt.Sprintf("encryption error (exit %d): %s", e.ExitCode, e.Message)
	}
	return "encryption error: password missing or incorrect"
}

func (e *EncryptionError) Is(target error) bool {
	t, ok := target.(*EncryptionError)
	return ok && e.ExitCode == t.ExitCode
}

// SourceUnreachableError represents a source unreadable error (exit code 4).
type SourceUnreachableError struct {
	Message  string
	ExitCode int
}

func (e *SourceUnreachableError) Error() string {
	if e.Message != "" {
		return fmt.Sprintf("source unreachable (exit %d): %s", e.ExitCode, e.Message)
	}
	return "source unreachable: file or URL cannot be read"
}

func (e *SourceUnreachableError) Is(target error) bool {
	t, ok := target.(*SourceUnreachableError)
	return ok && e.ExitCode == t.ExitCode
}

// RemoteFetchInterruptedError represents a network interruption error (exit code 5).
type RemoteFetchInterruptedError struct {
	Message  string
	ExitCode int
}

func (e *RemoteFetchInterruptedError) Error() string {
	if e.Message != "" {
		return fmt.Sprintf("remote fetch interrupted (exit %d): %s", e.ExitCode, e.Message)
	}
	return "remote fetch interrupted: network connection failed"
}

func (e *RemoteFetchInterruptedError) Is(target error) bool {
	t, ok := target.(*RemoteFetchInterruptedError)
	return ok && e.ExitCode == t.ExitCode
}

// TlsError represents a TLS/certificate error (exit code 6).
type TlsError struct {
	Message  string
	ExitCode int
}

func (e *TlsError) Error() string {
	if e.Message != "" {
		return fmt.Sprintf("TLS error (exit %d): %s", e.ExitCode, e.Message)
	}
	return "TLS error: certificate validation failed"
}

func (e *TlsError) Is(target error) bool {
	t, ok := target.(*TlsError)
	return ok && e.ExitCode == t.ExitCode
}

// ReceiptVerifyError represents a receipt verification failure (exit code 10).
type ReceiptVerifyError struct {
	Message  string
	ExitCode int
}

func (e *ReceiptVerifyError) Error() string {
	if e.Message != "" {
		return fmt.Sprintf("receipt verification failed (exit %d): %s", e.ExitCode, e.Message)
	}
	return "receipt verification failed"
}

func (e *ReceiptVerifyError) Is(target error) bool {
	t, ok := target.(*ReceiptVerifyError)
	return ok && e.ExitCode == t.ExitCode
}

// mapExitCodeToError converts CLI exit codes to Go error types.
func mapExitCodeToError(exitCode int, stderr string) error {
	msg := stderr
	if msg == "" {
		msg = "unknown error"
	}

	switch exitCode {
	case 2:
		return &CorruptPdfError{Message: msg, ExitCode: exitCode}
	case 3:
		return &EncryptionError{Message: msg, ExitCode: exitCode}
	case 4:
		return &SourceUnreachableError{Message: msg, ExitCode: exitCode}
	case 5:
		return &RemoteFetchInterruptedError{Message: msg, ExitCode: exitCode}
	case 6:
		return &TlsError{Message: msg, ExitCode: exitCode}
	case 10:
		return &ReceiptVerifyError{Message: msg, ExitCode: exitCode}
	default:
		return &PdftractError{Kind: ErrKindUnknown, Message: msg, ExitCode: exitCode}
	}
}

// As functions for errors.As matching

// AsCorruptPdfError returns the underlying *CorruptPdfError if present.
func AsCorruptPdfError(err error) (*CorruptPdfError, bool) {
	var corruptErr *CorruptPdfError
	if errors.As(err, &corruptErr) {
		return corruptErr, true
	}
	return nil, false
}

// AsEncryptionError returns the underlying *EncryptionError if present.
func AsEncryptionError(err error) (*EncryptionError, bool) {
	var encErr *EncryptionError
	if errors.As(err, &encErr) {
		return encErr, true
	}
	return nil, false
}

// AsSourceUnreachableError returns the underlying *SourceUnreachableError if present.
func AsSourceUnreachableError(err error) (*SourceUnreachableError, bool) {
	var srcErr *SourceUnreachableError
	if errors.As(err, &srcErr) {
		return srcErr, true
	}
	return nil, false
}

// AsRemoteFetchInterruptedError returns the underlying *RemoteFetchInterruptedError if present.
func AsRemoteFetchInterruptedError(err error) (*RemoteFetchInterruptedError, bool) {
	var fetchErr *RemoteFetchInterruptedError
	if errors.As(err, &fetchErr) {
		return fetchErr, true
	}
	return nil, false
}

// AsTlsError returns the underlying *TlsError if present.
func AsTlsError(err error) (*TlsError, bool) {
	var tlsErr *TlsError
	if errors.As(err, &tlsErr) {
		return tlsErr, true
	}
	return nil, false
}

// AsReceiptVerifyError returns the underlying *ReceiptVerifyError if present.
func AsReceiptVerifyError(err error) (*ReceiptVerifyError, bool) {
	var receiptErr *ReceiptVerifyError
	if errors.As(err, &receiptErr) {
		return receiptErr, true
	}
	return nil, false
}
