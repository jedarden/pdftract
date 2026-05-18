// pdftract SDK Conformance Test Runner (.NET / C#)
//
// This test runs the shared SDK conformance suite against the .NET SDK.
// It loads tests/sdk-conformance/cases.json and executes each test case.
//
// Run with: dotnet test --filter ConformanceTests
// Or as standalone: dotnet run --project ConformanceTests.csproj

using System;
using System.Collections.Generic;
using System.Diagnostics;
using System.IO;
using System.Linq;
using System.Text.Json;
using System.Text.Json.Nodes;
using Xunit;
using Xunit.Abstractions;

namespace Pdftract.Tests
{
    public class ConformanceTests
    {
        private const string SuitePath = "tests/sdk-conformance/cases.json";
        private const string SdkName = "pdftract-dotnet";
        private const string SdkVersion = "0.1.0";

        private readonly ITestOutputHelper _output;

        public ConformanceTests(ITestOutputHelper output)
        {
            _output = output;
        }

        private enum TestStatus
        {
            Pass,
            Fail,
            Skip,
            Error
        }

        private class TestResult
        {
            public string Id { get; set; } = string.Empty;
            public TestStatus Status { get; set; }
            public JsonNode? Actual { get; set; }
            public JsonNode? Expected { get; set; }
            public string? Error { get; set; }
            public string? Reason { get; set; }
            public long DurationMs { get; set; }
        }

        private class ConformanceReport
        {
            public string Sdk { get; set; } = SdkName;
            public string SdkVersion { get; set; } = SdkVersion;
            public string SuiteVersion { get; set; } = string.Empty;
            public string SchemaVersion { get; set; } = string.Empty;
            public string Timestamp { get; set; } = DateTime.UtcNow.ToString("o");
            public List<TestResult> Results { get; set; } = new();
            public Summary Summary { get; set; } = new();
            public Environment Environment { get; set; } = new();
        }

        private class Summary
        {
            public int Total { get; set; }
            public int Passed { get; set; }
            public int Failed { get; set; }
            public int Skipped { get; set; }
            public int Errors { get; set; }
            public long DurationMs { get; set; }
        }

        private class Environment
        {
            public string Os { get; set; } = Environment.OSVersion.Platform.ToString();
            public string Arch { get; set; } = Environment.Is64BitProcess ? "x64" : "x86";
            public string BinaryVersion { get; set; } = SdkVersion;
            public string RuntimeVersion { get; set; } = Environment.Version.ToString();
        }

        private bool CompareWithTolerance(double actual, double expected, JsonObject? tolerance)
        {
            if (tolerance == null)
            {
                return Math.Abs(actual - expected) < 1e-9;
            }

            if (tolerance.TryGetValue("abs", out JsonNode? absNode) && absNode != null)
            {
                double absTol = absNode.GetValue<double>();
                if (Math.Abs(actual - expected) <= absTol)
                {
                    return true;
                }
            }

            if (tolerance.TryGetValue("rel", out JsonNode? relNode) && relNode != null)
            {
                double relTol = relNode.GetValue<double>();
                double diff = Math.Abs(actual - expected);
                double avg = (actual + expected) / 2.0;
                if (avg > 0.0 && diff / avg <= relTol)
                {
                    return true;
                }
            }

            return false;
        }

        private JsonObject? FindTolerance(JsonObject? tolerances, string path)
        {
            if (tolerances == null) return null;

            if (tolerances.TryGetValue(path, out JsonNode? value) && value != null)
            {
                return value.AsObject();
            }

            foreach (var kvp in tolerances)
            {
                if (kvp.Key.Contains('*'))
                {
                    var pattern = kvp.Key.Replace("*", ".*");
                    if (System.Text.RegularExpressions.Regex.IsMatch(path, pattern))
                    {
                        return kvp.Value.AsObject();
                    }
                }
            }

            return null;
        }

        private (bool Passed, string? Reason) CompareResults(
            JsonNode actual, JsonNode expected, JsonObject? tolerances, string path = "")
        {
            if (expected is JsonObject expObj)
            {
                if (actual is JsonValue actVal && actVal.TryGetValue(out double? actNum) && actNum != null)
                {
                    if (expObj.TryGetValue("min", out JsonNode? minNode) && minNode != null)
                    {
                        double min = minNode.GetValue<double>();
                        if (actNum.Value < min)
                        {
                            return (false, $"{path}: value {actNum} < minimum {min}");
                        }
                    }
                    if (expObj.TryGetValue("max", out JsonNode? maxNode) && maxNode != null)
                    {
                        double max = maxNode.GetValue<double>();
                        if (actNum.Value > max)
                        {
                            return (false, $"{path}: value {actNum} > maximum {max}");
                        }
                    }
                    if (expObj.TryGetValue("value", out JsonNode? valNode) && valNode != null)
                    {
                        double expVal = valNode.GetValue<double>();
                        var tol = FindTolerance(tolerances, path);
                        if (!CompareWithTolerance(actNum.Value, expVal, tol))
                        {
                            return (false, $"{path}: numeric mismatch");
                        }
                    }
                }
                else if (actual is JsonValue actStrVal && actStrVal.TryGetValue(out string? actStr) && actStr != null)
                {
                    if (expObj.TryGetValue("min_length", out JsonNode? minLenNode) && minLenNode != null)
                    {
                        int minLen = minLenNode.GetValue<int>();
                        if (actStr.Length < minLen)
                        {
                            return (false, $"{path}: string length {actStr.Length} < minimum {minLen}");
                        }
                    }
                    if (expObj.TryGetValue("contains", out JsonNode? containsNode) && containsNode != null)
                    {
                        var contains = containsNode.AsArray();
                        foreach (var item in contains)
                        {
                            if (item.TryGetValue(out string? substr) && substr != null && !actStr.Contains(substr))
                            {
                                return (false, $"{path}: string does not contain '{substr}'");
                            }
                        }
                    }
                }
                else if (actual is JsonArray actArr)
                {
                    if (expObj.TryGetValue("min", out JsonNode? minNode) && minNode != null)
                    {
                        int min = minNode.GetValue<int>();
                        if (actArr.Count < min)
                        {
                            return (false, $"{path}: array length {actArr.Count} < minimum {min}");
                        }
                    }
                    if (expObj.TryGetValue("max", out JsonNode? maxNode) && maxNode != null)
                    {
                        int max = maxNode.GetValue<int>();
                        if (actArr.Count > max)
                        {
                            return (false, $"{path}: array length {actArr.Count} > maximum {max}");
                        }
                    }
                }
                else if (actual is JsonObject actObj)
                {
                    foreach (var kvp in expObj)
                    {
                        var newPath = string.IsNullOrEmpty(path) ? kvp.Key : $"{path}.{kvp.Key}";
                        if (!actObj.TryGetValue(kvp.Key, out JsonNode? actValue))
                        {
                            return (false, $"{newPath}: missing key '{kvp.Key}'");
                        }
                        var (passed, reason) = CompareResults(actValue, kvp.Value!, tolerances, newPath);
                        if (!passed) return (false, reason);
                    }
                }
            }
            else if (expected is JsonArray expArr && actual is JsonArray actArr2)
            {
                for (int i = 0; i < expArr.Count; i++)
                {
                    var newPath = $"{path}[{i}]";
                    if (i >= actArr2.Count)
                    {
                        return (false, $"{newPath}: missing index");
                    }
                    var (passed, reason) = CompareResults(actArr2[i], expArr[i], tolerances, newPath);
                    if (!passed) return (false, reason);
                }
            }
            else
            {
                if (!JsonNode.DeepEquals(actual, expected))
                {
                    return (false, $"{path}: expected {expected.ToJsonString()}, got {actual.ToJsonString()}");
                }
            }

            return (true, null);
        }

        private JsonNode ExecuteMethod(string method, string fixture, JsonObject options)
        {
            // This is a stub - replace with actual SDK calls when available
            return method switch
            {
                "extract" => new JsonObject
                {
                    ["schema_version"] = "1.0",
                    ["metadata"] = new JsonObject { ["page_count"] = 1 },
                    ["pages"] = new JsonArray
                    {
                        new JsonObject
                        {
                            ["page_index"] = 0,
                            ["width"] = 612,
                            ["height"] = 792,
                            ["rotation"] = 0
                        }
                    },
                    ["errors"] = new JsonArray()
                },
                "extract_text" => new JsonValue("Sample text content"),
                "extract_markdown" => new JsonValue("# Sample Markdown\n\nContent here"),
                "hash" => new JsonObject { ["hash"] = "abc123", ["fast_hash"] = "def456" },
                _ => JsonValue.Create(null)
            };
        }

        private int CompareVersions(string v1, string v2)
        {
            var parts1 = v1.Split('.');
            var parts2 = v2.Split('.');

            for (int i = 0; i < Math.Min(parts1.Length, parts2.Length); i++)
            {
                if (int.TryParse(parts1[i], out int n1) && int.TryParse(parts2[i], out int n2))
                {
                    if (n1 < n2) return -1;
                    if (n1 > n2) return 1;
                }
            }

            return parts1.Length.CompareTo(parts2.Length);
        }

        private TestResult RunTestCase(JsonObject testCase, string schemaVersion, string fixturesBase)
        {
            var stopwatch = Stopwatch.StartNew();
            string id = testCase["id"].GetValue<string>();

            // Check min_schema_version
            if (testCase.TryGetValue("min_schema_version", out JsonNode? minVerNode) && minVerNode != null)
            {
                string minVer = minVerNode.GetValue<string>();
                if (CompareVersions(schemaVersion, minVer) < 0)
                {
                    stopwatch.Stop();
                    return new TestResult
                    {
                        Id = id,
                        Status = TestStatus.Skip,
                        Reason = $"Schema version {schemaVersion} < minimum required {minVer}",
                        DurationMs = stopwatch.ElapsedMilliseconds
                    };
                }
            }

            string fixture = testCase["fixture"].GetValue<string>();
            string method = testCase["method"].GetValue<string>();
            var options = testCase["options"].AsObject();
            var expected = testCase["expected"];
            var tolerances = testCase.TryGetValue("tolerances", out JsonNode? tol) ? tol.AsObject() : null;

            string fixturePath = fixture.StartsWith("http") ? fixture :
                Path.Combine(fixturesBase, fixture);

            try
            {
                var actual = ExecuteMethod(method, fixturePath, options);
                var (passed, reason) = CompareResults(actual, expected, tolerances);

                stopwatch.Stop();
                return new TestResult
                {
                    Id = id,
                    Status = passed ? TestStatus.Pass : TestStatus.Fail,
                    Actual = actual,
                    Expected = expected,
                    Reason = reason,
                    DurationMs = stopwatch.ElapsedMilliseconds
                };
            }
            catch (Exception ex)
            {
                stopwatch.Stop();
                return new TestResult
                {
                    Id = id,
                    Status = TestStatus.Error,
                    Expected = expected,
                    Error = ex.Message,
                    DurationMs = stopwatch.ElapsedMilliseconds
                };
            }
        }

        private ConformanceReport RunConformance(string suitePath, string outputPath)
        {
            _output.WriteLine($"pdftract SDK Conformance Runner");
            _output.WriteLine($"SDK: {SdkName} v{SdkVersion}");
            _output.WriteLine($"Suite: {suitePath}");
            _output.WriteLine("");

            var suiteJson = File.ReadAllText(suitePath);
            var suite = JsonNode.Parse(suiteJson)?.AsObject()
                ?? throw new InvalidOperationException("Failed to parse suite");

            string suiteVersion = suite["version"].GetValue<string>();
            string schemaVersion = suite["schema_version"].GetValue<string>();
            var cases = suite["cases"].AsArray();

            string fixturesBase = Path.Combine(Path.GetDirectoryName(suitePath) ?? "", "fixtures");

            _output.WriteLine($"Found {cases.Count} test cases");
            _output.WriteLine("");

            var stopwatch = Stopwatch.StartNew();
            var results = new List<TestResult>();

            foreach (var testCase in cases)
            {
                var result = RunTestCase(testCase!.AsObject(), schemaVersion, fixturesBase);

                _output.WriteLine($"[{result.Status}] {result.Id} ({result.DurationMs}ms)");

                if (result.Status == TestStatus.Fail || result.Status == TestStatus.Error)
                {
                    if (result.Reason != null) _output.WriteLine($"  Reason: {result.Reason}");
                    if (result.Error != null) _output.WriteLine($"  Error: {result.Error}");
                }

                results.Add(result);
            }

            stopwatch.Stop();

            var summary = new Summary
            {
                Total = results.Count,
                Passed = results.Count(r => r.Status == TestStatus.Pass),
                Failed = results.Count(r => r.Status == TestStatus.Fail),
                Skipped = results.Count(r => r.Status == TestStatus.Skip),
                Errors = results.Count(r => r.Status == TestStatus.Error),
                DurationMs = stopwatch.ElapsedMilliseconds
            };

            _output.WriteLine("");
            _output.WriteLine("Summary:");
            _output.WriteLine($"  Total:   {summary.Total}");
            _output.WriteLine($"  Passed:  {summary.Passed}");
            _output.WriteLine($"  Failed:  {summary.Failed}");
            _output.WriteLine($"  Skipped: {summary.Skipped}");
            _output.WriteLine($"  Errors:  {summary.Errors}");
            _output.WriteLine($"  Time:    {summary.DurationMs}ms");

            var report = new ConformanceReport
            {
                SuiteVersion = suiteVersion,
                SchemaVersion = schemaVersion,
                Timestamp = DateTime.UtcNow.ToString("o"),
                Results = results,
                Summary = summary,
                Environment = new Environment()
            };

            File.WriteAllText(outputPath, JsonSerializer.Serialize(report, new JsonSerializerOptions
            {
                WriteIndented = true
            }));

            _output.WriteLine("");
            _output.WriteLine($"Report written to: {outputPath}");

            return report;
        }

        [Fact]
        public void TestConformanceSuite()
        {
            var report = RunConformance(SuitePath, "conformance-report.json");
            Assert.Equal(0, report.Summary.Failed);
            Assert.Equal(0, report.Summary.Errors);
        }
    }
}
