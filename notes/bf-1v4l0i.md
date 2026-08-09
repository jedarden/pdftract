# Unused Import Removal - bf-1v4l0i

**Task:** Remove unused imports identified by clippy

**Inventory:** 46 unused imports listed in `bf-1v4l0i-unused-imports.txt`

**Process:**
1. Verify each import is truly unused with grep
2. Remove the import line
3. Run `cargo check --tests` after each batch
4. Document any false positives discovered

**Progress:**

## Batch 1: Simple type imports (1-5)
- [ ] 1. DestArray | crates/pdftract-core/src/annotation/json.rs:6:32
- [ ] 2. Map | crates/pdftract-core/src/cache/key.rs:10:24
- [ ] 3. entry_path | crates/pdftract-core/src/cache/lru.rs:8:5
- [ ] 4. ObjRef | crates/pdftract-core/src/detection.rs:11:29
- [ ] 5. DiagCode | crates/pdftract-core/src/encryption/detection.rs:13:31

## Batch 2: Parser imports (6-10)
- [ ] 6. intern | crates/pdftract-core/src/parser/pages.rs:14:29
- [ ] 7. PdfDict | crates/pdftract-core/src/parser/resources.rs:10:45
- [ ] 8. MemorySource | crates/pdftract-core/src/parser/xref.rs:11:29
- [ ] 9. TableSpan | crates/pdftract-core/src/table/output.rs:7:41
- [ ] 10. PdfDict | crates/pdftract-core/src/attachment/filespec.rs:583:41

## Batch 3: More PdfDict and std imports (11-20)
- [ ] 11. PdfDict | crates/pdftract-core/src/attachment/name_tree.rs:420:41
- [ ] 12. std::io::Cursor | crates/pdftract-core/src/audit.rs:183:9
- [ ] 13. indexmap::indexmap | crates/pdftract-core/src/decoder/jbig2.rs:154:9
- [ ] 14. crate::diagnostics::DiagCode | crates/pdftract-core/src/extract.rs:3299:13
- [ ] 15. crate::parser::object::intern | crates/pdftract-core/src/font/type3_rasterizer.rs:2214:13
- [ ] 16. PdfDict | crates/pdftract-core/src/font/type3_rasterizer.rs:2239:44
- [ ] 17. std::sync::Arc | crates/pdftract-core/src/font/type3_rasterizer.rs:2354:13
- [ ] 18. std::sync::Arc | crates/pdftract-core/src/font/type3_rasterizer.rs:2398:13
- [ ] 19. Mutex | crates/pdftract-core/src/font/type3_rasterizer.rs:2868:30
- [ ] 20. PdfDict | crates/pdftract-core/src/font/type3_rasterizer.rs:3073:44

## Batch 4: Type3 rasterizer continued (21-30)
- [ ] 21. crate::parser::xref::XrefResolver | crates/pdftract-core/src/font/type3_rasterizer.rs:3132:13
- [ ] 22. std::sync::Arc | crates/pdftract-core/src/font/type3_rasterizer.rs:3134:13
- [ ] 23. PdfDict | crates/pdftract-core/src/font/type3_rasterizer.rs:3161:52
- [ ] 24. PdfDict | crates/pdftract-core/src/font/type3_rasterizer.rs:3190:52
- [ ] 25. std::sync::Arc | crates/pdftract-core/src/font/type3_rasterizer.rs:3209:13
- [ ] 26. crate::parser::stream::PdfSource | crates/pdftract-core/src/font/type3_rasterizer.rs:3278:13
- [ ] 27. std::sync::Arc | crates/pdftract-core/src/font/type3_rasterizer.rs:3634:13
- [ ] 28. std::sync::Arc | crates/pdftract-core/src/font/type3_rasterizer.rs:3669:13
- [ ] 29. std::sync::Arc | crates/pdftract-core/src/font/type3_rasterizer.rs:3702:13
- [ ] 30. crate::font::encoding::NamedEncoding | crates/pdftract-core/src/font/type3_rasterizer_test.rs:23:5

## Batch 5: Test and remaining imports (31-46)
- [ ] 31. crate::graphics_state::Matrix3x3 | crates/pdftract-core/src/font/type3_rasterizer_test.rs:26:5
- [ ] 32. std::sync::Arc | crates/pdftract-core/src/forms/mod.rs:834:9
- [ ] 33. super::* | crates/pdftract-core/src/layout/correction.rs:1207:9
- [ ] 34. std::io::Cursor | crates/pdftract-core/src/output/ndjson/frames.rs:286:9
- [ ] 35. super::* | crates/pdftract-core/src/output/ndjson/pipeline.rs:140:9
- [ ] 36. std::hash::Hasher | crates/pdftract-core/src/page_class.rs:248:13
- [ ] 37. std::sync::Arc | crates/pdftract-core/src/parser/ocg.rs:428:9
- [ ] 38. Hasher | crates/pdftract-core/src/parser/stream.rs:1914:31
- [ ] 39. secrecy::ExposeSecret | crates/pdftract-core/src/parser/stream.rs:3983:9
- [ ] 40. secrecy::SecretString | crates/pdftract-core/src/parser/stream.rs:5104:13
- [ ] 41. Jbig2GlobalsRef | crates/pdftract-core/src/parser/stream.rs:6125:51
- [ ] 42. crate::parser::object::intern | crates/pdftract-core/src/parser/xref.rs:3670:13
- [ ] 43. std::fs | crates/pdftract-core/src/source/mmap.rs:178:9
- [ ] 44. super::* | crates/pdftract-core/src/sdk.rs:552:9
- [ ] 45. crate::font::UnicodeSource | crates/pdftract-core/src/span/mod.rs:1279:13
- [ ] 46. crate::table::Segment | crates/pdftract-core/src/table/output.rs:273:9

**False Positives Discovered:**
(None yet)

**Commits:**
(None yet)

**Test Results:**
(None yet)
