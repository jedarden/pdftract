package pdftract

// Source represents a PDF source (file path, URL, or raw bytes).
// The discriminator pattern allows type-switching to determine the source type.
type Source interface {
	isSource()
	sourceType() string
	value() any
}

// PathSource represents a local filesystem path.
type PathSource string

func (p PathSource) isSource()           {}
func (p PathSource) sourceType() string  { return "path" }
func (p PathSource) value() any          { return string(p) }

// URLSource represents a remote URL.
type URLSource string

func (u URLSource) isSource()           {}
func (u URLSource) sourceType() string   { return "url" }
func (u URLSource) value() any           { return string(u) }

// BytesSource represents in-memory PDF bytes.
type BytesSource []byte

func (b BytesSource) isSource()           {}
func (b BytesSource) sourceType() string  { return "bytes" }
func (b BytesSource) value() any          { return []byte(b) }

// cleanup removes the temporary file created for BytesSource.
// This is a no-op if no temp file was created.
func (b *BytesSource) cleanup() {
	// BytesSource may write to a temp file in the future
	// For now, this is a no-op placeholder
}
