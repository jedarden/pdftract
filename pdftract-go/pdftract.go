package pdftract

import (
	"context"
	"fmt"
	"os"
	"os/exec"
)

// Client represents a pdftract SDK client.
type Client struct {
	binaryPath string
}

// NewClient creates a new Client with the pdftract binary at the given path.
// If path is empty, it searches for pdftract in PATH.
func NewClient(path string) (*Client, error) {
	binaryPath := path
	if binaryPath == "" {
		path, err := exec.LookPath("pdftract")
		if err != nil {
			return nil, fmt.Errorf("pdftract binary not found in PATH: %w", err)
		}
		binaryPath = path
	}

	// Verify the binary exists and is executable
	if _, err := os.Stat(binaryPath); err != nil {
		return nil, fmt.Errorf("pdftract binary not found at %s: %w", binaryPath, err)
	}

	return &Client{
		binaryPath: binaryPath,
	}, nil
}

// MustNewClient creates a new Client and panics if it fails.
// Useful for short-lived programs where the binary path is known.
func MustNewClient(path string) *Client {
	client, err := NewClient(path)
	if err != nil {
		panic(err)
	}
	return client
}

// Extract extracts structured data from a PDF.
func (c *Client) Extract(ctx context.Context, source Source, opts *ExtractOptions) (*Document, error) {
	args := []string{"extract", "--json"}
	args = append(args, source.source()...)

	if opts != nil {
		args = append(args, opts.toArgs()...)
	}

	var doc Document
	if err := c.invokeJSON(ctx, args, &doc); err != nil {
		return nil, err
	}

	return &doc, nil
}

// ExtractText extracts plain text from a PDF.
func (c *Client) ExtractText(ctx context.Context, source Source, opts *ExtractOptions) (string, error) {
	args := []string{"extract", "--text"}
	args = append(args, source.source()...)

	if opts != nil {
		args = append(args, opts.toArgs()...)
	}

	return c.invokeString(ctx, args)
}

// ExtractMarkdown extracts markdown-formatted text from a PDF.
func (c *Client) ExtractMarkdown(ctx context.Context, source Source, opts *ExtractOptions) (string, error) {
	args := []string{"extract", "--md"}
	args = append(args, source.source()...)

	if opts != nil {
		args = append(args, opts.toArgs()...)
	}

	return c.invokeString(ctx, args)
}

// ExtractStream extracts pages from a PDF as a stream.
func (c *Client) ExtractStream(ctx context.Context, source Source, opts *ExtractOptions) (<-chan PageResult, error) {
	return c.extractStream(ctx, source, opts)
}

// Search searches for a pattern in a PDF.
func (c *Client) Search(ctx context.Context, source Source, pattern string, opts *SearchOptions) (<-chan MatchResult, error) {
	return c.search(ctx, source, pattern, opts)
}

// GetMetadata extracts metadata from a PDF.
func (c *Client) GetMetadata(ctx context.Context, source Source, opts *ExtractOptions) (*Metadata, error) {
	args := []string{"extract", "--metadata-only"}
	args = append(args, source.source()...)

	if opts != nil {
		args = append(args, opts.toArgs()...)
	}

	var result struct {
		Metadata Metadata `json:"metadata"`
	}
	if err := c.invokeJSON(ctx, args, &result); err != nil {
		return nil, err
	}

	return &result.Metadata, nil
}

// Hash computes the fingerprint hash of a PDF.
func (c *Client) Hash(ctx context.Context, source Source, opts *HashOptions) (*Fingerprint, error) {
	args := []string{"hash"}
	args = append(args, source.source()...)

	if opts != nil {
		args = append(args, opts.toArgs()...)
	}

	var fp Fingerprint
	if err := c.invokeJSON(ctx, args, &fp); err != nil {
		return nil, err
	}

	return &fp, nil
}

// Classify classifies a PDF document.
func (c *Client) Classify(ctx context.Context, source Source) (*Classification, error) {
	args := []string{"classify"}
	args = append(args, source.source()...)

	var cls Classification
	if err := c.invokeJSON(ctx, args, &cls); err != nil {
		return nil, err
	}

	return &cls, nil
}

// VerifyReceipt verifies a cryptographic receipt for a PDF.
func (c *Client) VerifyReceipt(ctx context.Context, path string, receipt *Receipt) (bool, error) {
	receiptPath := path + ".receipt.json"

	// For now, we'll call the CLI with the receipt path
	// TODO: Implement proper receipt verification once the CLI supports it
	args := []string{"verify-receipt", path, receiptPath}

	_, err := c.invoke(ctx, args)
	if err != nil {
		if _, ok := err.(*ReceiptVerifyError); ok {
			// Receipt verification failed
			return false, nil
		}
		return false, err
	}

	return true, nil
}

// BinaryPath returns the path to the pdftract binary.
func (c *Client) BinaryPath() string {
	return c.binaryPath
}

// Version returns the pdftract binary version.
func (c *Client) Version(ctx context.Context) (string, error) {
	args := []string{"--version"}
	output, err := c.invoke(ctx, args)
	if err != nil {
		return "", err
	}
	return string(output), nil
}
