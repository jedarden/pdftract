# frozen_string_literal: true

require "minitest/autorun"
require "json"
require_relative "../../lib/pdftract"

module Pdftract
  module Codegen
    #
    # Conformance test suite for pdftract Ruby SDK
    # Auto-generated - do not edit manually
    #

    class ConformanceTest < Minitest::Test
      def setup
        @client = Client.new
        @suite_path = ENV["CONFORMANCE_SUITE"] || "tests/sdk-conformance/cases.json"

        return unless File.exist?(@suite_path)

        @suite = JSON.parse(File.read(@suite_path))
      end

      def test_conformance
        return unless @suite

        @suite["cases"].each do |tc|
          define_method("test_#{tc['id']}_#{tc['method']}") do
            fixture_path = "fixtures/#{tc['fixture']}"
            run_test_case(tc, fixture_path)
          end
        end
      end

      private

      def run_test_case(test_case, fixture_path)
        case test_case["method"]
        when "extract"
          test_extract(fixture_path, test_case["assertions"])
        when "extract_text"
          test_extract_text(fixture_path, test_case["assertions"])
        when "extract_markdown"
          test_extract_markdown(fixture_path, test_case["assertions"])
        when "get_metadata"
          test_get_metadata(fixture_path, test_case["assertions"])
        when "hash"
          test_hash(fixture_path, test_case["assertions"])
        when "classify"
          test_classify(fixture_path, test_case["assertions"])
        when "verify_receipt"
          test_verify_receipt(fixture_path, test_case["assertions"])
        else
          skip "Method not yet implemented: #{test_case['method']}"
        end
      end

      def test_extract(fixture_path, assertions)
        doc = @client.extract(PathSource.new(fixture_path))

        if assertions&.key?("page_count")
          assert_equal assertions["page_count"], doc.pages.length
        end

        if assertions&.dig("has_title")
          refute_empty doc.metadata.title
        end
      end

      def test_extract_text(fixture_path, assertions)
        text = @client.extract_text(PathSource.new(fixture_path))

        if assertions&.key?("min_length")
          assert_operator text.length, :>=, assertions["min_length"]
        end

        if assertions&.key?("contains")
          assertions["contains"].each do |substr|
            assert_includes text, substr
          end
        end
      end

      def test_extract_markdown(fixture_path, assertions)
        md = @client.extract_markdown(PathSource.new(fixture_path))

        if assertions&.key?("min_length")
          assert_operator md.length, :>=, assertions["min_length"]
        end
      end

      def test_get_metadata(fixture_path, assertions)
        metadata = @client.get_metadata(PathSource.new(fixture_path))

        if assertions&.key?("page_count")
          assert_equal assertions["page_count"], metadata.page_count
        end
      end

      def test_hash(fixture_path, assertions)
        fingerprint = @client.hash(PathSource.new(fixture_path))

        assert_equal 64, fingerprint.hash.length
        assert_equal 64, fingerprint.fast_hash.length

        if assertions&.key?("page_count")
          assert_equal assertions["page_count"], fingerprint.page_count
        end
      end

      def test_classify(fixture_path, assertions)
        classification = @client.classify(PathSource.new(fixture_path))

        refute_empty classification.category
        assert classification.confidence >= 0 && classification.confidence <= 1
      end

      def test_verify_receipt(fixture_path, assertions)
        return unless assertions&.key?("receipt")

        valid = @client.verify_receipt(fixture_path, assertions["receipt"])

        if assertions.key?("valid")
          assert_equal assertions["valid"], valid
        end
      end
    end
  end
end
