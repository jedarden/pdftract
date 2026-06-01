# frozen_string_literal: true

module Pdftract
  #
  # PdftractError is the base error type for all pdftract errors.
  #
  class Error < StandardError
    attr_reader :exit_code, :stderr

    def initialize(message, exit_code = nil, stderr = nil)
      @exit_code = exit_code
      @stderr = stderr
      super(message)
    end
  end

  #
  # CorruptPdfError represents a corrupt PDF error (exit code 2).
  #
  class CorruptPdfError < Error
    def initialize(message = nil, exit_code = 2, stderr = nil)
      message ||= "The PDF file is corrupt or invalid"
      super(message, exit_code, stderr)
    end
  end

  #
  # EncryptionError represents an encryption error (exit code 3).
  #
  class EncryptionError < Error
    def initialize(message = nil, exit_code = 3, stderr = nil)
      message ||= "The PDF is encrypted and password is missing or wrong"
      super(message, exit_code, stderr)
    end
  end

  #
  # SourceUnreachableError represents a source unreadable error (exit code 4).
  #
  class SourceUnreachableError < Error
    def initialize(message = nil, exit_code = 4, stderr = nil)
      message ||= "The source (file or URL) is unreadable"
      super(message, exit_code, stderr)
    end
  end

  #
  # RemoteFetchInterruptedError represents a network interruption error (exit code 5).
  #
  class RemoteFetchInterruptedError < Error
    def initialize(message = nil, exit_code = 5, stderr = nil)
      message ||= "Network interrupted during remote fetch"
      super(message, exit_code, stderr)
    end
  end

  #
  # TlsError represents a TLS/certificate error (exit code 6).
  #
  class TlsError < Error
    def initialize(message = nil, exit_code = 6, stderr = nil)
      message ||= "TLS certificate validation failed"
      super(message, exit_code, stderr)
    end
  end

  #
  # ReceiptVerifyError represents a receipt verification failure (exit code 10).
  #
  class ReceiptVerifyError < Error
    def initialize(message = nil, exit_code = 10, stderr = nil)
      message ||= "Receipt verification failed"
      super(message, exit_code, stderr)
    end
  end
end
