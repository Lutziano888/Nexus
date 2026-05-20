//! Typed errors for the network_fetch library.

use thiserror::Error;

/// All errors that can occur during a fetch operation.
#[derive(Debug, Error)]
pub enum FetchError {
    /// The server returned HTTP 404 – resource not found.
    #[error("HTTP 404 Not Found: {url}")]
    NotFound { url: String },

    /// The server returned HTTP 500 – internal server error.
    #[error("HTTP 500 Internal Server Error: {url}")]
    ServerError { url: String },

    /// Any other non-success HTTP status code.
    #[error("HTTP {status} error for URL: {url}")]
    HttpError { status: u16, url: String },

    /// A network-level failure (DNS, timeout, TLS, …).
    #[error("Network error: {0}")]
    NetworkError(#[from] reqwest::Error),

    /// The response body could not be decoded as UTF-8 text.
    #[error("Failed to decode response body: {0}")]
    DecodeError(String),

    /// An invalid URL was supplied.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),
}

/// Convenience conversion: map a `reqwest::StatusCode` + URL into a `FetchError`.
#[allow(dead_code)]
pub(crate) fn status_to_error(status: u16, url: &str) -> FetchError {
    match status {
        404 => FetchError::NotFound { url: url.to_string() },
        500 => FetchError::ServerError { url: url.to_string() },
        code => FetchError::HttpError { status: code, url: url.to_string() },
    }
}