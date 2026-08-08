# False-Positive #[test] Attribute Check

**Scan Date:** 2026-08-08
**Files Scanned:** 565
**Total #[test] Functions:** 4749
**Potential False Positives:** 951

## Methodology

This scan analyzes the COMPLETE function body of each #[test] function.

### Detection Criteria

A function is flagged as a potential false positive if it:
1. Does NOT contain assertions (assert!, assert_eq!, etc.)
2. Does NOT check for errors (expect(), unwrap())
3. Does NOT verify results (.is_ok(), .is_err(), .contains())
4. Has only setup/data code without verification

### Test Categories Recognized

The following patterns are recognized as VALID tests:
- Property-based tests (fuzz_*, proptest_*, prop_*)
- Crash/panic tests (verify code doesn't panic)
- Integration tests (verify end-to-end behavior)
- Smoke tests (verify basic functionality)

## Findings

### Found 951 potential issues:

#### 📄 crates/pdftract-cer-diff/src/main.rs

**Line 256: `test_normalize_to_text` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 272: `test_normalize_empty_pages` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/benches/grep_1000.rs

**Line 1003: `bench_grep_1000` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/classify.rs

**Line 206: `test_classification_output_serialization` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 225: `test_format_json_pretty` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/doctor/checks/binary.rs

**Line 33: `test_binary_check_always_ok` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/doctor/checks/cache_dir.rs

**Line 175: `test_cache_dir_not_exists` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/doctor/checks/network.rs

**Line 93: `test_network_check_name` (2 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/doctor/checks/profile_path.rs

**Line 291: `test_profile_check_valid_directory` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 316: `test_profile_check_detects_secrets` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 342: `test_profile_check_detects_auth_token` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 372: `test_profile_check_detects_nested_secrets` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/grep/event.rs

**Line 408: `test_file_only_json_serialization` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 421: `test_count_event_json_serialization` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/grep/matcher.rs

**Line 346: `test_regex_dollar_amount` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/inspect/api.rs

**Line 1191: `test_search_match_serialization` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1210: `test_render_page_svg_basic` (20 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1268: `test_render_page_svg_thumbnail` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1300: `test_render_page_svg_empty_page` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1336: `test_render_ocr_layer` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1377: `test_extract_columns_from_spans` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/inspect/args.rs

**Line 108: `test_validate_missing_file` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 122: `test_validate_non_loopback_without_token` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 136: `test_validate_non_loopback_with_token` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 151: `test_parse_bind` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 166: `test_server_url` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/inspect/render/confidence_heatmap.rs

**Line 145: `test_render_confidence_heatmap_single_span` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 174: `test_render_confidence_heatmap_low_confidence` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 196: `test_render_confidence_heatmap_no_confidence` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/inspect/render/reading_order.rs

**Line 127: `test_render_reading_order_single_block` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 144: `test_render_reading_order_two_blocks` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 190: `test_render_reading_order_three_blocks` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 238: `test_render_reading_order_non_sequential` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 294: `test_render_reading_order_max_arrows_limit` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 318: `test_block_center` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 335: `test_block_center_fractional` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 352: `test_render_reading_order_css_class` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 384: `test_render_reading_order_out_of_bounds_indices` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/inspect/render/spans.rs

**Line 122: `test_render_spans_single` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 204: `test_render_spans_data_attributes` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 232: `test_render_spans_span_index` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 288: `test_render_spans_multiple` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 344: `test_render_spans_css_class` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 390: `test_render_spans_float_bbox` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 417: `test_render_spans_output_is_valid_svg` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/mcp/framing/mod.rs

**Line 646: `test_reject_invalid_jsonrpc_version` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 655: `test_reject_missing_jsonrpc_field` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 691: `test_notification_deserialize` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/mcp/stdio.rs

**Line 512: `test_write_response_framing` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/mcp/tools/registry.rs

**Line 1182: `test_stub_tools_return_not_implemented` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/migrate.rs

**Line 252: `test_migration_registry_identity` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 267: `test_migration_registry_unsupported` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/profiles_cmd.rs

**Line 312: `test_profiles_command_enum` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/serve.rs

**Line 1211: `test_413_json_format` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1475: `test_build_options_with_all_fields` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1519: `test_build_options_max_decompress_gb_validation` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/src/validate.rs

**Line 134: `test_minimal_valid_json_passes` (33 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/TH-08-log-audit.rs

**Line 233: `test_log_audit_no_pdf_bytes_leak` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 333: `test_log_audit_audit_log_no_leak` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/comparison_mode_test.rs

**Line 8: `test_inspect_args_has_compare_field` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 28: `test_inspect_args_validate_without_compare` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 48: `test_diff_summary_serialization` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 75: `test_page_diff_serialization` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 120: `test_compare_page_data_serialization` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 211: `test_block_match_score` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 252: `test_span_match_score` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/fixture_discovery.rs

**Line 898: `test_fixture_info_from_path_derives_name_and_description` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 937: `test_fixture_info_display` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1042: `test_discover_all_fixture_infos_result_ok` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/forms_integration.rs

**Line 54: `test_discover_pdf_fixtures` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 226: `test_form_field_structure` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 250: `test_acroform_features` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 273: `test_xfa_detection` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/mcp-http.rs

**Line 77: `test_post_tools_list` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 113: `test_post_batch_request` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 157: `test_post_single_request_returns_single_response` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 193: `test_post_payload_too_large` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 313: `test_auth_required_for_non_loopback` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 349: `test_unknown_method` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 387: `test_50_concurrent_clients` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 466: `test_health_during_load` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/mcp-stdio.rs

**Line 82: `test_tools_list_roundtrip` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 232: `test_stdout_json_rpc_only` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 290: `test_request_response_timing` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 325: `test_unknown_method` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 354: `test_notification_no_response` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/mcp-tools-integration.rs

**Line 11: `test_get_metadata_performance_on_100_page_pdf` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 40: `test_hash_performance_on_100_page_pdf` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 103: `test_phase_7_stub_tools_return_not_implemented` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 156: `test_extract_tool_with_real_pdf` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 228: `test_nonexistent_file_returns_path_invalid` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 250: `test_encrypted_pdf_returns_pdf_encrypted_error` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/multi_output_validation.rs

**Line 259: `test_text_flag_with_dash_for_stdout` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/single_page_access.rs

**Line 181: `test_multi_page_first_page_access` (3 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 212: `test_multi_page_last_page_access` (3 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/test_book_chapter.rs

**Line 548: `test_load_book_chapter_profile` (11 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/test_contract.rs

**Line 410: `test_load_contract_profile` (11 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/test_encryption_errors.rs

**Line 328: `test_encrypted_pdf_extraction_workflow` (5 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/test_form.rs

**Line 188: `test_form_profile_schema` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 327: `test_form_readme_mentions_degenerate` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/test_legal_filing.rs

**Line 590: `test_load_legal_filing_profile` (11 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/test_scientific_paper.rs

**Line 492: `test_doi_regex_format` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 513: `test_load_scientific_paper_profile` (11 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-cli/tests/test_slide_deck.rs

**Line 647: `test_load_slide_deck_profile` (11 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/annotation/json.rs

**Line 213: `test_link_to_json_uri` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 231: `test_link_to_json_named_dest` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 247: `test_link_to_json_explicit_dest` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 280: `test_annotation_to_json_highlight` (11 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 327: `test_annotation_to_json_text_note` (11 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 370: `test_sort_links` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 416: `test_sort_annotations` (10 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 482: `test_fit_type_to_json_all_variants` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 519: `test_annotation_roundtrip_serialization` (11 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 553: `test_link_roundtrip_serialization` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/annotation/links.rs

**Line 941: `test_fit_type_partial_eq` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/annotation/other.rs

**Line 360: `test_extract_highlight_annotation_with_quads` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/attachment/name_tree.rs

**Line 665: `test_decode_name_key_latin1` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 786: `test_decode_utf16be_bom` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 799: `test_decode_utf16be_raw` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/cache/key.rs

**Line 298: `test_cache_key_version_pinned` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 332: `test_cache_key_hash_eq` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 440: `test_acceptance_different_version_changes_hash` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 463: `test_acceptance_sorted_key_canonical` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 606: `test_canonical_json_mixed` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/cache/layout.rs

**Line 447: `test_index_roundtrip` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 491: `test_index_schema_version_mismatch` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/cache/lru.rs

**Line 1040: `test_eviction_sweep_performance` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/cache/multi_process.rs

**Line 629: `test_concurrent_writers_same_key` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 982: `test_stress_concurrent_access` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1084: `test_acceptance_concurrent_same_fingerprint` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/classify.rs

**Line 1351: `test_cell_data_classify_vector` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1361: `test_cell_data_classify_scanned` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1371: `test_cell_data_classify_mixed` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1524: `test_determinism_btree_set` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1544: `test_cell_index_invalid_row` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1550: `test_cell_index_invalid_col` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2322: `test_microbenchmark_classify_page_performance` (15 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2758: `test_all_tr3_with_full_page_image_exact_match` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2995: `test_image_coverage_fraction_single_image_90_percent` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/cmap/codespace.rs

**Line 906: `test_display` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/cmap/tokenize.rs

**Line 468: `test_all_bytes_0x00_to_0xff_empty_codespace` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/content_stream.rs

**Line 2275: `test_glyph_position_hint` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2329: `test_process_with_mode_simple` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2352: `test_process_with_mode_bbox_identical` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2454: `test_position_hint_faster_than_normal` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2855: `test_image_xobject_new` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2873: `test_execution_result_new` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2973: `test_overflow_diagnostic_emitted_once_per_page` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 4027: `test_resource_stack_lookup_color_space_shadowing` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 4095: `test_resource_stack_lookup_ext_gstate_shadowing` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 4129: `test_resource_stack_lookup_ext_gstate_fallback_to_page` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 4156: `test_resource_stack_lookup_ext_gstate_form_with_empty_dict` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/decoder/jpx.rs

**Line 336: `test_raw_j2k_codestream_not_valid_jp2` (3 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 385: `test_has_libopenjp2_runtime_check` (3 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/dpi.rs

**Line 373: `test_select_dpi_override` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/encryption/aes_256.rs

**Line 590: `test_decrypt_ue_or_oe_no_padding_roundtrip_no_panic` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/extract.rs

**Line 3371: `test_extraction_result_assert_exit_code_success` (22 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3403: `test_extraction_result_assert_exit_code_mismatch` (22 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3438: `test_extraction_result_assert_exit_code_with_errors` (22 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3471: `test_extraction_result_assert_exit_code_error_message` (22 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/fingerprint/mod.rs

**Line 647: `test_catalog_flags_encode` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 659: `test_catalog_flags_all_set` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 779: `test_compute_fingerprint_simple` (11 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 807: `test_compute_fingerprint_inv3_reproducibility` (11 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 833: `test_compute_fingerprint_different_page_count` (11 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 883: `test_compute_fingerprint_different_geometry` (11 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 924: `test_compute_fingerprint_different_flags` (11 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 974: `test_inv13_fingerprint_format` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1004: `test_inv13_multiple_outputs_match_format` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1088: `test_performance_100_page_pdf` (13 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/font/agl.rs

**Line 143: `test_agl_quoteright` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 149: `test_agl_uni20ac` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 155: `test_agl_u1f600` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 177: `test_agl_multi_fi` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 184: `test_agl_multi_ffi` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 191: `test_agl_multi_ff` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 198: `test_agl_multi_fl` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 211: `test_agl_multi_hebrew` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 224: `test_parse_algorithmic_uni` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 237: `test_parse_algorithmic_u` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 275: `test_agl_quotes` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 283: `test_agl_euro` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/font/cjk_encoding.rs

**Line 194: `test_decode_malformed_shift_jis` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 205: `test_decode_malformed_gb18030` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/font/cmap.rs

**Line 553: `test_parse_bfchar_fb01_ligature` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 594: `test_parse_bfrange_explicit_array` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/font/embedded.rs

**Line 662: `test_subset_font_behavior` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 783: `test_empty_font_metrics_graceful_handling` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/font/encoding.rs

**Line 821: `test_font_encoding_glyph_name_with_differences` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 847: `test_font_encoding_glyph_name_no_base` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 872: `test_font_encoding_unknown_glyph_name` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 894: `test_font_encoding_lookup_order` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/font/fingerprint.rs

**Line 240: `test_hash_stability_across_runs` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/font/mod.rs

**Line 512: `test_classify_font_opentype_cff` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/font/resolver.rs

**Line 792: `test_resolved_glyph_failure` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1133: `test_resolve_type3_fallback_to_fffd` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/font/shape.rs

**Line 430: `test_phash_deterministic` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/font/type0.rs

**Line 649: `test_descendant_get_width_default` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 665: `test_descendant_get_width_from_w` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 684: `test_descendant_get_gid_identity` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 701: `test_descendant_get_gid_cidfonttype0` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 965: `test_cid_to_gid_map_from_stream` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1024: `test_cid_to_gid_map_truncated` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/font/type3.rs

**Line 949: `test_type3_font_mock_works_with_rasterize_type3_glyph` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1089: `test_mock_works_with_rasterize_type3_glyph_complex` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1118: `test_mock_works_with_rasterize_type3_glyph_stroke` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1147: `test_mock_works_with_rasterize_type3_glyph_unknown_glyph` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/font/type3_charproc_test.rs

**Line 71: `test_charproc_simple_rectangle` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 105: `test_charproc_move_line_close` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 135: `test_charproc_multiple_shapes` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 164: `test_charproc_stroke_rectangle` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 193: `test_charproc_close_stroke_triangle` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 222: `test_charproc_empty_stream` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 253: `test_charproc_whitespace_only` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 281: `test_charproc_noop_path` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 312: `test_charproc_complex_polygon` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 341: `test_charproc_consistent_rendering` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/font/type3_rasterizer.rs

**Line 1957: `test_rasterize_type3_glyph_unknown_returns_none` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2068: `test_deref_char_proc_ref_without_resolver_returns_error` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2089: `test_deref_char_proc_ref_without_source_returns_error` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2113: `test_type3_error_missing_char_proc_ref` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2125: `test_type3_error_circular_ref` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2197: `test_type3_error_invalid_char_proc_type` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2211: `test_deref_char_proc_ref_validates_structure_before_returning` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2238: `test_deref_char_proc_ref_validation_includes_ref_context` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2260: `test_deref_char_proc_ref_passes_valid_stream` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2284: `test_detect_char_proc_type_returns_unknown_for_failed_deref` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2307: `test_extract_content_stream_bytes_without_resolver_returns_type3_error` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2328: `test_rasterize_type3_glyph_with_missing_glyph_returns_none` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2352: `test_rasterize_type3_glyph_with_failed_resolution_returns_none` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2396: `test_rasterize_type3_glyph_with_malformed_stream_returns_none` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2481: `test_execute_type3_glyph_with_font_matrix_transformation` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2537: `test_execute_type3_glyph_with_identity_font_matrix` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2685: `test_resolve_stream_callback_receives_parameters` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2768: `test_resolve_stream_callback_captures_context_parameters` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2866: `test_resolve_stream_callback_with_helper_function_pattern` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2984: `test_detect_char_proc_type_stream` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3073: `test_detect_char_proc_type_indirect` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3090: `test_detect_char_proc_type_with_context_direct_stream` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3131: `test_detect_char_proc_type_with_context_ref_with_valid_context` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3161: `test_detect_char_proc_type_with_context_ref_to_dict` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3190: `test_detect_char_proc_type_with_context_circular_reference` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3242: `test_detect_char_proc_type_with_context_ref_to_integer` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3258: `test_detect_char_proc_type_with_context_ref_without_resolver` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 3276: `test_detect_char_proc_type_with_context_ref_without_source` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 3298: `test_detect_char_proc_type_backwards_compatibility` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3332: `test_validate_char_proc_structure_valid_stream` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3348: `test_validate_char_proc_structure_stream_missing_type` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3372: `test_validate_char_proc_structure_stream_missing_subtype` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3396: `test_validate_char_proc_structure_stream_missing_width` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3420: `test_validate_char_proc_structure_stream_missing_height` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3444: `test_validate_char_proc_structure_stream_missing_all_keys` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3464: `test_validate_char_proc_structure_valid_dict` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3477: `test_validate_char_proc_structure_dict_missing_type` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3498: `test_validate_char_proc_structure_dict_missing_subtype` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3614: `test_validate_char_proc_structure_error_message_formatting` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3632: `test_fill_polygon_edge_activation_at_y_min` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3667: `test_fill_polygon_edge_removal_after_y_max` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3700: `test_fill_polygon_intersection_x_accuracy` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 4278: `test_intersection_x_positive_values` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4294: `test_intersection_x_negative_values` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4310: `test_intersection_x_half_cases` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4358: `test_intersection_x_with_various_integer_inputs` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4394: `test_edge_x_field_access_from_aet` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4489: `test_round_x_integration_with_edge_intersection_x` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4516: `test_aet_intersection_collection_loop` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4719: `test_mock_works_with_rasterize_type3_glyph` (2 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/font/type3_rasterizer_test.rs

**Line 102: `test_resolve_stream_callback_captures_resolver` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 136: `test_resolve_stream_callback_captures_source` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 170: `test_resolve_stream_callback_captures_counter` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 244: `test_resolve_stream_callback_returns_none` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 269: `test_resolve_stream_callback_returns_valid_bytes` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 303: `test_resolve_stream_helper_function_pattern` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 372: `test_edge_activation_at_y_min` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 425: `test_edge_removal_after_y_max` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 467: `test_intersection_x_calculation` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 543: `test_slope_based_x_increment` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 696: `test_aet_sorting_by_x_position` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/font/type3_test_fixtures.rs

**Line 1056: `test_main_content_stream_no_compile_errors` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/forms/combiner.rs

**Line 383: `test_combine_both_overlapping` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 421: `test_xfa_boolean_to_checkbox` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 441: `test_empty_xfa_wins_over_nonempty_acro` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 487: `test_choice_value_single` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 515: `test_choice_value_multi_select` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/forms/mod.rs

**Line 1411: `test_acro_form_field_flag_accessors` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1447: `test_acro_form_field_btn_flag_accessors` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1477: `test_extract_values_tx_btn_ch_critical` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1611: `test_extract_values_skips_sig_fields` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1704: `test_extract_values_selected_radio` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1739: `test_extract_values_multi_select_list` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1785: `test_extract_values_combo_with_opt_tuples` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1831: `test_extract_values_multiline_text` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/forms/value_text.rs

**Line 544: `test_decode_pdf_string_pdfdocencoding_lower_latin1` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 572: `test_decode_pdf_string_pdfdocencoding_quotes` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/glyph/mod.rs

**Line 996: `test_glyph_replacement_char` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/graphics_state.rs

**Line 1085: `test_64_q_plus_64_q_restores_initial_state` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/hybrid.rs

**Line 864: `test_process_hybrid_page_no_duplicate_text_from_overlap` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 927: `test_process_hybrid_page_low_vector_confidence_ocr_wins` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 978: `test_process_hybrid_page_non_hybrid_classification` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 1016: `test_process_hybrid_page_empty_hybrid_cells` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/layout/code.rs

**Line 376: `test_classify_code_all_courier_indented` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 389: `test_classify_code_not_indented` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 402: `test_classify_code_mixed_font` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 420: `test_classify_code_one_serif_at_end` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 442: `test_classify_code_fixed_pitch_flag` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 454: `test_compute_column_baseline` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 495: `test_compute_column_baseline_no_paragraphs` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 520: `test_classify_page_code_blocks` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/layout/columns.rs

**Line 1052: `test_detect_column_gaps_leading_and_trailing` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1094: `test_confirm_columns_two_column_both_confirmed` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1118: `test_confirm_columns_two_column_one_confirmed` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1140: `test_confirm_columns_single_column_confirmed` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1157: `test_confirm_columns_single_column_insufficient_lines` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1183: `test_confirm_columns_no_gaps_insufficient_lines` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1198: `test_confirm_columns_exactly_three_lines` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1213: `test_confirm_columns_three_column_all_confirmed` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1243: `test_confirm_columns_three_column_middle_insufficient` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1271: `test_confirm_columns_lines_in_gap_unassigned` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1298: `test_confirm_columns_lines_with_no_spans` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1319: `test_confirm_columns_leading_gap` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1336: `test_confirm_columns_trailing_gap` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/layout/correction.rs

**Line 1233: `test_clean_utf8_no_change` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1259: `test_mojibake_detected_and_repaired` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1277: `test_mojibake_multiple_indicators` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1294: `test_mojibake_single_indicator_threshold` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1431: `test_mixed_ascii_and_mojibake` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1481: `test_multiple_mojibake_patterns` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1503: `test_exact_epsilon_boundary` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1540: `test_hyphenation_join_basic` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1562: `test_hyphenation_capital_start_no_join` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1583: `test_hyphenation_not_at_right_edge` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1602: `test_hyphenation_different_columns` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1621: `test_hyphenation_soft_hyphen` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1641: `test_hyphenation_non_breaking_hyphen` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1661: `test_hyphenation_empty_span_removed` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1683: `test_hyphenation_multi_word_continuation` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1704: `test_hyphenation_multiple_repairs` (11 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1787: `test_normalize_word_breaks_latin_zero_width_space` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1797: `test_normalize_word_breaks_latin_bom` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1807: `test_normalize_word_breaks_latin_zwnj_zwj` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1817: `test_normalize_word_breaks_arabic_preserves_zwnj_zwj` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1828: `test_normalize_word_breaks_arabic_strips_zw_space` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1838: `test_normalize_word_breaks_arabic_strips_bom` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1848: `test_normalize_word_breaks_unknown_script_strips_all` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1858: `test_normalize_word_breaks_devanagari_preserves_zwnj_zwj` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1868: `test_normalize_word_breaks_devanagari_strips_zw_space` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1878: `test_normalize_word_breaks_auto_detect_latin` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1888: `test_normalize_word_breaks_auto_detect_arabic` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1898: `test_normalize_word_breaks_auto_detect_devanagari` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1918: `test_normalize_word_breaks_multiple_zero_width_chars` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1928: `test_normalize_word_breaks_hebrew_preserves_joiners` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1938: `test_normalize_word_breaks_thai_preserves_joiners` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1948: `test_normalize_word_breaks_bengali_preserves_joiners` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1958: `test_normalize_word_breaks_indic_preserves_joiners` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2005: `test_ligature_repair_fi_adjacent` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2066: `test_ligature_repair_no_adjacent_ligature` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2148: `test_ligature_repair_gap_too_large` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2202: `test_ligature_repair_fl_ligature` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2255: `test_ligature_repair_fl_with_l_following` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2308: `test_ligature_repair_multiple_fffd` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2544: `test_ligature_is_component` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2555: `test_ligature_repair_ffi_ligature` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2624: `test_ligature_repair_ffl_ligature` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2693: `test_ligature_repair_ff_ligature` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/layout/header_footer.rs

**Line 662: `test_detect_headers_and_footers_ten_pages_footer_with_page_numbers` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/layout/line.rs

**Line 1050: `test_line_accessors` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1459: `test_classify_heading_18pt_block_12pt_body_one_line_heading` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1478: `test_classify_heading_14pt_block_12pt_body_one_line_not_heading` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1498: `test_classify_heading_18pt_block_three_lines_not_heading` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1517: `test_classify_heading_12pt_block_12pt_body_not_heading` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1537: `test_classify_heading_threshold_exactly_1_2_not_heading` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1557: `test_classify_heading_threshold_just_above_1_2_is_heading` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1577: `test_classify_heading_empty_lines_not_heading` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1592: `test_classify_heading_two_lines_not_heading` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1610: `test_classify_heading_small_page_body_median` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1630: `test_classify_heading_large_page_body_median` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1650: `test_classify_page_headings_multiple` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/layout/readability.rs

**Line 562: `test_all_replacement_chars` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 636: `test_ligature_split_penalty` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/layout/reading_order.rs

**Line 1008: `test_xy_cut_small_region_count` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1105: `test_xy_cut_result_docstrum_trigger` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1241: `test_docstrum_k_nearest_neighbors` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/markdown.rs

**Line 1476: `test_block_to_markdown_heading_with_anchor` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1559: `test_roundtrip_extract_and_parse` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1611: `test_block_to_markdown_formula_display` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1692: `test_page_to_markdown_with_nested_list` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2635: `test_threads_to_markdown_single_thread` (14 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2660: `test_threads_to_markdown_multiple_threads` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2697: `test_threads_to_markdown_untitled_thread` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2715: `test_collapse_page_ranges_single_page` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2725: `test_collapse_page_ranges_contiguous` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2745: `test_collapse_page_ranges_gaps` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2765: `test_collapse_page_ranges_mixed` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2826: `test_emit_table_simple_3x3` (33 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2876: `test_emit_table_merged_cells_html_fallback` (23 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2917: `test_emit_table_rowspan_html_fallback` (23 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2975: `test_emit_table_with_pipe_in_cell` (23 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3011: `test_emit_table_with_newline_in_cell` (23 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3045: `test_emit_table_empty` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3064: `test_emit_table_single_row` (14 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3093: `test_emit_table_no_header` (23 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3130: `test_emit_html_table_header_cells` (23 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3168: `test_emit_html_table_row_and_colspan` (23 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3203: `test_emit_gfm_table_variable_width` (25 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3246: `test_page_to_markdown_with_links_and_footnotes_emits_footnote_ref_and_def` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3327: `test_page_to_markdown_with_links_and_footnotes_no_footnotes_emits_no_markers` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3387: `test_page_to_markdown_with_links_and_footnotes_emits_inline_link` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3461: `test_page_to_markdown_with_links_emits_internal_page_link` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3520: `test_markdown_no_page_breaks_omits_horizontal_rule` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3565: `test_markdown_with_page_breaks_emits_horizontal_rule` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3610: `test_spans_to_markdown_with_links_and_footnotes_footnote_takes_precedence` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3655: `test_block_to_markdown_with_links_and_footnotes_empty_footnotes` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/ocr.rs

**Line 637: `test_resolve_tessdata_path_explicit` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 663: `test_resolve_tessdata_path_explicit_overrides_env` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1491: `test_hocr_word_width_height` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1503: `test_hocr_word_confidence` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1602: `benchmark_hocr_parsing` (11 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1637: `test_hocr_word_equality` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1699: `test_to_pdf_bbox_basic_conversion` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1742: `test_to_pdf_bbox_y_flip_sanity` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1775: `test_to_pdf_bbox_padding_subtraction` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1798: `test_to_pdf_bbox_different_dpi` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1835: `test_to_pdf_bbox_hybrid_cell_offset` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1864: `test_to_pdf_bbox_clamps_negative_coords` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1882: `test_to_pdf_bbox_rotation_90` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1903: `test_to_pdf_bbox_rotation_180` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1919: `test_to_pdf_bbox_rotation_270` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1935: `test_to_pdf_bbox_invalid_rotation` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1964: `test_apply_rotation_to_bbox_preserves_dimensions` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2161: `test_run_tesseract_returns_spans` (3 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2184: `test_run_tesseract_on_cell_offset` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2690: `test_validation_filter_near_glyph` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2709: `test_validation_filter_far_from_glyph` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2727: `test_validation_filter_confidence_already_below_cap` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2744: `test_validation_filter_no_glyphs` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2761: `test_validation_filter_multiple_words_preserves_order` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2802: `test_validation_filter_distance_threshold` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2828: `test_region_level_policy_high_confidence_region` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2858: `test_region_level_policy_low_confidence_region` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2882: `test_region_level_policy_medium_confidence_region` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2906: `test_region_level_policy_multiple_regions` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2942: `test_group_words_by_region_single_word` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/ocr/preprocessing/contrast.rs

**Line 222: `test_histogram_stretch_normal_range` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 299: `test_histogram_stretch_full_range` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 318: `test_histogram_stretch_narrow_range` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/ocr/preprocessing/denoise.rs

**Line 64: `test_median_denoise_creates_output` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 94: `test_median_denoise_preserves_uniform_image` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 110: `test_median_denoise_preserves_uniform_black` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 173: `test_median_denoise_salt_noise_removed` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 194: `test_median_denoise_pepper_noise_removed` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/ocr/preprocessing/otsu.rs

**Line 143: `test_otsu_binary_output_only` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 170: `test_otsu_uniform_image` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/ocr/preprocessing/sauvola.rs

**Line 354: `test_sauvola_uniform_image` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/options.rs

**Line 619: `test_extraction_options_deserialize` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 669: `test_extraction_options_serialize_ocr_language` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 773: `test_output_options_deserialize` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 783: `test_extraction_options_with_output` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/output/inspector/layers.rs

**Line 645: `test_reading_order_max_arrows_limit` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/output/json.rs

**Line 308: `test_result_to_output_basic` (22 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 342: `test_page_result_to_page_json` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 388: `test_compute_extraction_quality` (51 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/output/ndjson/buffer.rs

**Line 441: `test_backpressure_blocks_when_full` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/output/ndjson/frames.rs

**Line 288: `test_ndjson_frame_header_discriminator` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 305: `test_ndjson_frame_page_discriminator` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 347: `test_write_frame_includes_newline_and_flush` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 367: `test_roundtrip_header_frame` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 398: `test_roundtrip_page_frame` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/output/pipeline.rs

**Line 326: `test_multi_sink_pipeline_cross_format_consistency` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/page_class.rs

**Line 268: `test_page_classification_serialize_hybrid_with_cells` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 564: `test_page_type_enum_schema_set` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/parser/catalog.rs

**Line 722: `test_page_label_format` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 736: `test_page_labels_tree_get_label` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 989: `test_page_label_format_with_prefix` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1009: `test_page_labels_tree_with_prefix` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1091: `test_mark_info_requires_coverage_check` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/parser/hint_stream.rs

**Line 752: `test_hint_table_predict_page_range` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 776: `test_hint_table_page_count` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/parser/lexer/mod.rs

**Line 2050: `name_proptest_never_panics_on_random_bytes` (3 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2068: `name_proptest_always_produces_valid_token` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/parser/object/cache.rs

**Line 483: `test_lru_eviction` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 628: `test_peek_lru` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 656: `test_is_lru` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/parser/object/cycle.rs

**Line 272: `test_capacity_sufficient_for_typical_depth` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/parser/object/parser.rs

**Line 920: `test_depth_exceeded_at_256` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1077: `test_parse_indirect_object_simple` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1134: `test_parse_indirect_object_integer_overflow` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/parser/object/types.rs

**Line 484: `test_pdf_dict_roundtrip_order` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 537: `test_as_stream` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 614: `test_pdf_stream_len_hint` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 629: `test_pdf_stream_no_len_hint` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 641: `test_pdf_indirect` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 651: `test_pdf_object_indirect_variant` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/parser/objstm.rs

**Line 572: `test_obj_stm_error_display` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 779: `test_missing_key_n` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 792: `test_missing_key_first` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/parser/ocg.rs

**Line 704: `test_unknown_ocg_treated_as_visible` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 765: `test_ocmd_evaluation_all_on` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 791: `test_ocmd_evaluation_any_on` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/parser/outline.rs

**Line 904: `test_decode_pdfdocencoding_bullet` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 913: `test_decode_pdfdocencoding_em_dash` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 922: `test_decode_pdfdocencoding_fi_ligature` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 931: `test_dest_anchor_xyz` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1304: `test_parse_outlines_goto_action` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/parser/stream.rs

**Line 1915: `test_asciihex_roundtrip_random` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2174: `test_jpxstream_name` (3 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 4195: `test_bomb_limit_enforcement` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 4233: `test_flate_decode_bomb_limit` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 4339: `test_document_level_bomb_limit` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 4429: `test_th01_decompression_bomb_abort` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 4719: `test_bytes_per_pixel` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4738: `test_bytes_per_row` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4766: `test_tiff_predictor_2_grayscale` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4779: `test_tiff_predictor_2_rgb` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4792: `test_png_predictor_10_none` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4806: `test_png_predictor_11_sub` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4820: `test_png_predictor_12_up` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4838: `test_png_predictor_13_average` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4852: `test_png_predictor_14_paeth` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4867: `test_png_predictor_15_optimum_all_selectors` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4900: `test_png_predictor_rgb_sub` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4914: `test_png_predictor_rgba_up` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4935: `test_png_predictor_invalid_selector` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 4995: `test_flate_decode_bomb_limit_with_predictor` (3 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 5060: `test_predictor_with_odd_bits_per_component` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 5071: `test_predictor_multiple_rows_tiff` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 5087: `test_png_predictor_selector_0` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 5101: `test_png_predictor_selector_1` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 5116: `test_extraction_options_deserialize_password` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 5171: `test_png_predictor_14_rgba_paeth` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 5234: `test_flate_decode_performance_100mb` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 5320: `test_png_predictor_budget_enforcement_small_fixture` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 5376: `test_tiff_predictor_2_budget_enforcement_small_fixture` (2 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 5427: `test_png_predictor_multiple_selectors_budget_per_row` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 5479: `test_tiff_predictor_2_rgb_budget_enforcement` (3 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 6137: `test_jbig2_extract_globals_ref` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/parser/struct_tree.rs

**Line 2232: `test_block_kind_heading_h` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2242: `test_block_kind_heading_h1` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2250: `test_block_kind_heading_h2` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2258: `test_block_kind_heading_all_levels` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2464: `test_mapping_result_for_heading_with_level` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3270: `test_compute_coverage_below_threshold` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3338: `test_compute_coverage_above_threshold` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3441: `test_compute_coverage_threshold_edge_case` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3508: `test_compute_coverage_with_orphan_mcids` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3573: `test_check_coverage_suspects_false_low_coverage` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3653: `test_check_coverage_suspects_true_high_coverage` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3725: `test_check_coverage_suspects_true_low_coverage` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 3820: `test_check_coverage_multi_page_one_fallback` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3914: `test_check_coverage_no_marked_content` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/parser/xref.rs

**Line 2353: `test_add_entry` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2366: `test_get_entry` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2433: `test_xref_section_add_entry` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2455: `test_xref_entry_in_use` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2470: `test_xref_entry_free` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2485: `test_xref_entry_compressed` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2500: `test_xref_resolver_from_section` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2556: `test_parse_simple_xref_space_newline` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2623: `test_parse_xref_carriage_return_newline` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2661: `test_parse_xref_lf_only_19_byte_entries` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2700: `test_parse_multi_subsection_xref` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2739: `test_parse_xref_with_malformed_entry` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2839: `test_parse_xref_entry_20_byte` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 2858: `test_parse_xref_entry_free` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 3011: `test_forward_scan_with_generations` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 3120: `test_forward_scan_multi_revision` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 3340: `test_parse_xref_stream_multi_subsection` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 3388: `test_parse_xref_stream_field_width_zero_gen` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 3427: `test_parse_xref_stream_type2_compressed` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 3505: `test_parse_xref_stream_invalid_entry_type` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 3841: `test_merge_hybrid_traditional_priority` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 3900: `test_merge_hybrid_free_inuse_conflict` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 3940: `test_merge_hybrid_gap_fill` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 4056: `test_merge_hybrid_stream_only` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 4085: `test_merge_hybrid_traditional_only` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 4232: `test_merge_linearized_xrefs` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 4310: `test_merge_linearized_xrefs_conflict_free_vs_inuse` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 4344: `test_merge_linearized_xrefs_empty_first_page` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 4784: `test_prev_chain_depth_limit` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/preprocess.rs

**Line 368: `test_pix_to_grayimage_roundtrip` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/profiles/apply_profile.rs

**Line 186: `test_apply_extraction_tuning` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 209: `test_apply_extraction_tuning_partial` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/profiles/engine.rs

**Line 640: `test_classify_invoice_profile` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 668: `test_classify_scientific_paper_profile` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 703: `test_classify_below_threshold_returns_unknown` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 724: `test_classify_score_normalization` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 757: `test_classify_runner_up` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 792: `test_classify_tie_breaking_by_predicate_count` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 840: `test_reason_ordering_reproducible` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 914: `test_text_contains_min_hits` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 937: `test_text_contains_below_min_hits` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 960: `test_page_count_in_range` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 983: `test_page_count_outside_range` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1005: `test_font_diversity_in_range` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1027: `test_heading_depth_at_least` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 1048: `test_heading_depth_below_threshold` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 1068: `test_glyph_density_in_range` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1090: `test_has_footer_page_numbers` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1108: `test_structural_has_table` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 1129: `test_structural_has_table_below_min` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 1161: `test_classify_determinism` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1203: `test_custom_threshold` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1225: `test_exhaustive_match_predicate` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/profiles/extraction.rs

**Line 318: `test_match_expr_any` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 373: `test_field_spec_simple` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 422: `test_full_profile_roundtrip` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/profiles/match_eval.rs

**Line 403: `test_text_contains_match` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 415: `test_text_contains_no_match` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 426: `test_heading_matches` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 437: `test_has_currency_pattern` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 448: `test_structural_has_table` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 466: `test_match_expr_all` (11 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 491: `test_match_expr_any` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 509: `test_match_expr_none` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 524: `test_match_expr_complex` (18 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/profiles/signals.rs

**Line 451: `test_math_operator_regex_matches` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/profiles/types.rs

**Line 309: `test_match_predicate_text_contains_serialization` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 341: `test_match_predicate_structural_serialization` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 354: `test_match_predicate_page_count_range_serialization` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 369: `test_profile_roundtrip` (16 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 444: `test_load_profile_from_yaml_with_all_predicate_kinds` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/receipts/mod.rs

**Line 282: `test_content_hash_nfc_normalization` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/receipts/svg.rs

**Line 443: `test_svg_generator_empty_glyph_list` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 459: `test_svg_generator_filters_glyphs_by_bbox` (17 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 489: `test_svg_output_is_valid_xml` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 509: `test_svg_output_no_external_references` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 534: `test_svg_viewbox_normalization` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 580: `test_svg_groups_by_color` (24 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 644: `test_svg_validates_via_quick_xml` (20 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 690: `test_svg_handles_missing_glyph_outline` (27 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 771: `test_svg_aggregate_size_estimate` (24 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/receipts/verifier.rs

**Line 352: `test_compute_content_hash_nfc_normalization` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 416: `test_verify_receipt_success` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 436: `test_verify_receipt_fingerprint_mismatch` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 456: `test_verify_receipt_bbox_mismatch` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 477: `test_verify_receipt_content_mismatch` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 498: `test_verify_receipt_best_match_selected` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 528: `test_iou_threshold_verification` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 550: `test_iou_threshold_pass` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 570: `test_verify_receipt_with_unicode_normalization` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/render/image_compositing.rs

**Line 1604: `test_image_xobject_with_inline` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/render/pdfium_path.rs

**Line 290: `test_render_invalid_page_index` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/schema/mod.rs

**Line 1601: `test_span_json_serialization` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1630: `test_span_json_with_confidence` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1652: `test_span_json_with_receipt` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1681: `test_block_json_serialization` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1703: `test_block_json_heading_with_level` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1722: `test_block_json_with_receipt` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1746: `test_receipt_not_in_json_when_none` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1772: `test_schema_stability` (17 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1857: `test_extraction_quality_serialization` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1879: `test_extraction_quality_serialization_minimal` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1922: `test_table_json_serialization` (33 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1972: `test_table_json_borderless` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1989: `test_table_json_continued_flags` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2009: `test_table_json_continued_from_prev` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2029: `test_row_json_serialization` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2053: `test_cell_json_serialization` (8 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2079: `test_v_1_0_table_schema_roundtrip` (61 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2167: `test_tables_array_emitted_on_page_output` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2219: `test_table_block_emission_shape` (4 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2366: `test_document_metadata_optional_fields_skipped` (18 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2412: `test_document_metadata_with_all_fields` (18 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2458: `test_outline_node_serialization` (12 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2488: `test_outline_node_nested` (20 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2525: `test_destination_json_xyz` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2548: `test_page_json_minimal` (11 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2581: `test_page_json_with_content` (48 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2647: `test_diagnostic_json_serialization` (9 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2676: `test_diagnostic_json_document_level` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2697: `test_output_roundtrip` (11 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2760: `test_page_json_with_page_labels_roman_numerals` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2822: `test_page_json_without_page_labels_absent` (11 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2889: `test_page_json_roundtrip_with_all_fields` (32 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/semaphore.rs

**Line 179: `test_semaphore_blocking` (2 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/signature/mod.rs

**Line 1181: `test_extract_signature_metadata_full` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1217: `test_extract_signature_metadata_unsigned` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1242: `test_extract_signature_metadata_missing_optional_fields` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1274: `test_extract_signatures_multiple` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1381: `test_coverage_fraction_full_coverage` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1409: `test_coverage_fraction_partial` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1437: `test_coverage_fraction_no_file_size` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1465: `test_coverage_fraction_invalid_byte_range` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/source/http_range.rs

**Line 810: `test_http_range_source_with_headers` (5 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/span/mod.rs

**Line 1978: `test_assemble_text_rtl_arabic_preserved_in_source_order` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 2147: `test_assemble_text_preserves_special_unicode_chars` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/table/detector.rs

**Line 957: `test_detect_5x3_table` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1201: `test_group_by_x0_tolerance` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 1233: `test_find_row_candidates_basic` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 1299: `test_is_single_column_reflow_true` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 1337: `test_is_single_column_reflow_false` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/table/mod.rs

**Line 73: `test_page_context_creation` (14 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/text.rs

**Line 437: `test_serialize_page_text_code` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 745: `test_serialize_page_text_invisible_span_filtered` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 769: `test_serialize_page_text_invisible_span_included_when_flagged` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 792: `test_serialize_page_text_all_invisible_block_omitted` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 837: `test_serialize_page_text_mixed_blocks_with_invisible` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 901: `test_serialize_document_text_ten_pages` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/src/word_boundary.rs

**Line 373: `test_detector_recalibration_after_20_samples` (3 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 415: `test_detector_reset` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 464: `test_manager_reset_font` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/TH-03-mcp-no-auth.rs

**Line 476: `test_case_7_localhost_without_token` (2 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/TH-04-js-presence.rs

**Line 97: `test_no_javascript` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 161: `test_json_output_includes_javascript_actions` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/TH-05-ssrf-block.rs

**Line 1157: `test_is_ssrf_blocked_error_with_code_in_data` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1174: `test_is_ssrf_blocked_error_with_message` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1190: `test_is_ssrf_blocked_error_not_blocked` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1206: `test_is_ssrf_blocked_error_success_response` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1219: `test_extract_error_info_success` (7 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1238: `test_extract_error_info_not_an_error` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1251: `test_parse_response_success` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1264: `test_parse_response_error` (6 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1281: `test_parse_response_invalid_json` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1290: `test_parse_response_missing_jsonrpc_field` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1298: `test_read_framed_response_simple` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1309: `test_read_framed_response_with_extra_whitespace` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1330: `test_read_framed_response_missing_content_length` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1340: `test_write_framed_message_simple` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1354: `test_tool_call_result_success` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1365: `test_tool_call_result_error` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1386: `test_tool_call_result_has_error_code` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1399: `test_tool_call_result_has_error_code_no_data` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1410: `test_tool_call_result_has_error_code_malformed_data` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1424: `test_json_rpc_error_is_ssrf_blocked_with_code_in_data` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1434: `test_json_rpc_error_is_ssrf_blocked_with_message` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1444: `test_json_rpc_error_is_ssrf_blocked_not_blocked` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1454: `test_json_rpc_error_is_ssrf_blocked_empty_data` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1464: `test_json_rpc_error_is_ssrf_blocked_different_code_in_data` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1474: `test_json_rpc_error_is_ssrf_blocked_case_sensitive_in_message` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1484: `test_json_rpc_error_is_ssrf_blocked_case_sensitive_in_data` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1494: `test_json_rpc_error_is_ssrf_blocked_partial_match_in_message` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1504: `test_json_rpc_error_is_ssrf_blocked_both_data_and_message` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1516: `test_standalone_is_ssrf_blocked_with_code_in_data` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1527: `test_standalone_is_ssrf_blocked_with_message` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1538: `test_standalone_is_ssrf_blocked_not_blocked` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1549: `test_standalone_is_ssrf_blocked_empty_data` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1560: `test_standalone_is_ssrf_blocked_different_code_in_data` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1571: `test_standalone_is_ssrf_blocked_case_sensitive_in_message` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1582: `test_standalone_is_ssrf_blocked_case_sensitive_in_data` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1593: `test_standalone_is_ssrf_blocked_partial_match_in_message` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 1604: `test_standalone_is_ssrf_blocked_both_data_and_message` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 2174: `test_mcp_process_cleanup_on_completion` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/classifier_corpus.rs

**Line 298: `test_classifier_corpus_accuracy` (3 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/cmap_unmapped_glyphs.rs

**Line 338: `test_differences_overlay_filters_unmapped_glyphs` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 487: `test_differences_overlay_consecutive_with_unmapped_filtering` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 575: `test_differences_overlay_filters_null_glyph` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 619: `test_differences_overlay_filters_all_g_series_unmapped` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/debug_fingerprint.rs

**Line 85: `debug_direct_content_stream_hash` (13 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/document_model.rs

**Line 11: `debug_ocg_default_off` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 296: `test_encrypted_rc4` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 302: `test_encrypted_aes128` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 308: `test_encrypted_aes256` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 314: `test_encrypted_empty_password` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 320: `test_encrypted_unknown_handler` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 326: `test_tagged_3_level_outline` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 332: `test_ocg_default_off` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 338: `test_multi_revision_3` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 344: `test_inheritance_grandparent_mediabox` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 350: `test_missing_mediabox` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 356: `test_partial_resource_override` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 362: `test_js_in_openaction` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 368: `test_xfa_form` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 374: `test_pdfa_1b_conformance` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/encryption_aes_128_test.rs

**Line 121: `test_aes_128_decrypt_roundtrip_with_valid_padding` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 167: `test_aes_128_decrypt_fails_with_corrupted_padding` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 211: `test_aes_128_decrypt_wrong_key_produces_garbage` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 298: `test_aes_128_decrypt_one_block` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 339: `test_aes_128_decrypt_multiple_blocks` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/encryption_aes_256_test.rs

**Line 187: `test_aes256_decrypt_stream_roundtrip` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 238: `test_aes256_decrypt_stream_fails_with_corrupted_padding` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 285: `test_aes_256_decrypt_convenience` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 414: `test_aes256_decrypt_multiple_blocks` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 461: `test_aes256_decrypt_one_block` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 507: `test_aes256_key_sensitivity` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/fingerprint_reproducibility.rs

**Line 161: `test_inv13_fingerprint_format` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/http_range_integration.rs

**Line 326: `test_boundary_conditions` (3 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/json_schema.rs

**Line 343: `test_synthetic_output_validates` (33 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 400: `debug_list_available_fixtures` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/object_parser.rs

**Line 245: `test_all_fixtures` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 254: `test_nested_dict` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 259: `test_mixed_array` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 264: `test_indirect_simple` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 269: `test_indirect_stream` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 274: `test_objstm_basic` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 279: `test_objstm_extends` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 284: `test_circular_self` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 289: `test_circular_three` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 294: `test_truncated_dict` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/ocr_integration.rs

**Line 148: `test_run_tesseract_span_structure` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 236: `test_full_page_coordinate_conversion` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 266: `test_cell_coordinate_conversion` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 296: `test_language_validation` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 331: `test_multi_language_string` (2 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/orphaned_process_verification_test.rs

**Line 66: `test_verification_succeeds_in_clean_state` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 128: `test_error_message_formatting` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 165: `test_process_pattern_detection` (4 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 192: `example_explicit_verification` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/page_classification.rs

**Line 437: `test_reproducibility_gate_with_perturbation` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/remote_fetch_integration.rs

**Line 46: `test_forward_scan_disabled_for_remote` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 80: `test_range_batching` (10 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

**Line 119: `test_head_failure_modes` (4 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

**Line 137: `test_remote_no_forward_scan` (7 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

**Line 147: `test_performance_requirement` (9 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

**Line 160: `test_page_5_fetch_behavior` (4 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

**Line 175: `test_large_tail_fetch` (4 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/remote_fetch_sequence.rs

**Line 332: `test_head_probe_captures_metadata` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 452: `test_bandwidth_partial_extraction` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 491: `test_page_by_page_on_demand_fetch` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 526: `test_progressive_tail_fetch` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 551: `test_custom_headers` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 573: `test_basic_authentication` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 593: `test_forward_scan_disabled_remote` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 635: `test_connection_reuse` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 665: `test_prefetch_hint` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 692: `test_cache_hit_on_repeated_read` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 721: `test_block_boundary_handling` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 751: `test_inv8_no_panic_on_errors` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/remote_forward_scan_disable.rs

**Line 51: `test_forward_scan_disabled_for_remote` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 108: `test_forward_scan_enabled_for_local` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 135: `test_forward_scan_disabled_for_linearized` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 172: `test_linearized_remote_diagnostic_priority` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/remote_http_source_tests.rs

**Line 246: `test_http_source_basic` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 262: `test_constants_are_correct` (4 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

**Line 280: `test_inv8_no_panic_on_network_errors` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 292: `test_url_validation` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 338: `test_cache_size` (2 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

**Line 348: `test_read_seek_traits` (2 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/stream_decoder_fixtures.rs

**Line 400: `test_each_filter_exercised` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/struct_tree_coverage.rs

**Line 118: `test_suspects_false_trusts_tree` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 178: `test_suspects_true_high_coverage_no_fallback` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/test_helpers/process_guard.rs

**Line 341: `test_error_display` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/test_page_access.rs

**Line 49: `test_access_single_page` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 86: `test_access_multiple_pages` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 120: `test_page_type_assertions` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 162: `test_page_vector_access_patterns` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 210: `test_page_field_access` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/test_xref_debug.rs

**Line 6: `test_debug_xref_parsing` (5 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/th06_checksum_test.rs

**Line 29: `test_tampering_detection` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/unmapped_glyph_names_config.rs

**Line 26: `test_unmapped_glyph_names_defaults_to_empty` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 85: `test_unmapped_glyph_names_specified` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 146: `test_unmapped_glyph_names_empty_array` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-core/tests/verify_proptest_catches_bugs.rs

**Line 33: `verify_prop_dict_order_preserved_catches_nondeterminism` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 62: `verify_infrastructure_complete` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-py/tests/test_search_integration.rs

**Line 29: `test_case_1_basic` (1 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

**Line 34: `test_case_2_token` (1 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

**Line 39: `test_case_3_ipv4_loopback` (1 lines)**

- Category: Too short (0 lines), no verification
- Parameters: ``

#### 📄 crates/pdftract-schema-migrate/src/lib.rs

**Line 286: `test_migration_registry_identity` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 301: `test_migration_registry_unsupported` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 325: `test_migrate_convenience_function` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 src/graphics_state/stack.rs

**Line 140: `test_multiple_diagnostics` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 160: `test_clear_diagnostics` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 tests/debug_span_access.rs

**Line 16: `test_access_spans_from_page_result` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 52: `test_access_spans_from_multiple_pages` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 86: `test_single_span_access` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 125: `test_multiple_spans_access` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 170: `test_span_type_assertions` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 235: `test_span_iteration_patterns` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 287: `test_empty_span_handling` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 310: `test_span_indexing_bounds` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 352: `test_span_field_access` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 411: `test_spans_from_different_pages` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 tests/document_model.rs

**Line 287: `test_encrypted_rc4` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 292: `test_encrypted_aes128` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 297: `test_encrypted_aes256` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 302: `test_encrypted_empty_password` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 307: `test_tagged_3_level_outline` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 312: `test_ocg_default_off` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 317: `test_multi_revision_3` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 322: `test_inheritance_grandparent_mediabox` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 327: `test_missing_mediabox` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 332: `test_partial_resource_override` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 337: `test_js_in_openaction` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 342: `test_xfa_form` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 347: `test_pdfa_1b_conformance` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 352: `test_page_labels_roman_arabic` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 tests/document_model/mod.rs

**Line 236: `test_encrypted_rc4` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 242: `test_encrypted_aes128` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 248: `test_encrypted_aes256` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 254: `test_encrypted_empty_password` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 260: `test_encrypted_unknown_handler` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 266: `test_tagged_3_level_outline` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 272: `test_ocg_default_off` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 278: `test_multi_revision_3` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 284: `test_inheritance_grandparent_mediabox` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 290: `test_missing_mediabox` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 296: `test_partial_resource_override` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 302: `test_js_in_openaction` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 308: `test_xfa_form` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 314: `test_pdfa_1b_conformance` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 tests/encryption_fixtures.rs

**Line 729: `test_execution_result_creation` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 746: `test_execution_result_with_fixture` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 761: `test_execution_result_assert_exit_code` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 775: `test_execution_result_assert_exit_code_failure` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 790: `test_execution_result_assert_success` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 804: `test_execution_result_assert_success_failure` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 819: `test_execution_result_assert_failure` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 833: `test_execution_result_assert_stderr_contains` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 847: `test_execution_result_assert_stderr_contains_failure` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 862: `test_execution_result_assert_stdout_contains` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 876: `test_execution_result_assert_output_contains` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 891: `test_execution_result_combined_output` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 907: `test_execution_result_assert_unsupported_encryption` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 921: `test_execution_result_assert_password_required` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 935: `test_execution_result_assert_wrong_password` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 949: `test_execution_result_assert_empty_output` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 963: `test_execution_result_assert_empty_output_failure` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 978: `test_execution_result_method_chaining` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 998: `test_execution_result_from_impl` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 tests/encryption_fixtures_usage_example.rs

**Line 49: `test_assertion_helpers_compile` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 tests/fingerprint_fixtures.rs

**Line 146: `test_inv13_fingerprint_format` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 tests/fingerprint_reproducibility.rs

**Line 125: `test_inv13_fingerprint_format` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 148: `test_acrobat_resave_fixture` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 153: `test_qpdf_resave_fixture` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 158: `test_pdftk_resave_fixture` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 163: `test_linearization_toggle_fixture` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 168: `test_metadata_only_fixture` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 173: `test_content_edit_one_glyph_fixture` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 178: `test_content_edit_one_paragraph_fixture` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 tests/fixtures/hybrid/mod.rs

**Line 1170: `test_extract_grid_coverage_json_with_coverage` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1180: `test_extract_grid_coverage_json_with_percentage_string` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1190: `test_extract_grid_coverage_json_with_cell_count` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1206: `test_extract_grid_coverage_non_hybrid_page_type` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1238: `test_extract_grid_coverage_text_format_cells` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1269: `test_extract_grid_coverage_malformed_json` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1278: `test_extract_grid_coverage_missing_coverage_fields` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1295: `test_extract_grid_coverage_invalid_coverage_number` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1304: `test_extract_grid_coverage_out_of_range` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

**Line 1335: `test_extract_grid_coverage_edge_cases` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 tests/forms_integration.rs

**Line 42: `test_discover_pdf_fixtures` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 tests/log_secret_fuzz.rs

**Line 149: `test_http_header_redaction` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

#### 📄 tests/test_assertion_methods.rs

**Line 12: `test_assert_stderr_contains_pass` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 24: `test_assert_stderr_contains_fail` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 37: `test_assert_exit_code_pass` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 49: `test_assert_exit_code_fail` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 62: `test_assert_success_pass` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 74: `test_assert_success_fail` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 87: `test_assert_stderr_contains_empty_string` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 99: `test_assert_stderr_contains_with_empty_stderr` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 112: `test_assert_exit_code_none_value` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

**Line 128: `test_method_chaining` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 tests/test_extract_content_stream_bytes.rs

**Line 63: `test_extract_from_uncompressed_stream` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 tests/test_page_access.rs

**Line 15: `test_access_pages_from_extraction_result` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 49: `test_access_pages_from_parse_result` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 74: `test_single_page_access` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 104: `test_multiple_pages_access` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 144: `test_page_type_assertions` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 190: `test_pagedict_access_from_parse` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 224: `test_page_iteration_patterns` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 282: `test_page_indexing_bounds` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

#### 📄 tests/verify_encryption_fixtures.rs

**Line 57: `test_assertion_functions_exist` (3 lines)**

- Category: Only setup code, no verification
- Parameters: ``

#### 📄 xtask/src/migrate/mod.rs

**Line 229: `test_migrate_identity` (2 lines)**

- Category: Too short (2 lines), no verification
- Parameters: ``

**Line 242: `test_migrate_unsupported` (1 lines)**

- Category: Too short (1 lines), no verification
- Parameters: ``

## Recommendations

For each flagged function:
1. Review the function to confirm it lacks verification logic
2. If it's a helper: Remove #[test] and make it a regular function
3. If it's a smoke test: Add a comment explaining it tests no-crash behavior
4. If it needs verification: Add assertions or result checks

