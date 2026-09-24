# frozen_string_literal: true

require 'ostruct'

module Pdftract
  #
  # Data classes for pdftract return types.
  # These immutable structs represent the JSON output from the pdftract CLI.
  #

  #
  # Document represents a PDF document with pages and metadata.
  #
  Document = Data.define(:schema_version, :pages, :metadata, :errors)

  # PDF indirect-object location attached to a diagnostic.
  DiagnosticLocation = Data.define(:object_number, :generation_number)

  # Canonical structured extraction diagnostic. The compact result's legacy
  # metadata string array remains available separately.
  Diagnostic = Data.define(:code, :message, :severity, :page_index, :location, :hint)

  #
  # Page represents a single page in the document.
  #
  Page = Data.define(:page, :width, :height, :rotation, :spans, :blocks)

  #
  # Span represents a text span with font and position information.
  #
  Span = Data.define(:text, :bbox, :font, :size, :confidence)

  #
  # Block represents a structural block (paragraph, heading, table, etc.).
  #
  Block = Data.define(:kind, :text, :bbox, :level)

  #
  # Match represents a search match result.
  #
  Match = Data.define(:text, :page, :bbox, :context)
  MatchContext = Data.define(:before, :after)

  #
  # Fingerprint represents document hash information.
  #
  Fingerprint = Data.define(:hash, :page_count, :fast_hash, :metadata)

  #
  # Classification represents document classification results.
  #
  Classification = Data.define(:category, :confidence, :tags, :heuristics)

  #
  # Metadata represents document metadata.
  #
  Metadata = Data.define(:title, :author, :subject, :keywords, :creator,
                         :producer, :created, :modified, :page_count,
                         :diagnostics, :diagnostics_detailed)

  #
  # Helper module for converting JSON hashes to Data classes.
  #
  module ModelConverter
    class << self
      def from_hash(hash, klass)
        return nil if hash.nil?

        # Convert hash keys to symbols
        symbolized = hash.transform_keys(&:to_sym)

        # Handle nested structures
        case klass.name
        when 'Pdftract::Document'
          convert_document(symbolized)
        when 'Pdftract::Page'
          convert_page(symbolized)
        when 'Pdftract::Span'
          convert_span(symbolized)
        when 'Pdftract::Block'
          convert_block(symbolized)
        when 'Pdftract::Match'
          convert_match(symbolized)
        when 'Pdftract::Fingerprint'
          convert_fingerprint(symbolized)
        when 'Pdftract::Classification'
          convert_classification(symbolized)
        when 'Pdftract::Metadata'
          convert_metadata(symbolized)
        when 'Pdftract::Diagnostic'
          convert_diagnostic(symbolized)
        when 'Pdftract::DiagnosticLocation'
          convert_diagnostic_location(symbolized)
        else
          klass.new(**symbolized)
        end
      end

      private

      def convert_document(h)
        Document.new(
          schema_version: h[:schema_version],
          pages: h[:pages]&.map { |p| convert_page(p.transform_keys(&:to_sym)) },
          metadata: h[:metadata] ? convert_metadata(h[:metadata].transform_keys(&:to_sym)) : nil,
          errors: h[:errors]&.map { |d| convert_diagnostic(d.transform_keys(&:to_sym)) } || []
        )
      end

      def convert_page(h)
        Page.new(
          page: h[:page],
          width: h[:width],
          height: h[:height],
          rotation: h[:rotation],
          spans: h[:spans]&.map { |s| convert_span(s.transform_keys(&:to_sym)) },
          blocks: h[:blocks]&.map { |b| convert_block(b.transform_keys(&:to_sym)) }
        )
      end

      def convert_span(h)
        Span.new(
          text: h[:text],
          bbox: h[:bbox],
          font: h[:font],
          size: h[:size],
          confidence: h[:confidence]
        )
      end

      def convert_block(h)
        Block.new(
          kind: h[:kind],
          text: h[:text],
          bbox: h[:bbox],
          level: h[:level]
        )
      end

      def convert_match(h)
        Match.new(
          text: h[:text],
          page: h[:page],
          bbox: h[:bbox],
          context: h[:context] ? convert_match_context(h[:context].transform_keys(&:to_sym)) : nil
        )
      end

      def convert_match_context(h)
        MatchContext.new(
          before: h[:before],
          after: h[:after]
        )
      end

      def convert_fingerprint(h)
        Fingerprint.new(
          hash: h[:hash],
          page_count: h[:page_count],
          fast_hash: h[:fast_hash],
          metadata: h[:metadata] ? convert_metadata(h[:metadata].transform_keys(&:to_sym)) : nil
        )
      end

      def convert_classification(h)
        Classification.new(
          category: h[:category],
          confidence: h[:confidence],
          tags: h[:tags] || [],
          heuristics: h[:heuristics] || {}
        )
      end

      def convert_metadata(h)
        Metadata.new(
          title: h[:title],
          author: h[:author],
          subject: h[:subject],
          keywords: h[:keywords] || [],
          creator: h[:creator],
          producer: h[:producer],
          created: h[:created],
          modified: h[:modified],
          page_count: h[:page_count],
          diagnostics: h[:diagnostics] || [],
          diagnostics_detailed: h[:diagnostics_detailed]&.map { |d| convert_diagnostic(d.transform_keys(&:to_sym)) } || []
        )
      end

      def convert_diagnostic_location(h)
        DiagnosticLocation.new(
          object_number: h[:object_number],
          generation_number: h[:generation_number]
        )
      end

      def convert_diagnostic(h)
        Diagnostic.new(
          code: h[:code],
          message: h[:message],
          severity: h[:severity],
          page_index: h[:page_index],
          location: h[:location] ? convert_diagnostic_location(h[:location].transform_keys(&:to_sym)) : nil,
          hint: h[:hint]
        )
      end
    end
  end
end
