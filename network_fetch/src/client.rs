//! Async HTTP client – the primary interface for the library.

use crate::{
    error::FetchError,
    response::{FetchResponse, HeaderMap},
};
use reqwest::header::{HeaderValue, ACCEPT, ACCEPT_LANGUAGE, CONNECTION, UPGRADE_INSECURE_REQUESTS};

/// Fetch a URL asynchronously and return a structured [`FetchResponse`].
///
/// # Errors
/// - [`FetchError::NotFound`]    – HTTP 404
/// - [`FetchError::ServerError`] – HTTP 500
/// - [`FetchError::HttpError`]   – any other non-2xx status
/// - [`FetchError::NetworkError`] – connection / TLS failures
/// - [`FetchError::InvalidUrl`]  – malformed URL
pub async fn fetch(url: &str) -> Result<FetchResponse, FetchError> {
    if url.is_empty() {
        return Err(FetchError::InvalidUrl("URL must not be empty".into()));
    }

    let client = build_client()?;
    let response = client.get(url).send().await?;

    let status = response.status().as_u16();
    let final_url = response.url().to_string();

    let headers = extract_headers(response.headers());
    let content_type = headers
        .get("content-type")
        .cloned()
        .unwrap_or_default();

    let body = response
        .text()
        .await
        .map_err(|e| FetchError::DecodeError(e.to_string()))?;

    Ok(FetchResponse {
        url: final_url,
        status_code: status,
        headers,
        body,
        content_type,
    })
}

/// Fetch a URL asynchronously and return the raw bytes.
/// Für Binärdaten (Bilder, etc.) – vermeidet UTF-8-Dekodierung die Binärdaten korrumpiert.
pub async fn fetch_bytes(url: &str) -> Result<Vec<u8>, FetchError> {
    if url.is_empty() {
        return Err(FetchError::InvalidUrl("URL must not be empty".into()));
    }

    let client = build_client()?;
    let response = client.get(url).send().await?;

    response
        .bytes()
        .await
        .map(|b| b.to_vec())
        .map_err(|e| FetchError::DecodeError(e.to_string()))
}

/// Build a reusable `reqwest::Client` with sensible defaults.
fn build_client() -> Result<reqwest::Client, FetchError> {
    let mut headers = reqwest::header::HeaderMap::new();
    headers.insert(ACCEPT, HeaderValue::from_static("text/html,application/xhtml+xml,application/xml;q=0.9,image/avif,image/webp,*/*;q=0.8"));
    headers.insert(ACCEPT_LANGUAGE, HeaderValue::from_static("en-US,en;q=0.5"));
    headers.insert(CONNECTION, HeaderValue::from_static("keep-alive"));
    headers.insert(UPGRADE_INSECURE_REQUESTS, HeaderValue::from_static("1"));

    reqwest::Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .default_headers(headers)
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(FetchError::NetworkError)
}

/// Extract reqwest headers into our own `HeaderMap` (lowercase keys).
pub(crate) fn extract_headers(headers: &reqwest::header::HeaderMap) -> HeaderMap {
    headers
        .iter()
        .filter_map(|(name, value)| {
            let key = name.as_str().to_lowercase();
            let val = value.to_str().ok()?.to_string();
            Some((key, val))
        })
        .collect()
}