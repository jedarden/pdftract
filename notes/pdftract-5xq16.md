# pdftract-5xq16: JSON-RPC 2.0 framing layer

## Summary

Implemented hand-rolled JSON-RPC 2.0 framing layer at `crates/pdftract-cli/src/mcp/framing/mod.rs`.

## Files Changed

- `crates/pdftract-cli/src/mcp/framing/mod.rs` - New module (658 lines)
- `crates/pdftract-cli/src/mcp/mod.rs` - Added `pub mod framing;` and re-exports

## Implementation Details

### Types Defined

1. **`Id` enum** - Request identifier preserving JSON type:
   - `Id::Number(i64)` - Numeric IDs
   - `Id::String(String)` - String IDs
   - `Id::Null` - Null IDs (for parse errors)

2. **`Request`** - JSON-RPC request with:
   - `jsonrpc` field (validated to be "2.0")
   - `method: String`
   - `params: Option<Value>`
   - `id: Option<Id>` (None = notification)

3. **`Notification`** - Request without id field (no response)

4. **`Response`** - Response with mutually exclusive `result` or `error`

5. **`ErrorObject`** - Error with code, message, and optional data

6. **`BatchMessage`** - Single or batch request wrapper

### Error Codes Implemented

| Code | Name | Constructor |
|------|------|-------------|
| -32700 | Parse error | `ErrorObject::parse_error()` |
| -32600 | Invalid Request | `ErrorObject::invalid_request()` |
| -32601 | Method not found | `ErrorObject::method_not_found(method)` |
| -32602 | Invalid params | `ErrorObject::invalid_params()` |
| -32603 | Internal error | `ErrorObject::internal_error()` |
| -32099..-32000 | Server errors | `ErrorObject::server_error(code, msg)` |

## Acceptance Criteria

All PASS:

- ✅ **Round-trip test**: `test_request_round_trip` - Request serializes and deserializes correctly
- ✅ **ID preservation**: `test_id_number_preservation`, `test_id_string_preservation`, `test_id_null_preservation` - All 3 ID types preserve JSON type through round-trip
- ✅ **Parse-error path**: `test_parse_error_response` - Parse error response has `id: null`
- ✅ **Method-not-found**: `test_method_not_found` - Returns code -32601 with method name in data field
- ✅ **Notification path**: `test_notification_no_id`, `test_notification_deserialize` - Notifications have no id field; deserialize correctly
- ✅ **Batch round-trip**: `test_batch_round_trip` - Batch of 3 requests serializes/deserializes correctly
- ✅ **Error codes**: `test_all_error_codes` - All 6 spec-defined error codes return correct values
- ✅ **jsonrpc validation**: `test_reject_invalid_jsonrpc_version`, `test_reject_missing_jsonrpc_field` - Invalid versions rejected with clear error
- ✅ **Empty batch**: `test_empty_batch_rejected` - Empty array returns Invalid Request error

## Test Results

```
running 16 tests
........................
test result: ok. 16 passed; 0 failed; 0 ignored; 0 measured
```

## Commit

`3c10dfd` - feat(pdftract-5xq16): implement JSON-RPC 2.0 framing layer
