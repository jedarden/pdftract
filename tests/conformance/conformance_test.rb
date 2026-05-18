# frozen_string_literal: true

# pdftract SDK Conformance Test Runner (Ruby)
#
# This test runs the shared SDK conformance suite against the Ruby SDK.
# It loads tests/sdk-conformance/cases.json and executes each test case.
#
# Run with: ruby test/conformance/conformance_test.rb
# Or as a standalone: ruby tests/conformance/conformance_test.rb <suite-path> <output-path>

require 'json'
require 'fileutils'
require 'time'

SUITE_PATH = 'tests/sdk-conformance/cases.json'
SDK_NAME = 'pdftract-ruby'
SDK_VERSION = '0.1.0'

module ConformanceTest
  STATUS_PASS = 'pass'
  STATUS_FAIL = 'fail'
  STATUS_SKIP = 'skip'
  STATUS_ERROR = 'error'

  TestResult = Struct.new(
    :id,
    :status,
    :actual,
    :expected,
    :error,
    :reason,
    :duration_ms,
    keyword_init: true
  )

  class ConformanceReport
    attr_accessor :sdk, :sdk_version, :suite_version, :schema_version,
                  :timestamp, :results, :summary, :environment

    def to_h
      {
        sdk: @sdk,
        sdk_version: @sdk_version,
        suite_version: @suite_version,
        schema_version: @schema_version,
        timestamp: @timestamp,
        results: @results.map(&:to_h),
        summary: @summary.to_h,
        environment: @environment.to_h
      }
    end
  end

  Summary = Struct.new(:total, :passed, :failed, :skipped, :errors, :duration_ms, keyword_init: true)
  Environment = Struct.new(:os, :arch, :binary_version, :runtime_version, keyword_init: true)

  def self.compare_with_tolerance(actual, expected, tolerance)
    return (actual - expected).abs < Float::EPSILON unless tolerance

    if tolerance['abs']
      return true if (actual - expected).abs <= tolerance['abs']
    end

    if tolerance['rel']
      diff = (actual - expected).abs
      avg = (actual + expected) / 2.0
      return true if avg > 0.0 && diff / avg <= tolerance['rel']
    end

    false
  end

  def self.find_tolerance(tolerances, path)
    return nil unless tolerances

    return tolerances[path] if tolerances.key?(path)

    tolerances.each do |key, val|
      next unless key.include?('*')

      pattern = Regexp.new(key.gsub('*', '.*'))
      return val if path.match?(pattern)
    end

    nil
  end

  def self.compare_results(actual, expected, tolerances, path = '')
    case expected
    when Hash
      case actual
      when Numeric
        if expected.key?('min')
          return [false, "#{path}: value #{actual} < minimum #{expected['min']}"] if actual < expected['min']
        end
        if expected.key?('max')
          return [false, "#{path}: value #{actual} > maximum #{expected['max']}"] if actual > expected['max']
        end
        if expected.key?('value')
          tol = find_tolerance(tolerances, path)
          unless compare_with_tolerance(actual.to_f, expected['value'].to_f, tol)
            return [false, "#{path}: numeric mismatch"]
          end
        end
      when String
        if expected.key?('min_length')
          return [false, "#{path}: string length #{actual.length} < minimum #{expected['min_length']}"] if actual.length < expected['min_length']
        end
        if expected['contains']
          expected['contains'].each do |substring|
            return [false, "#{path}: string does not contain '#{substring}'"] unless actual.include?(substring)
          end
        end
      when Array
        if expected.key?('min')
          return [false, "#{path}: array length #{actual.length} < minimum #{expected['min']}"] if actual.length < expected['min']
        end
        if expected.key?('max')
          return [false, "#{path}: array length #{actual.length} > maximum #{expected['max']}"] if actual.length > expected['max']
        end
      when Hash
        expected.each do |key, exp_val|
          new_path = path.empty? ? key : "#{path}.#{key}"
          unless actual.key?(key)
            return [false, "#{new_path}: missing key '#{key}'"]
          end

          passed, reason = compare_results(actual[key], exp_val, tolerances, new_path)
          return [false, reason] unless passed
        end
      end
    when Array
      if actual.is_a?(Array)
        expected.each_with_index do |exp_val, i|
          new_path = "#{path}[#{i}]"
          return [false, "#{new_path}: missing index"] if i >= actual.length

          passed, reason = compare_results(actual[i], exp_val, tolerances, new_path)
          return [false, reason] unless passed
        end
      else
        return [false, "#{path}: expected array, got #{actual.class}"]
      end
    else
      return [false, "#{path}: expected #{expected.inspect}, got #{actual.inspect}"] unless actual == expected
    end

    [true, nil]
  end

  def self.execute_method(method, fixture, options)
    # This is a stub - replace with actual SDK calls when available
    case method
    when 'extract'
      {
        'schema_version' => '1.0',
        'metadata' => { 'page_count' => 1 },
        'pages' => [
          {
            'page_index' => 0,
            'width' => 612,
            'height' => 792,
            'rotation' => 0
          }
        ],
        'errors' => []
      }
    when 'extract_text'
      'Sample text content'
    when 'extract_markdown'
      '# Sample Markdown\n\nContent here'
    when 'hash'
      { 'hash' => 'abc123', 'fast_hash' => 'def456' }
    else
      nil
    end
  end

  def self.compare_versions(v1, v2)
    parts1 = v1.split('.').map(&:to_i)
    parts2 = v2.split('.').map(&:to_i)

    parts1.zip(parts2).each do |a, b|
      next if a.nil? || b.nil?
      return -1 if a < b
      return 1 if a > b
    end

    parts1.length <=> parts2.length
  end

  def self.run_test_case(test_case, schema_version, fixtures_base)
    start_time = Time.now

    id = test_case['id']

    # Check min_schema_version
    if test_case['min_schema_version']
      min_ver = test_case['min_schema_version']
      if compare_versions(schema_version, min_ver) < 0
        return TestResult.new(
          id: id,
          status: STATUS_SKIP,
          reason: "Schema version #{schema_version} < minimum required #{min_ver}",
          duration_ms: ((Time.now - start_time) * 1000).to_i
        )
      end
    end

    fixture = test_case['fixture']
    method = test_case['method']
    options = test_case['options'] || {}
    expected = test_case['expected'] || {}
    tolerances = test_case['tolerances']

    fixture_path = fixture.start_with?('http') ? fixture : File.join(fixtures_base, fixture)

    begin
      actual = execute_method(method, fixture_path, options)
      passed, reason = compare_results(actual, expected, tolerances)

      if passed
        TestResult.new(
          id: id,
          status: STATUS_PASS,
          actual: actual,
          expected: expected,
          duration_ms: ((Time.now - start_time) * 1000).to_i
        )
      else
        TestResult.new(
          id: id,
          status: STATUS_FAIL,
          actual: actual,
          expected: expected,
          reason: reason,
          duration_ms: ((Time.now - start_time) * 1000).to_i
        )
      end
    rescue => e
      TestResult.new(
        id: id,
        status: STATUS_ERROR,
        expected: expected,
        error: e.message,
        duration_ms: ((Time.now - start_time) * 1000).to_i
      )
    end
  end

  def self.run_conformance(suite_path: SUITE_PATH, output_path: 'conformance-report.json')
    puts 'pdftract SDK Conformance Runner'
    puts "SDK: #{SDK_NAME} v#{SDK_VERSION}"
    puts "Suite: #{suite_path}"
    puts ''

    suite = JSON.parse(File.read(suite_path))
    suite_version = suite['version']
    schema_version = suite['schema_version']
    cases = suite['cases']

    fixtures_base = File.join(File.dirname(suite_path), 'fixtures')

    puts "Found #{cases.length} test cases"
    puts ''

    start_time = Time.now
    results = []

    cases.each do |test_case|
      result = run_test_case(test_case, schema_version, fixtures_base)

      status_sym = case result.status
                   when STATUS_PASS then 'PASS'
                   when STATUS_FAIL then 'FAIL'
                   when STATUS_SKIP then 'SKIP'
                   when STATUS_ERROR then 'ERROR'
                   end

      puts "[#{status_sym}] #{result.id} (#{result.duration_ms}ms)"

      if result.status == STATUS_FAIL || result.status == STATUS_ERROR
        puts "  Reason: #{result.reason}" if result.reason
        puts "  Error: #{result.error}" if result.error
      end

      results << result
    end

    duration_ms = ((Time.now - start_time) * 1000).to_i

    summary = Summary.new(
      total: results.length,
      passed: results.count { |r| r.status == STATUS_PASS },
      failed: results.count { |r| r.status == STATUS_FAIL },
      skipped: results.count { |r| r.status == STATUS_SKIP },
      errors: results.count { |r| r.status == STATUS_ERROR },
      duration_ms: duration_ms
    )

    puts ''
    puts 'Summary:'
    puts "  Total:   #{summary.total}"
    puts "  Passed:  #{summary.passed}"
    puts "  Failed:  #{summary.failed}"
    puts "  Skipped: #{summary.skipped}"
    puts "  Errors:  #{summary.errors}"
    puts "  Time:    #{summary.duration_ms}ms"

    report = ConformanceReport.new
    report.sdk = SDK_NAME
    report.sdk_version = SDK_VERSION
    report.suite_version = suite_version
    report.schema_version = schema_version
    report.timestamp = Time.now.utc.iso8601
    report.results = results.map do |r|
      {
        id: r.id,
        status: r.status,
        actual: r.actual,
        expected: r.expected,
        error: r.error,
        reason: r.reason,
        duration_ms: r.duration_ms
      }
    end
    report.summary = summary
    report.environment = Environment.new(
      os: RbConfig::CONFIG['host_os'],
      arch: RbConfig::CONFIG['host_cpu'],
      binary_version: SDK_VERSION,
      runtime_version: RUBY_VERSION
    )

    File.write(output_path, JSON.pretty_generate(report.to_h))

    puts ''
    puts "Report written to: #{output_path}"

    report
  end
end

# CLI entry point
if __FILE__ == $PROGRAM_NAME
  suite_arg = ARGV[0]
  output_arg = ARGV[1]

  report = ConformanceTest.run_conformance(
    suite_path: suite_arg || SUITE_PATH,
    output_path: output_arg || 'conformance-report.json'
  )

  exit((report.summary.failed + report.summary.errors) > 0 ? 1 : 0)
end
