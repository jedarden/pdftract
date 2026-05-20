package pdftract

import (
	"fmt"
	"os"
	"path/filepath"
	"strings"
)

// Source represents a PDF source (file path, URL, or raw bytes).
type Source interface {
	source() []string
}

// PathSource represents a local filesystem path.
type PathSource string

func (p PathSource) source() []string {
	path := string(p)
	if !filepath.IsAbs(path) {
		abs, err := filepath.Abs(path)
		if err == nil {
			path = abs
		}
	}
	return []string{path}
}

// URLSource represents a remote URL.
type URLSource string

func (u URLSource) source() []string {
	return []string{"--url", string(u)}
}

// BytesSource represents in-memory PDF bytes.
// The temporary file created for subprocess consumption is cleaned up after use.
type BytesSource struct {
	data    []byte
	tmpPath string
}

// MemorySource is a convenience constructor that creates a BytesSource from a byte slice.
func MemorySource(data []byte) Source {
	return &BytesSource{data: data}
}

func (b *BytesSource) source() []string {
	if b.tmpPath != "" {
		return []string{b.tmpPath}
	}

	// Write to a temporary file for subprocess consumption
	tmpFile, err := os.CreateTemp("", "pdftract-*.pdf")
	if err != nil {
		panic(fmt.Sprintf("failed to create temp file for BytesSource: %v", err))
	}
	defer tmpFile.Close()

	if _, err := tmpFile.Write(b.data); err != nil {
		panic(fmt.Sprintf("failed to write data to temp file: %v", err))
	}

	b.tmpPath = tmpFile.Name()
	return []string{b.tmpPath}
}

// cleanup removes the temporary file if it was created.
func (b *BytesSource) cleanup() {
	if b.tmpPath != "" && !strings.HasPrefix(b.tmpPath, "--error:") {
		os.Remove(b.tmpPath)
		b.tmpPath = ""
	}
}

// FileSource is a convenience constructor that creates a PathSource from a string.
func FileSource(path string) Source {
	return PathSource(path)
}

// RemoteSource is a convenience constructor that creates a URLSource from a string.
func RemoteSource(url string) Source {
	if !strings.HasPrefix(url, "http://") && !strings.HasPrefix(url, "https://") {
		panic(fmt.Sprintf("invalid URL: %s (must start with http:// or https://)", url))
	}
	return URLSource(url)
}


// ReadFileSource reads a file and returns a BytesSource.
func ReadFileSource(path string) (Source, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read file: %w", err)
	}
	return &BytesSource{data: data}, nil
}
