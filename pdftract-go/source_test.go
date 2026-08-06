package pdftract

import (
	"testing"
)

func TestSourceInterface(t *testing.T) {
	// Test PathSource
	ps := PathSource("test.pdf")
	if ps.sourceType() != "path" {
		t.Errorf("PathSource.sourceType() = %s, want 'path'", ps.sourceType())
	}
	if ps.value() != "test.pdf" {
		t.Errorf("PathSource.value() = %v, want 'test.pdf'", ps.value())
	}

	// Test URLSource
	us := URLSource("https://example.com/doc.pdf")
	if us.sourceType() != "url" {
		t.Errorf("URLSource.sourceType() = %s, want 'url'", us.sourceType())
	}
	if us.value() != "https://example.com/doc.pdf" {
		t.Errorf("URLSource.value() = %v, want 'https://example.com/doc.pdf'", us.value())
	}

	// Test BytesSource
	bs := BytesSource([]byte("%PDF-test"))
	if bs.sourceType() != "bytes" {
		t.Errorf("BytesSource.sourceType() = %s, want 'bytes'", bs.sourceType())
	}
	if len(bs.value().([]byte)) != 9 {
		t.Errorf("BytesSource.value() length = %d, want 9", len(bs.value().([]byte)))
	}
}

func TestSourceTypeSwitch(t *testing.T) {
	sources := []Source{
		PathSource("test.pdf"),
		URLSource("https://example.com/doc.pdf"),
		BytesSource([]byte("%PDF-test")),
	}

	for i, s := range sources {
		switch v := s.(type) {
		case PathSource:
			if v != "test.pdf" {
				t.Errorf("Source[%d] expected PathSource 'test.pdf', got %v", i, v)
			}
		case URLSource:
			if v != "https://example.com/doc.pdf" {
				t.Errorf("Source[%d] expected URLSource, got %v", i, v)
			}
		case BytesSource:
			if len(v) != 9 {
				t.Errorf("Source[%d] expected BytesSource with 9 bytes, got %d", i, len(v))
			}
		default:
			t.Errorf("Source[%d] has unknown type", i)
		}
	}
}
