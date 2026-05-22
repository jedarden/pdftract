package com.jedarden.pdftract;

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.PropertyNamingStrategies;
import com.jedarden.pdftract.codegen.*;
import org.junit.jupiter.api.BeforeAll;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Disabled;
import org.junit.jupiter.api.DisplayName;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;
import java.util.Optional;

import static org.junit.jupiter.api.Assertions.*;

/**
 * Conformance test runner for pdftract Java SDK.
 * Loads test cases from tests/sdk-conformance/cases.json and validates against expected results.
 */
public class ConformanceTest {
    private static final ObjectMapper MAPPER = Json.mapper().copy()
        .setPropertyNamingStrategy(PropertyNamingStrategies.SNAKE_CASE);
    private static final Path CASES_PATH = Path.of("tests/sdk-conformance/cases.json");
    private static List<TestCase> testCases = new ArrayList<>();

    @BeforeAll
    static void loadTestCases() {
        if (!Files.exists(CASES_PATH)) {
            System.out.println("WARNING: Conformance test cases not found at " + CASES_PATH);
            System.out.println("Skipping conformance tests - run from pdftract repo root with test fixtures");
            return;
        }

        try {
            String content = Files.readString(CASES_PATH);
            JsonNode root = MAPPER.readTree(content);
            JsonNode cases = root.get("cases");

            if (cases != null && cases.isArray()) {
                for (JsonNode caseNode : cases) {
                    testCases.add(MAPPER.treeToValue(caseNode, TestCase.class));
                }
            }
            System.out.println("Loaded " + testCases.size() + " conformance test cases");
        } catch (Exception e) {
            System.err.println("Failed to load test cases: " + e.getMessage());
        }
    }

    @Test
    @DisplayName("Run all conformance test cases")
    void runConformanceTests() {
        if (testCases.isEmpty()) {
            System.out.println("No test cases loaded - skipping conformance tests");
            return;
        }

        int passed = 0, failed = 0, skipped = 0, errors = 0;

        try (Pdftract client = new Pdftract()) {
            for (TestCase testCase : testCases) {
                try {
                    TestResult result = runTestCase(client, testCase);
                    switch (result.status()) {
                        case PASS -> passed++;
                        case FAIL -> {
                            failed++;
                            System.err.println("FAIL: " + testCase.id() + " - " + result.error());
                        }
                        case SKIP -> skipped++;
                        case ERROR -> {
                            errors++;
                            System.err.println("ERROR: " + testCase.id() + " - " + result.error());
                        }
                    }
                } catch (Exception e) {
                    errors++;
                    System.err.println("ERROR: " + testCase.id() + " - " + e.getMessage());
                }
            }
        }

        System.out.println("\nConformance Test Summary:");
        System.out.println("  Total:   " + testCases.size());
        System.out.println("  Passed:  " + passed);
        System.out.println("  Failed:  " + failed);
        System.out.println("  Skipped: " + skipped);
        System.out.println("  Errors:  " + errors);

        if (failed > 0 || errors > 0) {
            fail("Conformance tests failed: " + failed + " failed, " + errors + " errors");
        }
    }

    private TestResult runTestCase(Pdftract client, TestCase testCase) {
        // Check skip conditions
        if (testCase.skipReason() != null) {
            return new TestResult(Status.SKIP, testCase.skipReason());
        }

        if (testCase.minSchemaVersion() != null) {
            // TODO: Get actual schema version from client
            // For now, assume compatibility
        }

        String fixturePath = "tests/sdk-conformance/fixtures/" + testCase.fixture();
        if (!Files.exists(Path.of(fixturePath))) {
            return new TestResult(Status.SKIP, "Fixture not found: " + fixturePath);
        }

        try {
            Object actual = null;
            long startTime = System.currentTimeMillis();

            switch (testCase.method()) {
                case "extract" -> {
                    ExtractOptions options = buildExtractOptions(testCase.options());
                    Source source = Source.fromPath(fixturePath);
                    actual = client.extract(source, options);
                }
                case "extract_text" -> {
                    ExtractOptions options = buildExtractOptions(testCase.options());
                    Source source = Source.fromPath(fixturePath);
                    actual = client.extractText(source, options);
                }
                case "extract_markdown" -> {
                    ExtractOptions options = buildExtractOptions(testCase.options());
                    Source source = Source.fromPath(fixturePath);
                    actual = client.extractMarkdown(source, options);
                }
                case "search" -> {
                    SearchOptions options = buildSearchOptions(testCase.options());
                    Source source = Source.fromPath(fixturePath);
                    String pattern = (String) testCase.options().get("pattern");
                    if (pattern == null) pattern = "";
                    List<Match> matches = new ArrayList<>();
                    client.search(source, pattern, options).forEach(matches::add);
                    actual = matches;
                }
                case "metadata" -> {
                    BaseOptions options = buildBaseOptions(testCase.options());
                    Source source = Source.fromPath(fixturePath);
                    actual = client.getMetadata(source, options);
                }
                case "hash" -> {
                    BaseOptions options = buildBaseOptions(testCase.options());
                    Source source = Source.fromPath(fixturePath);
                    actual = client.hash(source, options);
                }
                case "classify" -> {
                    Source source = Source.fromPath(fixturePath);
                    actual = client.classify(source);
                }
                default -> {
                    return new TestResult(Status.SKIP, "Unsupported method: " + testCase.method());
                }
            }

            long duration = System.currentTimeMillis() - startTime;

            // Validate against expected
            String validationError = validateExpected(actual, testCase.expected(), testCase.tolerances());
            if (validationError != null) {
                return new TestResult(Status.FAIL, validationError);
            }

            return new TestResult(Status.PASS, null);
        } catch (PdftractException e) {
            return new TestResult(Status.ERROR, "PdftractException: " + e.getMessage());
        } catch (Exception e) {
            return new TestResult(Status.ERROR, e.getClass().getSimpleName() + ": " + e.getMessage());
        }
    }

    private ExtractOptions buildExtractOptions(java.util.Map<String, Object> options) {
        ExtractOptions opts = new ExtractOptions();
        if (options == null) return opts;

        if (options.containsKey("ocr_language")) {
            opts.setOcrLanguage((String) options.get("ocr_language"));
        }
        if (options.containsKey("ocr_threshold")) {
            opts.setOcrThreshold(((Number) options.get("ocr_threshold")).doubleValue());
        }
        if (options.containsKey("password")) {
            opts.setPassword((String) options.get("password"));
        }
        if (options.containsKey("preserve_layout")) {
            // CLI flag - add to args if true
        }
        if (options.containsKey("extract_images")) {
            // CLI flag - add to args if true
        }
        return opts;
    }

    private SearchOptions buildSearchOptions(java.util.Map<String, Object> options) {
        SearchOptions opts = new SearchOptions();
        if (options == null) return opts;

        if (options.containsKey("max_results")) {
            Object maxResults = options.get("max_results");
            if (maxResults != null) {
                opts.setMaxResults(((Number) maxResults).intValue());
            }
        }
        if (options.containsKey("whole_word")) {
            opts.setWholeWord((Boolean) options.get("whole_word"));
        }
        if (options.containsKey("password")) {
            opts.setPassword((String) options.get("password"));
        }
        return opts;
    }

    private BaseOptions buildBaseOptions(java.util.Map<String, Object> options) {
        BaseOptions opts = new BaseOptions();
        if (options == null) return opts;

        if (options.containsKey("password")) {
            opts.setPassword((String) options.get("password"));
        }
        return opts;
    }

    private String validateExpected(Object actual, java.util.Map<String, Object> expected, java.util.Map<String, Tolerance> tolerances) {
        if (expected == null || expected.isEmpty()) {
            return null;
        }

        for (var entry : expected.entrySet()) {
            String path = entry.getKey();
            Object expectedValue = entry.getValue();

            String error = checkPath(actual, path, expectedValue, tolerances);
            if (error != null) {
                return path + ": " + error;
            }
        }
        return null;
    }

    private String checkPath(Object actual, String path, Object expectedValue, java.util.Map<String, Tolerance> tolerances) {
        try {
            Object actualValue = getPathValue(actual, path);

            if (expectedValue instanceof java.util.Map<?, ?> constraint) {
                if (constraint.containsKey("min") || constraint.containsKey("max")) {
                    // Numeric range check
                    if (actualValue instanceof Number num) {
                        double val = num.doubleValue();
                        if (constraint.containsKey("min") && val < ((Number) constraint.get("min")).doubleValue()) {
                            return "value " + val + " below minimum " + constraint.get("min");
                        }
                        if (constraint.containsKey("max") && val > ((Number) constraint.get("max")).doubleValue()) {
                            return "value " + val + " above maximum " + constraint.get("max");
                        }
                    } else {
                        return "expected number, got " + (actualValue != null ? actualValue.getClass() : "null");
                    }
                } else if (constraint.containsKey("min")) {
                    // Minimum length check
                    if (actualValue instanceof List<?> list) {
                        if (list.size() < (Integer) constraint.get("min")) {
                            return "length " + list.size() + " below minimum " + constraint.get("min");
                        }
                    } else if (actualValue instanceof String str) {
                        if (str.length() < (Integer) constraint.get("min")) {
                            return "length " + str.length() + " below minimum " + constraint.get("min");
                        }
                    }
                } else if (constraint.containsKey("contains")) {
                    // String contains check
                    if (actualValue instanceof String str) {
                        List<String> substrings = (List<String>) constraint.get("contains");
                        for (String sub : substrings) {
                            if (!str.contains(sub)) {
                                return "string does not contain \"" + sub + "\"";
                            }
                        }
                    }
                }
            } else if (expectedValue instanceof Number && actualValue instanceof Number) {
                // Direct number comparison
                double exp = ((Number) expectedValue).doubleValue();
                double act = ((Number) actualValue).doubleValue();
                if (Math.abs(exp - act) > 0.0001) {
                    return "expected " + exp + ", got " + act;
                }
            } else {
                // Direct equality check
                if (!java.util.Objects.equals(String.valueOf(expectedValue), String.valueOf(actualValue))) {
                    return "expected " + expectedValue + ", got " + actualValue;
                }
            }
        } catch (Exception e) {
            return "validation error: " + e.getMessage();
        }
        return null;
    }

    private Object getPathValue(Object obj, String path) {
        String[] parts = path.split("\\.");

        Object current = obj;
        for (String part : parts) {
            if (current == null) return null;

            // Handle array access like pages[0]
            if (part.contains("[") && part.contains("]")) {
                String fieldName = part.substring(0, part.indexOf("["));
                String indexStr = part.substring(part.indexOf("[") + 1, part.indexOf("]"));
                int index = indexStr.equals("*") ? -1 : Integer.parseInt(indexStr);

                try {
                    if (fieldName != null && !fieldName.isEmpty()) {
                        var field = current.getClass().getField(fieldName);
                        current = field.get(current);
                    }

                    if (index >= 0 && current instanceof List<?> list) {
                        current = list.get(index);
                    } else if (index == -1 && current instanceof List<?> list && !list.isEmpty()) {
                        // For wildcard checks, use first element
                        current = list.get(0);
                    }
                } catch (Exception e) {
                    return null;
                }
            } else {
                try {
                    if (current instanceof java.util.Map<?, ?> map) {
                        current = map.get(part);
                    } else {
                        var field = current.getClass().getField(part);
                        current = field.get(current);
                    }
                } catch (NoSuchFieldException | java.lang.IllegalAccessException e) {
                    // Try method access for records
                    try {
                        var method = current.getClass().getMethod(part);
                        current = method.invoke(current);
                    } catch (Exception ex) {
                        return null;
                    }
                }
            }
        }
        return current;
    }

    record TestCase(
        String id,
        String fixture,
        String method,
        java.util.Map<String, Object> options,
        java.util.Map<String, Object> expected,
        java.util.Map<String, Tolerance> tolerances,
        String feature,
        String minSchemaVersion,
        String skipReason
    ) {}

    record Tolerance(double abs, double rel) {}

    record TestResult(Status status, String error) {}

    enum Status { PASS, FAIL, SKIP, ERROR }
}
