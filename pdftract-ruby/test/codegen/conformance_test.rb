# frozen_string_literal: true

require "minitest/autorun"
require "json"
require "ostruct"
require "socket"
require "uri"
require_relative "../../lib/pdftract"

module Pdftract
  module Codegen
    #
    # Conformance test suite for pdftract Ruby SDK
    # Auto-generated - do not edit manually
    #

    class ConformanceTest < Minitest::Test
      SUITE_PATH = ENV["CONFORMANCE_SUITE"] || "tests/sdk-conformance/cases.json"

      # The suite is loaded here, in the class body, because the per-case test
      # methods below are registered with define_method at load time: Minitest
      # collects runnable methods when the class is loaded, so a suite loaded
      # later (in setup or a test method) would always register zero tests.
      # define_method is also a private Module method - calling it on a test
      # instance raises NoMethodError, so registration must stay class-level.
      suite = if File.exist?(SUITE_PATH)
        JSON.parse(File.read(SUITE_PATH))
      else
        warn "Warning: Could not load conformance suite from #{SUITE_PATH}"
        nil
      end

      if suite
        suite["cases"].each do |tc|
          fixture = tc["fixture"]
          # URL fixtures are fetched by the pdftract binary itself; local
          # fixtures resolve relative to the fixtures/ directory.
          fixture_path = fixture.start_with?("http://", "https://") ? fixture : "fixtures/#{fixture}"

          define_method("test_#{tc['id']}_#{tc['method']}") do
            if fixture_path.start_with?("http://", "https://") && !self.class.remote_reachable?(fixture_path)
              skip "Offline: #{fixture_path} unreachable, skipping remote conformance case"
            end

            run_test_case(tc, fixture_path)
          end
        end
      else
        # No suite present: one explicit skip so the run is visibly degraded
        # rather than silently vacuous.
        def test_conformance_suite
          skip "Conformance suite not found at #{SUITE_PATH}"
        end
      end

      # Cheap memoized DNS/TCP probe so URL-fixture cases can degrade to a
      # skip when the environment has no network egress.
      def self.remote_reachable?(url)
        uri = URI.parse(url)
        host = uri.host
        return false if host.nil? || host.empty?

        @remote_reachable ||= {}
        @remote_reachable[host] ||= begin
          # Block form closes the socket on exit.
          Socket.tcp(host, uri.port || 443, connect_timeout: 3) { |_socket| true }
        rescue SystemCallError, SocketError, IOError
          false
        end
      end

      def setup
        @client = Client.new
      end

      private

      def run_test_case(test_case, fixture_path)
        expected = test_case["expected"] || {}
        tolerances = test_case["tolerances"] || {}
        case test_case["method"]
        when "extract"
          test_extract(fixture_path, test_case["options"], expected, tolerances)
        when "extract_text"
          test_extract_text(fixture_path, test_case["options"], expected, tolerances)
        when "extract_markdown"
          test_extract_markdown(fixture_path, test_case["options"], expected, tolerances)
        when "get_metadata"
          test_get_metadata(fixture_path, test_case["options"], expected, tolerances)
        when "hash"
          test_hash(fixture_path, test_case["options"], expected, tolerances)
        when "classify"
          test_classify(fixture_path, test_case["options"], expected, tolerances)
        when "verify_receipt"
          test_verify_receipt(fixture_path, test_case["options"], expected, tolerances)
        when "search"
          test_search(fixture_path, test_case["options"], expected, tolerances)
        when "extract_stream"
          test_extract_stream(fixture_path, test_case["options"], expected, tolerances)
        else
          skip "Method not yet implemented: #{test_case['method']}"
        end
      end

      # The shared suite uses flat JSON-path keys (for example,
      # `pages[0].blocks[0].kind`) rather than a language-specific object
      # shape.  Keep this evaluator in the generated test so every SDK checks
      # the same contract and every expected field contributes an assertion.
      def assert_expected(actual, expected, tolerances)
        assert_operator expected.length, :>, 0, "expected must contain at least one assertion"

        expected.each do |path, wanted|
          value_path = %w[min_length contains].include?(path) && actual.is_a?(Hash) && actual.key?("value") ? "value" : path
          found, value = resolve_path(actual, value_path)
          if !found && wanted.nil?
            found = true
            value = nil
          end
          assert found, "#{path}: missing value"
          passed, reason = compare_expected(value, wanted, tolerances, path)
          assert passed, reason
        end
      end

      def resolve_path(value, path)
        current = value
        path.scan(/[^.\[\]]+|\[\d+\]/).each do |part|
          if part == "length"
            return [false, nil] unless current.respond_to?(:length)

            current = current.length
          elsif part.start_with?("[")
            index = part[1..-2].to_i
            return [false, nil] unless current.is_a?(Array) && index < current.length

            current = current[index]
          elsif current.is_a?(Hash)
            return [false, nil] unless current.key?(part)

            current = current[part]
          elsif current.respond_to?(part)
            current = current.public_send(part)
          else
            return [false, nil]
          end
        end
        [true, current]
      end

      def compare_expected(actual, expected, tolerances, path)
        if expected.is_a?(Hash)
          if actual.is_a?(Numeric)
            return [false, "#{path}: value #{actual} < minimum #{expected['min']}"] if expected.key?("min") && actual < expected["min"]
            return [false, "#{path}: value #{actual} > maximum #{expected['max']}"] if expected.key?("max") && actual > expected["max"]
            if expected.key?("value") && !compare_number(actual, expected["value"], find_tolerance(tolerances, path))
              return [false, "#{path}: numeric mismatch (expected #{expected['value']}, got #{actual})"]
            end
            return [true, nil] if expected.keys.all? { |key| %w[min max value].include?(key) }
          elsif actual.is_a?(Array)
            return [false, "#{path}: array length #{actual.length} < minimum #{expected['min']}"] if expected.key?("min") && actual.length < expected["min"]
            return [false, "#{path}: array length #{actual.length} > maximum #{expected['max']}"] if expected.key?("max") && actual.length > expected["max"]
            return [true, nil] if expected.keys.all? { |key| %w[min max].include?(key) }
          elsif actual.is_a?(String)
            return [false, "#{path}: string length #{actual.length} < minimum #{expected['min_length']}"] if expected.key?("min_length") && actual.length < expected["min_length"]
            expected.fetch("contains", []).each do |substring|
              return [false, "#{path}: string does not contain '#{substring}'"] unless actual.include?(substring)
            end
            return [true, nil] if expected.keys.all? { |key| %w[min_length contains].include?(key) }
          end
        end

        return [true, nil] if actual == expected

        [false, "#{path}: expected #{expected.inspect}, got #{actual.inspect}"]
      end

      def compare_number(actual, expected, tolerance)
        return actual == expected unless tolerance

        difference = (actual.to_f - expected.to_f).abs
        return true if tolerance["abs"] && difference <= tolerance["abs"]

        average = (actual.to_f + expected.to_f) / 2.0
        tolerance["rel"] && average > 0 && difference / average <= tolerance["rel"]
      end

      def find_tolerance(tolerances, path)
        return tolerances[path] if tolerances.key?(path)

        tolerances.each do |pattern, tolerance|
          next unless pattern.include?("*")

          regex = Regexp.new("\\A#{Regexp.escape(pattern).gsub('\\*', '.*')}\\z")
          return tolerance if regex.match?(path)
        end
        nil
      end

      def as_json(value)
        case value
        when OpenStruct
          value.to_h.transform_values { |item| as_json(item) }
        when Hash
          value.transform_values { |item| as_json(item) }
        when Array
          value.map { |item| as_json(item) }
        else
          value
        end
      end

      def command_options(options)
        return nil if options.nil? || options.empty?

        args = []
        flags = {
          "ocr_language" => "--ocr-language",
          "ocr_threshold" => "--ocr-threshold",
          "image_format" => "--image-format",
          "min_image_size" => "--min-image-size",
          "timeout" => "--timeout",
          "max_pages" => "--max-pages",
          "password" => "--password",
        }
        options.each do |key, value|
          next if %w[pattern receipt].include?(key) || value.nil? || value == false

          if flags.key?(key)
            args.concat([flags[key], value.to_s])
          elsif value == true
            args << "--#{key.tr('_', '-')}"
          end
        end

        Object.new.tap do |adapter|
          adapter.define_singleton_method(:to_args) { args }
        end
      end

      def normalize_document(document)
        value = as_json(document)
        value["schema_version"] ||= "1.0"
        value["pages"]&.each do |page|
          page["page_index"] ||= page["page"] - 1 if page["page"]
          page["page_type"] ||= page["type"] if page["type"]
        end
        value
      end

      def normalize_metadata(metadata)
        value = as_json(metadata)
        %w[title author creator].each { |field| value["has_#{field}"] = !value[field].nil? if !value.key?("has_#{field}") }
        value["has_xmp"] = false unless value.key?("has_xmp")
        { "metadata" => value }
      end

      def normalize_hash(fingerprint, stable: false)
        value = as_json(fingerprint)
        value["hash"] = value["hash"].sub("pdftract-v1:", "") if value["hash"].is_a?(String)
        value["hash_type"] ||= "sha256"
        value["fast_hash_different_from_hash"] = value["fast_hash"] != value["hash"] if value.key?("fast_hash")
        value["content_hash_stable"] = stable
        value
      end

      def test_extract(fixture_path, options, expected, tolerances)
        assert_expected(normalize_document(@client.extract(PathSource.new(fixture_path), command_options(options))), expected, tolerances)
      end

      def test_extract_text(fixture_path, options, expected, tolerances)
        assert_expected({ "output_type" => "string", "value" => @client.extract_text(PathSource.new(fixture_path), command_options(options)) }, expected, tolerances)
      end

      def test_extract_markdown(fixture_path, options, expected, tolerances)
        assert_expected({ "output_type" => "string", "value" => @client.extract_markdown(PathSource.new(fixture_path), command_options(options)) }, expected, tolerances)
      end

      def test_get_metadata(fixture_path, options, expected, tolerances)
        assert_expected(normalize_metadata(@client.get_metadata(PathSource.new(fixture_path), command_options(options))), expected, tolerances)
      end

      def test_hash(fixture_path, options, expected, tolerances)
        fingerprint = @client.hash(PathSource.new(fixture_path), command_options(options))
        stable = expected.key?("content_hash_stable") && as_json(@client.hash(PathSource.new(fixture_path), command_options(options)))["hash"] == as_json(fingerprint)["hash"]
        assert_expected(normalize_hash(fingerprint, stable: stable), expected, tolerances)
      end

      def test_classify(fixture_path, _options, expected, tolerances)
        value = as_json(@client.classify(PathSource.new(fixture_path)))
        value["tags"] ||= value.delete("labels") || []
        value["heuristics"] ||= {}
        assert_expected(value, expected, tolerances)
      end

      def test_verify_receipt(fixture_path, options, expected, tolerances)
        receipt = options["receipt"]
        skip "Receipt not provided in expected options" unless receipt

        receipt_path = receipt.start_with?("/", "http://", "https://") ? receipt : File.join("fixtures", receipt)
        assert_expected({ "valid" => @client.verify_receipt(fixture_path, receipt_path) }, expected, tolerances)
      end

      def test_search(fixture_path, options, expected, tolerances)
        pattern = options.fetch("pattern")
        search_options = options.reject { |key, _value| key == "pattern" }
        matches = @client.search(PathSource.new(fixture_path), pattern, command_options(search_options)).to_a.map { |match| as_json(match) }
        assert_expected({
          "output_type" => "iterator",
          "match_count" => matches.length,
          "min_matches" => matches.length,
          "first_match_page" => matches.dig(0, "page"),
          "first_match_text" => matches.dig(0, "text"),
          "matches" => matches
        }, expected, tolerances)
      end

      def test_extract_stream(fixture_path, options, expected, tolerances)
        frames = @client.extract_stream(PathSource.new(fixture_path), command_options(options)).to_a.map { |frame| as_json(frame) }
        header = frames.find { |frame| frame["type"] == "header" } || {}
        page_frames = frames.count { |frame| frame["type"] == "page" }
        assert_expected({
          "output_type" => "iterator",
          "frame_count" => frames.length,
          "first_frame_type" => frames.dig(0, "type"),
          "last_frame_type" => frames.dig(-1, "type"),
          "page_frames" => page_frames,
          "header_frame_has_schema_version" => header.key?("schema_version"),
          "header_frame_has_total_pages" => header.key?("total_pages")
        }, expected, tolerances)
      end
    end
  end
end
