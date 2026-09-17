# frozen_string_literal: true

module Pdftract
  module Codegen
    #
    # This file is auto-generated. Do not edit manually.
    #

    class Source
      def to_args
        raise NotImplementedError
      end
    end

    class PathSource < Source
      def initialize(path)
        @path = path
      end

      def to_args
        [@path]
      end
    end

    class URLSource < Source
      def initialize(url)
        @url = url
      end

      def to_args
        [@url]
      end
    end

    class BytesSource < Source
      def initialize(bytes)
        @bytes = bytes
      end

      def to_args
        # Write to temp file - implementation omitted for brevity
        raise NotImplementedError, "BytesSource requires temp file handling"
      end
    end
  end
end
