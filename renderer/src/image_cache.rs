// ─── Image Cache  ─────────────────────────────────────────────────────────────
// Lädt Bilder per URL in Background-Threads, cached sie als RGBA-Pixel.
// get_or_load() gibt sofort zurück (None wenn noch nicht geladen),
// on_done-Callback triggert Redraw sobald ein Bild fertig ist.

use std::collections::HashMap;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct DecodedImage {
    pub pixels: Vec<u32>, // 0x00RRGGBB (alpha vorgemischt auf Weiß)
    pub width:  u32,
    pub height: u32,
}

enum CacheEntry {
    Pending,
    Done(Option<DecodedImage>),
}

#[derive(Default)]
struct CacheInner {
    map: HashMap<String, CacheEntry>,
}

#[derive(Clone, Default)]
pub struct ImageCache {
    inner: Arc<Mutex<CacheInner>>,
}

impl ImageCache {
    pub fn new() -> Self { Self::default() }

    /// Gibt gecachtes Bild zurück oder startet Background-Load.
    /// `on_done` wird aufgerufen sobald das Bild fertig geladen ist (aus dem BG-Thread).
    pub fn get_or_load_with_callback<F>(&self, url: &str, on_done: F) -> Option<DecodedImage>
    where F: Fn() + Send + 'static
    {
        {
            let inner = self.inner.lock().unwrap();
            match inner.map.get(url) {
                Some(CacheEntry::Done(img)) => return img.clone(),
                Some(CacheEntry::Pending)   => return None,
                None => {}
            }
        }
        {
            let mut inner = self.inner.lock().unwrap();
            // Nochmal prüfen nach dem Lock-Upgrade (kein TOCTOU)
            if inner.map.contains_key(url) { return None; }
            inner.map.insert(url.to_string(), CacheEntry::Pending);
        }

        let url_owned = url.to_string();
        let cache_ref = self.inner.clone();
        std::thread::spawn(move || {
            let result = load_image(&url_owned);
            {
                let mut inner = cache_ref.lock().unwrap();
                inner.map.insert(url_owned, CacheEntry::Done(result));
            }
            on_done();
        });
        None
    }

    /// Einfache Version ohne Callback (für Rückwärtskompatibilität).
    pub fn get_or_load(&self, url: &str) -> Option<DecodedImage> {
        self.get_or_load_with_callback(url, || {})
    }
}

fn load_image(url: &str) -> Option<DecodedImage> {
    let url_string = if url.starts_with("//") {
        format!("https:{}", url)
    } else {
        url.to_string()
    };
    let url = &url_string;

    // Leere URLs und data:-Platzhalter (Wikipedia lazy-loading) überspringen
    if url.is_empty() || url.starts_with("data:") {
        return None;
    }

    println!("[IMG] Lade: {}", url);

    // fetch_bytes_blocking statt fetch_blocking verwenden!
    // fetch_blocking dekodiert den Body als UTF-8 → korrumpiert Binärdaten.
    let bytes = match network_fetch::fetch_bytes_blocking(url) {
        Ok(b) => b,
        Err(e) => {
            println!("[IMG] Netzwerkfehler {}: {}", url, e);
            return None;
        }
    };

    if bytes.is_empty() {
        println!("[IMG] Leer: {}", url);
        return None;
    }

    let img = match image::load_from_memory(&bytes) {
        Ok(i) => i,
        Err(e) => {
            println!("[IMG] Decode-Fehler {}: {}", url, e);
            return None;
        }
    };

    let rgba = img.to_rgba8();
    let (w, h) = rgba.dimensions();

    let pixels: Vec<u32> = rgba
        .chunks_exact(4)
        .map(|p| {
            let r = p[0] as u32;
            let g = p[1] as u32;
            let b = p[2] as u32;
            let a = p[3] as u32;
            if a == 0 {
                // Vollständig transparent → weißer Hintergrund
                0x00_FF_FF_FF
            } else if a == 255 {
                // Vollständig opak → direkt übernehmen
                (r << 16) | (g << 8) | b
            } else {
                // Teiltransparent → alpha auf Weiß vorrechnen
                let inv = 255 - a;
                let nr = (r * a + 255 * inv) / 255;
                let ng = (g * a + 255 * inv) / 255;
                let nb = (b * a + 255 * inv) / 255;
                (nr << 16) | (ng << 8) | nb
            }
        })
        .collect();

    println!("[IMG] OK: {}×{} {}", w, h, url);
    Some(DecodedImage { pixels, width: w, height: h })
}