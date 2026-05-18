// pdftract SDK Conformance Test Runner (Go)
//
// This test runs the shared SDK conformance suite against the Go SDK.
// It loads tests/sdk-conformance/cases.json and executes each test case.
//
// Run with: go test -v ./conformance_test.go
// Or as a standalone: go run conformance_test.go <suite-path> <output-path>

package main

import (
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"
)

const (
	SuitePath    = "tests/sdk-conformance/cases.json"
	SDKName      = "pdftract-go"
	SDKVersion   = "0.1.0"
)

type TestStatus string

const (
	StatusPass  TestStatus = "pass"
	StatusFail  TestStatus = "fail"
	StatusSkip  TestStatus = "skip"
	StatusError TestStatus = "error"
)

type TestResult struct {
	ID         string      `json:"id"`
	Status     TestStatus  `json:"status"`
	Actual     interface{} `json:"actual,omitempty"`
	Expected   interface{} `json:"expected,omitempty"`
	Error      string      `json:"error,omitempty"`
	Reason     string      `json:"reason,omitempty"`
	DurationMs int64       `json:"duration_ms"`
}

type Tolerance struct {
	Abs float64 `json:"abs,omitempty"`
	Rel float64 `json:"rel,omitempty"`
}

type Summary struct {
	Total     int   `json:"total"`
	Passed    int   `json:"passed"`
	Failed    int   `json:"failed"`
	Skipped   int   `json:"skipped"`
	Errors    int   `json:"errors"`
	DurationMs int64 `json:"duration_ms"`
}

type Environment struct {
	OS            string `json:"os"`
	Arch          string `json:"arch"`
	BinaryVersion string `json:"binary_version"`
	RuntimeVersion string `json:"runtime_version"`
}

type ConformanceReport struct {
	SDK          string                 `json:"sdk"`
	SDKVersion   string                 `json:"sdk_version"`
	SuiteVersion string                 `json:"suite_version"`
	SchemaVersion string                 `json:"schema_version"`
	Timestamp    string                 `json:"timestamp"`
	Results      []TestResult           `json:"results"`
	Summary      Summary                `json:"summary"`
	Environment  Environment            `json:"environment"`
}

type TestCase struct {
	ID               string              `json:"id"`
	Fixture          string              `json:"fixture"`
	Method           string              `json:"method"`
	Options          map[string]interface{} `json:"options"`
	Expected         interface{}         `json:"expected"`
	Tolerances       map[string]Tolerance `json:"tolerances,omitempty"`
	Feature          string              `json:"feature,omitempty"`
	MinSchemaVersion string              `json:"min_schema_version,omitempty"`
	SkipReason       string              `json:"skip_reason,omitempty"`
}

type TestSuite struct {
	Version       string     `json:"version"`
	SchemaVersion string     `json:"schema_version"`
	Cases         []TestCase `json:"cases"`
}

func loadSuite(path string) (*TestSuite, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read suite: %w", err)
	}

	var suite TestSuite
	if err := json.Unmarshal(data, &suite); err != nil {
		return nil, fmt.Errorf("failed to parse suite: %w", err)
	}

	return &suite, nil
}

func compareWithTolerance(actual, expected float64, tol *Tolerance) bool {
	if tol == nil {
		diff := actual - expected
		if diff < 0 {
			diff = -diff
		}
		return diff < 1e-9
	}

	if tol.Abs > 0 {
		diff := actual - expected
		if diff < 0 {
			diff = -diff
		}
		if diff <= tol.Abs {
			return true
		}
	}

	if tol.Rel > 0 {
		diff := actual - expected
		if diff < 0 {
			diff = -diff
		}
		avg := (actual + expected) / 2.0
		if avg > 0.0 && diff/avg <= tol.Rel {
			return true
		}
	}

	return false
}

func findTolerance(tolerances map[string]Tolerance, path string) *Tolerance {
	if tolerances == nil {
		return nil
	}

	if tol, ok := tolerances[path]; ok {
		return &tol
	}

	for key, val := range tolerances {
		if strings.Contains(key, "*") {
			pattern := strings.ReplaceAll(key, "*", ".*")
			if strings.HasPrefix(path, pattern) || strings.Contains(path, strings.TrimSuffix(pattern, ".*")) {
				return &val
			}
		}
	}

	return nil
}

func compareResults(actual, expected interface{}, tolerances map[string]Tolerance, path string) (bool, string) {
	// Handle min/max constraints
	switch exp := expected.(type) {
	case map[string]interface{}:
		switch act := actual.(type) {
		case float64:
			if min, ok := exp["min"].(float64); ok {
				if act < min {
					return false, fmt.Sprintf("%s: value %v < minimum %v", path, act, min)
				}
			}
			if max, ok := exp["max"].(float64); ok {
				if act > max {
					return false, fmt.Sprintf("%s: value %v > maximum %v", path, act, max)
				}
			}
			if val, ok := exp["value"].(float64); ok {
				tol := findTolerance(tolerances, path)
				if !compareWithTolerance(act, val, tol) {
					return false, fmt.Sprintf("%s: numeric mismatch", path)
				}
			}
		case string:
			if minLen, ok := exp["min_length"].(float64); ok {
				if float64(len(act)) < minLen {
					return false, fmt.Sprintf("%s: string length %d < minimum %v", path, len(act), minLen)
				}
			}
			if contains, ok := exp["contains"].([]interface{}); ok {
				for _, item := range contains {
					if substr, ok := item.(string); ok {
						if !strings.Contains(act, substr) {
							return false, fmt.Sprintf("%s: string does not contain '%s'", path, substr)
						}
					}
				}
			}
		case []interface{}:
			if min, ok := exp["min"].(float64); ok {
				if float64(len(act)) < min {
					return false, fmt.Sprintf("%s: array length %d < minimum %v", path, len(act), min)
				}
			}
			if max, ok := exp["max"].(float64); ok {
				if float64(len(act)) > max {
					return false, fmt.Sprintf("%s: array length %d > maximum %v", path, len(act), max)
				}
			}
		case map[string]interface{}:
			for key, expVal := range exp {
				newPath := path
				if path == "" {
					newPath = key
				} else {
					newPath = fmt.Sprintf("%s.%s", path, key)
				}

				actVal, ok := act[key]
				if !ok {
					return false, fmt.Sprintf("%s: missing key '%s'", newPath, key)
				}

				passed, reason := compareResults(actVal, expVal, tolerances, newPath)
				if !passed {
					return false, reason
				}
			}
		}
	case []interface{}:
		actArray, ok := actual.([]interface{})
		if !ok {
			return false, fmt.Sprintf("%s: expected array, got %T", path, actual)
		}
		for i, expVal := range exp {
			newPath := fmt.Sprintf("%s[%d]", path, i)
			if i >= len(actArray) {
				return false, fmt.Sprintf("%s: missing index", newPath)
			}
			passed, reason := compareResults(actArray[i], expVal, tolerances, newPath)
			if !passed {
				return false, reason
			}
		}
	default:
		if actual != expected {
			return false, fmt.Sprintf("%s: expected %v, got %v", path, expected, actual)
		}
	}

	return true, ""
}

func executeMethod(method, fixture string, options map[string]interface{}) (interface{}, error) {
	// This is a stub - replace with actual SDK calls when available
	switch method {
	case "extract":
		return map[string]interface{}{
			"schema_version": "1.0",
			"metadata": map[string]interface{}{
				"page_count": float64(1),
			},
			"pages": []interface{}{
				map[string]interface{}{
					"page_index": float64(0),
					"width":      float64(612),
					"height":     float64(792),
					"rotation":   float64(0),
				},
			},
			"errors": []interface{}{},
		}, nil
	case "extract_text":
		return "Sample text content", nil
	case "extract_markdown":
		return "# Sample Markdown\n\nContent here", nil
	case "extract_stream":
		return map[string]interface{}{
			"output_type":  "iterator",
			"frame_count":  float64(3),
		}, nil
	case "search":
		return map[string]interface{}{
			"output_type": "iterator",
			"matches": []interface{}{
				map[string]interface{}{
					"page": float64(0),
					"text": "found",
				},
			},
		}, nil
	case "get_metadata":
		return map[string]interface{}{
			"metadata": map[string]interface{}{
				"page_count": float64(1),
				"title":      "Test",
				"author":     "Test",
			},
		}, nil
	case "hash":
		return map[string]interface{}{
			"hash":       "abc123",
			"fast_hash":  "def456",
		}, nil
	case "classify":
		return map[string]interface{}{
			"category":   "scientific_paper",
			"confidence": 0.85,
			"tags":       []interface{}{"academic"},
		}, nil
	case "verify_receipt":
		return map[string]interface{}{
			"valid": true,
		}, nil
	default:
		return nil, nil
	}
}

func runTestCase(suite *TestSuite, case TestCase, fixturesBase string) TestResult {
	start := time.Now()

	// Check min_schema_version
	if case.MinSchemaVersion != "" {
		if compareVersions(suite.SchemaVersion, case.MinSchemaVersion) < 0 {
			return TestResult{
				ID:     case.ID,
				Status: StatusSkip,
				Reason: fmt.Sprintf("Schema version %s < minimum required %s", suite.SchemaVersion, case.MinSchemaVersion),
				DurationMs: time.Since(start).Milliseconds(),
			}
		}
	}

	var fixturePath string
	if strings.HasPrefix(case.Fixture, "http://") || strings.HasPrefix(case.Fixture, "https://") {
		fixturePath = case.Fixture
	} else {
		fixturePath = filepath.Join(fixturesBase, case.Fixture)
	}

	actual, err := executeMethod(case.Method, fixturePath, case.Options)
	if err != nil {
		return TestResult{
			ID:         case.ID,
			Status:     StatusError,
			Expected:   case.Expected,
			Error:      err.Error(),
			DurationMs: time.Since(start).Milliseconds(),
		}
	}

	passed, reason := compareResults(actual, case.Expected, case.Tolerances, "")
	if !passed {
		return TestResult{
			ID:       case.ID,
			Status:   StatusFail,
			Actual:   actual,
			Expected: case.Expected,
			Reason:   reason,
			DurationMs: time.Since(start).Milliseconds(),
		}
	}

	return TestResult{
		ID:       case.ID,
		Status:   StatusPass,
		Actual:   actual,
		Expected: case.Expected,
		DurationMs: time.Since(start).Milliseconds(),
	}
}

func compareVersions(v1, v2 string) int {
	// Simple version comparison (assumes "major.minor" format)
	parts1 := strings.Split(v1, ".")
	parts2 := strings.Split(v2, ".")

	for i := 0; i < len(parts1) && i < len(parts2); i++ {
		var n1, n2 int
		fmt.Sscanf(parts1[i], "%d", &n1)
		fmt.Sscanf(parts2[i], "%d", &n2)

		if n1 < n2 {
			return -1
		}
		if n1 > n2 {
			return 1
		}
	}

	if len(parts1) < len(parts2) {
		return -1
	}
	if len(parts1) > len(parts2) {
		return 1
	}
	return 0
}

func runConformance(suitePath, outputPath string) (*ConformanceReport, error) {
	fmt.Printf("pdftract SDK Conformance Runner\n")
	fmt.Printf("SDK: %s v%s\n", SDKName, SDKVersion)
	fmt.Printf("Suite: %s\n\n", suitePath)

	suite, err := loadSuite(suitePath)
	if err != nil {
		return nil, err
	}

	fixturesBase := filepath.Join(filepath.Dir(suitePath), "fixtures")
	fmt.Printf("Found %d test cases\n\n", len(suite.Cases))

	start := time.Now()
	results := make([]TestResult, 0, len(suite.Cases))

	for _, testCase := range suite.Cases {
		result := runTestCase(suite, testCase, fixturesBase)

		statusSym := map[TestStatus]string{
			StatusPass:  "PASS",
			StatusFail:  "FAIL",
			StatusSkip:  "SKIP",
			StatusError: "ERROR",
		}[result.Status]

		fmt.Printf("[%s] %s (%dms)\n", statusSym, result.ID, result.DurationMs)

		if result.Status == StatusFail || result.Status == StatusError {
			if result.Reason != "" {
				fmt.Printf("  Reason: %s\n", result.Reason)
			}
			if result.Error != "" {
				fmt.Printf("  Error: %s\n", result.Error)
			}
		}

		results = append(results, result)
	}

	durationMs := time.Since(start).Milliseconds()

	summary := Summary{
		Total:      len(results),
		Passed:     countStatus(results, StatusPass),
		Failed:     countStatus(results, StatusFail),
		Skipped:    countStatus(results, StatusSkip),
		Errors:     countStatus(results, StatusError),
		DurationMs: durationMs,
	}

	fmt.Println()
	fmt.Println("Summary:")
	fmt.Printf("  Total:   %d\n", summary.Total)
	fmt.Printf("  Passed:  %d\n", summary.Passed)
	fmt.Printf("  Failed:  %d\n", summary.Failed)
	fmt.Printf("  Skipped: %d\n", summary.Skipped)
	fmt.Printf("  Errors:  %d\n", summary.Errors)
	fmt.Printf("  Time:    %dms\n", summary.DurationMs)

	report := &ConformanceReport{
		SDK:          SDKName,
		SDKVersion:   SDKVersion,
		SuiteVersion: suite.Version,
		SchemaVersion: suite.SchemaVersion,
		Timestamp:    time.Now().UTC().Format(time.RFC3339),
		Results:      results,
		Summary:      summary,
		Environment: Environment{
			OS:            "linux", // Runtime detection would go here
			Arch:          "amd64",
			BinaryVersion: SDKVersion,
			RuntimeVersion: "go1.21",
		},
	}

	data, err := json.MarshalIndent(report, "", "  ")
	if err != nil {
		return nil, fmt.Errorf("failed to marshal report: %w", err)
	}

	if err := os.WriteFile(outputPath, data, 0644); err != nil {
		return nil, fmt.Errorf("failed to write report: %w", err)
	}

	fmt.Println()
	fmt.Printf("Report written to: %s\n", outputPath)

	return report, nil
}

func countStatus(results []TestResult, status TestStatus) int {
	count := 0
	for _, r := range results {
		if r.Status == status {
			count++
		}
	}
	return count
}

func main() {
	suitePath := SuitePath
	outputPath := "conformance-report.json"

	if len(os.Args) > 1 {
		suitePath = os.Args[1]
	}
	if len(os.Args) > 2 {
		outputPath = os.Args[2]
	}

	report, err := runConformance(suitePath, outputPath)
	if err != nil {
		fmt.Fprintf(os.Stderr, "Error: %v\n", err)
		os.Exit(1)
	}

	if report.Summary.Failed > 0 || report.Summary.Errors > 0 {
		os.Exit(1)
	}
}
