// ─── Text Measurement Trait ───────────────────────────────────────────────
// 
// Abstrahiert die Text-Messung für die Layout-Engine.
// Der renderer kann dies implementieren, um echte Textbreiten zu liefern.

use std::sync::Arc;
use std::sync::OnceLock;

pub trait TextMeasurer: Send + Sync {
    /// Misst die Breite eines Text-Strings bei gegebener Schriftgröße
    /// Gibt die Breite in Pixeln zurück
    fn measure_text_width(&self, text: &str, font_size_px: f32) -> f32;
    
    /// Misst Höhe basierend auf font_size (typisch: font_size * 1.4 für line-height)
    fn measure_text_height(&self, font_size_px: f32) -> f32 {
        font_size_px * 1.4
    }
}

/// Default-Implementierung: Grobe Schätzung
/// Wird verwendet, wenn kein echter TextMeasurer verfügbar ist
pub struct DefaultTextMeasurer;

impl TextMeasurer for DefaultTextMeasurer {
    fn measure_text_width(&self, text: &str, font_size_px: f32) -> f32 {
        // Grobe Schätzung: durchschnittlich ~0.5 * font_size pro Zeichen
        // Bei 16px font: ~8px pro Zeichen durchschnittlich
        text.len() as f32 * (font_size_px * 0.55)
    }
    
    fn measure_text_height(&self, font_size_px: f32) -> f32 {
        font_size_px * 1.4
    }
}

/// Globaler Text-Measurer für die Layout-Engine
/// Kann von außen mit einem echten Measurer initialisiert werden
static GLOBAL_TEXT_MEASURER: OnceLock<Arc<dyn TextMeasurer>> = OnceLock::new();

/// Setzt den globalen Text-Measurer (nur möglich vor erstem Zugriff)
pub fn set_text_measurer(measurer: Arc<dyn TextMeasurer>) {
    let _ = GLOBAL_TEXT_MEASURER.set(measurer);
}

/// Gibt den aktuellen Text-Measurer zurück
pub fn get_text_measurer() -> Arc<dyn TextMeasurer> {
    GLOBAL_TEXT_MEASURER
        .get_or_init(|| Arc::new(DefaultTextMeasurer))
        .clone()
}
