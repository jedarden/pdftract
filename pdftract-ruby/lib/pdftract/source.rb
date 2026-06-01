# frozen_string_literal: true

require 'tempfile'

module Pdftract
  #
  # Source represents a PDF source (file path, URL, or raw bytes).
  #
  class Source
    #
    # Converts the source to CLI arguments.
    # Returns an array of strings to be passed to the subprocess.
    #
    def to_args
      raise NotImplementedError, 'Subclasses must implement to_args'
    end
  end

  #
  # PathSource represents a local filesystem path.
  #
  class PathSource < Source
    attr_reader :path

    def initialize(path)
      @path = File.expand_path(path)
    end

    def to_args
      [@path]
    end
  end

  #
  # URLSource represents a remote URL.
  #
  class URLSource < Source
    attr_reader :url

    def initialize(url)
      unless url.start_with?('http://', 'https://')
        raise ArgumentError, "Invalid URL: #{url} (must start with http:// or https://)"
      end
      @url = url
    end

    def to_args
      ['--url', @url]
    end
  end

  #
  # BytesSource represents in-memory PDF bytes.
  # The temporary file created for subprocess consumption is cleaned up after use.
  #
  class BytesSource < Source
    attr_reader :data, :tmp_path

    def initialize(data)
      @data = data
      @tmp_path = nil
    end

    def to_args
      # Write to a temporary file for subprocess consumption
      @tmp_path = Tempfile.new(['pdftract-', '.pdf']).path
      File.binwrite(@tmp_path, @data)
      [@tmp_path]
    end

    #
    # cleanup removes the temporary file if it was created.
    #
    def cleanup
      return unless @tmp_path && File.exist?(@tmp_path)

      File.delete(@tmp_path)
      @tmp_path = nil
    end
  end

  #
  # Helper methods for creating Source instances.
  #
  module SourceHelper
    #
    # Creates a PathSource from a file path.
    #
    def self.path(path)
      PathSource.new(path)
    end

    #
    # Creates a URLSource from a URL string.
    #
    def self.url(url)
      URLSource.new(url)
    end

    #
    # Creates a BytesSource from a byte string.
    #
    def self.bytes(data)
      BytesSource.new(data)
    end

    #
    # Reads a file and returns a BytesSource.
    #
    def self.from_file(path)
      BytesSource.new(File.binread(path))
    end
  end
end
