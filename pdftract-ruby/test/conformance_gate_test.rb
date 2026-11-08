# frozen_string_literal: true

require 'minitest/autorun'
require 'json'
require 'tmpdir'
require 'fileutils'
require 'open3'
require 'rbconfig'

class ConformanceGateTest < Minitest::Test
  SDK_DIR = File.expand_path('..', __dir__)

  def with_gate(cases)
    Dir.mktmpdir('pdftract-ruby-gate-') do |dir|
      FileUtils.mkdir_p(File.join(dir, 'fixtures'))
      FileUtils.mkdir_p(File.join(dir, 'bin'))
      File.write(File.join(dir, 'cases.json'), JSON.generate('cases' => cases))
      File.write(File.join(dir, 'fixtures', 'doc.pdf'), 'fixture')
      File.write(File.join(dir, 'fixtures', 'receipt.json'), '{}')
      binary = File.join(dir, 'bin', 'pdftract')
      File.write(binary, <<~SH)
        #!/bin/sh
        printf '%s\n' "$1" >> "$PDFTRACT_INVOCATIONS"
        case "$1" in
          --version|verify-receipt) exit 0 ;;
          *) exit 2 ;;
        esac
      SH
      File.chmod(0o755, binary)
      env = {
        'CONFORMANCE_SUITE' => File.join(dir, 'cases.json'),
        'PDFTRACT_INVOCATIONS' => File.join(dir, 'invocations'),
        'PATH' => "#{File.join(dir, 'bin')}:#{ENV.fetch('PATH')}"
      }
      yield(dir, env)
    end
  end

  def run_gate(env)
    Open3.capture3(env, RbConfig.ruby, '-Ilib', 'test/conformance_test.rb', chdir: SDK_DIR)
  end

  def receipt_case(expected = { 'valid' => true }, fixture = 'doc.pdf')
    { 'id' => 'receipt', 'method' => 'verify_receipt', 'fixture' => fixture,
      'options' => { 'receipt' => 'receipt.json' }, 'expected' => expected }
  end

  def test_receipt_case_invokes_cli_and_checks_expected_result
    with_gate([receipt_case]) do |dir, env|
      output, errors, status = run_gate(env)
      assert status.success?, "#{output}\n#{errors}"
      assert_includes output, '1 passed, 0 skipped, 0 failed'
      assert_includes File.read(File.join(dir, 'invocations')), 'verify-receipt'
    end
  end

  def test_empty_suite_fails
    with_gate([]) do |_dir, env|
      output, errors, status = run_gate(env)
      refute status.success?, "#{output}\n#{errors}"
      assert_includes output, 'suite contains no cases'
    end
  end

  def test_case_without_an_assertion_fails
    with_gate([receipt_case({})]) do |_dir, env|
      output, errors, status = run_gate(env)
      refute status.success?, "#{output}\n#{errors}"
      assert_includes output, 'completed without checking an expectation'
    end
  end

  def test_missing_fixture_fails
    with_gate([receipt_case({ 'valid' => true }, 'absent.pdf')]) do |_dir, env|
      output, errors, status = run_gate(env)
      refute status.success?, "#{output}\n#{errors}"
      assert_includes output, 'Fixture not found'
    end
  end

  def test_missing_binary_fails
    with_gate([receipt_case]) do |dir, env|
      env['PATH'] = File.join(dir, 'empty')
      output, errors, status = run_gate(env)
      refute status.success?, "#{output}\n#{errors}"
      assert_includes output, 'pdftract CLI is unavailable'
    end
  end

  def test_unimplemented_method_fails
    with_gate([{ 'id' => 'search', 'method' => 'search', 'fixture' => 'doc.pdf' }]) do |_dir, env|
      output, errors, status = run_gate(env)
      refute status.success?, "#{output}\n#{errors}"
      assert_includes output, 'Conformance method is not implemented'
    end
  end
end
