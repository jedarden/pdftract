# frozen_string_literal: true

require 'open3'
require 'json'
require_relative 'errors'
require_relative 'source'
require_relative 'models'

module Pdftract
  #
  # Client is the main interface for invoking the pdftract CLI.
  # All methods execute the pdftract binary as a subprocess and parse the output.
  #
  class Client
    attr_reader :binary_path, :version

    def initialize(binary_path = 'pdftract')
      @binary_path = binary_path
      @version = '1.0.0'
    end

    #
    # Extract structured data from a PDF.
    #
    # @param source [String, Source] PDF source (file path or Source object)
    # @param options [Hash] Extraction options (optional)
    # @return [Document] Extracted document with pages and metadata
    # @raise [Pdftract::Error] On subprocess error
    #
    def extract(source, options = nil)
      src = normalize_source(source)
      args = ['extract', '--json', *src.to_args]
      args.concat(options_to_args(options)) if options

      output = exec(*args)
      ModelConverter.from_hash(JSON.parse(output), Document)
    ensure
      src.cleanup if src.respond_to?(:cleanup)
    end

    #
    # Extract plain text from a PDF.
    #
    # @param source [String, Source] PDF source
    # @param options [Hash] Extraction options (optional)
    # @return [String] Plain text content
    # @raise [Pdftract::Error] On subprocess error
    #
    def extract_text(source, options = nil)
      src = normalize_source(source)
      args = ['extract', '--text', *src.to_args]
      args.concat(options_to_args(options)) if options

      exec(*args)
    ensure
      src.cleanup if src.respond_to?(:cleanup)
    end

    #
    # Extract Markdown-formatted text from a PDF.
    #
    # @param source [String, Source] PDF source
    # @param options [Hash] Extraction options (optional)
    # @return [String] Markdown formatted content
    # @raise [Pdftract::Error] On subprocess error
    #
    def extract_markdown(source, options = nil)
      src = normalize_source(source)
      args = ['extract', '--md', *src.to_args]
      args.concat(options_to_args(options)) if options

      exec(*args)
    ensure
      src.cleanup if src.respond_to?(:cleanup)
    end

    #
    # Extract pages from a PDF as a stream.
    #
    # @param source [String, Source] PDF source
    # @param options [Hash] Extraction options (optional)
    # @return [Enumerator<Page>] Lazy iterator yielding Page objects
    # @raise [Pdftract::Error] On subprocess error
    #
    def extract_stream(source, options = nil)
      src = normalize_source(source)
      args = ['extract', '--ndjson', *src.to_args]
      args.concat(options_to_args(options)) if options

      Open3.popen3(@binary_path, *args) do |stdin, stdout, stderr, wait_thr|
        return Enumerator.new do |yielder|
          begin
            stdout.each_line do |line|
              next if line.strip.empty?

              page_data = JSON.parse(line)
              yielder << ModelConverter.from_hash(page_data, Page)
            end
          ensure
            # Check exit status after consuming all output
            status = wait_thr.value
            unless status.success?
              stderr_text = stderr.read
              raise map_error(stderr_text, status.exitstatus)
            end
          end
        end
      end
    ensure
      src.cleanup if src.respond_to?(:cleanup)
    end

    #
    # Search for text in a PDF.
    #
    # @param source [String, Source] PDF source
    # @param pattern [String] Search pattern
    # @param options [Hash] Search options (optional)
    # @return [Enumerator<Match>] Lazy iterator yielding Match objects
    # @raise [Pdftract::Error] On subprocess error
    #
    def search(source, pattern, options = nil)
      src = normalize_source(source)
      args = ['grep', pattern, *src.to_args]
      args.concat(options_to_args(options, search: true)) if options

      Open3.popen3(@binary_path, *args) do |stdin, stdout, stderr, wait_thr|
        return Enumerator.new do |yielder|
          begin
            stdout.each_line do |line|
              next if line.strip.empty?

              match_data = JSON.parse(line)
              yielder << ModelConverter.from_hash(match_data, Match)
            end
          ensure
            # Check exit status after consuming all output
            status = wait_thr.value
            unless status.success?
              stderr_text = stderr.read
              raise map_error(stderr_text, status.exitstatus)
            end
          end
        end
      end
    ensure
      src.cleanup if src.respond_to?(:cleanup)
    end

    #
    # Get metadata from a PDF.
    #
    # @param source [String, Source] PDF source
    # @param options [Hash] Options (optional)
    # @return [Metadata] Document metadata
    # @raise [Pdftract::Error] On subprocess error
    #
    def get_metadata(source, options = nil)
      src = normalize_source(source)
      args = ['extract', '--metadata-only', *src.to_args]
      args.concat(options_to_args(options)) if options

      output = exec(*args)
      ModelConverter.from_hash(JSON.parse(output), Metadata)
    ensure
      src.cleanup if src.respond_to?(:cleanup)
    end

    #
    # Compute hash fingerprint of a PDF.
    #
    # @param source [String, Source] PDF source
    # @param options [Hash] Options (optional)
    # @return [Fingerprint] Document fingerprint
    # @raise [Pdftract::Error] On subprocess error
    #
    def hash(source, options = nil)
      src = normalize_source(source)
      args = ['hash', *src.to_args]
      args.concat(options_to_args(options)) if options

      output = exec(*args)
      ModelConverter.from_hash(JSON.parse(output), Fingerprint)
    ensure
      src.cleanup if src.respond_to?(:cleanup)
    end

    #
    # Classify a PDF document.
    #
    # @param source [String, Source] PDF source
    # @return [Classification] Document classification
    # @raise [Pdftract::Error] On subprocess error
    #
    def classify(source)
      src = normalize_source(source)
      args = ['classify', *src.to_args]

      output = exec(*args)
      ModelConverter.from_hash(JSON.parse(output), Classification)
    ensure
      src.cleanup if src.respond_to?(:cleanup)
    end

    #
    # Verify a receipt.
    #
    # @param pdf_path [String] Path to the PDF file
    # @param receipt [String] Path to receipt JSON file, or inline receipt JSON
    # @return [Boolean] True if receipt is valid, false otherwise
    # @raise [Pdftract::Error] On subprocess error (except verification failures)
    #
    def verify_receipt(pdf_path, receipt)
      # Check if receipt is a file path or inline JSON
      if File.exist?(receipt)
        args = [pdf_path, receipt]
      else
        # Inline JSON - pass via --inline flag
        args = ['--inline', receipt, pdf_path]
      end

      stdout, stderr, status = Open3.capture3(@binary_path, 'verify-receipt', *args)

      # Exit code 0 means verification succeeded
      status.success?
    end

    private

    #
    # Execute the pdftract binary and return stdout.
    #
    def exec(*args)
      stdout, stderr, status = Open3.capture3(@binary_path, *args)

      unless status.success?
        raise map_error(stderr, status.exitstatus)
      end

      stdout
    end

    #
    # Map exit codes to specific error types.
    #
    def map_error(stderr, exit_code)
      msg = stderr.strip.empty? ? nil : stderr.strip

      case exit_code
      when 2
        CorruptPdfError.new(msg, exit_code, stderr)
      when 3
        EncryptionError.new(msg, exit_code, stderr)
      when 4
        SourceUnreachableError.new(msg, exit_code, stderr)
      when 5
        RemoteFetchInterruptedError.new(msg, exit_code, stderr)
      when 6
        TlsError.new(msg, exit_code, stderr)
      when 10
        ReceiptVerifyError.new(msg, exit_code, stderr)
      else
        Error.new(msg || "Unknown error (exit #{exit_code})", exit_code, stderr)
      end
    end

    #
    # Normalize source argument to a Source object.
    #
    def normalize_source(source)
      return source if source.is_a?(Source)

      # Check if it's a URL
      if source.is_a?(String) && source.start_with?('http://', 'https://')
        URLSource.new(source)
      else
        PathSource.new(source)
      end
    end

    #
    # Convert options hash to CLI arguments.
    #
    def options_to_args(options, search: false)
      return [] unless options

      args = []

      options.each do |key, value|
        cli_flag = camel_to_snake(key).to_s.gsub('_', '-')
        next if value.nil?

        case value
        when true
          args << "--#{cli_flag}"
        when false
          # Skip false values
        when Array
          # Array values (e.g., keywords) - may need special handling
          # For now, skip or convert to comma-separated
        when Hash
          # Skip nested hashes for now
        else
          args << "--#{cli_flag}=#{value}"
        end
      end

      args
    end

    #
    # Convert camelCase or PascalCase to snake_case.
    #
    def camel_to_snake(str)
      str.to_s
         .gsub(/([A-Z]+)([A-Z][a-z])/,'\1_\2')
         .gsub(/([a-z\d])([A-Z])/,'\1_\2')
         .downcase
    end
  end
end
