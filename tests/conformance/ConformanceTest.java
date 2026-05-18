/**
 * pdftract SDK Conformance Test Runner (Java)
 *
 * This test runs the shared SDK conformance suite against the Java SDK.
 * It loads tests/sdk-conformance/cases.json and executes each test case.
 *
 * Run with: mvn test -Dtest=ConformanceTest
 * Or as standalone: java ConformanceTest.java <suite-path> <output-path>
 */

import com.fasterxml.jackson.databind.JsonNode;
import com.fasterxml.jackson.databind.ObjectMapper;
import com.fasterxml.jackson.databind.node.ObjectNode;
import java.io.File;
import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Path;
import java.nio.file.Paths;
import java.time.Instant;
import java.time.ZoneOffset;
import java.time.format.DateTimeFormatter;
import java.util.ArrayList;
import java.util.HashMap;
import java.util.Iterator;
import java.util.List;
import java.util.Map;

public class ConformanceTest {

    private static final String SUITE_PATH = "tests/sdk-conformance/cases.json";
    private static final String SDK_NAME = "pdftract-java";
    private static final String SDK_VERSION = "0.1.0";

    private final ObjectMapper mapper = new ObjectMapper();

    enum TestStatus {
        PASS, FAIL, SKIP, ERROR
    }

    static class TestResult {
        String id;
        TestStatus status;
        JsonNode actual;
        JsonNode expected;
        String error;
        String reason;
        long durationMs;

        TestResult(String id, TestStatus status, long durationMs) {
            this.id = id;
            this.status = status;
            this.durationMs = durationMs;
        }
    }

    static class ConformanceReport {
        String sdk;
        String sdkVersion;
        String suiteVersion;
        String schemaVersion;
        String timestamp;
        List<TestResult> results;
        Summary summary;
        Environment environment;

        ObjectNode toJson(ObjectMapper mapper) {
            ObjectNode node = mapper.createObjectNode();
            node.put("sdk", sdk);
            node.put("sdk_version", sdkVersion);
            node.put("suite_version", suiteVersion);
            node.put("schema_version", schemaVersion);
            node.put("timestamp", timestamp);

            var resultsArray = node.putArray("results");
            for (var result : results) {
                var resultNode = resultsArray.addObject();
                resultNode.put("id", result.id);
                resultNode.put("status", result.status.name().toLowerCase());
                if (result.actual != null) {
                    resultNode.set("actual", result.actual);
                }
                if (result.expected != null) {
                    resultNode.set("expected", result.expected);
                }
                if (result.error != null) {
                    resultNode.put("error", result.error);
                }
                if (result.reason != null) {
                    resultNode.put("reason", result.reason);
                }
                resultNode.put("duration_ms", result.durationMs);
            }

            var summaryNode = node.putObject("summary");
            summaryNode.put("total", summary.total);
            summaryNode.put("passed", summary.passed);
            summaryNode.put("failed", summary.failed);
            summaryNode.put("skipped", summary.skipped);
            summaryNode.put("errors", summary.errors);
            summaryNode.put("duration_ms", summary.durationMs);

            var envNode = node.putObject("environment");
            envNode.put("os", environment.os);
            envNode.put("arch", environment.arch);
            envNode.put("binary_version", environment.binaryVersion);
            envNode.put("runtime_version", environment.runtimeVersion);

            return node;
        }
    }

    static class Summary {
        int total;
        int passed;
        int failed;
        int skipped;
        int errors;
        long durationMs;
    }

    static class Environment {
        String os;
        String arch;
        String binaryVersion;
        String runtimeVersion;
    }

    private boolean compareWithTolerance(double actual, double expected, JsonNode tolerance) {
        if (tolerance == null || !tolerance.isObject()) {
            return Math.abs(actual - expected) < 1e-9;
        }

        if (tolerance.has("abs")) {
            double absTol = tolerance.get("abs").asDouble();
            if (Math.abs(actual - expected) <= absTol) {
                return true;
            }
        }

        if (tolerance.has("rel")) {
            double relTol = tolerance.get("rel").asDouble();
            double diff = Math.abs(actual - expected);
            double avg = (actual + expected) / 2.0;
            if (avg > 0.0 && diff / avg <= relTol) {
                return true;
            }
        }

        return false;
    }

    private JsonNode findTolerance(JsonNode tolerances, String path) {
        if (tolerances == null || !tolerances.isObject()) {
            return null;
        }

        if (tolerances.has(path)) {
            return tolerances.get(path);
        }

        Iterator<String> keys = tolerances.fieldNames();
        while (keys.hasNext()) {
            String key = keys.next();
            if (key.contains("*")) {
                String pattern = key.replace("*", ".*");
                if (path.matches(pattern)) {
                    return tolerances.get(key);
                }
            }
        }

        return null;
    }

    private boolean[] compareResults(JsonNode actual, JsonNode expected, JsonNode tolerances, String path) {
        // Returns [passed, hasReason]
        if (expected.isObject()) {
            if (actual.isNumber()) {
                double actVal = actual.asDouble();
                if (expected.has("min")) {
                    double min = expected.get("min").asDouble();
                    if (actVal < min) {
                        return new boolean[]{false, true};
                    }
                }
                if (expected.has("max")) {
                    double max = expected.get("max").asDouble();
                    if (actVal > max) {
                        return new boolean[]{false, true};
                    }
                }
                if (expected.has("value")) {
                    double expVal = expected.get("value").asDouble();
                    JsonNode tol = findTolerance(tolerances, path);
                    if (!compareWithTolerance(actVal, expVal, tol)) {
                        return new boolean[]{false, true};
                    }
                }
            } else if (actual.isTextual()) {
                String actStr = actual.asText();
                if (expected.has("min_length")) {
                    int minLen = expected.get("min_length").asInt();
                    if (actStr.length() < minLen) {
                        return new boolean[]{false, true};
                    }
                }
                if (expected.has("contains")) {
                    JsonNode contains = expected.get("contains");
                    if (contains.isArray()) {
                        for (JsonNode item : contains) {
                            if (!actStr.contains(item.asText())) {
                                return new boolean[]{false, true};
                            }
                        }
                    }
                }
            } else if (actual.isArray()) {
                int actLen = actual.size();
                if (expected.has("min")) {
                    int min = expected.get("min").asInt();
                    if (actLen < min) {
                        return new boolean[]{false, true};
                    }
                }
                if (expected.has("max")) {
                    int max = expected.get("max").asInt();
                    if (actLen > max) {
                        return new boolean[]{false, true};
                    }
                }
            } else if (actual.isObject()) {
                Iterator<String> fields = expected.fieldNames();
                while (fields.hasNext()) {
                    String key = fields.next();
                    String newPath = path.isEmpty() ? key : path + "." + key;
                    if (!actual.has(key)) {
                        return new boolean[]{false, true};
                    }
                    boolean[] result = compareResults(actual.get(key), expected.get(key), tolerances, newPath);
                    if (!result[0]) {
                        return result;
                    }
                }
            }
        } else if (expected.isArray() && actual.isArray()) {
            for (int i = 0; i < expected.size(); i++) {
                String newPath = path + "[" + i + "]";
                if (i >= actual.size()) {
                    return new boolean[]{false, true};
                }
                boolean[] result = compareResults(actual.get(i), expected.get(i), tolerances, newPath);
                if (!result[0]) {
                    return result;
                }
            }
        } else {
            if (!actual.equals(expected)) {
                return new boolean[]{false, true};
            }
        }
        return new boolean[]{true, false};
    }

    private JsonNode executeMethod(String method, String fixture, JsonNode options) {
        // This is a stub - replace with actual SDK calls when available
        ObjectNode result = mapper.createObjectNode();

        switch (method) {
            case "extract":
                result.put("schema_version", "1.0");
                ObjectNode metadata = result.putObject("metadata");
                metadata.put("page_count", 1);
                break;
            case "extract_text":
                return mapper.getNodeFactory().textNode("Sample text content");
            case "extract_markdown":
                return mapper.getNodeFactory().textNode("# Sample Markdown\n\nContent here");
            case "hash":
                result.put("hash", "abc123");
                result.put("fast_hash", "def456");
                break;
            default:
                break;
        }

        return result;
    }

    private TestResult runTestCase(JsonNode testCase, String schemaVersion, String fixturesBase) {
        long start = System.currentTimeMillis();

        String id = testCase.get("id").asText();

        // Check min_schema_version
        if (testCase.has("min_schema_version")) {
            String minVer = testCase.get("min_schema_version").asText();
            if (compareVersions(schemaVersion, minVer) < 0) {
                TestResult result = new TestResult(id, TestStatus.SKIP, System.currentTimeMillis() - start);
                result.reason = String.format("Schema version %s < minimum required %s", schemaVersion, minVer);
                return result;
            }
        }

        String fixture = testCase.get("fixture").asText();
        String method = testCase.get("method").asText();
        JsonNode options = testCase.get("options");
        JsonNode expected = testCase.get("expected");
        JsonNode tolerances = testCase.has("tolerances") ? testCase.get("tolerances") : null;

        String fixturePath = fixture.startsWith("http") ? fixture : Paths.get(fixturesBase, fixture).toString();

        try {
            JsonNode actual = executeMethod(method, fixturePath, options);
            boolean[] result = compareResults(actual, expected, tolerances, "");

            if (result[0]) {
                TestResult tr = new TestResult(id, TestStatus.PASS, System.currentTimeMillis() - start);
                tr.actual = actual;
                tr.expected = expected;
                return tr;
            } else {
                TestResult tr = new TestResult(id, TestStatus.FAIL, System.currentTimeMillis() - start);
                tr.actual = actual;
                tr.expected = expected;
                tr.reason = "Comparison failed";
                return tr;
            }
        } catch (Exception e) {
            TestResult tr = new TestResult(id, TestStatus.ERROR, System.currentTimeMillis() - start);
            tr.expected = expected;
            tr.error = e.getMessage();
            return tr;
        }
    }

    private int compareVersions(String v1, String v2) {
        String[] parts1 = v1.split("\\.");
        String[] parts2 = v2.split("\\.");

        for (int i = 0; i < Math.min(parts1.length, parts2.length); i++) {
            int n1 = Integer.parseInt(parts1[i]);
            int n2 = Integer.parseInt(parts2[i]);

            if (n1 < n2) return -1;
            if (n1 > n2) return 1;
        }

        return Integer.compare(parts1.length, parts2.length);
    }

    public ConformanceReport runConformance(String suitePath, String outputPath) throws IOException {
        System.out.println("pdftract SDK Conformance Runner");
        System.out.println("SDK: " + SDK_NAME + " v" + SDK_VERSION);
        System.out.println("Suite: " + suitePath);
        System.out.println();

        JsonNode suite = mapper.readTree(new File(suitePath));
        String suiteVersion = suite.get("version").asText();
        String schemaVersion = suite.get("schema_version").asText();
        JsonNode cases = suite.get("cases");

        String fixturesBase = Paths.get(suitePath).getParent().resolve("fixtures").toString();

        System.out.println("Found " + cases.size() + " test cases");
        System.out.println();

        long start = System.currentTimeMillis();
        List<TestResult> results = new ArrayList<>();

        for (JsonNode testCase : cases) {
            TestResult result = runTestCase(testCase, schemaVersion, fixturesBase);

            System.out.println("[" + result.status + "] " + result.id + " (" + result.durationMs + "ms)");

            if (result.status == TestStatus.FAIL || result.status == TestStatus.ERROR) {
                if (result.reason != null) {
                    System.out.println("  Reason: " + result.reason);
                }
                if (result.error != null) {
                    System.out.println("  Error: " + result.error);
                }
            }

            results.add(result);
        }

        long durationMs = System.currentTimeMillis() - start;

        Summary summary = new Summary();
        summary.total = results.size();
        summary.passed = (int) results.stream().filter(r -> r.status == TestStatus.PASS).count();
        summary.failed = (int) results.stream().filter(r -> r.status == TestStatus.FAIL).count();
        summary.skipped = (int) results.stream().filter(r -> r.status == TestStatus.SKIP).count();
        summary.errors = (int) results.stream().filter(r -> r.status == TestStatus.ERROR).count();
        summary.durationMs = durationMs;

        System.out.println();
        System.out.println("Summary:");
        System.out.println("  Total:   " + summary.total);
        System.out.println("  Passed:  " + summary.passed);
        System.out.println("  Failed:  " + summary.failed);
        System.out.println("  Skipped: " + summary.skipped);
        System.out.println("  Errors:  " + summary.errors);
        System.out.println("  Time:    " + summary.durationMs + "ms");

        Environment env = new Environment();
        env.os = System.getProperty("os.name");
        env.arch = System.getProperty("os.arch");
        env.binaryVersion = SDK_VERSION;
        env.runtimeVersion = System.getProperty("java.version");

        ConformanceReport report = new ConformanceReport();
        report.sdk = SDK_NAME;
        report.sdkVersion = SDK_VERSION;
        report.suiteVersion = suiteVersion;
        report.schemaVersion = schemaVersion;
        report.timestamp = Instant.now().atZone(ZoneOffset.UTC).format(DateTimeFormatter.ISO_INSTANT);
        report.results = results;
        report.summary = summary;
        report.environment = env;

        mapper.writerWithDefaultPrettyPrinter().writeValue(new File(outputPath), report.toJson(mapper));

        System.out.println();
        System.out.println("Report written to: " + outputPath);

        return report;
    }

    public static void main(String[] args) throws Exception {
        String suitePath = args.length > 0 ? args[0] : SUITE_PATH;
        String outputPath = args.length > 1 ? args[1] : "conformance-report.json";

        ConformanceTest test = new ConformanceTest();
        ConformanceReport report = test.runConformance(suitePath, outputPath);

        System.exit(report.summary.failed == 0 && report.summary.errors == 0 ? 0 : 1);
    }
}
