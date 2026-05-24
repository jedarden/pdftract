//! JSON-RPC 2.0 framing layer for MCP server.
//!
//! This module provides a hand-rolled JSON-RPC 2.0 implementation shared by
//! the stdio and HTTP+SSE transports. It enforces strict spec conformance,
//! particularly around id type preservation and the jsonrpc version field.

use serde::{Deserialize, Deserializer, Serialize, Serializer};
use serde_json::Value;

/// JSON-RPC 2.0 protocol version.
/// Must be exactly "2.0" - we reject "1.0", "2", or missing values.
const JSONRPC_VERSION: &str = "2.0";

/// Custom deserializer for the jsonrpc field that enforces it must be "2.0".
fn deserialize_jsonrpc<'de, D>(deserializer: D) -> Result<(), D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    if s == JSONRPC_VERSION {
        Ok(())
    } else {
        Err(serde::de::Error::custom(format!(
            "invalid jsonrpc version: expected '{JSONRPC_VERSION}', got '{s}'"
        )))
    }
}

/// Serializer for the jsonrpc field that always writes "2.0".
fn serialize_jsonrpc<S>(_value: &(), serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    serializer.serialize_str(JSONRPC_VERSION)
}

/// A JSON-RPC request identifier.
///
/// Per the JSON-RPC 2.0 spec, the id field can be a number, string, or null.
/// We preserve the exact JSON type because the response MUST return an id
/// with the same type as the request.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Id {
    Number(i64),
    String(String),
    Null,
}

impl Serialize for Id {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Id::Number(n) => n.serialize(serializer),
            Id::String(s) => s.serialize(serializer),
            Id::Null => serializer.serialize_none(),
        }
    }
}

impl<'de> Deserialize<'de> for Id {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        let value = Value::deserialize(deserializer)?;
        match value {
            Value::Number(n) => {
                if let Some(n) = n.as_i64() {
                    Ok(Id::Number(n))
                } else if let Some(f) = n.as_f64() {
                    // JSON-RPC doesn't specify float behavior; we reject as out-of-spec
                    Err(serde::de::Error::custom(format!(
                        "float id not supported: {f}"
                    )))
                } else {
                    Err(serde::de::Error::custom("invalid number id"))
                }
            }
            Value::String(s) => Ok(Id::String(s)),
            Value::Null => Ok(Id::Null),
            _ => Err(serde::de::Error::custom(
                "id must be number, string, or null",
            )),
        }
    }
}

/// A JSON-RPC request object.
///
/// ```json
/// {
///   "jsonrpc": "2.0",
///   "method": "tools/list",
///   "params": { "name": "foo" },
///   "id": 1
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Request {
    /// Protocol version - validated to be exactly "2.0"
    #[serde(
        serialize_with = "serialize_jsonrpc",
        deserialize_with = "deserialize_jsonrpc"
    )]
    jsonrpc: (),

    /// The method name to invoke.
    /// Method names beginning with "rpc." are reserved by the spec.
    pub method: String,

    /// Parameters for the method. Must be an array or object if present.
    pub params: Option<Value>,

    /// Request identifier. None indicates a notification (no response expected).
    pub id: Option<Id>,
}

impl Request {
    /// Create a new request with the given method and optional params.
    pub fn new(method: impl Into<String>, params: Option<Value>, id: Option<Id>) -> Self {
        Self {
            jsonrpc: (),
            method: method.into(),
            params,
            id,
        }
    }

    /// Returns true if this is a notification (no id field).
    pub fn is_notification(&self) -> bool {
        self.id.is_none()
    }

    /// Get the request ID, or Id::Null for notifications.
    pub fn request_id(&self) -> Id {
        self.id.clone().unwrap_or(Id::Null)
    }
}

/// A JSON-RPC notification object.
///
/// A notification is a request without an id field - it receives no response.
///
/// ```json
/// {
///   "jsonrpc": "2.0",
///   "method": "notifications/message",
///   "params": { "message": "hello" }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct Notification {
    /// Protocol version - validated to be exactly "2.0"
    #[serde(
        serialize_with = "serialize_jsonrpc",
        deserialize_with = "deserialize_jsonrpc"
    )]
    jsonrpc: (),

    /// The method name being notified.
    pub method: String,

    /// Parameters for the notification.
    pub params: Option<Value>,
}

impl Notification {
    /// Create a new notification with the given method and optional params.
    pub fn new(method: impl Into<String>, params: Option<Value>) -> Self {
        Self {
            jsonrpc: (),
            method: method.into(),
            params,
        }
    }
}

impl From<Notification> for Request {
    fn from(notif: Notification) -> Self {
        Request {
            jsonrpc: (),
            method: notif.method,
            params: notif.params,
            id: None,
        }
    }
}

/// A JSON-RPC error object.
///
/// ```json
/// {
///   "code": -32601,
///   "message": "Method not found",
///   "data": "unknown_method"
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq, Deserialize, Serialize)]
pub struct ErrorObject {
    /// The error code. Spec-defined codes are in the -32700..-32000 range.
    pub code: i64,

    /// A short human-readable error message.
    pub message: String,

    /// Additional error data (optional).
    pub data: Option<Value>,
}

impl ErrorObject {
    /// Create a new error object.
    pub fn new(code: i64, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }

    /// Add optional data to the error.
    pub fn with_data(mut self, data: Value) -> Self {
        self.data = Some(data);
        self
    }

    /// Override the message of the error.
    pub fn with_message(mut self, message: impl Into<String>) -> Self {
        self.message = message.into();
        self
    }
}

impl std::fmt::Display for ErrorObject {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (code {})", self.message, self.code)
    }
}

impl ErrorObject {
    // JSON-RPC 2.0 spec-defined error constructors

    /// Parse error (-32700): Invalid JSON was received.
    pub fn parse_error() -> Self {
        Self::new(-32700, "Parse error")
    }

    /// Invalid Request (-32600): The JSON sent is not a valid Request object.
    pub fn invalid_request() -> Self {
        Self::new(-32600, "Invalid Request")
    }

    /// Method not found (-32601): The method does not exist / is not available.
    pub fn method_not_found(method: &str) -> Self {
        Self::new(-32601, "Method not found").with_data(method.into())
    }

    /// Invalid params (-32602): Invalid method parameters.
    pub fn invalid_params() -> Self {
        Self::new(-32602, "Invalid params")
    }

    /// Internal error (-32603): Internal JSON-RPC error.
    pub fn internal_error() -> Self {
        Self::new(-32603, "Internal error")
    }

    // Server error range: -32099..-32000
    /// Create a server error with implementation-defined code and message.
    /// The code must be in the range -32099..-32000.
    pub fn server_error(code: i64, message: impl Into<String>) -> Self {
        assert!(
            (-32099..=-32000).contains(&code),
            "server error code must be in -32099..-32000"
        );
        Self::new(code, message)
    }
}

/// A JSON-RPC response object.
///
/// Exactly one of `result` or `error` must be present.
///
/// ```json
/// {
///   "jsonrpc": "2.0",
///   "result": { "tools": [...] },
///   "id": 1
/// }
/// ```
#[derive(Clone, Debug)]
pub struct Response {
    /// Protocol version - always "2.0"
    jsonrpc: (),

    /// The successful result value, if any.
    result: Option<Value>,

    /// The error object, if any.
    error: Option<ErrorObject>,

    /// The request ID - must preserve the type from the request.
    pub id: Id,
}

impl Response {
    /// Create a successful response with the given result.
    pub fn success(id: Id, result: Value) -> Self {
        Self {
            jsonrpc: (),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response with the given error object.
    pub fn error(id: Id, error: ErrorObject) -> Self {
        Self {
            jsonrpc: (),
            result: None,
            error: Some(error),
            id,
        }
    }

    /// Returns true if this is a successful response.
    pub fn is_success(&self) -> bool {
        self.result.is_some()
    }

    /// Returns true if this is an error response.
    pub fn is_error(&self) -> bool {
        self.error.is_some()
    }

    /// Get the result value, if this is a success response.
    pub fn get_result(&self) -> Option<&Value> {
        self.result.as_ref()
    }

    /// Get the error object, if this is an error response.
    pub fn get_error(&self) -> Option<&ErrorObject> {
        self.error.as_ref()
    }

    /// Validate that exactly one of result or error is present.
    fn validate(&self) -> Result<(), String> {
        match (&self.result, &self.error) {
            (None, None) => Err("Response must have either result or error".into()),
            (Some(_), Some(_)) => Err("Response cannot have both result and error".into()),
            _ => Ok(()),
        }
    }
}

impl Serialize for Response {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Validate before serializing
        self.validate().map_err(serde::ser::Error::custom)?;

        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(4))?;
        map.serialize_entry("jsonrpc", JSONRPC_VERSION)?;

        if let Some(result) = &self.result {
            map.serialize_entry("result", result)?;
        }
        if let Some(error) = &self.error {
            map.serialize_entry("error", error)?;
        }

        map.serialize_entry("id", &self.id)?;
        map.end()
    }
}

impl<'de> Deserialize<'de> for Response {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        struct RawResponse {
            #[serde(
                serialize_with = "serialize_jsonrpc",
                deserialize_with = "deserialize_jsonrpc"
            )]
            jsonrpc: (),
            result: Option<Value>,
            error: Option<ErrorObject>,
            id: Id,
        }

        let raw = RawResponse::deserialize(deserializer)?;

        let response = Response {
            jsonrpc: raw.jsonrpc,
            result: raw.result,
            error: raw.error,
            id: raw.id,
        };

        response.validate().map_err(serde::de::Error::custom)?;

        Ok(response)
    }
}

/// A batch message containing either a single request or an array of requests.
///
/// Per the JSON-RPC spec, an empty array is an invalid request.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchMessage {
    Single(Request),
    Batch(Vec<Request>),
}

impl BatchMessage {
    /// Returns true if this is a batch message.
    pub fn is_batch(&self) -> bool {
        matches!(self, BatchMessage::Batch(_))
    }

    /// Returns the number of requests in this message.
    pub fn len(&self) -> usize {
        match self {
            BatchMessage::Single(_) => 1,
            BatchMessage::Batch(reqs) => reqs.len(),
        }
    }

    /// Returns true if this contains no requests (empty batch).
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Extract all requests as a vector.
    pub fn into_requests(self) -> Vec<Request> {
        match self {
            BatchMessage::Single(req) => vec![req],
            BatchMessage::Batch(reqs) => reqs,
        }
    }
}

impl Serialize for BatchMessage {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            BatchMessage::Single(req) => req.serialize(serializer),
            BatchMessage::Batch(reqs) => reqs.serialize(serializer),
        }
    }
}

impl<'de> Deserialize<'de> for BatchMessage {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Use serde_json::Value to detect array vs object
        let value = Value::deserialize(deserializer)?;

        match value {
            Value::Array(arr) => {
                if arr.is_empty() {
                    // Empty array is an invalid request per spec
                    return Err(serde::de::Error::custom(
                        "empty batch array is not a valid request",
                    ));
                }
                // Deserialize each array element as a Request
                let mut reqs = Vec::with_capacity(arr.len());
                for item in arr {
                    let req = Request::deserialize(item).map_err(serde::de::Error::custom)?;
                    reqs.push(req);
                }
                Ok(BatchMessage::Batch(reqs))
            }
            Value::Object(obj) => {
                let req =
                    Request::deserialize(Value::Object(obj)).map_err(serde::de::Error::custom)?;
                Ok(BatchMessage::Single(req))
            }
            _ => Err(serde::de::Error::custom("expected JSON object or array")),
        }
    }
}

impl From<Request> for BatchMessage {
    fn from(req: Request) -> Self {
        BatchMessage::Single(req)
    }
}

impl From<Vec<Request>> for BatchMessage {
    fn from(reqs: Vec<Request>) -> Self {
        BatchMessage::Batch(reqs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Round-trip test
    #[test]
    fn test_request_round_trip() {
        let req = Request::new("tools/list", None, Some(Id::Number(1)));
        let json = serde_json::to_string(&req).unwrap();
        let deserialized: Request = serde_json::from_str(&json).unwrap();
        assert_eq!(req, deserialized);
    }

    // ID preservation tests
    #[test]
    fn test_id_number_preservation() {
        let id = Id::Number(42);
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "42");
        let deserialized: Id = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_id_string_preservation() {
        let id = Id::String("abc-123".to_string());
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "\"abc-123\"");
        let deserialized: Id = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_id_null_preservation() {
        let id = Id::Null;
        let json = serde_json::to_string(&id).unwrap();
        assert_eq!(json, "null");
        let deserialized: Id = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    // Parse error path
    #[test]
    fn test_parse_error_response() {
        let err = ErrorObject::parse_error();
        let resp = Response::error(Id::Null, err);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("\"code\":-32700"));
        assert!(json.contains("\"id\":null"));
    }

    // Method not found
    #[test]
    fn test_method_not_found() {
        let err = ErrorObject::method_not_found("unknown_method");
        assert_eq!(err.code, -32601);
        assert_eq!(err.message, "Method not found");
        assert_eq!(err.data, Some(Value::String("unknown_method".to_string())));
    }

    // Notification
    #[test]
    fn test_notification_no_id() {
        let notif = Notification::new("notifications/message", None);
        let json = serde_json::to_string(&notif).unwrap();
        assert!(!json.contains("\"id\""));

        // Deserialize as Request should work with id: None
        let req: Request = serde_json::from_str(&json).unwrap();
        assert!(req.is_notification());
    }

    // Batch round-trip
    #[test]
    fn test_batch_round_trip() {
        let reqs = vec![
            Request::new("tools/list", None, Some(Id::Number(1))),
            Request::new(
                "tools/call",
                Some(Value::Object(serde_json::Map::new())),
                Some(Id::Number(2)),
            ),
            Request::new("prompts/list", None, Some(Id::String("abc".to_string()))),
        ];
        let batch = BatchMessage::Batch(reqs.clone());
        let json = serde_json::to_string(&batch).unwrap();
        let deserialized: BatchMessage = serde_json::from_str(&json).unwrap();
        assert_eq!(batch, deserialized);
    }

    // Error code constants
    #[test]
    fn test_all_error_codes() {
        assert_eq!(ErrorObject::parse_error().code, -32700);
        assert_eq!(ErrorObject::invalid_request().code, -32600);
        assert_eq!(ErrorObject::method_not_found("x").code, -32601);
        assert_eq!(ErrorObject::invalid_params().code, -32602);
        assert_eq!(ErrorObject::internal_error().code, -32603);
    }

    // jsonrpc field validation
    #[test]
    fn test_reject_invalid_jsonrpc_version() {
        let json = r#"{"jsonrpc":"1.0","method":"test","id":1}"#;
        let result: Result<Request, _> = serde_json::from_str(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("invalid jsonrpc version"));
    }

    #[test]
    fn test_reject_missing_jsonrpc_field() {
        let json = r#"{"method":"test","id":1}"#;
        let result: Result<Request, _> = serde_json::from_str(json);
        assert!(result.is_err());
    }

    // Empty batch is invalid
    #[test]
    fn test_empty_batch_rejected() {
        let json = r#"[]"#;
        let result: Result<BatchMessage, _> = serde_json::from_str(json);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("empty batch"));
    }

    // Response validation
    #[test]
    fn test_response_success() {
        let resp = Response::success(Id::Number(1), Value::String("ok".to_string()));
        assert!(resp.is_success());
        assert!(!resp.is_error());
        assert_eq!(resp.get_result(), Some(&Value::String("ok".to_string())));
    }

    #[test]
    fn test_response_error() {
        let err = ErrorObject::method_not_found("test");
        let resp = Response::error(Id::Number(1), err);
        assert!(!resp.is_success());
        assert!(resp.is_error());
        assert!(resp.get_error().is_some());
    }

    // Notification deserialization
    #[test]
    fn test_notification_deserialize() {
        let json = r#"{"jsonrpc":"2.0","method":"test","params":null}"#;
        let req: Request = serde_json::from_str(json).unwrap();
        assert!(req.is_notification());
        assert_eq!(req.method, "test");
    }

    // Response with null id (parse error case)
    #[test]
    fn test_response_null_id_serializes() {
        let resp = Response::error(Id::Null, ErrorObject::parse_error());
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains(r#""id":null"#));
    }
}
