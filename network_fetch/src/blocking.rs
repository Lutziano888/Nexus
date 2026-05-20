//! Synchronous (blocking) wrapper around the async fetch functions.
//!
//! Useful when you are not in an async context or when integrating with
//! engines / FFI layers that don't support `async/await`.

use crate::{error::FetchError, response::FetchResponse};

/// Synchronous version of [`fetch`](crate::fetch).
///
/// Spawns a one-shot Tokio runtime on a fresh thread so it never conflicts
/// with a parent async runtime.
pub fn fetch_blocking(url: &str) -> Result<FetchResponse, FetchError> {
    let url = url.to_string();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build Tokio runtime for blocking fetch");

        rt.block_on(crate::fetch(&url))
    })
    .join()
    .unwrap_or_else(|_| Err(FetchError::DecodeError("Blocking thread panicked".into())))
}

/// Synchronous version of [`fetch_bytes`](crate::fetch_bytes).
///
/// Lädt Binärdaten (z.B. Bilder) ohne UTF-8-Dekodierung.
/// Muss für alle Nicht-Text-Ressourcen verwendet werden.
pub fn fetch_bytes_blocking(url: &str) -> Result<Vec<u8>, FetchError> {
    let url = url.to_string();

    std::thread::spawn(move || {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("Failed to build Tokio runtime for blocking fetch_bytes");

        rt.block_on(crate::fetch_bytes(&url))
    })
    .join()
    .unwrap_or_else(|_| Err(FetchError::DecodeError("Blocking thread panicked".into())))
}