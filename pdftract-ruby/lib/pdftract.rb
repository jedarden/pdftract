# frozen_string_literal: true

require_relative 'pdftract/errors'
require_relative 'pdftract/models'
require_relative 'pdftract/source'
require_relative 'pdftract/client'

module Pdftract
  VERSION = '1.0.0'

  class << self
    #
    # Create a new Client instance.
    #
    # @param binary_path [String] Path to the pdftract binary (default: 'pdftract')
    # @return [Client] A new client instance
    #
    def client(binary_path = 'pdftract')
      Client.new(binary_path)
    end

    #
    # Delegate common methods to a default client for convenience.
    #
    %i[extract extract_text extract_markdown extract_stream search
       get_metadata hash classify verify_receipt].each do |method|
      define_method(method) do |*args, **kwargs|
        client.public_send(method, *args, **kwargs)
      end
    end
  end

  # Re-export Source helpers
  SourceHelper = Pdftract::SourceHelper

  # Re-export Source classes
  PathSource = Pdftract::PathSource
  URLSource = Pdftract::URLSource
  BytesSource = Pdftract::BytesSource
end
