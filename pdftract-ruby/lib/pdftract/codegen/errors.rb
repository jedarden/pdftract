# frozen_string_literal: true

module Pdftract
  module Codegen
    #
    # This file is auto-generated. Do not edit manually.
    #

    class PdftractError < StandardError
      attr_reader :exit_code

      def initialize(message, exit_code)
        @exit_code = exit_code
        super(message)
      end
    end

    
    
    #
    # Corrupt PDF
    #
    class CorruptPdfError < PdftractError
      def initialize(message, exit_code)
        super(message, exit_code)
      end
    end
    
    
    
    #
    # Encrypted / password missing/wrong
    #
    class EncryptionError < PdftractError
      def initialize(message, exit_code)
        super(message, exit_code)
      end
    end
    
    
    
    #
    # Source unreadable
    #
    class SourceUnreachableError < PdftractError
      def initialize(message, exit_code)
        super(message, exit_code)
      end
    end
    
    
    
    #
    # Network interrupted
    #
    class RemoteFetchInterruptedError < PdftractError
      def initialize(message, exit_code)
        super(message, exit_code)
      end
    end
    
    
    
    #
    # TLS / cert failure
    #
    class TlsError < PdftractError
      def initialize(message, exit_code)
        super(message, exit_code)
      end
    end
    
    
    
    

    
    
    
    
    
    
    
    
    
    
    
    
    #
    # Receipt verify failed
    #
    class ReceiptVerifyError < PdftractError
      def initialize(message, exit_code)
        super(message, exit_code)
      end
    end
    
    
  end
end
