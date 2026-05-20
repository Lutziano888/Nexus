//! Response structures returned by fetch operations.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A simple key→value map of HTTP response headers.
pub type HeaderMap = HashMap<String, String>;

/// The complete result of a successful HTTP fetch.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FetchResponse {
    /// The final URL after any redirects.
    pub url: String,

    /// HTTP status code (e.g. 200, 301, …).
    pub status_code: u16,

    /// All response headers, normalised to lowercase keys.
    pub headers: HeaderMap,

    /// The full response body as a UTF-8 string.
    pub body: String,

    /// Value of the `Content-Type` header (empty string if absent).
    pub content_type: String,
}

impl FetchResponse {
    /// Returns `true` when the response body looks like HTML.
    pub fn is_html(&self) -> bool {
        self.content_type.contains("text/html")
    }

    /// Returns `true` when the response body looks like JSON.
    pub fn is_json(&self) -> bool {
        self.content_type.contains("application/json")
    }

    /// Attempt to parse the body as JSON into any deserializable type.
    pub fn json<T: for<'de> Deserialize<'de>>(&self) -> Result<T, serde_json::Error> {
        serde_json::from_str(&self.body)
    }

    /// Return the value of a specific header (case-insensitive lookup).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.get(&name.to_lowercase()).map(String::as_str)
    }
}