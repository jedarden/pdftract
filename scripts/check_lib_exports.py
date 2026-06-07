#!/usr/bin/env python3
"""Check which lib.rs re-exports have examples."""

import re
from pathlib import Path

# Key re-exports from lib.rs that users interact with
KEY_API_ITEMS = {
    # source
    "FileSource", "MmapSource", "HttpRangeSource", "RemoteOpts",
    # confidence
    "ConfidenceSource", "map_confidence_source",
    # document
    "Document", "PageExtraction", "PageIter", "PdfExtractor",
    # extract
    "extract_pdf", "extract_pdf_ndjson", "extract_pdf_streaming", "extract_text",
    "ExtractionMetadata", "ExtractionResult", "PageResult",
    # font
    "get_std14_metrics", "NamedEncoding", "Std14Metrics",
    # forms
    "combine", "walk_acroform_fields", "AcroFieldType", "AcroFormField",
    "ChoiceValue", "FormFieldValue",
    # markdown
    "MarkdownOptions", "page_to_markdown", "page_to_markdown_with_links",
    "parse_anchors", "block_to_markdown", "form_fields_to_markdown",
    "span_to_markdown", "Anchor",
    # options
    "ExtractionOptions", "OutputOptions", "ReceiptsMode",
    # page_class
    "PageClass", "PageClassification", "page_type_string",
    # parser
    "count_pages_tree", "LazyPageIter", "PageDict", "DEFAULT_MEDIABOX",
    # table
    "GridCandidate", "TableDetector", "TablePageContext",
    # text
    "serialize_page_text", "TextOptions",
    # word_boundary
    "TextState", "WordBoundaryDetector", "WordBoundaryManager",
    # glyph
    "Glyph", "emit_glyph", "new_raw_glyph_list",
    # span
    "Span", "merge_glyphs_to_spans", "CssHexColor",
}

# Items we've confirmed have examples
CONFIRMED_WITH_EXAMPLES = {
    "Document", "PageExtraction", "PageIter", "PdfExtractor",
    "extract_pdf", "extract_pdf_ndjson", "extract_pdf_streaming", "extract_text",
    "ExtractionMetadata", "ExtractionResult", "PageResult",
    "ExtractionOptions", "OutputOptions", "ReceiptsMode",
    "MarkdownOptions", "parse_anchors", "Anchor",
}

def main():
    print(f"=== Key Public API Items ===")
    print()
    print(f"Total API items: {len(KEY_API_ITEMS)}")
    print(f"With confirmed examples: {len(CONFIRMED_WITH_EXAMPLES)}")
    print(f"Coverage: {len(CONFIRMED_WITH_EXAMPLES) / len(KEY_API_ITEMS) * 100:.1f}%")
    print()
    
    need_examples = KEY_API_ITEMS - CONFIRMED_WITH_EXAMPLES
    if need_examples:
        print(f"Items needing example verification ({len(need_examples)}):")
        for item in sorted(need_examples):
            print(f"  - {item}")
    else:
        print("All key API items have confirmed examples!")

if __name__ == '__main__':
    main()
