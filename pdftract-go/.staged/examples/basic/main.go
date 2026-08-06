package main

import (
	"context"
	"fmt"
	"log"
	"os"

	"github.com/jedarden/pdftract-go"
)

func main() {
	if len(os.Args) < 2 {
		fmt.Fprintf(os.Stderr, "Usage: %s <pdf-file>\n", os.Args[0])
		os.Exit(1)
	}

	// Create a client (searches PATH for pdftract binary)
	client, err := pdftract.NewClient("")
	if err != nil {
		log.Fatalf("Failed to create client: %v", err)
	}

	ctx := context.Background()
	source := pdftract.FileSource(os.Args[1])

	// Extract metadata
	meta, err := client.GetMetadata(ctx, source, nil)
	if err != nil {
		log.Fatalf("Failed to get metadata: %v", err)
	}

	fmt.Printf("Title: %s\n", meta.Title)
	fmt.Printf("Author: %s\n", meta.Author)
	fmt.Printf("Page count: %d\n", meta.PageCount)

	// Extract full document
	doc, err := client.Extract(ctx, source, &pdftract.ExtractOptions{
		OCRLanguage:  "eng",
		OCRThreshold: 0.7,
	})
	if err != nil {
		log.Fatalf("Failed to extract: %v", err)
	}

	fmt.Printf("Schema version: %s\n", doc.SchemaVersion)
	fmt.Printf("Pages: %d\n", len(doc.Pages))

	// Print first page info
	if len(doc.Pages) > 0 {
		page := doc.Pages[0]
		fmt.Printf("Page 1: %dx%d, rotation=%d\n",
			int(page.Width), int(page.Height), page.Rotation)
		fmt.Printf("  Spans: %d, Blocks: %d\n", len(page.Spans), len(page.Blocks))
	}
}
