//! Demo example – run with:  cargo run --example demo

use network_fetch::{fetch, fetch_blocking, FetchError};

#[tokio::main]
async fn main() {
    println!("═══════════════════════════════════════");
    println!("  network_fetch  –  Demo");
    println!("═══════════════════════════════════════\n");

    // ── 1. Successful async fetch ────────────────────────────────────────────
    println!("▶ [ASYNC] Fetching https://httpbin.org/get …");
    match fetch("https://httpbin.org/get").await {
        Ok(resp) => {
            println!("  Status      : {}", resp.status_code);
            println!("  Content-Type: {}", resp.content_type);
            println!("  Headers     : {} total", resp.headers.len());
            println!("  Body (first 200 chars):\n{}\n", &resp.body[..resp.body.len().min(200)]);
        }
        Err(e) => eprintln!("  Error: {e}"),
    }

    // ── 2. 404 error handling ────────────────────────────────────────────────
    println!("▶ [ASYNC] Triggering a 404 …");
    match fetch("https://httpbin.org/status/404").await {
        Ok(_) => println!("  Unexpected success"),
        Err(FetchError::NotFound { url }) => {
            println!("  ✓ Caught NotFound for: {url}\n")
        }
        Err(e) => eprintln!("  Unexpected error: {e}"),
    }

    // ── 3. 500 error handling ────────────────────────────────────────────────
    println!("▶ [ASYNC] Triggering a 500 …");
    match fetch("https://httpbin.org/status/500").await {
        Ok(_) => println!("  Unexpected success"),
        Err(FetchError::ServerError { url }) => {
            println!("  ✓ Caught ServerError for: {url}\n")
        }
        Err(e) => eprintln!("  Unexpected error: {e}"),
    }

    // ── 4. Blocking variant ──────────────────────────────────────────────────
    println!("▶ [BLOCKING] Fetching https://httpbin.org/headers …");
    match fetch_blocking("https://httpbin.org/headers") {
        Ok(resp) => {
            println!("  Status: {}", resp.status_code);
            println!("  Body  :\n{}\n", &resp.body[..resp.body.len().min(300)]);
        }
        Err(e) => eprintln!("  Error: {e}"),
    }

    println!("═══════════════════════════════════════");
    println!("  Demo complete.");
    println!("═══════════════════════════════════════");
}