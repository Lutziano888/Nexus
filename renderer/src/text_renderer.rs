// ─── Text Renderer  (cosmic-text 0.12) ───────────────────────────────────────

use cosmic_text::{
    Attrs, Buffer, Color as CosmicColor, FontSystem, Metrics,
    Shaping, SwashCache,
};
use softbuffer::Buffer as SbBuffer;
use std::sync::{Arc, Mutex};
use winit::window::Window;

static FONT_BYTES: &[u8] = include_bytes!("../assets/font.ttf");

struct FontState {
    font_system: FontSystem,
    swash_cache: SwashCache,
}

impl FontState {
    fn new() -> Self {
        let mut font_system = FontSystem::new();
        font_system.db_mut().load_font_data(FONT_BYTES.to_vec());
        Self { font_system, swash_cache: SwashCache::new() }
    }
}

pub struct TextRenderer {
    state:   Mutex<FontState>,
    px_size: f32,
}

impl TextRenderer {
    pub fn new(px_size: f32) -> Self {
        Self { state: Mutex::new(FontState::new()), px_size }
    }

    pub fn draw(
        &self,
        sb_buf: &mut SbBuffer<'_, Arc<Window>, Arc<Window>>,
        win_w:  u32,
        win_h:  u32,
        text:   &str,
        x:      i32,
        y:      i32,
        color:  u32,
        font_size: Option<f32>,
    ) {
        if text.is_empty() { return; }

        let r = ((color >> 16) & 0xFF) as u8;
        let g = ((color >>  8) & 0xFF) as u8;
        let b = ( color        & 0xFF) as u8;
        let fg = CosmicColor::rgb(r, g, b);

        let mut state = self.state.lock().unwrap();
        let FontState { font_system, swash_cache } = &mut *state;

        let px = font_size.unwrap_or(self.px_size).clamp(8.0, 72.0);
        let line_h = px * 1.2;
        let metrics = Metrics::new(px, line_h);

        // borrow_with bindet font_system an den Buffer –
        // set_text, shape und draw alle auf demselben borrow_with-Ergebnis
        let mut ct_buf = Buffer::new(font_system, metrics);
        let mut buf = ct_buf.borrow_with(font_system);
        buf.set_size(Some(win_w as f32), Some(line_h * 1.5));
        buf.set_text(text, Attrs::new(), Shaping::Advanced);
        buf.shape_until_scroll(false);

        buf.draw(swash_cache, fg, |px_rel, py_rel, _w, _h, col: CosmicColor| {
            let alpha = col.a() as f32 / 255.0;
            if alpha < 0.01 { return; }

            let px = x + px_rel;
            let py = y + py_rel;
            if px < 0 || py < 0 { return; }
            let px = px as u32;
            let py = py as u32;
            if px >= win_w || py >= win_h { return; }

            let idx = (py * win_w + px) as usize;
            if idx >= sb_buf.len() { return; }

            let bg  = sb_buf[idx];
            let br  = ((bg >> 16) & 0xFF) as f32;
            let bg_ = ((bg >>  8) & 0xFF) as f32;
            let bb  = ( bg        & 0xFF) as f32;
            let fr  = col.r() as f32;
            let fg_ = col.g() as f32;
            let fb  = col.b() as f32;

            let nr = (fr * alpha + br * (1.0 - alpha)) as u32;
            let ng = (fg_ * alpha + bg_ * (1.0 - alpha)) as u32;
            let nb = (fb * alpha + bb * (1.0 - alpha)) as u32;
            sb_buf[idx] = (nr << 16) | (ng << 8) | nb;
        });
    }

    pub fn measure_width(&self, text: &str, font_size: Option<f32>) -> i32 {
        if text.is_empty() { return 0; }

        let mut state = self.state.lock().unwrap();
        let FontState { font_system, .. } = &mut *state;

        let px = font_size.unwrap_or(self.px_size).clamp(8.0, 72.0);
        let line_h = px * 1.2;
        let mut ct_buf = Buffer::new(font_system, Metrics::new(px, line_h));
        let mut buf = ct_buf.borrow_with(font_system);
        buf.set_size(Some(10_000.0), Some(line_h));
        buf.set_text(text, Attrs::new(), Shaping::Advanced);
        buf.shape_until_scroll(false);

        let mut total = 0.0_f32;
        for run in buf.layout_runs() {
            for glyph in run.glyphs { total += glyph.w; }
        }
        total.ceil() as i32
    }
    /// Text mit Zeilenumbruch rendern.
    /// max_w: maximale Breite in Pixeln
    /// Gibt die tatsaechlich gerenderte Hoehe zurueck.
    pub fn draw_wrapped(
        &self,
        sb_buf: &mut SbBuffer<'_, Arc<Window>, Arc<Window>>,
        win_w:  u32,
        win_h:  u32,
        text:   &str,
        x:      i32,
        y:      i32,
        max_w:  i32,
        color:  u32,
        font_size: Option<f32>,
    ) -> i32 {
        if text.is_empty() { return 0; }

        let r = ((color >> 16) & 0xFF) as u8;
        let g = ((color >>  8) & 0xFF) as u8;
        let b = ( color        & 0xFF) as u8;
        let fg = CosmicColor::rgb(r, g, b);

        let mut state = self.state.lock().unwrap();
        let FontState { font_system, swash_cache } = &mut *state;

        let px      = font_size.unwrap_or(self.px_size).clamp(8.0, 72.0);
        let line_h  = px * 1.4;
        let metrics = Metrics::new(px, line_h);

        let max_h = (win_h as f32).min(8000.0);
        let mut ct_buf = Buffer::new(font_system, metrics);
        let mut buf = ct_buf.borrow_with(font_system);
        buf.set_size(Some(max_w as f32), Some(max_h));
        buf.set_text(text, Attrs::new(), Shaping::Advanced);
        buf.shape_until_scroll(false);

        let mut rendered_h = 0i32;
        buf.draw(swash_cache, fg, |px_rel, py_rel, _w, _h, col: CosmicColor| {
            let alpha = col.a() as f32 / 255.0;
            if alpha < 0.01 { return; }
            let draw_x = x + px_rel;
            let draw_y = y + py_rel;
            if draw_x < 0 || draw_y < 0 { return; }
            let draw_x = draw_x as u32;
            let draw_y = draw_y as u32;
            if draw_x >= win_w || draw_y >= win_h { return; }
            let idx = (draw_y * win_w + draw_x) as usize;
            if idx >= sb_buf.len() { return; }
            let bg  = sb_buf[idx];
            let br  = ((bg >> 16) & 0xFF) as f32;
            let bg_ = ((bg >>  8) & 0xFF) as f32;
            let bb  = ( bg        & 0xFF) as f32;
            let fr  = col.r() as f32;
            let fg_ = col.g() as f32;
            let fb  = col.b() as f32;
            let nr = (fr * alpha + br * (1.0 - alpha)) as u32;
            let ng = (fg_ * alpha + bg_ * (1.0 - alpha)) as u32;
            let nb = (fb * alpha + bb * (1.0 - alpha)) as u32;
            sb_buf[idx] = (nr << 16) | (ng << 8) | nb;
            if py_rel + 1 > rendered_h { rendered_h = py_rel + 1; }
        });
        rendered_h
    }

    /// Berechnet die Hoehe die draw_wrapped brauchen wuerde.
    pub fn measure_height(&self, text: &str, max_w: i32, font_size: Option<f32>) -> i32 {
        if text.is_empty() { return 0; }
        let mut state = self.state.lock().unwrap();
        let FontState { font_system, .. } = &mut *state;
        let px     = font_size.unwrap_or(self.px_size).clamp(8.0, 72.0);
        let line_h = px * 1.4;
        let mut ct_buf = Buffer::new(font_system, Metrics::new(px, line_h));
        let mut buf = ct_buf.borrow_with(font_system);
        buf.set_size(Some(max_w as f32), Some(8000.0));
        buf.set_text(text, Attrs::new(), Shaping::Advanced);
        buf.shape_until_scroll(false);
        let lines = buf.layout_runs().count() as i32;
        (lines as f32 * line_h).ceil() as i32
    }
}