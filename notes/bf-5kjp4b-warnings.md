# pdftract Cargo Check Warnings - Structured Documentation

**Generated:** 2026-08-09
**Bead:** bf-5kjp4b
**Source:** cargo check output

## Summary Statistics

**Total Warnings:** 705

| Warning Type | Count | Percentage |
|--------------|-------|------------|
| unused_imports | 242 | 34.3% |
| other | 182 | 25.8% |
| unused_variables | 131 | 18.6% |
| unused_mut | 84 | 11.9% |
| dead_code | 55 | 7.8% |
| unused_doc_comments | 4 | 0.6% |
| deprecated | 2 | 0.3% |
| unreachable_patterns | 1 | 0.1% |
| mismatched_lifetime_syntaxes | 1 | 0.1% |
| redundant_semicolons | 1 | 0.1% |
| non_snake_case | 1 | 0.1% |
| noop_method_call | 1 | 0.1% |

---

## Unused Imports

**Count:** 242 warnings

**Severity:** warning (non-breaking)

### crates/pdftract-schema-migrate/src/bin/migrate-schema.rs:9:31

**Message:** unused imports: `read_json` and `write_json`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^                 ^^^^^^^^^^
```

**Help/Notes:**
  - = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-core/src/annotation/json.rs:6:32

**Message:** unused import: `DestArray`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^
```

**Help/Notes:**
  - = note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-core/src/cache/key.rs:10:24

**Message:** unused import: `Map`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^
```

---

### crates/pdftract-core/src/cache/lru.rs:8:5

**Message:** unused import: `entry_path`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^
```

---

### crates/pdftract-core/src/conformance.rs:17:5

**Message:** unused import: `crate::parser::object::PdfObject`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/conformance.rs:20:5

**Message:** unused import: `anyhow::Result`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/content_stream.rs:34:29

**Message:** unused import: `intern`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^
```

---

### crates/pdftract-core/src/content_stream.rs:2016:41

**Message:** unused import: `PdfDict`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^
```

---

### crates/pdftract-core/src/detection.rs:11:29

**Message:** unused import: `ObjRef`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^
```

---

### crates/pdftract-core/src/document.rs:21:76

**Message:** unused imports: `LinearizationInfo` and `XrefSection`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/encryption/detection.rs:13:19

**Message:** unused import: `DiagCode`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^
```

---

### crates/pdftract-core/src/encryption/decryptor.rs:12:32

**Message:** unused import: `derive_aes_128_object_key`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/encryption/decryptor.rs:22:5

**Message:** unused import: `secrecy::SecretString`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/extract.rs:23:57

**Message:** unused import: `AcroFormField`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/extract.rs:33:60

**Message:** unused import: `parse_struct_tree`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/extract.rs:44:64

**Message:** unused import: `PageContext`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^
```

---

### crates/pdftract-core/src/extract.rs:46:39

**Message:** unused import: `TableSpan`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^
```

---

### crates/pdftract-core/src/extract.rs:49:20

**Message:** unused imports: `emit_glyph` and `new_raw_glyph_list`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/extract.rs:50:5

**Message:** unused import: `crate::graphics_state::GraphicsState`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/extract.rs:53:5

**Message:** unused imports: `BlockInput`, `Block`, `Column`, `Line`, `PageContext as LayoutPageContext`, `assign_columns_to_lines`, `classify_caption`, `classify_code`, `classify_figure`, `classify_formula`, `classify_list`, `classify_watermark`, `compute_baseline`, and `detect_headers_and_footers`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^^^                      ^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/extract.rs:59:32

**Message:** unused import: `Span`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^
```

---

### crates/pdftract-core/src/extract.rs:67:5

**Message:** unused import: `std::cmp::Ordering`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/extract.rs:930:13

**Message:** unused import: `crate::parser::xref::XrefResolver`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/font/agl.rs:14:5

**Message:** unused import: `crate::diagnostics::DiagCode`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/font/fingerprint.rs:26:5

**Message:** unused import: `std::sync::Arc`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/font/resolver.rs:24:26

**Message:** unused imports: `lookup_shape` and `phash_glyph`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^  ^^^^^^^^^^^
```

---

### crates/pdftract-core/src/font/resolver.rs:26:37

**Message:** unused imports: `DocumentContext as Type3DocumentContext`, `StreamResolverFn`, and `rasterize_type3_glyph`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/font/resolver.rs:27:29

**Message:** unused imports: `ExtractionOptions` and `decode_stream`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^  ^^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/font/type0.rs:11:43

**Message:** unused import: `OpenTypeMetrics`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/font/type3_rasterizer.rs:19:5

**Message:** unused import: `std::collections::HashSet`

**Type:** unused_imports

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^^^^^
```

---

*... and 212 more `unused_imports` warnings*


## Other

**Count:** 182 warnings

**Severity:** warning (non-breaking)

### crates/pdftract-core/src/parser/lexer/mod.rs:528:13

**Message:** variable `sign_count` is assigned to, but never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^
```

**Help/Notes:**
  - = note: consider using `_sign_count` instead

---

### crates/pdftract-core/src/cache/key.rs:127:4

**Message:** function `canonical_json` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/cache/key.rs:145:4

**Message:** function `canonical_json_value` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/classify.rs:275:8

**Message:** method `name` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
--------------- method in this trait
```

---

### crates/pdftract-core/src/content_stream.rs:200:8

**Message:** method `depth` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
--------------------- method in this implementation
```

---

### crates/pdftract-core/src/content_stream.rs:1602:5

**Message:** variants `Stream` and `Error` are never constructed

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
-------------------- variants in this enum
```

---

### crates/pdftract-core/src/extract.rs:1315:4

**Message:** function `extract_page` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/layout/correction.rs:838:8

**Message:** associated function `is_component` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
------------- associated function in this implementation
```

---

### crates/pdftract-core/src/layout/header_footer.rs:225:4

**Message:** function `is_repeated_header_footer` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/layout/reading_order.rs:27:7

**Message:** constant `REGION_COUNT_THRESHOLD` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/layout/reading_order.rs:567:4

**Message:** function `union_bboxes` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^
```

---

### crates/pdftract-core/src/parser/catalog.rs:421:8

**Message:** method `emit_diagnostic` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
------------ method in this implementation
```

---

### crates/pdftract-core/src/parser/hint_stream.rs:165:8

**Message:** method `has_bits` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
-------------- method in this implementation
```

---

### crates/pdftract-core/src/parser/lexer/mod.rs:1108:8

**Message:** method `lex_unknown` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
------------------ method in this implementation
```

---

### crates/pdftract-core/src/parser/marked_content.rs:83:8

**Message:** method `emit_diagnostic` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
---------------- method in this implementation
```

---

### crates/pdftract-core/src/parser/ocg.rs:69:8

**Message:** associated function `from_name` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
--------------- associated function in this implementation
```

---

### crates/pdftract-core/src/parser/ocg.rs:99:8

**Message:** associated function `parse` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
--------- associated function in this implementation
```

---

### crates/pdftract-core/src/parser/xref.rs:876:4

**Message:** function `read_line` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^
```

---

### crates/pdftract-core/src/receipts/ocr_fallback.rs:239:4

**Message:** function `round_coord` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^
```

---

### crates/pdftract-core/src/text.rs:329:4

**Message:** function `get_block_text` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^
```

---

### /home/coding/pdftract/target/debug/build/pdftract-core-dd8ea6b2d053e647/out/font_fingerprints.rs:7:8

**Message:** static variable `HASH_56a45233d29f11b4dfb86d248e921939d115778f87325e7ae8cc108383d6664d` should have an upper case name

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

**Help/Notes:**
  - = note: `#[warn(non_upper_case_globals)]` (part of `#[warn(nonstandard_style)]`) on by default

---

### crates/pdftract-cli/src/profiles_cmd.rs:188:5

**Message:** unreachable expression

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
--------------------------------------------------------------------------------- any code following this expression is unreachable
```

**Help/Notes:**
  - = note: `#[warn(unreachable_code)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-cli/src/profiles_cmd.rs:221:5

**Message:** unreachable expression

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
--------------------------------------------------------------------------------- any code following this expression is unreachable
```

---

### crates/pdftract-cli/src/profiles_cmd.rs:274:5

**Message:** unreachable expression

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
--------------------------------------------------------------------------------- any code following this expression is unreachable
```

---

### crates/pdftract-cli/src/profiles_cmd.rs:305:5

**Message:** unreachable expression

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
--------------------------------------------------------------------------------- any code following this expression is unreachable
```

---

### crates/pdftract-cli/src/serve.rs:805:9

**Message:** variable `pdf_bytes` is assigned to, but never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^
```

**Help/Notes:**
  - = note: consider using `_pdf_bytes` instead

---

### crates/pdftract-cli/src/inspect/api.rs:1096:4

**Message:** function `render_ocr_layer` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-cli/src/mcp/tools/args.rs:12:12

**Message:** struct `PasswordArg` is never constructed

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^
```

---

### crates/pdftract-cli/src/mcp/tools/registry.rs:135:4

**Message:** function `find_startxref_offset` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^
```

---

### crates/pdftract-cli/src/serve.rs:317:12

**Message:** function `parse_float` is never used

**Type:** other

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^
```

---

*... and 152 more `other` warnings*


## Unused Variables

**Count:** 131 warnings

**Severity:** warning (non-breaking)

### crates/pdftract-core/src/annotation/json.rs:29:5

**Message:** unused variable: `page_ref_to_index`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_page_ref_to_index`
```

**Help/Notes:**
  - = note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-core/src/classify.rs:690:31

**Message:** unused variable: `ctx`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^ help: if this is intentional, prefix it with an underscore: `_ctx`
```

---

### crates/pdftract-core/src/encryption/rc4.rs:242:10

**Message:** unused variable: `k`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^ help: if this is intentional, prefix it with an underscore: `_k`
```

---

### crates/pdftract-core/src/encryption/rc4.rs:295:9

**Message:** unused variable: `padded_password`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_padded_password`
```

---

### crates/pdftract-core/src/extract.rs:158:5

**Message:** unused variable: `resolver`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_resolver`
```

---

### crates/pdftract-core/src/extract.rs:159:5

**Message:** unused variable: `page_index`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_page_index`
```

---

### crates/pdftract-core/src/extract.rs:569:9

**Message:** unused variable: `decryption_context`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_decryption_context`
```

---

### crates/pdftract-core/src/extract.rs:658:9

**Message:** unused variable: `semaphore`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_semaphore`
```

---

### crates/pdftract-core/src/extract.rs:1101:5

**Message:** unused variable: `resolver`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_resolver`
```

---

### crates/pdftract-core/src/extract.rs:1102:5

**Message:** unused variable: `catalog`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^ help: if this is intentional, prefix it with an underscore: `_catalog`
```

---

### crates/pdftract-core/src/extract.rs:1130:13

**Message:** unused variable: `kind`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^ help: try ignoring the field: `kind: _`
```

---

### crates/pdftract-core/src/extract.rs:1159:13

**Message:** unused variable: `is_combo`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^ help: try ignoring the field: `is_combo: _`
```

---

### crates/pdftract-core/src/extract.rs:1734:9

**Message:** unused variable: `page_count`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_page_count`
```

---

### crates/pdftract-core/src/font/embedded.rs:345:26

**Message:** unused variable: `expected_type`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_expected_type`
```

---

### crates/pdftract-core/src/font/resolver.rs:536:5

**Message:** unused variable: `resolver`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_resolver`
```

---

### crates/pdftract-core/src/font/resolver.rs:537:5

**Message:** unused variable: `source`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^ help: if this is intentional, prefix it with an underscore: `_source`
```

---

### crates/pdftract-core/src/font/resolver.rs:538:5

**Message:** unused variable: `doc_decompress_counter`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_doc_decompress_counter`
```

---

### crates/pdftract-core/src/font/resolver.rs:559:9

**Message:** unused variable: `glyph_name_for_l4`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_glyph_name_for_l4`
```

---

### crates/pdftract-core/src/font/type3_rasterizer.rs:1024:13

**Message:** unused variable: `name`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^ help: if this is intentional, prefix it with an underscore: `_name`
```

---

### crates/pdftract-core/src/font/type3_rasterizer.rs:1717:5

**Message:** unused variable: `doc_context`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_doc_context`
```

---

### crates/pdftract-core/src/forms/mod.rs:552:5

**Message:** unused variable: `diagnostics`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_diagnostics`
```

---

### crates/pdftract-core/src/glyph/mod.rs:459:22

**Message:** unused variable: `font_dict`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_font_dict`
```

---

### crates/pdftract-core/src/glyph/mod.rs:459:43

**Message:** unused variable: `char_code`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_char_code`
```

---

### crates/pdftract-core/src/glyph/mod.rs:519:45

**Message:** unused variable: `char_code`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_char_code`
```

---

### crates/pdftract-core/src/glyph/mod.rs:559:40

**Message:** unused variable: `descriptor_ref`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_descriptor_ref`
```

---

### crates/pdftract-core/src/layout/correction.rs:904:9

**Message:** unused variable: `original_text`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_original_text`
```

---

### crates/pdftract-core/src/layout/correction.rs:921:10

**Message:** unused variable: `char_idx`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^ help: if this is intentional, prefix it with an underscore: `_char_idx`
```

---

### crates/pdftract-core/src/layout/header_footer.rs:203:10

**Message:** unused variable: `x0`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^ help: if this is intentional, prefix it with an underscore: `_x0`
```

---

### crates/pdftract-core/src/layout/header_footer.rs:203:18

**Message:** unused variable: `x1`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^ help: if this is intentional, prefix it with an underscore: `_x1`
```

---

### crates/pdftract-core/src/layout/line.rs:395:5

**Message:** unused variable: `avg_x0`

**Type:** unused_variables

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^ help: if this is intentional, prefix it with an underscore: `_avg_x0`
```

---

*... and 101 more `unused_variables` warnings*


## Unused Mut

**Count:** 84 warnings

**Severity:** warning (non-breaking)

### crates/pdftract-core/src/attachment/filespec.rs:156:9

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^^^^
```

**Help/Notes:**
  - = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-core/src/cache/compression.rs:131:13

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^
```

---

### crates/pdftract-core/src/cache/compression.rs:255:9

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^
```

---

### crates/pdftract-core/src/encryption/aes_128.rs:62:9

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^
```

---

### crates/pdftract-core/src/extract.rs:676:9

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^^^
```

---

### crates/pdftract-core/src/extract.rs:1734:9

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^^^
```

---

### crates/pdftract-core/src/extract.rs:1763:9

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^^^
```

---

### crates/pdftract-core/src/extract.rs:2357:9

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^
```

---

### crates/pdftract-core/src/parser/inline_image.rs:668:29

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^
```

---

### crates/pdftract-core/src/parser/xref.rs:826:41

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^
```

---

### crates/pdftract-core/src/table/grid.rs:52:9

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^^^^^^
```

---

### crates/pdftract-cli/src/url.rs:301:9

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^
```

**Help/Notes:**
  - = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-cli/../../tests/list_pdf_fixtures.rs:14:13

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^
```

**Help/Notes:**
  - = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-core/src/atomic_file_writer.rs:295:13

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^
```

**Help/Notes:**
  - = note: `#[warn(unused_mut)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-core/src/attachment/filespec.rs:156:9

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^^^^
```

---

### crates/pdftract-core/src/attachment/filespec.rs:598:13

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^
```

---

### crates/pdftract-core/src/attachment/name_tree.rs:473:13

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^^^
```

---

### crates/pdftract-core/src/encryption/aes_128.rs:221:13

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^
```

---

### crates/pdftract-core/src/font/encoding.rs:1195:13

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^^^^
```

---

### crates/pdftract-core/src/font/type3_rasterizer.rs:2221:13

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^^^^
```

---

### crates/pdftract-core/src/forms/mod.rs:840:13

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^
```

---

### crates/pdftract-core/src/forms/mod.rs:931:13

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^
```

---

### crates/pdftract-core/src/forms/mod.rs:992:14

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^
```

---

### crates/pdftract-core/src/forms/mod.rs:992:27

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^
```

---

### crates/pdftract-core/src/forms/mod.rs:1065:14

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^
```

---

### crates/pdftract-core/src/forms/mod.rs:1065:27

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^
```

---

### crates/pdftract-core/src/forms/mod.rs:1112:14

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^
```

---

### crates/pdftract-core/src/forms/mod.rs:1112:27

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^
```

---

### crates/pdftract-core/src/forms/mod.rs:1154:14

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^
```

---

### crates/pdftract-core/src/forms/mod.rs:1154:27

**Message:** variable does not need to be mutable

**Type:** unused_mut

**Severity:** warning

**Code Snippet:**
```rust
----^^^^^^^^
```

---

*... and 54 more `unused_mut` warnings*


## Dead Code

**Count:** 55 warnings

**Severity:** warning (non-breaking)

### crates/pdftract-core/build.rs:37:5

**Message:** fields `description` and `version` are never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
------------------------ fields in this struct
```

**Help/Notes:**
  - = note: `UnmappedGlyphNamesConfig` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis
  - = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-core/src/layout/reading_order.rs:112:28

**Message:** value assigned to `region_count` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^ this value is reassigned later and never used
```

**Help/Notes:**
  - = note: `#[warn(unused_assignments)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-core/src/layout/reading_order.rs:113:34

**Message:** value assigned to `small_region_count` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^ this value is reassigned later and never used
```

---

### crates/pdftract-core/src/parser/lexer/mod.rs:537:13

**Message:** value assigned to `sign_count` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^
```

**Help/Notes:**
  - = help: maybe it is overwritten before being read?

---

### crates/pdftract-core/src/parser/pages.rs:1588:25

**Message:** value assigned to `inherited` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^
```

**Help/Notes:**
  - = help: maybe it is overwritten before being read?

---

### crates/pdftract-core/src/parser/pages.rs:1569:41

**Message:** value assigned to `inherited` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^
```

**Help/Notes:**
  - = help: maybe it is overwritten before being read?

---

### crates/pdftract-core/src/parser/pages.rs:1557:37

**Message:** value assigned to `inherited` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^
```

**Help/Notes:**
  - = help: maybe it is overwritten before being read?

---

### crates/pdftract-core/src/parser/pages.rs:1580:33

**Message:** value assigned to `inherited` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^
```

**Help/Notes:**
  - = help: maybe it is overwritten before being read?

---

### crates/pdftract-core/src/parser/pages.rs:1521:29

**Message:** value assigned to `inherited` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^
```

**Help/Notes:**
  - = help: maybe it is overwritten before being read?

---

### crates/pdftract-core/src/parser/pages.rs:1512:29

**Message:** value assigned to `inherited` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^
```

**Help/Notes:**
  - = help: maybe it is overwritten before being read?

---

### crates/pdftract-core/src/parser/pages.rs:1593:21

**Message:** value assigned to `inherited` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^
```

**Help/Notes:**
  - = help: maybe it is overwritten before being read?

---

### crates/pdftract-core/src/parser/xref.rs:909:21

**Message:** value assigned to `depth` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^ this value is reassigned later and never used
```

---

### crates/pdftract-core/src/annotation/links.rs:491:4

**Message:** function `extract_destination_name` is never used

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^^^^
```

**Help/Notes:**
  - = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-core/src/classify.rs:1202:5

**Message:** field `rotation` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
-------------- field in this struct
```

---

### crates/pdftract-core/src/cmap/codespace.rs:644:16

**Message:** field `0` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
---------- ^^
```

**Help/Notes:**
  - = note: `Token` has a derived impl for the trait `Debug`, but this is intentionally ignored during dead code analysis

---

### crates/pdftract-core/src/document.rs:949:5

**Message:** field `source` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
-------- field in this struct
```

---

### crates/pdftract-core/src/extract.rs:371:9

**Message:** field `page_height` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
------------------ field in this struct
```

**Help/Notes:**
  - = note: `PageResultInternal` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

---

### crates/pdftract-core/src/font/embedded.rs:202:5

**Message:** field `has_valid_encoding` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
------------ field in this struct
```

---

### crates/pdftract-core/src/font/type3_rasterizer.rs:718:5

**Message:** field `font` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
----------------- field in this struct
```

---

### crates/pdftract-core/src/layout/reading_order.rs:720:5

**Message:** fields `distance` and `angle` are never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
---- fields in this struct
```

**Help/Notes:**
  - = note: `Edge` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

---

### crates/pdftract-core/src/parser/hint_stream.rs:180:5

**Message:** fields `shared_object_number_bits`, `shared_group_length_bits`, and `shared_group_count` are never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
---------- fields in this struct
```

---

### crates/pdftract-core/src/word_boundary.rs:39:5

**Message:** field `font_id` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
-------------------- field in this struct
```

**Help/Notes:**
  - = note: `WordBoundaryDetector` has derived impls for the traits `Clone` and `Debug`, but these are intentionally ignored during dead code analysis

---

### crates/pdftract-cli/src/serve.rs:844:13

**Message:** value assigned to `pdf_bytes` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^
```

**Help/Notes:**
  - = help: maybe it is overwritten before being read?
  - = note: `#[warn(unused_assignments)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-cli/src/classify.rs:159:4

**Message:** function `canonicalize_profiles_dir` is never used

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^^^^^
```

**Help/Notes:**
  - = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-cli/src/mcp/framing/mod.rs:331:5

**Message:** field `jsonrpc` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
-------- field in this struct
```

**Help/Notes:**
  - = note: `Response` has derived impls for the traits `Debug` and `Clone`, but these are intentionally ignored during dead code analysis

---

### crates/pdftract-cli/src/mcp/tools/registry.rs:187:5

**Message:** fields `path` and `xref_section` are never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
---------- fields in this struct
```

---

### crates/pdftract-py/src/lib.rs:160:4

**Message:** function `kwargs_to_options` is never used

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^
```

**Help/Notes:**
  - = note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-core/src/font/type3_rasterizer.rs:2648:9

**Message:** value assigned to `dummy_array` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

**Help/Notes:**
  - = help: maybe it is overwritten before being read?
  - = note: `#[warn(unused_assignments)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-core/src/layout/reading_order.rs:112:28

**Message:** value assigned to `region_count` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^ this value is reassigned later and never used
```

---

### crates/pdftract-cli/src/grep/worker.rs:467:9

**Message:** value assigned to `last_font_size` is never read

**Type:** dead_code

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
```

**Help/Notes:**
  - = help: maybe it is overwritten before being read?
  - = note: `#[warn(unused_assignments)]` (part of `#[warn(unused)]`) on by default

---

*... and 25 more `dead_code` warnings*


## Unused Doc Comments

**Count:** 4 warnings

**Severity:** warning (non-breaking)

### crates/pdftract-core/src/parser/object/cache.rs:50:1

**Message:** unused doc comment

**Type:** unused_doc_comments

**Severity:** warning

**Code Snippet:**
```rust
rustdoc does not generate documentation for macro invocations
```

**Help/Notes:**
  - = help: to document an item produced by a macro, the macro must produce the documentation as part of its expansion
  - = note: `#[warn(unused_doc_comments)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-core/src/parser/object/cycle.rs:33:1

**Message:** unused doc comment

**Type:** unused_doc_comments

**Severity:** warning

**Code Snippet:**
```rust
rustdoc does not generate documentation for macro invocations
```

**Help/Notes:**
  - = help: to document an item produced by a macro, the macro must produce the documentation as part of its expansion

---

### crates/pdftract-libpdftract/src/api.rs:888:1

**Message:** unused doc comment

**Type:** unused_doc_comments

**Severity:** warning

**Code Snippet:**
```rust
rustdoc does not generate documentation for macro invocations
```

**Help/Notes:**
  - = help: to document an item produced by a macro, the macro must produce the documentation as part of its expansion
  - = note: `#[warn(unused_doc_comments)]` (part of `#[warn(unused)]`) on by default

---

### crates/pdftract-core/tests/encoding_recovery.rs:229:5

**Message:** unused doc comment

**Type:** unused_doc_comments

**Severity:** warning

**Code Snippet:**
```rust
------------------------------ rustdoc does not generate documentation for statements
```

**Help/Notes:**
  - = help: use `//` for a plain comment
  - = note: `#[warn(unused_doc_comments)]` (part of `#[warn(unused)]`) on by default

---


## Deprecated

**Count:** 2 warnings

**Severity:** warning (non-breaking)

### crates/pdftract-cli/src/panic_hook.rs:7:24

**Message:** use of deprecated type alias `std::panic::PanicInfo`: use `PanicHookInfo` instead

**Type:** deprecated

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^
```

**Help/Notes:**
  - = note: `#[warn(deprecated)]` on by default

---

### crates/pdftract-cli/src/panic_hook.rs:23:49

**Message:** use of deprecated type alias `std::panic::PanicInfo`: use `PanicHookInfo` instead

**Type:** deprecated

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^^
```

---


## Unreachable Patterns

**Count:** 1 warnings

**Severity:** warning (non-breaking)

### crates/pdftract-core/src/layout/correction.rs:376:13

**Message:** unreachable pattern

**Type:** unreachable_patterns

**Severity:** warning

**Code Snippet:**
```rust
------ matches all the relevant values
```

**Help/Notes:**
  - = note: `#[warn(unreachable_patterns)]` (part of `#[warn(unused)]`) on by default

---


## Mismatched Lifetime Syntaxes

**Count:** 1 warnings

**Severity:** warning (non-breaking)

### crates/pdftract-core/src/layout/readability.rs:244:13

**Message:** hiding a lifetime that's elided elsewhere is confusing

**Type:** mismatched_lifetime_syntaxes

**Severity:** warning

**Code Snippet:**
```rust
^^^^^     ^^^^^^^^ the same lifetime is hidden here
```

**Help/Notes:**
  - = help: the same lifetime is referred to in inconsistent ways, making the signature confusing
  - = note: `#[warn(mismatched_lifetime_syntaxes)]` on by default

---


## Redundant Semicolons

**Count:** 1 warnings

**Severity:** warning (non-breaking)

### crates/pdftract-cli/src/serve.rs:736:10

**Message:** unnecessary trailing semicolon

**Type:** redundant_semicolons

**Severity:** warning

**Code Snippet:**
```rust
^ help: remove this semicolon
```

**Help/Notes:**
  - = note: `#[warn(redundant_semicolons)]` (part of `#[warn(unused)]`) on by default

---


## Non Snake Case

**Count:** 1 warnings

**Severity:** warning (non-breaking)

### crates/pdftract-py/src/lib.rs:130:9

**Message:** variable `PyErr` should have a snake case name

**Type:** non_snake_case

**Severity:** warning

**Code Snippet:**
```rust
^^^^^ help: convert the identifier to snake case: `py_err`
```

**Help/Notes:**
  - = note: `#[warn(non_snake_case)]` (part of `#[warn(nonstandard_style)]`) on by default

---


## Noop Method Call

**Count:** 1 warnings

**Severity:** warning (non-breaking)

### crates/pdftract-py/src/lib.rs:297:12

**Message:** call to `.clone()` on a reference in this situation does nothing

**Type:** noop_method_call

**Severity:** warning

**Code Snippet:**
```rust
^^^^^^^^
```

**Help/Notes:**
  - = note: the type `PyDict` does not implement `Clone`, so calling `clone` on `&PyDict` copies the reference, which does not do anything and can be removed
  - = note: `#[warn(noop_method_call)]` on by default

---

