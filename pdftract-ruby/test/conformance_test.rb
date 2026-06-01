# frozen_string_literal: true

require 'minitest/autorun'
require 'json'
require_relative '../lib/pdftract'

module Pdftract
  #
  # Conformance test suite for pdftract Ruby SDK
  #
  class ConformanceTest < Minitest::Test
    def setup
      @client = Client.new
      @suite_path = ENV['CONFORMANCE_SUITE'] || 'tests/sdk-conformance/cases.json'

      return unless File.exist?(@suite_path)

      @suite = JSON.parse(File.read(@suite_path))
    end

    def test_conformance
      return unless @suite

      @suite['cases'].each do |tc|
        define_method("test_#{tc['id']}_#{tc['method']}") do
          fixture_path = "tests/sdk-conformance/fixtures/#{tc['fixture']}"
          run_test_case(tc, fixture_path)
        end
      end
    end

    private

    def run_test_case(test_case, fixture_path)
      case test_case['method']
      when 'extract'
        test_extract(fixture_path, test_case['expected'])
      when 'extract_text'
        test_extract_text(fixture_path, test_case['expected'])
      when 'extract_markdown'
        test_extract_markdown(fixture_path, test_case['expected'])
      when 'get_metadata'
        test_get_metadata(fixture_path, test_case['expected'])
      when 'hash'
        test_hash(fixture_path, test_case['expected'])
      when 'classify'
        test_classify(fixture_path, test_case['expected'])
      when 'verify_receipt'
        test_verify_receipt(fixture_path, test_case['expected'])
      else
        skip "Method not yet implemented: #{test_case['method']}"
      end
    end

    def test_extract(fixture_path, assertions)
      skip "Fixture not found: #{fixture_path}" unless File.exist?(fixture_path)

      doc = @client.extract(fixture_path)

      if assertions&.key?('page_count')
        assert_equal assertions['page_count'], doc.pages.length, "Page count mismatch"
      end

      if assertions&.dig('has_title')
        refute_empty doc.metadata.title, "Expected non-empty title"
      end
    end

    def test_extract_text(fixture_path, assertions)
      skip "Fixture not found: #{fixture_path}" unless File.exist?(fixture_path)

      text = @client.extract_text(fixture_path)

      if assertions&.key?('min_length')
        assert_operator text.length, :>=, assertions['min_length'], "Text too short"
      end

      if assertions&.key?('contains')
        assertions['contains'].each do |substr|
          assert_includes text, substr, "Expected to contain '#{substr}'"
        end
      end
    end

    def test_extract_markdown(fixture_path, assertions)
      skip "Fixture not found: #{fixture_path}" unless File.exist?(fixture_path)

      md = @client.extract_markdown(fixture_path)

      if assertions&.key?('min_length')
        assert_operator md.length, :>=, assertions['min_length'], "Markdown too short"
      end
    end

    def test_get_metadata(fixture_path, assertions)
      skip "Fixture not found: #{fixture_path}" unless File.exist?(fixture_path)

      metadata = @client.get_metadata(fixture_path)

      if assertions&.key?('page_count')
        assert_equal assertions['page_count'], metadata.page_count, "Page count mismatch"
      end
    end

    def test_hash(fixture_path, assertions)
      skip "Fixture not found: #{fixture_path}" unless File.exist?(fixture_path)

      fingerprint = @client.hash(fixture_path)

      assert_equal 64, fingerprint.hash.length, "Hash should be 64 chars (SHA-256)"
      assert_equal 64, fingerprint.fast_hash.length, "Fast hash should be 64 chars (BLAKE3)"

      if assertions&.key?('page_count')
        assert_equal assertions['page_count'], fingerprint.page_count, "Page count mismatch"
      end
    end

    def test_classify(fixture_path, assertions)
      skip "Fixture not found: #{fixture_path}" unless File.exist?(fixture_path)

      classification = @client.classify(fixture_path)

      refute_empty classification.category, "Expected non-empty category"
      assert classification.confidence >= 0 && classification.confidence <= 1, "Confidence out of range"
    end

    def test_verify_receipt(fixture_path, assertions)
      return unless assertions&.key?('receipt')

      valid = @client.verify_receipt(fixture_path, assertions['receipt'])

      if assertions.key?('valid')
        assert_equal assertions['valid'], valid, "Receipt validity mismatch"
      end
    end
  end
end
