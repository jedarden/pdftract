package pdftract

import (
	"context"
	"encoding/json"
	"os"
	"path/filepath"
	"strings"
	"testing"
	"time"
)

// ConformanceTest represents a single test case from the conformance suite.
type ConformanceTest struct {
	ID        string                 `json:"id"`
	Fixture   string                 `json:"fixture"`
	Method    string                 `json:"method"`
	Options   map[string]interface{} `json:"options"`
	Expected  map[string]interface{} `json:"expected"`
	Tolerances map[string]interface{} `json:"tolerances"`
}

// ConformanceSuite represents the conformance test suite.
type ConformanceSuite struct {
	Version        string              `json:"version"`
	SchemaVersion  string              `json:"schema_version"`
	Cases          []ConformanceTest   `json:"cases"`
}

// TestConformance runs the SDK conformance suite.
func TestConformance(t *testing.T) {
	// Find the pdftract binary
	client, err := NewClient("")
	if err != nil {
		t.Skipf("pdftract binary not found: %v", err)
	}

	// Load the conformance suite
	suitePath := "../tests/sdk-conformance/cases.json"
	suiteData, err := os.ReadFile(suitePath)
	if err != nil {
		t.Fatalf("Failed to load conformance suite: %v", err)
	}

	var suite ConformanceSuite
	if err := json.Unmarshal(suiteData, &suite); err != nil {
		t.Fatalf("Failed to parse conformance suite: %v", err)
	}

	// Run each test case
	for _, tc := range suite.Cases {
		t.Run(tc.ID, func(t *testing.T) {
			runConformanceTest(t, client, tc)
		})
	}
}

func runConformanceTest(t *testing.T, client *Client, tc ConformanceTest) {
	ctx, cancel := context.WithTimeout(context.Background(), 120*time.Second)
	defer cancel()

	// Determine fixture path
	var fixturePath string
	if strings.HasPrefix(tc.Fixture, "http://") || strings.HasPrefix(tc.Fixture, "https://") {
		fixturePath = tc.Fixture
	} else {
		fixturePath = filepath.Join("../tests/sdk-conformance/fixtures", tc.Fixture)
	}

	// Check if fixture exists (skip if missing for optional features)
	if !strings.HasPrefix(fixturePath, "http") && !strings.HasPrefix(fixturePath, "https") {
		if _, err := os.Stat(fixturePath); os.IsNotExist(err) {
			t.Skipf("Fixture not found: %s", fixturePath)
		}
	}

	// Run the test based on method
	switch tc.Method {
	case "extract":
		testExtract(t, ctx, client, fixturePath, tc)
	case "extract_text":
		testExtractText(t, ctx, client, fixturePath, tc)
	case "extract_markdown":
		testExtractMarkdown(t, ctx, client, fixturePath, tc)
	case "extract_stream":
		testExtractStream(t, ctx, client, fixturePath, tc)
	case "search":
		testSearch(t, ctx, client, fixturePath, tc)
	case "get_metadata":
		testGetMetadata(t, ctx, client, fixturePath, tc)
	case "hash":
		testHash(t, ctx, client, fixturePath, tc)
	case "classify":
		testClassify(t, ctx, client, fixturePath, tc)
	case "verify_receipt":
		testVerifyReceipt(t, ctx, client, fixturePath, tc)
	default:
		t.Fatalf("Unknown method: %s", tc.Method)
	}
}

func getSource(fixturePath string) Source {
	if strings.HasPrefix(fixturePath, "http://") || strings.HasPrefix(fixturePath, "https://") {
		return RemoteSource(fixturePath)
	}
	return FileSource(fixturePath)
}

func getExtractOpts(opts map[string]interface{}) *ExtractOptions {
	if opts == nil {
		return nil
	}

	extractOpts := &ExtractOptions{}
	if v, ok := opts["password"].(string); ok {
		extractOpts.Password = v
	}
	if v, ok := opts["ocr_language"].(string); ok {
		extractOpts.OCRLanguage = v
	}
	if v, ok := opts["ocr_threshold"].(float64); ok {
		extractOpts.OCRThreshold = v
	}
	if v, ok := opts["preserve_layout"].(bool); ok {
		extractOpts.PreserveLayout = v
	}
	if v, ok := opts["extract_images"].(bool); ok {
		extractOpts.ExtractImages = v
	}
	if v, ok := opts["image_format"].(string); ok {
		extractOpts.ImageFormat = v
	}
	if v, ok := opts["min_image_size"].(float64); ok {
		extractOpts.MinImageSize = int(v)
	}
	return extractOpts
}

func getSearchOpts(opts map[string]interface{}) (*string, *SearchOptions) {
	if opts == nil {
		return nil, nil
	}

	searchOpts := &SearchOptions{}
	var pattern string
	if v, ok := opts["pattern"].(string); ok {
		pattern = v
	}
	if v, ok := opts["case_insensitive"].(bool); ok {
		searchOpts.CaseInsensitive = v
	}
	if v, ok := opts["regex"].(bool); ok {
		searchOpts.Regex = v
	}
	if v, ok := opts["whole_word"].(bool); ok {
		searchOpts.WholeWord = v
	}
	if v, ok := opts["max_results"].(float64); ok {
		maxResults := int(v)
		searchOpts.MaxResults = &maxResults
	}
	return &pattern, searchOpts
}

func testExtract(t *testing.T, ctx context.Context, client *Client, fixturePath string, tc ConformanceTest) {
	source := getSource(fixturePath)
	opts := getExtractOpts(tc.Options)

	doc, err := client.Extract(ctx, source, opts)
	if err != nil {
		if tc.Expected["errors.length"] != nil {
			// Error is expected
			return
		}
		t.Fatalf("Extract failed: %v", err)
	}

	// Check expectations
	checkExpected(t, doc, tc.Expected)
}

func testExtractText(t *testing.T, ctx context.Context, client *Client, fixturePath string, tc ConformanceTest) {
	source := getSource(fixturePath)
	opts := getExtractOpts(tc.Options)

	text, err := client.ExtractText(ctx, source, opts)
	if err != nil {
		t.Fatalf("ExtractText failed: %v", err)
	}

	// Check expectations
	if minLen, ok := tc.Expected["min_length"].(float64); ok {
		if len(text) < int(minLen) {
			t.Errorf("Text length %d < min %d", len(text), int(minLen))
		}
	}
	if contains, ok := tc.Expected["contains"].([]interface{}); ok {
		for _, c := range contains {
			if str, ok := c.(string); ok {
				if !strings.Contains(text, str) {
					t.Errorf("Text does not contain %q", str)
				}
			}
		}
	}
}

func testExtractMarkdown(t *testing.T, ctx context.Context, client *Client, fixturePath string, tc ConformanceTest) {
	source := getSource(fixturePath)
	opts := getExtractOpts(tc.Options)

	md, err := client.ExtractMarkdown(ctx, source, opts)
	if err != nil {
		t.Fatalf("ExtractMarkdown failed: %v", err)
	}

	// Check expectations
	if minLen, ok := tc.Expected["min_length"].(float64); ok {
		if len(md) < int(minLen) {
			t.Errorf("Markdown length %d < min %d", len(md), int(minLen))
		}
	}
	if contains, ok := tc.Expected["contains"].([]interface{}); ok {
		for _, c := range contains {
			if str, ok := c.(string); ok {
				if !strings.Contains(md, str) {
					t.Errorf("Markdown does not contain %q", str)
				}
			}
		}
	}
}

func testExtractStream(t *testing.T, ctx context.Context, client *Client, fixturePath string, tc ConformanceTest) {
	source := getSource(fixturePath)
	opts := getExtractOpts(tc.Options)

	resultChan, err := client.ExtractStream(ctx, source, opts)
	if err != nil {
		t.Fatalf("ExtractStream failed: %v", err)
	}

	pageCount := 0
	for result := range resultChan {
		if result.Err != nil {
			t.Fatalf("ExtractStream page error: %v", result.Err)
		}
		pageCount++
		if result.Page != nil {
			// Got a valid page
		}
	}

	// Check expectations
	if minPages, ok := tc.Expected["page_frames"].(map[string]interface{}); ok {
		if min, ok := minPages["min"].(float64); ok {
			if pageCount < int(min) {
				t.Errorf("Page count %d < min %d", pageCount, int(min))
			}
		}
	}
}

func testSearch(t *testing.T, ctx context.Context, client *Client, fixturePath string, tc ConformanceTest) {
	source := getSource(fixturePath)
	pattern, opts := getSearchOpts(tc.Options)

	resultChan, err := client.Search(ctx, source, *pattern, opts)
	if err != nil {
		t.Fatalf("Search failed: %v", err)
	}

	matchCount := 0
	for result := range resultChan {
		if result.Err != nil {
			t.Fatalf("Search match error: %v", result.Err)
		}
		matchCount++
	}

	// Check expectations
	if minMatches, ok := tc.Expected["min_matches"].(float64); ok {
		if matchCount < int(minMatches) {
			t.Errorf("Match count %d < min %d", matchCount, int(minMatches))
		}
	}
	if expectedCount, ok := tc.Expected["match_count"].(float64); ok {
		if matchCount != int(expectedCount) {
			t.Errorf("Match count %d != expected %d", matchCount, int(expectedCount))
		}
	}
}

func testGetMetadata(t *testing.T, ctx context.Context, client *Client, fixturePath string, tc ConformanceTest) {
	source := getSource(fixturePath)
	opts := getExtractOpts(tc.Options)

	meta, err := client.GetMetadata(ctx, source, opts)
	if err != nil {
		t.Fatalf("GetMetadata failed: %v", err)
	}

	// Check expectations
	checkExpected(t, meta, tc.Expected)
}

func testHash(t *testing.T, ctx context.Context, client *Client, fixturePath string, tc ConformanceTest) {
	source := getSource(fixturePath)

	fp, err := client.Hash(ctx, source, nil)
	if err != nil {
		t.Fatalf("Hash failed: %v", err)
	}

	// Check expectations
	if len(fp.Hash) != 64 {
		t.Errorf("Hash length %d != 64", len(fp.Hash))
	}
	if len(fp.FastHash) != 64 {
		t.Errorf("FastHash length %d != 64", len(fp.FastHash))
	}
}

func testClassify(t *testing.T, ctx context.Context, client *Client, fixturePath string, tc ConformanceTest) {
	source := getSource(fixturePath)

	cls, err := client.Classify(ctx, source)
	if err != nil {
		t.Fatalf("Classify failed: %v", err)
	}

	// Check expectations
	if expectedCat, ok := tc.Expected["category"].(string); ok {
		if cls.Category != expectedCat {
			t.Errorf("Category %q != expected %q", cls.Category, expectedCat)
		}
	}
	if minConf, ok := tc.Expected["confidence"].(map[string]interface{}); ok {
		if min, ok := minConf["min"].(float64); ok {
			if cls.Confidence < min {
				t.Errorf("Confidence %.2f < min %.2f", cls.Confidence, min)
			}
		}
	}
}

func testVerifyReceipt(t *testing.T, ctx context.Context, client *Client, fixturePath string, tc ConformanceTest) {
	// Receipt verification requires a separate receipt file
	// For now, skip this test
	t.Skip("Receipt verification not fully implemented")
}

func checkExpected(t *testing.T, result interface{}, expected map[string]interface{}) {
	// Helper to check expected values against the result
	// This is a simplified version - a full implementation would
	// handle nested path expressions like "metadata.page_count"

	switch v := result.(type) {
	case *Document:
		if pageCount, ok := expected["metadata.page_count"].(float64); ok {
			if v.Metadata.PageCount != int(pageCount) {
				t.Errorf("PageCount %d != expected %d", v.Metadata.PageCount, int(pageCount))
			}
		}
		if pagesLen, ok := expected["pages.length"].(float64); ok {
			if len(v.Pages) != int(pagesLen) {
				t.Errorf("Pages length %d != expected %d", len(v.Pages), int(pagesLen))
			}
		}
	case *Metadata:
		if pageCount, ok := expected["page_count"].(float64); ok {
			if v.PageCount != int(pageCount) {
				t.Errorf("PageCount %d != expected %d", v.PageCount, int(pageCount))
			}
		}
	}
}

// TestContextCancellation tests that context cancellation properly terminates the subprocess.
func TestContextCancellation(t *testing.T) {
	client, err := NewClient("")
	if err != nil {
		t.Skipf("pdftract binary not found: %v", err)
	}

	ctx, cancel := context.WithCancel(context.Background())
	cancel() // Cancel immediately

	source := FileSource("../tests/sdk-conformance/fixtures/scientific_paper/01.pdf")
	_, err = client.Extract(ctx, source, nil)
	if err == nil {
		t.Error("Expected error from cancelled context, got nil")
	}
	if err != context.Canceled && err != ctx.Err() {
		t.Logf("Got error (may be expected): %v", err)
	}
}
