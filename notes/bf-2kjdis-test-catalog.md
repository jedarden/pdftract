# PDF Test Catalog

**Generated:** 2026-08-08  
**Bead ID:** bf-3zoen6  
**Total Test Functions:** 540  
**Total Test Files:** 46  

## Summary

| Directory | Test Count |
|-----------|------------|
| `crates/pdftract-cli/tests/` | 97 |
| `crates/pdftract-core/tests/` | 318 |
| `crates/pdftract-core/src/` (unit tests) | 35 |
| `tests/` (root-level integration) | 90 |

## Test Category Breakdown

| Category | Count |
|----------|-------|
| **Security/Threat Tests** | 71 |
| - TH-01: Stream Bomb | 5 |
| - TH-02: Path Traversal | 23 |
| - TH-03: MCP No Auth | 11 |
| - TH-04: JS Presence | 3 |
| - TH-05: SSRF Protection | 19 |
| - TH-06: Checksum | 2 |
| - TH-08: Log Audit | 6 |
| - TH-09: Inspector XSS | 5 |
| - TH-10: Cache Poison | 10 |
| **Integration Tests** | 243 |
| **Unit Tests** | 226 |
| **Helper Functions (parametrized)** | ~7 |

---

## Files by Test Count

| File | Tests |
|------|-------|
| `crates/pdftract-core/src/font/type3_rasterizer_test.rs` | 25 |
| `crates/pdftract-core/tests/http_range_integration.rs` | 22 |
| `crates/pdftract-core/tests/encryption_integration_tests.rs` | 19 |
| `tests/fingerprint_reproducibility.rs` | 18 |
| `tests/stream_decoder_fixtures.rs` | 16 |
| `tests/document_model/mod.rs` | 16 |
| `crates/pdftract-core/tests/remote_fetch_sequence.rs` | 16 |
| `crates/pdftract-core/tests/document_model.rs` | 16 |
| `crates/pdftract-core/tests/remote_fetch_integration.rs` | 13 |
| `crates/pdftract-core/tests/hint_stream_integration.rs` | 13 |
| `crates/pdftract-cli/tests/root-path-protection.rs` | 13 |
| `crates/pdftract-cli/tests/multi_output_validation.rs` | 13 |

---

## CLI Tests (`crates/pdftract-cli/tests/`)

### MCP Protocol (18 tests)

**`mcp-http.rs` (10 tests)**
- test_post_tools_list:78
- test_post_batch_request:114
- test_post_single_request_returns_single_response:158
- test_post_payload_too_large:194
- test_get_health:236
- test_get_sse_stream:264
- test_auth_required_for_non_loopback:314
- test_unknown_method:350
- test_50_concurrent_clients:388
- test_health_during_load:467

**`mcp-stdio.rs` (8 tests)**
- test_tools_list_roundtrip:83
- test_eof_clean_shutdown:126
- test_parse_error_response:157
- test_parse_error_recovery:188
- test_stdout_json_rpc_only:233
- test_request_response_timing:291
- test_unknown_method:326
- test_notification_no_response:355

**`mcp-cli-args.rs` (5 tests)**
- test_stdio_and_bind_mutually_exclusive:15
- test_default_to_stdio:52
- test_stdio_flag_valid:81
- test_bind_flag_valid:101
- test_help_mentions_adr_006:122

### MCP Tools Integration (11 tests)

**`mcp-tools-integration.rs`**
- test_get_metadata_performance_on_100_page_pdf:12
- test_hash_performance_on_100_page_pdf:41
- test_tools_list_has_all_10_tools:69
- test_phase_7_stub_tools_return_not_implemented:104
- test_unknown_tool_name_returns_method_not_found:134
- test_missing_required_path_returns_error:142
- test_extract_tool_with_real_pdf:157
- test_search_tool_with_invalid_regex:183
- test_path_resolution:201
- test_nonexistent_file_returns_path_invalid:229
- test_encrypted_pdf_returns_pdf_encrypted_error:252

### Multi-Output Validation (13 tests)

**`multi_output_validation.rs`**
- test_default_single_json_to_stdout:24
- test_json_flag_creates_file:35
- test_json_and_md_flags_create_two_files:52
- test_duplicate_json_flag_rejected:81
- test_ndjson_conflicts_with_md:100
- test_ndjson_conflicts_with_format_list:117
- test_md_to_stdout_and_json_to_file:135
- test_multiple_stdout_rejected:153
- test_format_with_output_base:165
- test_format_requires_output_base:195
- test_invalid_format_name:206
- test_format_text_md_json_creates_three_files:223
- test_text_flag_with_dash_for_stdout:260

### TH-02: Path Traversal (23 tests)

**`TH-02-path-traversal.rs` (10 tests)**
- test_root_mode_rejects_all_traversal_payloads:51
- test_root_mode_accepts_valid_paths:98
- test_without_root_paths_pass_through:143
- test_https_urls_bypass_root_check:165
- test_symlink_escape_rejected:195
- test_url_encoded_traversal_rejected:236
- test_windows_reserved_name_handling:274
- test_special_filesystem_paths_rejected:320
- test_nested_traversal_with_valid_prefix:354
- test_deep_traversal_rejected:379

**`root-path-protection.rs` (13 tests)**
- test_acceptance_criteria_path_traversal_rejected:16
- test_acceptance_criteria_valid_path_within_root:36
- test_acceptance_criteria_absolute_path_rejected:55
- test_acceptance_criteria_https_url_bypasses_check:67
- test_acceptance_criteria_no_root_trust_the_caller:80
- test_acceptance_criteria_symlink_escape_rejected:91
- test_acceptance_criteria_nonexistent_root_startup_error:117
- test_acceptance_criteria_file_not_directory_startup_error:124
- test_plan_critical_test_path_traversal_with_root:135
- test_http_url_bypasses_check:159
- test_dotdot_at_boundary_rejected:172
- test_nonexistent_file_within_root_returns_error:185
- test_complex_path_traversal_patterns:204

### TH-05: SSRF Protection (7 tests)

**`TH-05-ssrf-block.rs`**
- test_ipv4_loopback_blocked:191
- test_ipv4_wildcard_blocked:230
- test_cloud_metadata_blocked:267
- test_rfc1918_private_blocked:304
- test_ipv6_loopback_blocked:341
- test_http_scheme_rejected:440
- test_no_network_connection_attempted:481

### TH-08: Log Audit (6 tests)

**`TH-08-log-audit.rs`**
- test_log_audit_no_content_leak_trace:63
- test_log_audit_no_content_leak_with_debug:134
- test_log_audit_no_bearer_token_leak:194
- test_log_audit_no_pdf_bytes_leak:234
- test_log_audit_no_sensitive_headers_leak:303
- test_log_audit_audit_log_no_leak:334

### TH-09: Inspector XSS (5 tests)

**`TH-09-inspector-xss.rs`**
- test_csp_header_on_index:68
- test_csp_header_on_api_endpoints:118
- test_inspector_renders_svg:159
- test_inspector_handles_normal_content:195
- test_headless_browser_no_script_execution:242

### Comparison Mode (11 tests)

**`comparison_mode_test.rs`**
- test_inspect_args_has_compare_field:9
- test_inspect_args_validate_without_compare:29
- test_diff_summary_serialization:49
- test_page_diff_serialization:76
- test_compare_document_meta_serialization:100
- test_compare_page_data_serialization:121
- test_bbox_overlap_score:152
- test_text_similarity_score:175
- test_levenshtein_distance:192
- test_block_match_score:212
- test_span_match_score:253

### Single Page Access (11 tests)

**`single_page_access.rs`**
- test_single_page_access_by_index:41
- test_single_page_access_by_first_page_helper:84
- test_single_page_spans_access:112
- test_page_count_for_single_page:138
- test_is_single_page_for_single_page:162
- test_multi_page_first_page_access:182
- test_multi_page_last_page_access:213
- test_error_handling_out_of_bounds_index:242
- test_error_handling_invalid_page_number_zero:273
- test_error_handling_out_of_bounds_page_number:296
- test_get_all_pages_single_page:322

### Forms Integration (6 tests)

**`forms_integration.rs`**
- test_discover_pdf_fixtures:55
- test_forms_fixtures_discovery:79
- test_extract_all_discovered_pdfs:132
- test_form_field_structure:227
- test_acroform_features:251
- test_xfa_detection:274

### CLI Invocation Fixtures (5 tests)

**`cli_invocation_fixtures.rs`**
- test_main_fixtures_dir_exists:44
- test_discover_all_pdf_fixtures:57
- test_discover_fixtures_by_category:99
- test_fixture_enumeration_for_cli:122
- test_cli_invocation_on_fixture_sample:163

---

## Core Library Tests (`crates/pdftract-core/tests/`)

### Remote Fetch (29 tests)

**`remote_fetch_integration.rs` (13 tests)**
- test_open_remote_head_probe:12
- test_tail_fetch_size:28
- test_forward_scan_disabled_for_remote:48
- test_page_by_page_on_demand:67
- test_range_batching:81
- test_acceptance_criteria_500_page:103
- test_head_failure_modes:121
- test_remote_no_forward_scan:138
- test_performance_requirement:148
- test_page_5_fetch_behavior:161
- test_large_tail_fetch:177
- test_linearized_hint_stream:187
- test_tls_failure_handling:198

**`remote_fetch_sequence.rs` (16 tests)**
- test_head_probe_captures_metadata:333
- test_405_fallback_to_get_probe:356
- test_unauthorized_returns_error:377
- test_no_content_length_handled:401
- test_no_range_support_detected:422
- test_bandwidth_partial_extraction:454
- test_page_by_page_on_demand_fetch:492
- test_progressive_tail_fetch:527
- test_custom_headers:552
- test_basic_authentication:574
- test_forward_scan_disabled_remote:594
- test_connection_reuse:636
- test_prefetch_hint:666
- test_cache_hit_on_repeated_read:693
- test_block_boundary_handling:722
- test_inv8_no_panic_on_errors:752

**`remote_forward_scan_disable.rs` (4 tests)**
- test_forward_scan_disabled_for_remote:52
- test_forward_scan_enabled_for_local:109
- test_forward_scan_disabled_for_linearized:136
- test_linearized_remote_diagnostic_priority:173

### HTTP Range Support (22 tests)

**`http_range_integration.rs`**
- test_head_request_captures_metadata:13
- test_read_range_block_calculation:34
- test_cache_hit_on_repeated_read:54
- test_block_boundary_crossing:62
- test_empty_read_range:79
- test_large_read_spans_many_blocks:95
- test_partial_block_read:113
- test_random_reads_no_panic:134
- test_network_error_classification:172
- test_prefetch_hint:186
- test_range_header_format:195
- test_cache_capacity:213
- test_accept_ranges_detection:224
- test_no_range_support_error_kind:243
- test_thread_safety:255
- test_content_length_parsing:265
- test_url_validation:284
- test_custom_headers:302
- test_content_length_stored:319
- test_boundary_conditions:327
- test_memory_footprint:358
- test_timeout_configuration:374

**`remote_http_source_tests.rs` (12 tests)**
- test_http_source_basic:247
- test_constants_are_correct:263
- test_is_remote_trait_method:271
- test_inv8_no_panic_on_network_errors:281
- test_url_validation:293
- test_bandwidth_calculations:304
- test_block_calculation:320
- test_cache_size:339
- test_read_seek_traits:349
- test_send_sync_traits:356
- test_custom_headers_construction:363
- test_performance_calculations:377

**`remote_mock_server_tests.rs`**
- test_inv8_no_panic_on_network_errors:823

### Encryption (19 tests)

**`encryption_integration_tests.rs`**
- test_ec04_rc4_encryption_detection:95
- test_ec05_aes128_encryption_detection:121
- test_ec06_aes256_encryption_detection:149
- test_unsupported_encryption_filter:185
- test_rc4_key_derivation:212
- test_rc4_object_key_different_objects:237
- test_rc4_object_key_same_object:249
- test_rc4_decrypt_roundtrip:261
- test_aes128_object_key_derivation:274
- test_aes128_decrypt_requires_iv:289
- test_aes256_decryptor_creation:302
- test_aes256_decryptor_invalid_length:325
- test_password_padding_empty:351
- test_password_padding_short:359
- test_password_padding_long:368
- test_decrypt_with_password_missing_id:378
- test_non_encrypted_pdf:412
- test_proptest_random_encrypt_dict:433
- test_encryption_performance:466

### TH-01: Stream Bomb (5 tests)

**`TH-01-stream-bomb.rs`**
- test_bomb_default_cap_allows_reasonable_decompression:55
- test_bomb_lowered_cap_triggers_stream_bomb:100
- test_bomb_fixture_has_high_compression_ratio:145
- test_bomb_limit_checked_incrementally:181
- test_bomb_limit_truncation_behavior:235

### TH-03: MCP No Auth (11 tests)

**`TH-03-mcp-no-auth.rs`**
- test_case_1_ipv4_all_without_token:270
- test_case_2_ipv6_all_without_token:311
- test_case_3_ipv4_loopback_without_token:337
- test_case_4_ipv6_loopback_without_token:368
- test_case_5_ipv4_all_with_env_token:401
- test_case_6_ipv4_all_with_token_file:435
- test_case_7_localhost_without_token:477
- test_case_8_mixed_hostname_resolution:540
- test_atomic_failure_no_listener_during_failure:566
- test_exit_code_is_78_not_any_nonzero:596
- test_parallel_bind_attempts_all_fail:620

### TH-04: JS Presence (3 tests)

**`TH-04-js-presence.rs`**
- test_javascript_detection:31
- test_no_javascript:98
- test_no_js_engine_in_deps:137

### TH-05: SSRF Block (12 tests)

**`TH-05-ssrf-block.rs`**
- test_ssrf_protection_blocks_all_dangerous_payloads:198
- test_allow_private_networks_bypass:222
- test_public_urls_are_accepted:255
- test_http_scheme_always_rejected:275
- test_file_scheme_always_rejected:282
- test_ftp_scheme_always_rejected:288
- test_url_with_basic_auth_rejected:294
- test_ipv6_zone_id_detected_as_link_local:301
- test_metadata_subdomain_detected:308
- test_url_validation_returns_correct_diagnostic_code:315
- test_private_ipv4_boundary_addresses:325
- test_current_network_range_blocked:354

### TH-06: Checksum (2 tests)

**`th06_checksum_test.rs`**
- test_tampering_detection:30
- test_normal_build_checksums_pass:112

### TH-10: Cache Poison (10 tests)

**`TH-10-cache-poison.rs`**
- test_cache_init_creates_key_with_mode_0600:22
- test_legitimate_entry_has_valid_hmac:48
- test_forged_entry_with_wrong_hmac_rejected:77
- test_forged_entry_triggers_cache_miss:138
- test_forged_entry_with_correct_hmac_key_compromise:192
- test_hmac_input_is_fingerprint_opts_hash_and_blob:246
- test_cache_rewrites_forged_entry_on_miss:288
- test_multiple_forgeries_all_rejected:341
- test_key_file_persistence:372
- test_repeated_poisoning_attack_simulation:404

### Document Model (16 tests)

**`document_model.rs`**
- test_fixture:121 [HELPER - parametrized test runner]
- test_encrypted_rc4:297
- test_encrypted_aes128:303
- test_encrypted_aes256:309
- test_encrypted_empty_password:315
- test_encrypted_unknown_handler:321
- test_tagged_3_level_outline:327
- test_ocg_default_off:333
- test_multi_revision_3:339
- test_inheritance_grandparent_mediabox:345
- test_missing_mediabox:351
- test_partial_resource_override:357
- test_js_in_openaction:363
- test_xfa_form:369
- test_pdfa_1b_conformance:375
- test_page_labels_roman_arabic:381

### Object Parser (12 tests)

**`object_parser.rs`**
- test_object_parser_fixtures:92
- test_fixture:124 [HELPER - parametrized test runner]
- test_all_fixtures:246
- test_nested_dict:255
- test_mixed_array:260
- test_indirect_simple:265
- test_indirect_stream:270
- test_objstm_basic:275
- test_objstm_extends:280
- test_circular_self:285
- test_circular_three:290
- test_truncated_dict:295
- test_deep_nesting:300

### Hint Stream (13 tests)

**`hint_stream_integration.rs`**
- test_parse_hint_stream_valid:61
- test_parse_hint_stream_malformed_version:94
- test_parse_hint_stream_zero_page_count:111
- test_hint_predict_shared_objects_minimal:134
- test_hint_stream_out_of_bounds_page:150
- test_hint_table_predict_page_range:165
- test_linearized_pdf_with_hint_stream:327
- test_hint_stream_no_panic_on_corrupt_data:360
- test_hint_prefetch_performance:372
- test_prefetch_from_hint_stream_basic:443
- test_prefetch_from_hint_stream_out_of_bounds:474
- test_prefetch_from_hint_stream_empty_page_list:500
- test_prefetch_from_hint_stream_malformed_hint_stream:525

### Schema Validation (7 tests)

**`schema_validate_fixtures.rs`**
- test_fixture:115 [HELPER - parametrized test runner]
- test_all_fixtures_schema_compliance:186
- test_simple_invoice:202
- test_sample:217
- test_encrypted_rc4:232
- test_encrypted_aes128:247
- test_valid_minimal:262

### JSON Schema (6 tests)

**`json_schema.rs`**
- test_all_fixtures_validate_against_schema:178
- test_schema_itself_is_valid:200
- test_schema_has_required_document_level_fields:223
- test_schema_page_json_structure:266
- test_schema_span_json_structure:315
- test_synthetic_output_validates:344

### Conformance (1 test)

**`conformance.rs`**
- test_sdk_conformance:980

### Struct Tree Coverage (3 tests)

**`struct_tree_coverage.rs`**
- test_suspects_true_fallback_to_xy_cut:53
- test_suspects_false_trusts_tree:119
- test_suspects_true_high_coverage_no_fallback:179

### Unmapped Glyph Names Config (4 tests)

**`unmapped_glyph_names_config.rs`**
- test_unmapped_glyph_names_defaults_to_empty:27
- test_unmapped_glyph_names_specified:86
- test_unmapped_glyph_names_empty_array:147
- test_unmapped_glyph_names_minimal_config:187

### CMap Unmapped Glyphs (7 tests)

**`cmap_unmapped_glyphs.rs`**
- test_cmap_unmapped_glyph_skip:19
- test_cmap_multiple_mappings_with_unmapped_check:200
- test_cmap_range_mapping_with_unmapped_awareness:272
- test_differences_overlay_filters_unmapped_glyphs:339
- test_differences_overlay_consecutive_with_unmapped_filtering:488
- test_differences_overlay_filters_null_glyph:576
- test_differences_overlay_filters_all_g_series_unmapped:620

### OCR Integration (10 tests)

**`ocr_integration.rs`**
- test_wer_calculation_known_inputs:37
- test_clean_lorem_ipsum_wer:60
- test_multilang_eng_fra_wer:107
- test_run_tesseract_span_structure:150
- test_wer_threshold_validation:183
- test_performance_10_pages:207
- test_full_page_coordinate_conversion:238
- test_cell_coordinate_conversion:268
- test_language_validation:298
- test_multi_language_string:333

### Page Classification (5 tests)

**`page_classification.rs`**
- test_page_classification_fixtures:252
- test_page_classification_reproducibility:329
- test_fixture_files_exist_and_size:362
- test_expected_json_validity:407
- test_reproducibility_gate_with_perturbation:438

### Classifier Corpus (3 tests)

**`classifier_corpus.rs`**
- test_classifier_corpus_accuracy:299
- test_classifier_reproducibility:356
- test_corpus_manifest_validity:402

### CJK Encoding (6 tests)

**`cjk_encoding.rs`**
- test_cjk_fixture:59 [HELPER - parametrized test runner]
- test_cjk_gb18030_chinese:88
- test_cjk_shiftjis_japanese:109
- test_cjk_euckr_korean:130
- test_cjk_big5_traditional_chinese:151
- test_all_cjk_fixtures_exist:172

### Encoding Recovery (7 tests)

**`encoding_recovery.rs`**
- test_encoding_fixture:109 [HELPER - parametrized test runner]
- test_no_mapping_fixture:162
- test_agl_only_fixture:174
- test_fingerprint_match_fixture:192
- test_shape_match_fixture:202
- test_all_encoding_fixtures_exist:212
- test_corpus_recovery_rate:228

### Memory Guard Tests (10 tests)

**`memory_guard_tests.rs`**
- test_large_vector_allocation_fails_gracefully:16
- test_oversized_decompression_fails_gracefully:33
- test_hashmap_under_memory_pressure:57
- test_try_reserve_propagates_failure:76
- test_string_try_reserve_fails_gracefully:102
- test_box_allocation_under_limit:116
- test_multiple_allocations_under_tight_budget:130
- test_vec_resize_fails_gracefully:148
- test_string_from_large_bytes_fails_gracefully:162
- test_nested_allocations_under_limit:175

### Fingerprint Reproducibility (10 tests)

**`fingerprint_reproducibility.rs`**
- test_inv3_reproducibility_100_invocations:34
- test_fixture_byte_identical:56
- test_fixture_qpdf_resave:70
- test_fixture_acrobat_resave:81
- test_fixture_pdftk_resave:95
- test_fixture_linearization_toggle:106
- test_fixture_metadata_only:120
- test_fixture_content_edit_one_glyph:134
- test_fixture_content_edit_one_paragraph:148
- test_inv13_fingerprint_format:162

### XRef Integration (4 tests)

**`xref_integration_test.rs`**
- test_xref_fixtures:245
- test_forward_scan_recovery:287
- test_prev_chain_depth_limit:315
- test_circular_prev_detection:336

### Error Recovery Integration (8 tests)

**`error_recovery_integration.rs`**
- test_xref_30pct_bad_offsets:77
- test_missing_mediabox_all_pages:107
- test_missing_endobj:141
- test_truncated_mid_stream:165
- test_int_overflow_bbox:195
- test_nested_failure:225
- test_combined_failures:251
- test_inv_8_no_panics_across_all_fixtures:283

### Orphaned Process Verification (7 tests)

**`orphaned_process_verification_test.rs`**
- test_verification_succeeds_in_clean_state:67
- test_custom_pattern_verification:87
- test_orphaned_process_guard_lifecycle:100
- test_orphaned_process_guard_custom_patterns:116
- test_error_message_formatting:129
- test_kill_orphaned_processes_safe_when_clean:151
- test_process_pattern_detection:166

### Stream Decoder Fixtures (2 tests)

**`stream_decoder_fixtures.rs`**
- test_all_stream_decoder_fixtures:257
- test_each_filter_exercised:401

---

## Unit Tests in Source Files (`crates/pdftract-core/src/`)

### Type3 Font CharProc (10 tests)

**`src/font/type3_charproc_test.rs`**
- test_charproc_simple_rectangle:72
- test_charproc_move_line_close:106
- test_charproc_multiple_shapes:136
- test_charproc_stroke_rectangle:165
- test_charproc_close_stroke_triangle:194
- test_charproc_empty_stream:223
- test_charproc_whitespace_only:254
- test_charproc_noop_path:282
- test_charproc_complex_polygon:313
- test_charproc_consistent_rendering:342

### Type3 Font Rasterizer (25 tests)

**`src/font/type3_rasterizer_test.rs`**
- test_resolve_stream_callback_receives_objref:67
- test_resolve_stream_callback_captures_resolver:103
- test_resolve_stream_callback_captures_source:137
- test_resolve_stream_callback_captures_counter:171
- test_resolve_stream_callback_multiple_glyphs:205
- test_resolve_stream_callback_returns_none:245
- test_resolve_stream_callback_returns_valid_bytes:270
- test_resolve_stream_helper_function_pattern:304
- test_edge_activation_at_y_min:373
- test_edge_removal_after_y_max:426
- test_intersection_x_calculation:468
- test_slope_based_x_increment:544
- test_horizontal_edge_skipping:657
- test_aet_sorting_by_x_position:697
- test_detect_char_proc_type_dict:747
- test_detect_char_proc_type_stream:766
- test_detect_char_proc_type_other_integer:787
- test_detect_char_proc_type_other_string:807
- test_detect_char_proc_type_other_name:827
- test_detect_char_proc_type_other_array:847
- test_detect_char_proc_type_other_boolean:871
- test_detect_char_proc_type_other_null:899
- test_detect_char_proc_type_reference:920
- test_detect_char_proc_type_other_real:941
- test_detect_char_proc_type_empty_dict:961

---

## Root-Level Integration Tests (`tests/`)

### Document Model (16 tests)

**`document_model/mod.rs`**
- test_fixture:74 [HELPER - parametrized test runner]
- test_encrypted_rc4:237
- test_encrypted_aes128:243
- test_encrypted_aes256:249
- test_encrypted_empty_password:255
- test_encrypted_unknown_handler:261
- test_tagged_3_level_outline:267
- test_ocg_default_off:273
- test_multi_revision_3:279
- test_inheritance_grandparent_mediabox:285
- test_missing_mediabox:291
- test_partial_resource_override:297
- test_js_in_openaction:303
- test_xfa_form:309
- test_pdfa_1b_conformance:315
- test_page_labels_roman_arabic:321

### Encryption Errors (8 tests)

**`encryption_errors.rs`**
- test_encryption_unsupported_livecycle:250
- test_exit_code_3_no_password:282
- test_wrong_password_encryption_unsupported:301
- test_encryption_error_consistency:326
- test_encryption_unsupported_livecycle:364 [duplicate]
- test_exit_code_3_no_password:395 [duplicate]
- test_wrong_password_encryption_unsupported:413 [duplicate]
- test_encryption_error_consistency:437 [duplicate]

### JSON Schema (7 tests)

**`json_schema.rs`**
- test_fixture:101 [HELPER - parametrized test runner]
- test_all_fixtures_schema_compliance:163
- test_simple_invoice:175
- test_sample:187
- test_encrypted_rc4:199
- test_encrypted_aes128:211
- test_valid_minimal:223

### Fingerprint Reproducibility (18 tests)

**`fingerprint_reproducibility.rs`**
- test_fingerprint_fixture_pairs:60
- test_inv3_reproducibility:123
- test_inv13_fingerprint_format:147
- test_performance_fixture_corpus:166
- test_fingerprint_fixture_pairs:478 [duplicate]
- test_inv3_reproducibility_100_invocations:102
- test_inv13_fingerprint_format:126
- test_acrobat_resave_fixture:149
- test_qpdf_resave_fixture:154
- test_pdftk_resave_fixture:159
- test_linearization_toggle_fixture:164
- test_metadata_only_fixture:169
- test_content_edit_one_glyph_fixture:174
- test_content_edit_one_paragraph_fixture:179
- test_byte_identical_fixture:184
- test_fixture_pair:189 [HELPER]
- test_fingerprint_performance:210
- test_byte_identical_produces_same_fingerprint:238
- test_metadata_ignored_in_fingerprint:254
- test_linearization_independent:270
- test_single_glyph_changes_fingerprint:286
- test_paragraph_edit_changes_fingerprint:302

### Stream Decoder Fixtures (16 tests)

**`stream_decoder_fixtures.rs`**
- test_stream_decoder_fixtures:220
- test_flate_simple:234
- test_flate_truncated:247
- test_flate_bomb_3gb:260
- test_ascii85_z_shortcut:284
- test_ascii85_terminator:297
- test_asciihex_odd_length:310
- test_runlength_basic:323
- test_lzw_early_change_0:336
- test_lzw_early_change_1:352
- test_dct_valid_jpeg:365
- test_dct_missing_eoi:383
- test_jbig2_passthrough:397
- test_crypt_identity:411
- test_filter_array_a85_then_flate:427
- test_unknown_filter:448

### Forms Integration (3 tests)

**`forms_integration.rs`**
- test_discover_pdf_fixtures:43
- test_cli_extract_json_on_fixtures:141
- test_forms_extraction:247

### Encryption Fixtures Usage (4 tests)

**`encryption_fixtures_usage_example.rs`**
- test_fixture_module_constants:25
- test_fixture_module_functions:34
- test_assertion_helpers_compile:50
- test_mock_builders:65

### Smoke Tests (3 tests)

**`smoke_test.rs`**
- test_basic_pdf_extraction:16
- test_sample_pdf_extraction:70
- test_extract_returns_typed_document:109

### Fingerprint Test Single One (2 tests)

**`fingerprint_test_single_one.rs`**
- test_single_fixture_byte_identical:7
- test_single_fixture_content_edit_one_glyph:24

### Log Secret Fuzz (7 tests)

**`log_secret_fuzz.rs`**
- test_secret_string_debug_display_redaction:54
- test_panic_hook_redacts_secret_string:115
- test_http_header_redaction:150
- test_header_redaction_structure:193
- test_credential_variable_detection:222
- test_log_policy_script:260
- test_expose_secret:336

### Object Parser (1 test)

**`object_parser.rs`**
- test_object_parser_fixtures:92

### Remote Integration (5 tests)

**`remote/integration.rs`**
- test_bandwidth_tracker:494
- test_assert_bytes_transferred_pass:508
- test_assert_bytes_transferred_fail:518
- test_assert_range_request_count_pass:527
- test_assert_range_request_count_fail:539

### Hybrid Fixtures (12 tests)

**`integration/hybrid_fixtures.rs`**
- test_all_hybrid_fixtures_classify_as_mixed:39
- test_hybrid_001_vector_header_over_scan:110
- test_hybrid_002_vector_form_over_scan:117
- test_hybrid_003_mixed_column_layout:124
- test_hybrid_004_watermark_over_scan:131
- test_hybrid_005_vector_footer_over_scan:138
- test_hybrid_006_stamp_annotation:145
- test_hybrid_007_textbox_overlay:152
- test_hybrid_008_rotated_vector:159
- test_hybrid_009_transparent_vector:166
- test_hybrid_010_complex_layered:173
- test_hybrid_fixture_count_matches_ku2_requirement:180

### Advanced Profiles (4 tests)

**`integration/advanced/profiles.rs`**
- test_invalid_profiles_rejected:38
- test_valid_profiles_accepted:97
- test_profile_resolution_order:139
- test_invalid_fixture_error_types:199

### Fingerprint Fixtures (4 tests)

**`fingerprint_fixtures.rs`**
- test_fingerprint_fixture_pairs:123
- test_inv3_reproducibility:147
- test_inv13_fingerprint_format:166
- test_fixture_pair:189 [HELPER]

### Proptest Panic Verification (1 test)

**`proptest-panic-verification.rs`**
- test_proptest_catches_deliberate_panic:14

### Proptest Lexer (1 test)

**`proptest/lexer.rs`**
- test_panic_injection_for_prop_test_verification:484

---

## Helper Functions

These functions are parametrized test runners, not standalone tests:
- `test_fixture` in document_model/mod.rs - runs parametrized fixtures
- `test_fixture` in schema_validate_fixtures.rs - runs parametrized fixtures
- `test_fixture` in json_schema.rs - runs parametrized fixtures
- `test_fixture` in object_parser.rs - runs parametrized fixtures
- `test_cjk_fixture` in cjk_encoding.rs - runs parametrized CJK fixtures
- `test_encoding_fixture` in encoding_recovery.rs - runs parametrized encoding fixtures
- `test_fixture_pair` in fingerprint_reproducibility.rs - runs paired fixture tests

---

## Methodology

Generated using: `rg "^fn test_" --type rust -n`

Each entry shows:
- Test function name
- Line number where it's defined

---

**End of Catalog**
