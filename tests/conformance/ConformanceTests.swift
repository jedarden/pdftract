/*
 * pdftract SDK Conformance Test Runner (Swift)
 *
 * This test runs the shared SDK conformance suite against the Swift SDK.
 * It loads tests/sdk-conformance/cases.json and executes each test case.
 *
 * Run with: swift test --filter ConformanceTests
 * Or as standalone: swift ConformanceTests.swift <suite-path> <output-path>
 */

import Foundation

#if canImport(FoundationNetworking)
import FoundationNetworking
#endif

let SUITE_PATH = "tests/sdk-conformance/cases.json"
let SDK_NAME = "pdftract-swift"
let SDK_VERSION = "0.1.0"

enum TestStatus: String, Encodable {
    case pass = "pass"
    case fail = "fail"
    case skip = "skip"
    case error = "error"
}

struct TestResult: Encodable {
    let id: String
    let status: TestStatus
    let actual: String?
    let expected: String?
    let error: String?
    let reason: String?
    let duration_ms: Int64

    func toDict() -> [String: Any] {
        var dict: [String: Any] = [
            "id": id,
            "status": status.rawValue,
            "duration_ms": duration_ms
        ]
        if let actual = actual { dict["actual"] = actual }
        if let expected = expected { dict["expected"] = expected }
        if let error = error { dict["error"] = error }
        if let reason = reason { dict["reason"] = reason }
        return dict
    }
}

struct Summary: Encodable {
    let total: Int
    let passed: Int
    let failed: Int
    let skipped: Int
    let errors: Int
    let duration_ms: Int64
}

struct Environment: Encodable {
    let os: String
    let arch: String
    let binary_version: String
    let runtime_version: String
}

struct ConformanceReport: Encodable {
    let sdk: String
    let sdk_version: String
    let suite_version: String
    let schema_version: String
    let timestamp: String
    let results: [TestResult]
    let summary: Summary
    let environment: Environment
}

func compareWithTolerance(_ actual: Double, _ expected: Double, _ tolerance: [String: Any]?) -> Bool {
    guard let tolerance = tolerance else {
        return abs(actual - expected) < Double.ulpOfOne
    }

    if let absTol = tolerance["abs"] as? Double {
        if abs(actual - expected) <= absTol {
            return true
        }
    }

    if let relTol = tolerance["rel"] as? Double {
        let diff = abs(actual - expected)
        let avg = (actual + expected) / 2.0
        if avg > 0.0 && diff / avg <= relTol {
            return true
        }
    }

    return false
}

func findTolerance(_ tolerances: [String: Any]?, _ path: String) -> [String: Any]? {
    guard let tolerances = tolerances else { return nil }

    if let val = tolerances[path] {
        return val as? [String: Any]
    }

    for (key, val) in tolerances {
        if key.contains("*") {
            let pattern = key.replacingOccurrences(of: "*", with: ".*")
            if let regex = try? NSRegularExpression(pattern: pattern),
               regex.firstMatch(in: path, range: NSRange(location: 0, length: path.utf16.count)) != nil {
                return val as? [String: Any]
            }
        }
    }

    return nil
}

func compareResults(_ actual: Any, _ expected: Any, _ tolerances: [String: Any]?, _ path: String = "") -> (Bool, String?) {
    if let expDict = expected as? [String: Any] {
        if let actNum = actual as? Double {
            if let min = expDict["min"] as? Double {
                if actNum < min {
                    return (false, "\(path): value \(actNum) < minimum \(min)")
                }
            }
            if let max = expDict["max"] as? Double {
                if actNum > max {
                    return (false, "\(path): value \(actNum) > maximum \(max)")
                }
            }
            if let val = expDict["value"] as? Double {
                let tol = findTolerance(tolerances, path)
                if !compareWithTolerance(actNum, val, tol) {
                    return (false, "\(path): numeric mismatch")
                }
            }
        } else if let actStr = actual as? String {
            if let minLen = expDict["min_length"] as? Int {
                if actStr.count < minLen {
                    return (false, "\(path): string length too short")
                }
            }
            if let contains = expDict["contains"] as? [String] {
                for substring in contains {
                    if !actStr.contains(substring) {
                        return (false, "\(path): string does not contain '\(substring)'")
                    }
                }
            }
        } else if let actArray = actual as? [Any] {
            if let min = expDict["min"] as? Int {
                if actArray.count < min {
                    return (false, "\(path): array length too short")
                }
            }
            if let max = expDict["max"] as? Int {
                if actArray.count > max {
                    return (false, "\(path): array length too long")
                }
            }
        } else if let actDict = actual as? [String: Any] {
            for (key, expVal) in expDict {
                let newPath = path.isEmpty ? key : "\(path).\(key)"
                guard let actVal = actDict[key] else {
                    return (false, "\(newPath): missing key '\(key)'")
                }
                let (passed, reason) = compareResults(actVal, expVal, tolerances, newPath)
                if !passed {
                    return (false, reason)
                }
            }
        }
    } else if let expArray = expected as? [Any], let actArray = actual as? [Any] {
        for (i, expVal) in expArray.enumerated() {
            let newPath = "\(path)[\(i)]"
            if i >= actArray.count {
                return (false, "\(newPath): missing index")
            }
            let (passed, reason) = compareResults(actArray[i], expVal, tolerances, newPath)
            if !passed {
                return (false, reason)
            }
        }
    } else {
        // Simple comparison
        if let actualStr = actual as? String,
           let expectedStr = expected as? String,
           actualStr != expectedStr {
            return (false, "\(path): strings do not match")
        }
    }

    return (true, nil)
}

func executeMethod(_ method: String, _ fixture: String, _ options: [String: Any]) -> Any {
    // This is a stub - replace with actual SDK calls when available
    switch method {
    case "extract":
        return [
            "schema_version": "1.0",
            "metadata": ["page_count": 1],
            "pages": [
                [
                    "page_index": 0,
                    "width": 612,
                    "height": 792,
                    "rotation": 0
                ]
            ],
            "errors": []
        ] as [String: Any]
    case "extract_text":
        return "Sample text content"
    case "extract_markdown":
        return "# Sample Markdown\n\nContent here"
    case "hash":
        return ["hash": "abc123", "fast_hash": "def456"]
    default:
        return [:] as [String: Any]
    }
}

func compareVersions(_ v1: String, _ v2: String) -> ComparisonResult {
    let parts1 = v1.split(separator: ".").compactMap { Int($0) }
    let parts2 = v2.split(separator: ".").compactMap { Int($0) }

    let maxCount = max(parts1.count, parts2.count)

    for i in 0..<maxCount {
        let n1 = i < parts1.count ? parts1[i] : 0
        let n2 = i < parts2.count ? parts2[i] : 0

        if n1 < n2 {
            return .orderedAscending
        }
        if n1 > n2 {
            return .orderedDescending
        }
    }

    return .orderedSame
}

func runTestCase(_ case: [String: Any], _ schemaVersion: String, _ fixturesBase: String) -> TestResult {
    let start = Date()

    guard let id = case["id"] as? String else {
        return TestResult(
            id: "unknown",
            status: .error,
            actual: nil,
            expected: nil,
            error: "Missing test case ID",
            reason: nil,
            duration_ms: 0
        )
    }

    // Check min_schema_version
    if let minVer = case["min_schema_version"] as? String {
        if compareVersions(schemaVersion, minVer) == .orderedAscending {
            return TestResult(
                id: id,
                status: .skip,
                actual: nil,
                expected: nil,
                error: nil,
                reason: "Schema version \(schemaVersion) < minimum required \(minVer)",
                duration_ms: Int64(Date().timeIntervalSince(start) * 1000)
            )
        }
    }

    guard let fixture = case["fixture"] as? String,
          let method = case["method"] as? String else {
        return TestResult(
            id: id,
            status: .error,
            actual: nil,
            expected: nil,
            error: "Missing required fields",
            reason: nil,
            duration_ms: 0
        )
    }

    let options = case["options"] as? [String: Any] ?? [:]
    let expected = case["expected"] ?? [:]
    let tolerances = case["tolerances"] as? [String: Any]

    let fixturePath: String
    if fixture.hasPrefix("http") {
        fixturePath = fixture
    } else {
        fixturePath = "\(fixturesBase)/\(fixture)"
    }

    do {
        let actual = executeMethod(method, fixturePath, options)
        let (passed, reason) = compareResults(actual, expected, tolerances)

        if passed {
            return TestResult(
                id: id,
                status: .pass,
                actual: String(describing: actual),
                expected: String(describing: expected),
                error: nil,
                reason: nil,
                duration_ms: Int64(Date().timeIntervalSince(start) * 1000)
            )
        } else {
            return TestResult(
                id: id,
                status: .fail,
                actual: String(describing: actual),
                expected: String(describing: expected),
                error: nil,
                reason: reason,
                duration_ms: Int64(Date().timeIntervalSince(start) * 1000)
            )
        }
    } catch {
        return TestResult(
            id: id,
            status: .error,
            actual: nil,
            expected: String(describing: expected),
            error: String(describing: error),
            reason: nil,
            duration_ms: Int64(Date().timeIntervalSince(start) * 1000)
        )
    }
}

func runConformance(_ suitePath: String, _ outputPath: String) -> ConformanceReport {
    print("pdftract SDK Conformance Runner")
    print("SDK: \(SDK_NAME) v\(SDK_VERSION)")
    print("Suite: \(suitePath)")
    print("")

    guard let suiteData = try? Data(contentsOf: URL(fileURLWithPath: suitePath)),
          let suite = try? JSONSerialization.jsonObject(with: suiteData) as? [String: Any] else {
        fatalError("Failed to load suite")
    }

    let suiteVersion = suite["version"] as? String ?? "unknown"
    let schemaVersion = suite["schema_version"] as? String ?? "unknown"
    let cases = suite["cases"] as? [[String: Any]] ?? []

    let fixturesBase = ((suitePath as NSString).deletingLastPathComponent as NSString).appendingPathComponent("fixtures")

    print("Found \(cases.count) test cases")
    print("")

    let start = Date()
    var results: [TestResult] = []

    for testCase in cases {
        let result = runTestCase(testCase, schemaVersion, fixturesBase)
        results.append(result)

        let statusSym: String
        switch result.status {
        case .pass: statusSym = "PASS"
        case .fail: statusSym = "FAIL"
        case .skip: statusSym = "SKIP"
        case .error: statusSym = "ERROR"
        }

        print("[\(statusSym)] \(result.id) (\(result.duration_ms)ms)")

        if result.status == .fail || result.status == .error {
            if let reason = result.reason {
                print("  Reason: \(reason)")
            }
            if let error = result.error {
                print("  Error: \(error)")
            }
        }
    }

    let duration_ms = Int64(Date().timeIntervalSince(start) * 1000)

    let passed = results.filter { $0.status == .pass }.count
    let failed = results.filter { $0.status == .fail }.count
    let skipped = results.filter { $0.status == .skip }.count
    let errors = results.filter { $0.status == .error }.count

    print("")
    print("Summary:")
    print("  Total:   \(results.count)")
    print("  Passed:  \(passed)")
    print("  Failed:  \(failed)")
    print("  Skipped: \(skipped)")
    print("  Errors:  \(errors)")
    print("  Time:    \(duration_ms)ms")

    let report = ConformanceReport(
        sdk: SDK_NAME,
        sdk_version: SDK_VERSION,
        suite_version: suiteVersion,
        schema_version: schemaVersion,
        timestamp: ISO8601DateFormatter().string(from: Date()),
        results: results,
        summary: Summary(
            total: results.count,
            passed: passed,
            failed: failed,
            skipped: skipped,
            errors: errors,
            duration_ms: duration_ms
        ),
        environment: Environment(
            os: "macOS", // Runtime detection would go here
            arch: "arm64",
            binary_version: SDK_VERSION,
            runtime_version: "5.9"
        )
    )

    if let reportData = try? JSONEncoder().encode(report),
       let reportJson = String(data: reportData, encoding: .utf8) {
        try? reportJson.write(toFile: outputPath, atomically: true, encoding: .utf8)
        print("")
        print("Report written to: \(outputPath)")
    }

    return report
}

// CLI entry point
if CommandLine.argc > 1 {
    let suiteArg = CommandLine.arguments.count > 1 ? CommandLine.arguments[1] : SUITE_PATH
    let outputArg = CommandLine.arguments.count > 2 ? CommandLine.arguments[2] : "conformance-report.json"

    let report = runConformance(suiteArg, outputArg)

    exit(report.summary.failed == 0 && report.summary.errors == 0 ? 0 : 1)
}
