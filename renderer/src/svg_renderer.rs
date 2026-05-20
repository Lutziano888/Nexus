// ╔══════════════════════════════════════════════════════════════════════════╗
// ║   SVG Renderer – Rendering SVGs für moderne Websites (Icons, Graphics)   ║
// ╚══════════════════════════════════════════════════════════════════════════╝

use std::collections::HashMap;
use resvg::render;
use usvg::{Tree, Options};

/// Cache für bereits gerendertes SVG (URL -> Pixmap-Daten)
pub struct SvgCache {
    cache: HashMap<String, Vec<u8>>,
    max_size: usize,
}

impl SvgCache {
    pub fn new(max_size: usize) -> Self {
        SvgCache {
            cache: HashMap::new(),
            max_size,
        }
    }

    /// SVG von URL laden, parsen und rendern
    pub fn render_svg(&mut self, svg_url: &str, svg_data: &str, width: u32, height: u32) -> Option<Vec<u8>> {
        // Aus Cache prüfen
        if let Some(cached) = self.cache.get(svg_url) {
            return Some(cached.clone());
        }

        // SVG parsen
        let tree = match Tree::from_str(svg_data, &Options::default()) {
            Ok(t) => t,
            Err(e) => {
                eprintln!("SVG Parse Error: {}", e);
                return None;
            }
        };

        // In Pixmap rendern
        let mut pixmap = resvg::tiny_skia::Pixmap::new(width, height)?;
        {
            let mut pm = pixmap.as_mut();
            render(&tree, resvg::tiny_skia::Transform::default(), &mut pm);
        }

        let data = pixmap.encode_png().ok()?;

        // In Cache speichern (falls nicht zu groß)
        if data.len() < self.max_size {
            self.cache.insert(svg_url.to_string(), data.clone());
        }

        Some(data)
    }

    /// SVG von Datei laden und rendern
    pub fn render_svg_file(&mut self, file_path: &str, width: u32, height: u32) -> Option<Vec<u8>> {
        let svg_data = std::fs::read_to_string(file_path).ok()?;
        self.render_svg(file_path, &svg_data, width, height)
    }

    /// Cache leeren
    pub fn clear(&mut self) {
        self.cache.clear();
    }

    /// Cache-Statistiken
    pub fn stats(&self) -> (usize, usize) {
        (self.cache.len(), self.cache.values().map(|v| v.len()).sum())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_svg_parse() {
        let mut cache = SvgCache::new(1024 * 1024);
        let simple_svg = r#"<svg viewBox="0 0 100 100" xmlns="http://www.w3.org/2000/svg">
            <circle cx="50" cy="50" r="40" fill="blue" />
        </svg>"#;

        let result = cache.render_svg("test.svg", simple_svg, 100, 100);
        assert!(result.is_some());
        let (count, _size) = cache.stats();
        assert_eq!(count, 1);
    }
}
