#!/usr/bin/env python3
"""List public API items that need examples."""

import re
from pathlib import Path

# Items from lib.rs re-exports
LIB_EXPORTS = [
    "PdfSource", "FileSource", "MmapSource", "HttpRangeSource", "RemoteOpts",
    "ConfidenceSource", "map_confidence_source",
    "Document", "PageExtraction", "PageIter", "PdfExtractor",
    "extract_pdf", "extract_pdf_ndjson", "extract_pdf_streaming", "extract_text",
    "ExtractionMetadata", "ExtractionResult", "PageResult",
    "NamedEncoding", "Std14Metrics", "get_std14_metrics",
    "combine", "walk_acroform_fields", "AcroFieldType", "AcroFormField", "ChoiceValue", "FormFieldValue",
    "MarkdownOptions", "page_to_markdown", "page_to_markdown_with_links", "parse_anchors",
    "block_to_markdown", "form_fields_to_markdown", "span_to_markdown", "Anchor",
    "ExtractionOptions", "OutputOptions", "ReceiptsMode",
    "PageClass", "PageClassification", "page_type_string",
    "count_pages_tree", "LazyPageIter", "PageDict", "DEFAULT_MEDIABOX",
    "AttachmentJson", "BeadJson", "BlockJson", "CellJson", "ExtractionQuality",
    "RowJson", "SpanJson", "SpanRef", "TableJson", "ThreadJson",
    "GridCandidate", "TableDetector", "TablePageContext",
    "serialize_page_text", "TextOptions",
    "TextState", "WordBoundaryDetector", "WordBoundaryManager",
    "Glyph", "emit_glyph", "new_raw_glyph_list",
    "Span", "merge_glyphs_to_spans", "CssHexColor",
]

def find_items_needing_examples():
    """Find items in the public API that lack examples."""
    src_dir = Path('crates/pdftract-core/src')
    items_without = []
    
    for module_file in src_dir.glob('*.rs'):
        with open(module_file, 'r') as f:
            content = f.read()
            lines = content.split('\n')
            
        in_pub_struct = False
        struct_name = None
        doc_buffer = []
        
        for i, line in enumerate(lines):
            stripped = line.strip()
            
            if stripped.startswith('pub struct '):
                m = re.search(r'pub\s+struct\s+(\w+)', stripped)
                if m:
                    struct_name = m.group(1)
                    in_pub_struct = True
                    doc_buffer = []
            elif in_pub_struct and (stripped.startswith('pub ') or stripped == '}'):
                # Check if struct has example
                if struct_name in LIB_EXPORTS:
                    has_example = any('```rust' in d or '```no_run' in d for d in doc_buffer)
                    if not has_example:
                        items_without.append(('struct', struct_name, module_file.name))
                in_pub_struct = False
                doc_buffer = []
            elif in_pub_struct:
                doc_buffer.append(stripped)
    
    return items_without

if __name__ == '__main__':
    items = find_items_needing_examples()
    print(f"Public API items without examples ({len(items)}):")
    print()
    for kind, name, file in sorted(items):
        print(f"  {kind:6} {name:30} ({file})")
