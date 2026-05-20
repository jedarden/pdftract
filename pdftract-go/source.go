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
type BytesSource []byte

func (b BytesSource) source() []string {
	return []string{"--bytes-data", string(b)}
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

// MemorySource is a convenience constructor that creates a BytesSource from a byte slice.
func MemorySource(data []byte) Source {
	return BytesSource(data)
}

// ReadFileSource reads a file and returns a BytesSource.
func ReadFileSource(path string) (Source, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read file: %w", err)
	}
	return BytesSource(data), nil
}
