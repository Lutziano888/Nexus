//! # network_fetch
//!
//! A modular, async-ready HTTP fetch library built on `reqwest` + `tokio`.
//!
//! ## Features
//! - Async (`fetch`, `fetch_bytes`) and blocking (`fetch_blocking`, `fetch_bytes_blocking`) variants
//! - Structured response: body + typed headers
//! - Typed error handling for HTTP 404, 500, and network failures
//!
//! ## Quick Start (async)
//! ```rust,no_run
//! use network_fetch::{fetch, FetchError};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), FetchError> {
//!     let response = fetch("https://httpbin.org/get").await?;
//!     println!("Status : {}", response.status_code);
//!     println!("Body   : {}", &response.body[..200]);
//!     Ok(())
//! }
//! ```

pub mod error;
pub mod response;
pub mod client;
pub mod blocking;

pub use error::FetchError;
pub use response::{FetchResponse, HeaderMap};
pub use client::{fetch, fetch_bytes};
pub use blocking::{fetch_blocking, fetch_bytes_blocking};

