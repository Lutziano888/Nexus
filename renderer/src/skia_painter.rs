// ─── Skia Painter  v1.0  ──────────────────────────────────────────────────────
//
// Ersetzt den softbuffer-basierten Painter durch Skia als 2D-Backend.
//
// Architektur:
//   - SkiaPainter hält eine Skia-Surface (Raster, BGRA8888)
//   - paint_layout() zeichnet alles in die Skia-Surface
//   - finish() kopiert die Pixel in den softbuffer-Buffer
//
// Cargo.toml-Abhängigkeiten (hinzufügen):
//   skia-safe = { version = "0.70", features = ["textlayout"] }
//
// Die skia-safe-Crate bringt vorgebaute Skia-Binaries mit (via skia-bindings),
// kein separater Build nötig wenn das Feature "binary-cache" aktiv ist.
// Falls der Build zu lange dauert:
//   skia-safe = { version = "0.70", features = ["textlayout", "binary-cache"] }

use skia_safe::{
    Canvas, Color, FontMgr, FontStyle, ISize, ImageInfo, Paint, PaintStyle,
    Point, Rect, Surface, TileMode, gradient_shader,
    textlayout::{
        FontCollection, ParagraphBuilder, ParagraphStyle,
        TextAlign, TextStyle,
    },
};
use std::sync::Arc;
use winit::window::Window;
use softbuffer::Buffer as SbBuffer;

use layout_engine::layout::LayoutBox;
use layout_engine::cssom::{PositionValue, TextAlignValue, TextDecorationValue};
use crate::BrowserState;
use crate::layout_bridge::{HEADER_H, STATUS_H};
use crate::FocusedElement;
use crate::NodePtr;
use crate::image_cache::{DecodedImage, ImageCache};

// ─── Farbpalette (identisch mit painter.rs) ───────────────────────────────────
const COL_TOOLBAR_BG:      u32 = 0x00_32_33_36;
const COL_TOOLBAR_BOTTOM:  u32 = 0x00_20_20_22;
const COL_BTN_BG:          u32 = 0x00_42_43_47;
const COL_BTN_TEXT:        u32 = 0x00_E8_EA_ED;
const COL_URL_BG:          u32 = 0x00_20_21_24;
const COL_URL_BG_FOCUS:    u32 = 0x00_28_2A_2E;
const COL_URL_BORDER:      u32 = 0x00_8A_B4_F8;
const COL_URL_TEXT:        u32 = 0x00_E8_EA_ED;
const COL_URL_PLACEHOLDER: u32 = 0x00_9A_A0_A6;
const COL_PAGE_BG:         u32 = 0x00_FF_FF_FF;
const COL_PAGE_TEXT:       u32 = 0x00_20_21_24;
const COL_LINK:            u32 = 0x00_18_58_AB;
const COL_LINK_HOVER:      u32 = 0x00_0B_57_D0;
const COL_H1:              u32 = 0x00_10_11_14;
const COL_H2:              u32 = 0x00_20_21_24;
const COL_MUTED:           u32 = 0x00_70_75_7A;
const COL_INPUT_BG:        u32 = 0x00_FF_FF_FF;
const COL_INPUT_BORDER:    u32 = 0x00_C4_C7_C5;
const COL_INPUT_FOCUS:     u32 = 0x00_18_58_AB;
const COL_INPUT_TEXT:      u32 = 0x00_20_21_24;
const COL_BTN_PAGE_BG:     u32 = 0x00_18_58_AB;
const COL_BTN_PAGE_HOVER:  u32 = 0x00_0B_57_D0;
const COL_BTN_PAGE_TEXT:   u32 = 0x00_FF_FF_FF;
const COL_STATUS_BG:       u32 = 0x00_F8_F9_FA;
const COL_STATUS_TEXT:     u32 = 0x00_44_47_4A;
const COL_STATUS_BORDER:   u32 = 0x00_DA_DC_E0;
const COL_CURSOR:          u32 = 0x00_18_58_AB;
const COL_LOADING_BAR:     u32 = 0x00_18_58_AB;
const COL_CODE_BG:         u32 = 0x00_F1_F3_F4;
const COL_BLOCKQUOTE:      u32 = 0x00_18_58_AB;
const COL_SELECT_BG:       u32 = 0x00_FF_FF_FF;
const COL_TABLE_HEADER:    u32 = 0x00_F1_F3_F4;

// ─── Hilfsfunktion: u32-Farbe → Skia-Color ────────────────────────────────────

#[inline]
fn sk_color(rgb: u32) -> Color {
    let r = ((rgb >> 16) & 0xFF) as u8;
    let g = ((rgb >>  8) & 0xFF) as u8;
    let b = ( rgb        & 0xFF) as u8;
    Color::from_rgb(r, g, b)
}

#[inline]
fn sk_color_alpha(rgb: u32, alpha: u8) -> Color {
    let r = ((rgb >> 16) & 0xFF) as u8;
    let g = ((rgb >>  8) & 0xFF) as u8;
    let b = ( rgb        & 0xFF) as u8;
    Color::from_argb(alpha, r, g, b)
}

// ─── Node-Accessor-Hilfsfunktionen ────────────────────────────────────────────

fn node_tag(layout: &LayoutBox) -> &str {
    layout.tag_name.as_str()
}

fn is_text_node(layout: &LayoutBox) -> bool {
    layout.text.is_some()
}

fn node_text(layout: &LayoutBox) -> &str {
    layout.text.as_deref().unwrap_or("")
}

fn node_attr<'a>(layout: &'a LayoutBox, attr: &str) -> &'a str {
    layout.attributes.get(attr).map(|s| s.as_str()).unwrap_or("")
}

fn layout_max_bottom(layout: &LayoutBox) -> i32 {
    let own = (layout.y + layout.height) as i32;
    layout.children.iter().map(layout_max_bottom).fold(own, i32::max)
}

// ─── FontCollection (singleton-ähnlich) ───────────────────────────────────────

fn make_font_collection() -> FontCollection {
    let mut fc = FontCollection::new();
    fc.set_default_font_manager(FontMgr::new(), None);
    fc
}

// ─── Render-Kontext ───────────────────────────────────────────────────────────

struct RenderCtx<'a> {
    win_w: f32,
    win_h: f32,
    clip_x: f32,
    clip_y: f32,
    clip_w: f32,
    clip_h: f32,
    container_w: f32,
    scroll: f32,
    mouse_x: f32,
    mouse_y: f32,
    focused: &'a FocusedElement,
    input_values: &'a std::collections::HashMap<NodePtr, String>,
    hovered_href: Option<&'a str>,
    image_cache: &'a ImageCache,
    window: Option<&'a Arc<Window>>,
    font_collection: &'a FontCollection,
}

impl<'a> RenderCtx<'a> {
    fn is_input_focused(&self, ptr: NodePtr) -> bool {
        matches!(self.focused, FocusedElement::PageInput { ptr: p } if *p == ptr)
    }

    fn input_value(&self, ptr: NodePtr) -> &str {
        self.input_values.get(&ptr).map(|s| s.as_str()).unwrap_or("")
    }

    fn fixed_ctx(&self) -> RenderCtx<'_> {
        RenderCtx { scroll: 0.0, ..*self }
    }
}

// ─── Haupt-Painter ────────────────────────────────────────────────────────────

pub struct SkiaPainter {
    surface: Surface,
    win_w: u32,
    win_h: u32,
    font_collection: FontCollection,
}

impl SkiaPainter {
    pub fn new(win_w: u32, win_h: u32) -> Self {
        let surface = Self::make_surface(win_w, win_h);
        Self {
            surface,
            win_w,
            win_h,
            font_collection: make_font_collection(),
        }
    }

    fn make_surface(win_w: u32, win_h: u32) -> Surface {
        // Raster-Surface in BGRA8888 (passt zu softbuffer)
        let info = ImageInfo::new_n32_premul(
            ISize::new(win_w as i32, win_h as i32),
            None,
        );
        skia_safe::surfaces::raster(&info, None, None)
            .expect("Skia Surface konnte nicht erstellt werden")
    }

    /// Größe der Surface anpassen (bei Window-Resize aufrufen)
    pub fn resize(&mut self, win_w: u32, win_h: u32) {
        if win_w != self.win_w || win_h != self.win_h {
            self.win_w = win_w;
            self.win_h = win_h;
            self.surface = Self::make_surface(win_w, win_h);
        }
    }

    /// Alles rendern und das Ergebnis in den softbuffer schreiben
    pub fn paint_layout(
        &mut self,
        sb_buf:  &mut SbBuffer<'_, Arc<Window>, Arc<Window>>,
        browser: &BrowserState,
        window:  Option<&Arc<Window>>,
    ) {
        let win_w = self.win_w;
        let win_h = self.win_h;
        let canvas = self.surface.canvas();

        // ── Hintergrund ───────────────────────────────────────────────────
        canvas.clear(sk_color(COL_TOOLBAR_BG));

        let content_x = 0.0_f32;
        let content_y = HEADER_H;
        let content_w = win_w as f32;
        let content_h = (win_h as f32 - HEADER_H - STATUS_H).max(0.0);

        if let Some(page) = &browser.page_layout {
            // Seiteninhalt-Hintergrund
            fill_rect_skia(canvas, content_x, content_y, content_w, content_h, COL_PAGE_BG, 1.0);

            let scroll = browser.scroll_y as f32;
            let ctx = RenderCtx {
                win_w: win_w as f32,
                win_h: win_h as f32,
                clip_x: page.x,
                clip_y: content_y,
                clip_w: page.width,
                clip_h: content_h,
                container_w: page.width,
                scroll,
                mouse_x: browser.mouse_x as f32,
                mouse_y: browser.mouse_y as f32,
                focused: &browser.focused,
                input_values: &browser.input_values,
                hovered_href: browser.hovered_href.as_deref(),
                image_cache: &browser.image_cache,
                window,
                font_collection: &self.font_collection,
            };

            // Clip auf Content-Bereich setzen
            canvas.save();
            canvas.clip_rect(
                Rect::from_xywh(content_x, content_y, content_w, content_h),
                None,
                None,
            );
            paint_box(canvas, page, &ctx);
            paint_fixed_children(canvas, page, &ctx);
            canvas.restore();

            draw_scrollbar(canvas, win_w as f32, win_h as f32, page, browser.scroll_y as f32, content_h);

        } else if browser.is_loading {
            fill_rect_skia(canvas, content_x, content_y, content_w, content_h, 0x00_F8_F9_FA, 1.0);
            draw_loading(canvas, content_x, content_y, content_w, content_h, browser, &self.font_collection);
        } else {
            fill_rect_skia(canvas, content_x, content_y, content_w, content_h, COL_PAGE_BG, 1.0);
            draw_welcome(canvas, content_x, content_y, content_w, content_h, &self.font_collection);
        }

        // Chrome immer obendrauf
        draw_chrome(canvas, win_w as f32, win_h as f32, browser, &self.font_collection);

        // ── Pixel in softbuffer kopieren ──────────────────────────────────
        self.blit_to_softbuffer(sb_buf);
    }

    /// Skia-Pixmap → softbuffer
    fn blit_to_softbuffer(&mut self, sb_buf: &mut SbBuffer<'_, Arc<Window>, Arc<Window>>) {
        let win_w = self.win_w;
        let win_h = self.win_h;
        let pixmap = self.surface.peek_pixels().expect("peek_pixels fehlgeschlagen");
        let pixels = pixmap.bytes().expect("pixmap bytes fehlgeschlagen");

        // Skia liefert N32 Premul = BGRA8888 (little-endian = B, G, R, A)
        // softbuffer erwartet 0x00RRGGBB
        let buf_len = sb_buf.len();
        for y in 0..win_h as usize {
            for x in 0..win_w as usize {
                let src = (y * win_w as usize + x) * 4;
                let dst = y * win_w as usize + x;
                if src + 3 >= pixels.len() || dst >= buf_len { break; }
                let b = pixels[src    ] as u32;
                let g = pixels[src + 1] as u32;
                let r = pixels[src + 2] as u32;
                // Alpha ignorieren (softbuffer nutzt kein Alpha)
                sb_buf[dst] = (r << 16) | (g << 8) | b;
            }
        }
    }
}

// ─── Zeichenprimitive (Skia-basiert) ──────────────────────────────────────────

fn fill_rect_skia(canvas: &Canvas, x: f32, y: f32, w: f32, h: f32, color: u32, opacity: f32) {
    if w <= 0.0 || h <= 0.0 { return; }
    let alpha = (opacity * 255.0).round() as u8;
    let mut paint = Paint::default();
    paint.set_color(sk_color_alpha(color, alpha));
    paint.set_anti_alias(false);
    canvas.draw_rect(Rect::from_xywh(x, y, w, h), &paint);
}

fn fill_rect_aa(canvas: &Canvas, x: f32, y: f32, w: f32, h: f32, color: u32, opacity: f32) {
    if w <= 0.0 || h <= 0.0 { return; }
    let alpha = (opacity * 255.0).round() as u8;
    let mut paint = Paint::default();
    paint.set_color(sk_color_alpha(color, alpha));
    paint.set_anti_alias(true);
    canvas.draw_rect(Rect::from_xywh(x, y, w, h), &paint);
}

fn fill_rounded_rect_skia(
    canvas: &Canvas, x: f32, y: f32, w: f32, h: f32, color: u32, radius: f32, opacity: f32,
) {
    if w <= 0.0 || h <= 0.0 { return; }
    let alpha = (opacity * 255.0).round() as u8;
    let mut paint = Paint::default();
    paint.set_color(sk_color_alpha(color, alpha));
    paint.set_anti_alias(true);
    let r = radius.min(w / 2.0).min(h / 2.0).max(0.0);
    canvas.draw_round_rect(Rect::from_xywh(x, y, w, h), r, r, &paint);
}

fn stroke_rounded_rect_skia(
    canvas: &Canvas, x: f32, y: f32, w: f32, h: f32, color: u32, radius: f32, stroke_w: f32,
) {
    if w <= 0.0 || h <= 0.0 { return; }
    let mut paint = Paint::default();
    paint.set_color(sk_color(color));
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(stroke_w);
    paint.set_anti_alias(true);
    let r = radius.min(w / 2.0).min(h / 2.0).max(0.0);
    // Stroke wird nach innen/außen gleichmäßig verteilt – halb nach innen verschieben
    let inset = stroke_w / 2.0;
    canvas.draw_round_rect(
        Rect::from_xywh(x + inset, y + inset, w - stroke_w, h - stroke_w),
        r, r, &paint,
    );
}

fn stroke_rect_skia(
    canvas: &Canvas, x: f32, y: f32, w: f32, h: f32, color: u32, stroke_w: f32,
) {
    if w <= 0.0 || h <= 0.0 { return; }
    let mut paint = Paint::default();
    paint.set_color(sk_color(color));
    paint.set_style(PaintStyle::Stroke);
    paint.set_stroke_width(stroke_w);
    paint.set_anti_alias(false);
    let inset = stroke_w / 2.0;
    canvas.draw_rect(
        Rect::from_xywh(x + inset, y + inset, w - stroke_w, h - stroke_w),
        &paint,
    );
}

fn fill_linear_gradient_skia(
    canvas: &Canvas, x: f32, y: f32, w: f32, h: f32, c1: u32, c2: u32,
) {
    if w <= 0.0 || h <= 0.0 { return; }
    let colors = [sk_color(c1), sk_color(c2)];
    let shader = gradient_shader::linear(
        (Point::new(x, y), Point::new(x, y + h)),
        gradient_shader::GradientShaderColors::Colors(&colors),
        None,
        TileMode::Clamp,
        None,
        None,
    );
    if let Some(shader) = shader {
        let mut paint = Paint::default();
        paint.set_shader(shader);
        paint.set_anti_alias(false);
        canvas.draw_rect(Rect::from_xywh(x, y, w, h), &paint);
    }
}

/// Weicher Box-Shadow via mehrfache halbtransparente Rects
fn draw_box_shadow_skia(
    canvas: &Canvas, x: f32, y: f32, w: f32, h: f32,
    ox: f32, oy: f32, blur: f32, color: u32,
) {
    let steps = (blur as i32).max(1);
    let r = ((color >> 16) & 0xFF) as u8;
    let g = ((color >>  8) & 0xFF) as u8;
    let b = ( color        & 0xFF) as u8;

    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let alpha = ((1.0 - t) * 0.12 * 255.0).round() as u8;
        if alpha == 0 { continue; }
        let expand = i as f32;
        let mut paint = Paint::default();
        paint.set_color(Color::from_argb(alpha, r, g, b));
        paint.set_anti_alias(true);
        let r_px = expand.min(w / 2.0).min(h / 2.0);
        canvas.draw_round_rect(
            Rect::from_xywh(
                x + ox - expand, y + oy - expand,
                w + expand * 2.0, h + expand * 2.0,
            ),
            r_px, r_px, &paint,
        );
    }
}

// ─── Skia-Textrendering ───────────────────────────────────────────────────────

/// Einfaches einzeiliges Text-Zeichnen
fn draw_text_skia(
    canvas: &Canvas, fc: &FontCollection,
    text: &str, x: f32, y: f32,
    color: u32, font_size: f32,
    bold: bool,
) {
    if text.is_empty() { return; }

    let mut style = TextStyle::new();
    style.set_color(sk_color(color));
    style.set_font_size(font_size);
    style.set_font_style(if bold { FontStyle::bold() } else { FontStyle::normal() });

    let mut para_style = ParagraphStyle::new();
    para_style.set_text_align(TextAlign::Left);

    let mut builder = ParagraphBuilder::new(&para_style, fc);
    builder.push_style(&style);
    builder.add_text(text);
    let mut para = builder.build();
    para.layout(f32::INFINITY);
    para.paint(canvas, Point::new(x, y));
}

/// Umgebrochener Text mit max_w als Zeilenbreite
fn draw_text_wrapped_skia(
    canvas: &Canvas, fc: &FontCollection,
    text: &str, x: f32, y: f32, max_w: f32,
    color: u32, font_size: f32,
    bold: bool, italic: bool,
    underline: bool,
    align: TextAlign,
    families: &[String],
) -> f32 {
    if text.is_empty() { return 0.0; }

    let mut style = TextStyle::new();
    style.set_color(sk_color(color));
    style.set_font_size(font_size);
    if !families.is_empty() {
        style.set_font_families(families);
    }
    let font_style = match (bold, italic) {
        (true, true)   => FontStyle::bold_italic(),
        (true, false)  => FontStyle::bold(),
        (false, true)  => FontStyle::italic(),
        (false, false) => FontStyle::normal(),
    };
    style.set_font_style(font_style);
    if underline {
        use skia_safe::textlayout::{TextDecoration, TextDecorationMode};
        style.set_decoration_type(TextDecoration::UNDERLINE);
        style.set_decoration_mode(TextDecorationMode::Through);
        style.set_decoration_color(sk_color(color));
    }

    let mut para_style = ParagraphStyle::new();
    para_style.set_text_align(align);
    let mut builder = ParagraphBuilder::new(&para_style, fc);
    builder.push_style(&style);
    builder.add_text(text);
    let mut para = builder.build();
    para.layout(max_w.max(1.0));
    let height = para.height();
    para.paint(canvas, Point::new(x, y));
    height
}

/// Textbreite messen (ohne Zeichnen)
fn measure_text_width(fc: &FontCollection, text: &str, font_size: f32) -> f32 {
    if text.is_empty() { return 0.0; }
    let mut style = TextStyle::new();
    style.set_font_size(font_size);
    let para_style = ParagraphStyle::new();
    let mut builder = ParagraphBuilder::new(&para_style, fc);
    builder.push_style(&style);
    builder.add_text(text);
    let mut para = builder.build();
    para.layout(f32::INFINITY);
    para.max_intrinsic_width()
}

// ─── position:fixed ───────────────────────────────────────────────────────────

fn paint_fixed_children(canvas: &Canvas, layout: &LayoutBox, ctx: &RenderCtx<'_>) {
    if layout.style.position == PositionValue::Fixed {
        let fixed_ctx = ctx.fixed_ctx();
        paint_box(canvas, layout, &fixed_ctx);
        return;
    }
    for child in &layout.children {
        paint_fixed_children(canvas, child, ctx);
    }
}

// ─── Haupt-Box-Rendering ──────────────────────────────────────────────────────

fn paint_box(canvas: &Canvas, layout: &LayoutBox, ctx: &RenderCtx<'_>) {
    if layout.style.is_hidden() { return; }
    if layout.style.position == PositionValue::Fixed { return; }

    let x = layout.x;
    let y = layout.y - ctx.scroll;
    let w = layout.width;
    let h = layout.height;
    let opacity = layout.style.effective_opacity();
    if opacity <= 0.0 { return; }

    // Sichtbarkeitscheck
    let in_clip = y < ctx.clip_y + ctx.clip_h && y + h.max(1.0) > ctx.clip_y
        && x < ctx.clip_x + ctx.clip_w && x + w.max(1.0) > ctx.clip_x;

    let tag = node_tag(layout);

    if w == 0.0 && h == 0.0 {
        for child in &layout.children {
            paint_box(canvas, child, ctx);
        }
        return;
    }

    if in_clip {
        // ── Box-Shadow ────────────────────────────────────────────────
        if let Some((ox, oy, blur, sc)) = layout.style.box_shadow {
            draw_box_shadow_skia(canvas, x, y, w, h, ox, oy, blur, sc);
        }

        // ── Hintergrund ───────────────────────────────────────────────
        let s = &layout.style;
        let radius = s.effective_border_radius();

        if let Some((c1, c2)) = s.background_gradient {
            fill_linear_gradient_skia(canvas, x, y, w, h, c1, c2);
        } else if let Some(bg) = s.background_color {
            fill_rounded_rect_skia(canvas, x, y, w, h, bg, radius, opacity);
        }

        let is_hovered = ctx.mouse_x >= x && ctx.mouse_x < x + w.max(1.0)
            && ctx.mouse_y >= y && ctx.mouse_y < y + h.max(1.0);
        let ptr = layout as *const LayoutBox as usize;

        match tag {
            // ── Input / Textarea ──────────────────────────────────────
            "input" | "textarea" => {
                let focused = ctx.is_input_focused(ptr);
                let border_color = if focused { COL_INPUT_FOCUS }
                    else if is_hovered { 0x00_80_86_8B }
                    else { layout.style.border_color.unwrap_or(COL_INPUT_BORDER) };

                let bw = w.max(200.0);
                let bh = h.max(if tag == "textarea" { 80.0 } else { 36.0 });
                let br = s.effective_border_radius().max(4.0);

                fill_rounded_rect_skia(canvas, x, y, bw, bh, COL_INPUT_BG, br, 1.0);

                // Focus-Glow
                if focused {
                    draw_focus_glow(canvas, x, y, bw, bh, br, COL_INPUT_FOCUS);
                }

                stroke_rounded_rect_skia(
                    canvas, x, y, bw, bh, border_color, br,
                    if focused { 2.0 } else { 1.0 },
                );

                let value       = ctx.input_value(ptr);
                let placeholder = node_attr(layout, "placeholder");
                let font_sz     = s.font_size.unwrap_or(14.0);
                let text_y      = y + (bh - font_sz * 1.4) / 2.0;

                if value.is_empty() && !placeholder.is_empty() {
                    draw_text_skia(canvas, ctx.font_collection, placeholder,
                                   x + 10.0, text_y, COL_URL_PLACEHOLDER, font_sz, false);
                } else {
                    draw_text_skia(canvas, ctx.font_collection, value,
                                   x + 10.0, text_y, COL_INPUT_TEXT, font_sz, false);
                    if focused {
                        let tw = measure_text_width(ctx.font_collection, value, font_sz);
                        fill_rect_skia(canvas, x + 10.0 + tw, y + 6.0, 2.0, bh - 12.0, COL_CURSOR, 1.0);
                    }
                }
            }

            // ── Select ────────────────────────────────────────────────
            "select" => {
                let bw = w.max(150.0);
                fill_rect_skia(canvas, x, y, bw, h, COL_SELECT_BG, 1.0);
                stroke_rect_skia(canvas, x, y, bw, h, COL_INPUT_BORDER, 1.0);
                // Dropdown-Pfeil-Bereich
                fill_rect_skia(canvas, x + bw - 28.0, y, 28.0, h, 0x00_F1_F3_F4, 1.0);
                fill_rect_skia(canvas, x + bw - 28.0, y, 1.0, h, COL_INPUT_BORDER, 1.0);
                draw_text_skia(canvas, ctx.font_collection, "▾",
                               x + bw - 20.0, y + (h - 16.0) / 2.0, COL_MUTED, 13.0, false);
            }

            // ── Tabellen-Zellen ───────────────────────────────────────
            "th" => {
                fill_rect_skia(canvas, x, y, w, h, COL_TABLE_HEADER, 1.0);
                stroke_rect_skia(canvas, x, y, w, h, COL_STATUS_BORDER, 1.0);
            }
            "td" => {
                stroke_rect_skia(canvas, x, y, w, h, 0x00_E8_EA_ED, 1.0);
            }

            // ── Button ────────────────────────────────────────────────
            "button" | "submit" => {
                let bw = w.max(80.0);
                let bh = h.max(36.0);
                let br = s.effective_border_radius().max(4.0);
                let bg = if is_hovered { COL_BTN_PAGE_HOVER } else {
                    s.background_color.unwrap_or(COL_BTN_PAGE_BG)
                };

                // Schatten
                draw_box_shadow_skia(canvas, x, y, bw, bh, 0.0, 2.0, 4.0, 0x00_00_00_00);

                // Gradient für etwas Tiefe
                let bg_top = lighten_u32(bg, 10);
                fill_linear_gradient_skia(canvas, x, y, bw, bh, bg_top, bg);

                // Runden
                // (Gradient überschreibt, also nochmal als geclippter Rrect)
                {
                    let mut clip_paint = Paint::default();
                    clip_paint.set_anti_alias(true);
                    // Wir zeichnen den Gradient bereits oben; hier nur nochmal rund clippen
                }

                stroke_rounded_rect_skia(canvas, x, y, bw, bh, darken_u32(bg, 20), br, 1.0);
            }

            // ── Code / Pre ────────────────────────────────────────────
            "code" | "samp" | "kbd" => {
                fill_rounded_rect_skia(canvas, x, y, w.max(20.0), h.max(18.0), COL_CODE_BG, 3.0, 1.0);
                stroke_rounded_rect_skia(canvas, x, y, w.max(20.0), h.max(18.0), 0x00_C4_C7_C5, 3.0, 1.0);
            }
            "pre" => {
                fill_rounded_rect_skia(canvas, x, y, w, h.max(20.0), 0x00_28_2A_2E, 6.0, 1.0);
                stroke_rounded_rect_skia(canvas, x, y, w, h.max(20.0), 0x00_3C_40_43, 6.0, 1.0);
            }

            // ── Blockquote ────────────────────────────────────────────
            "blockquote" => {
                fill_rect_skia(canvas, x, y, w, h, 0x00_F8_F9_FA, 1.0);
                fill_rect_skia(canvas, x, y, 4.0, h, COL_BLOCKQUOTE, 1.0);
            }

            // ── HR ────────────────────────────────────────────────────
            "hr" => {
                fill_rect_skia(canvas, x, y + h / 2.0, w, 1.0, COL_STATUS_BORDER, 1.0);
            }

            // ── Bild ──────────────────────────────────────────────────
            "img" => {
                let iw = w.max(60.0);
                let ih = h.max(40.0);
                let src = node_attr(layout, "src");
                let drawn = if !src.is_empty() {
                    let window_clone = ctx.window.cloned();
                    let img = ctx.image_cache.get_or_load_with_callback(src, move || {
                        if let Some(ref w) = window_clone { w.request_redraw(); }
                    });
                    if let Some(img) = img {
                        blit_image_skia(canvas, x, y, iw, ih, &img);
                        true
                    } else { false }
                } else { false };

                if !drawn {
                    fill_rounded_rect_skia(canvas, x, y, iw, ih, 0x00_F1_F3_F4, 4.0, 1.0);
                    stroke_rounded_rect_skia(canvas, x, y, iw, ih, COL_STATUS_BORDER, 4.0, 1.0);
                    // Platzhalter-Icon
                    let icon_x = x + (iw - 24.0) / 2.0;
                    let icon_y = y + (ih - 18.0) / 2.0;
                    fill_rounded_rect_skia(canvas, icon_x, icon_y, 24.0, 18.0, 0x00_DA_DC_E0, 2.0, 1.0);
                    fill_rounded_rect_skia(canvas, icon_x + 2.0, icon_y + 3.0, 6.0, 6.0, 0x00_B0_B8_C0, 3.0, 1.0);
                    let alt = node_attr(layout, "alt");
                    if !alt.is_empty() {
                        draw_text_skia(canvas, ctx.font_collection, alt,
                                       x + 2.0, y + ih - 16.0, COL_MUTED, 11.0, false);
                    }
                }
            }

            // ── Generische Box ────────────────────────────────────────
            _ => {
                if w >= 2.0 && h >= 2.0 {
                    // Hintergrund (falls gradient nicht greift)
                    if s.background_gradient.is_none() {
                        if let Some(bg) = s.background_color {
                            fill_rounded_rect_skia(canvas, x, y, w, h, bg, radius, opacity);
                        }
                    }
                    draw_borders_skia(canvas, x, y, w, h, s);
                }
            }
        }

        // ── Text-Knoten ───────────────────────────────────────────────
        if is_text_node(layout) {
            paint_text_node(canvas, layout, ctx, x, y, w, tag);
        }

        // ── H1/H2-Unterstreichung ─────────────────────────────────────
        if tag == "h1" && h > 4.0 {
            fill_rect_skia(canvas, x, y + h - 2.0, w, 1.0, 0x00_DA_DC_E0, 1.0);
        }
        if tag == "h2" && h > 4.0 {
            fill_rect_skia(canvas, x, y + h - 1.0, w, 1.0, 0x00_E8_EA_ED, 1.0);
        }
    }

    // ── Kinder ────────────────────────────────────────────────────────
    let is_block = !is_text_node(layout) && w > 50.0;
    let child_container_w = if is_block { w } else { ctx.container_w };
    let child_clip_x      = if is_block { x } else { ctx.clip_x };

    let child_ctx = RenderCtx {
        win_w: ctx.win_w,
        win_h: ctx.win_h,
        clip_x: child_clip_x,
        clip_y: ctx.clip_y,
        clip_w: ctx.clip_w,
        clip_h: ctx.clip_h,
        container_w: child_container_w,
        scroll: ctx.scroll,
        mouse_x: ctx.mouse_x,
        mouse_y: ctx.mouse_y,
        focused: ctx.focused,
        input_values: ctx.input_values,
        hovered_href: ctx.hovered_href,
        image_cache: ctx.image_cache,
        window: ctx.window,
        font_collection: ctx.font_collection,
    };

    // Inline-Kinder gemeinsam rendern (für Links, Spans etc.)
    if tag != "a" && tag != "button" && tag != "pre"
        && !matches!(tag, "h1"|"h2"|"h3"|"h4"|"h5"|"h6")
        && in_clip
        && has_only_inline_children(layout)
        && !layout.children.is_empty()
    {
        paint_inline_children(canvas, layout, &child_ctx);
        return;
    }

    for child in &layout.children {
        let child_tag = node_tag(child);
        if tag == "a" {
            paint_link_child(canvas, child, &child_ctx, node_attr(layout, "href"));
        } else if matches!(child_tag, "h1"|"h2"|"h3"|"h4"|"h5"|"h6") {
            paint_heading_child(canvas, child, &child_ctx);
        } else if tag == "button" || tag == "submit" {
            paint_button_child(canvas, child, &child_ctx);
        } else if tag == "pre" {
            paint_pre_child(canvas, child, &child_ctx);
        } else {
            paint_box(canvas, child, &child_ctx);
        }
    }
}

// ─── Text-Node-Rendering ──────────────────────────────────────────────────────

fn paint_text_node(
    canvas: &Canvas, layout: &LayoutBox, ctx: &RenderCtx<'_>,
    x: f32, y: f32, _w: f32, tag: &str,
) {
    let raw  = node_text(layout);
    let text = raw.replace('\u{00A0}', " ");
    let text = text.trim();
    if text.is_empty() { return; }

    let col     = layout.style.color.unwrap_or(COL_PAGE_TEXT);
    let text_col = if tag == "pre" { 0x00_E8_EA_ED } else { col };
    let font_sz  = layout.style.font_size.unwrap_or(16.0);
    let bold     = matches!(layout.style.font_weight,
                   Some(layout_engine::cssom::FontWeightValue::Bold));
    let italic   = false; // TODO: font-style
    let mut families = Vec::new();
    if let Some(f) = &layout.style.font_family {
        families.push(f.clone());
    }
    families.push("sans-serif".to_string());

    let is_underline = matches!(tag, "a" | "u")
        || layout.style.text_decoration
            .as_ref()
            .map(|td| matches!(td, TextDecorationValue::Underline))
            .unwrap_or(false);

    let container_right = ctx.clip_x + ctx.container_w;
    let max_w = (container_right - x).max(50.0);

    let align = match layout.style.text_align {
        TextAlignValue::Center => TextAlign::Center,
        TextAlignValue::Right  => TextAlign::Right,
        _                      => TextAlign::Left,
    };

    draw_text_wrapped_skia(
        canvas, ctx.font_collection,
        text, x, y + 2.0, max_w, text_col, font_sz, bold, italic, is_underline, align, &families,
    );
}

// ─── Inline-Kinder ────────────────────────────────────────────────────────────

fn has_only_inline_children(layout: &LayoutBox) -> bool {
    const INLINE_TAGS: &[&str] = &[
        "a","span","b","i","em","strong","code","small",
        "sup","sub","abbr","cite","q","mark","u","s",
        "del","ins","kbd","samp","var","time","label",
    ];
    if layout.children.is_empty() { return false; }
    let mut has_block = false;
    for child in &layout.children {
        let t = node_tag(child);
        if child.text.is_none() && !INLINE_TAGS.contains(&t) {
            has_block = true;
            break;
        }
    }
    !has_block
}

fn paint_inline_children(canvas: &Canvas, layout: &LayoutBox, ctx: &RenderCtx<'_>) {
    let base_x = layout.x + layout.style.padding_left;
    let base_y = layout.y - ctx.scroll + layout.style.padding_top + 2.0;
    let max_w  = (layout.width - layout.style.padding_left - layout.style.padding_right).max(50.0);
    paint_inline_box(canvas, layout, ctx, base_x, base_y, max_w, None);
}

fn paint_inline_box(
    canvas: &Canvas, layout: &LayoutBox, ctx: &RenderCtx<'_>,
    start_x: f32, base_y: f32, max_w: f32,
    color_override: Option<u32>,
) -> f32 {
    let tag = node_tag(layout);
    let is_link = tag == "a";
    let is_code = matches!(tag, "code"|"kbd"|"samp");
    let font_sz = layout.style.font_size.unwrap_or(16.0);

    let text_color = if is_link {
        let href = node_attr(layout, "href");
        if ctx.hovered_href == Some(href) { COL_LINK_HOVER } else { COL_LINK }
    } else {
        color_override.unwrap_or_else(|| layout.style.color.unwrap_or(COL_PAGE_TEXT))
    };

    // Code-Hintergrund
    if is_code {
        let txt = collect_text_of(layout);
        let tw = measure_text_width(ctx.font_collection, txt.trim(), font_sz);
        let th = font_sz * 1.4;
        fill_rounded_rect_skia(canvas, start_x - 2.0, base_y - 2.0, tw + 4.0, th + 4.0, COL_CODE_BG, 3.0, 1.0);
    }

    if is_text_node(layout) {
        let raw  = node_text(layout);
        let text = raw.replace('\u{00A0}', " ");
        let text = text.trim();
        if !text.is_empty() {
            draw_text_wrapped_skia(
                canvas, ctx.font_collection,
                text, start_x, base_y, max_w,
                text_color, font_sz, false, false, is_link, TextAlign::Left,
                &[],
            );
            let tw = measure_text_width(ctx.font_collection, text, font_sz);
            return start_x + tw;
        }
        return start_x;
    }

    let mut cx = start_x;
    for child in &layout.children {
        let child_color = if is_link { Some(text_color) } else { None };
        cx = paint_inline_box(canvas, child, ctx, cx, base_y, max_w, child_color);
    }
    cx
}

fn collect_text_of(layout: &LayoutBox) -> String {
    let mut s = String::new();
    collect_inline_text(layout, &mut s);
    s
}

fn collect_inline_text(layout: &LayoutBox, out: &mut String) {
    if let Some(text) = &layout.text {
        let t = text.replace('\u{00A0}', " ");
        let t = t.trim();
        if !t.is_empty() {
            if !out.is_empty() && !out.ends_with(' ')
                && !t.starts_with([',', '.', ':', ';', '!', '?', ')', '\'', '"']) {
                out.push(' ');
            }
            out.push_str(t);
        }
        return;
    }
    for child in &layout.children {
        collect_inline_text(child, out);
    }
}

// ─── Link / Heading / Button / Pre ────────────────────────────────────────────

fn paint_link_child(canvas: &Canvas, layout: &LayoutBox, ctx: &RenderCtx<'_>, href: &str) {
    if is_text_node(layout) {
        let x = layout.x;
        let y = layout.y - ctx.scroll;
        let raw  = node_text(layout);
        let text = raw.replace('\u{00A0}', " ");
        let text = text.trim();
        if !text.is_empty() {
            let font_sz = layout.style.font_size.unwrap_or(16.0);
            let hovered = ctx.hovered_href == Some(href);
            let color = if hovered { COL_LINK_HOVER } else { COL_LINK };
            draw_text_wrapped_skia(
                canvas, ctx.font_collection,
                text, x + 2.0, y + 3.0, (layout.width).max(50.0),
                color, font_sz, false, false, true, TextAlign::Left,
                &[],
            );
        }
    }
    for child in &layout.children {
        paint_link_child(canvas, child, ctx, href);
    }
}

fn paint_heading_child(canvas: &Canvas, layout: &LayoutBox, ctx: &RenderCtx<'_>) {
    let tag = node_tag(layout);
    let color = layout.style.color.unwrap_or(match tag {
        "h1"        => COL_H1,
        "h2"        => COL_H2,
        "h3"|"h4"   => 0x00_30_31_34,
        _           => COL_PAGE_TEXT,
    });

    if is_text_node(layout) {
        let x = layout.x;
        let y = layout.y - ctx.scroll;
        let raw  = node_text(layout);
        let text = raw.replace('\u{00A0}', " ");
        let text = text.trim();
        if !text.is_empty() {
            let font_sz = layout.style.font_size.unwrap_or(16.0);
            let container_right = ctx.clip_x + ctx.container_w;
            let max_w = (container_right - x).max(50.0);
            draw_text_wrapped_skia(
                canvas, ctx.font_collection,
                text, x, y + 2.0, max_w,
                color, font_sz, true, false, false, TextAlign::Left,
                &[],
            );
        }
    }
    for child in &layout.children {
        paint_heading_child(canvas, child, ctx);
    }
}

fn paint_button_child(canvas: &Canvas, layout: &LayoutBox, ctx: &RenderCtx<'_>) {
    if is_text_node(layout) {
        let x = layout.x;
        let y = layout.y - ctx.scroll;
        let raw  = node_text(layout);
        let text = raw.replace('\u{00A0}', " ");
        let text = text.trim();
        if !text.is_empty() {
            let font_sz = layout.style.font_size.unwrap_or(14.0);
            let color   = layout.style.color.unwrap_or(COL_BTN_PAGE_TEXT);
            draw_text_skia(canvas, ctx.font_collection, text, x + 8.0, y + 11.0, color, font_sz, true);
        }
    }
    for child in &layout.children { paint_button_child(canvas, child, ctx); }
}

fn paint_pre_child(canvas: &Canvas, layout: &LayoutBox, ctx: &RenderCtx<'_>) {
    if is_text_node(layout) {
        let x = layout.x;
        let y = layout.y - ctx.scroll;
        let raw  = node_text(layout);
        let text = raw.trim_matches('\n');
        if !text.is_empty() {
            let font_sz = layout.style.font_size.unwrap_or(13.0);
            let max_w   = (layout.width).max(100.0) - 24.0;
            draw_text_wrapped_skia(
                canvas, ctx.font_collection,
                text, x + 12.0, y + 8.0, max_w,
                0x00_E8_EA_ED, font_sz, false, false, false, TextAlign::Left,
                &[],
            );
        }
    }
    for child in &layout.children { paint_pre_child(canvas, child, ctx); }
}

// ─── Borders ──────────────────────────────────────────────────────────────────

fn draw_borders_skia(canvas: &Canvas, x: f32, y: f32, w: f32, h: f32, s: &layout_engine::style::ComputedStyle) {
    // Wir verwenden die per-Seite-Border-Infos
    let top_w    = s.border_width_top();
    let right_w  = s.border_width_right();
    let bottom_w = s.border_width_bottom();
    let left_w   = s.border_width_left();

    if let Some(c) = s.border_color_top()    { if top_w    > 0.0 { fill_rect_skia(canvas, x, y, w, top_w, c, 1.0); } }
    if let Some(c) = s.border_color_right()  { if right_w  > 0.0 { fill_rect_skia(canvas, x + w - right_w, y, right_w, h, c, 1.0); } }
    if let Some(c) = s.border_color_bottom() { if bottom_w > 0.0 { fill_rect_skia(canvas, x, y + h - bottom_w, w, bottom_w, c, 1.0); } }
    if let Some(c) = s.border_color_left()   { if left_w   > 0.0 { fill_rect_skia(canvas, x, y, left_w, h, c, 1.0); } }
}

// ─── Focus-Glow ───────────────────────────────────────────────────────────────

fn draw_focus_glow(canvas: &Canvas, x: f32, y: f32, w: f32, h: f32, r: f32, color: u32) {
    let cr = ((color >> 16) & 0xFF) as u8;
    let cg = ((color >>  8) & 0xFF) as u8;
    let cb = ( color        & 0xFF) as u8;
    for i in 1..=3_i32 {
        let expand = i as f32;
        let alpha = (0.18 / i as f32 * 255.0).round() as u8;
        let mut paint = Paint::default();
        paint.set_color(Color::from_argb(alpha, cr, cg, cb));
        paint.set_anti_alias(true);
        let rr = (r + expand).min((w + expand * 2.0) / 2.0);
        canvas.draw_round_rect(
            Rect::from_xywh(x - expand, y - expand, w + expand * 2.0, h + expand * 2.0),
            rr, rr, &paint,
        );
    }
}

// ─── Bilder ───────────────────────────────────────────────────────────────────

fn blit_image_skia(canvas: &Canvas, x: f32, y: f32, w: f32, h: f32, img: &DecodedImage) {
    // Pixels als Skia-Bitmap aufbauen
    use skia_safe::{Bitmap, ColorType, AlphaType, ImageInfo as SkImageInfo};

    let info = SkImageInfo::new(
        ISize::new(img.width as i32, img.height as i32),
        ColorType::BGRA8888,
        AlphaType::Premul,
        None,
    );

    // u32-Pixel (0x00RRGGBB) in BGRA8888 umwandeln
    let bgra: Vec<u8> = img.pixels.iter().flat_map(|&p| {
        let r = ((p >> 16) & 0xFF) as u8;
        let g = ((p >>  8) & 0xFF) as u8;
        let b = ( p        & 0xFF) as u8;
        [b, g, r, 255u8]
    }).collect();

    let mut bitmap = Bitmap::new();
    if !bitmap.set_info(&info, None) { return; }
    unsafe {
        if !bitmap.install_pixels(&info, bgra.as_ptr() as *mut _, img.width as usize * 4) { return; }
    }

    let sk_img = skia_safe::images::raster_from_bitmap(&bitmap);
    if let Some(sk_img) = sk_img {
        let dst_rect = Rect::from_xywh(x, y, w, h);
        let src_rect = Rect::from_xywh(0.0, 0.0, img.width as f32, img.height as f32);
        let mut paint = Paint::default();
        paint.set_anti_alias(true);
        canvas.draw_image_rect(
            &sk_img,
            Some((&src_rect, skia_safe::canvas::SrcRectConstraint::Fast)),
            dst_rect,
            &paint,
        );
    }
}

// ─── Chrome ───────────────────────────────────────────────────────────────────

fn draw_chrome(canvas: &Canvas, win_w: f32, win_h: f32, browser: &BrowserState, fc: &FontCollection) {
    // Toolbar
    fill_rect_skia(canvas, 0.0, 0.0, win_w, HEADER_H, COL_TOOLBAR_BG, 1.0);
    fill_rect_skia(canvas, 0.0, 0.0, win_w, 1.0, lighten_u32(COL_TOOLBAR_BG, 12), 1.0);
    fill_rect_skia(canvas, 0.0, HEADER_H - 2.0, win_w, 2.0, COL_TOOLBAR_BOTTOM, 1.0);

    draw_nav_buttons(canvas, fc);
    draw_url_bar(canvas, win_w, browser, fc);

    // Statusleiste
    let sy = (win_h - STATUS_H).max(0.0);
    fill_rect_skia(canvas, 0.0, sy, win_w, STATUS_H, COL_STATUS_BG, 1.0);
    fill_rect_skia(canvas, 0.0, sy, win_w, 1.0, COL_STATUS_BORDER, 1.0);

    let status_text = browser.hovered_href.as_deref()
        .map(|h| format!("  {}", h))
        .unwrap_or_else(|| format!("  {}", browser.status_text));
    draw_text_skia(canvas, fc, &status_text, 6.0, sy + (STATUS_H - 14.0) / 2.0,
                   COL_STATUS_TEXT, 12.0, false);

    if browser.is_loading {
        draw_text_skia(canvas, fc, "⟳ Laden...", win_w - 80.0, (HEADER_H - 14.0) / 2.0,
                       0x00_8A_B4_F8, 12.0, false);
    }
}

fn draw_nav_buttons(canvas: &Canvas, fc: &FontCollection) {
    let btn: f32 = 32.0;
    let mg:  f32 = 5.0;
    let by   = (HEADER_H - btn) / 2.0;
    let buttons: &[(f32, &str)] = &[
        (mg,                    "←"),
        (mg + btn + mg,         "→"),
        (mg + (btn + mg) * 2.0, "↻"),
    ];
    for (bx, label) in buttons {
        fill_rounded_rect_skia(canvas, bx + 1.0, by + 1.0, btn - 2.0, btn - 2.0, COL_BTN_BG, 6.0, 1.0);
        let tw = measure_text_width(fc, label, 14.0);
        draw_text_skia(canvas, fc, label, bx + (btn - tw) / 2.0, by + (btn - 14.0) / 2.0,
                       COL_BTN_TEXT, 14.0, false);
    }
}

fn draw_url_bar(canvas: &Canvas, win_w: f32, browser: &BrowserState, fc: &FontCollection) {
    let btn:   f32 = 32.0;
    let mg:    f32 = 5.0;
    let bar_h: f32 = 30.0;
    let bar_x  = mg + (btn + mg) * 3.0 + 4.0;
    let bar_y  = (HEADER_H - bar_h) / 2.0;
    let bar_w  = (win_w - bar_x - mg - 4.0).max(0.0);
    if bar_w == 0.0 { return; }

    let focused = browser.url_focused || matches!(browser.focused, FocusedElement::UrlBar);
    let bg      = if focused { COL_URL_BG_FOCUS } else { COL_URL_BG };
    let border  = if focused { COL_URL_BORDER } else { 0x00_5F_63_68 };

    fill_rounded_rect_skia(canvas, bar_x, bar_y, bar_w, bar_h, bg, 15.0, 1.0);
    stroke_rounded_rect_skia(canvas, bar_x, bar_y, bar_w, bar_h, border, 15.0,
                              if focused { 2.0 } else { 1.0 });

    // Focus-Glow
    if focused {
        draw_focus_glow(canvas, bar_x, bar_y, bar_w, bar_h, 15.0, COL_URL_BORDER);
    }

    let text_y = bar_y + (bar_h - 14.0) / 2.0;

    if browser.url.is_empty() {
        draw_text_skia(canvas, fc, "URL eingeben...", bar_x + 12.0, text_y,
                       COL_URL_PLACEHOLDER, 13.0, false);
    } else {
        let url = &browser.url;
        if let Some(sep) = url.find("://") {
            let proto = &url[..sep + 3];
            let rest  = &url[sep + 3..];
            let pw = measure_text_width(fc, proto, 13.0);
            draw_text_skia(canvas, fc, proto, bar_x + 12.0, text_y, COL_URL_PLACEHOLDER, 13.0, false);
            draw_text_skia(canvas, fc, rest,  bar_x + 12.0 + pw, text_y, COL_URL_TEXT, 13.0, false);
        } else {
            draw_text_skia(canvas, fc, url, bar_x + 12.0, text_y, COL_URL_TEXT, 13.0, false);
        }
        if focused {
            let tw = measure_text_width(fc, url, 13.0);
            fill_rect_skia(canvas, bar_x + 12.0 + tw, text_y, 2.0, 16.0, COL_URL_BORDER, 1.0);
        }
    }
}

// ─── Willkommen / Laden ───────────────────────────────────────────────────────

fn draw_welcome(canvas: &Canvas, cx: f32, cy: f32, cw: f32, ch: f32, fc: &FontCollection) {
    let center_y = cy + ch / 2.0;
    let logo_x   = cx + cw / 2.0;
    let logo_y   = center_y - 70.0;

    // Logo-Kreis mit Skia (jetzt wirklich antialiased!)
    fill_rounded_rect_skia(canvas, logo_x - 40.0, logo_y - 40.0, 80.0, 80.0, 0x00_F3_F4_F6, 40.0, 1.0);
    stroke_rounded_rect_skia(canvas, logo_x - 40.0, logo_y - 40.0, 80.0, 80.0, 0x00_3B_82_F6, 40.0, 3.0);
    let n_tw = measure_text_width(fc, "N", 32.0);
    draw_text_skia(canvas, fc, "N", logo_x - n_tw / 2.0, logo_y - 16.0, 0x00_3B_82_F6, 32.0, true);

    let title = "Nexus Browser";
    let tw = measure_text_width(fc, title, 24.0);
    draw_text_skia(canvas, fc, title, cx + (cw - tw) / 2.0, center_y - 10.0, 0x00_11_18_27, 24.0, true);

    let sub = "URL eingeben und Enter drücken";
    let stw = measure_text_width(fc, sub, 13.0);
    draw_text_skia(canvas, fc, sub, cx + (cw - stw) / 2.0, center_y + 8.0, COL_MUTED, 13.0, false);

    fill_rect_skia(canvas, cx + cw / 4.0, center_y + 32.0, cw / 2.0, 1.0, 0x00_DA_DC_E0, 1.0);

    let tips = ["F5 = Neu laden", "ESC = Fokus aufheben", "Scroll = Seite scrollen"];
    for (i, tip) in tips.iter().enumerate() {
        let tw = measure_text_width(fc, tip, 12.0);
        draw_text_skia(canvas, fc, tip, cx + (cw - tw) / 2.0,
                       center_y + 46.0 + i as f32 * 20.0, COL_MUTED, 12.0, false);
    }
}

fn draw_loading(
    canvas: &Canvas, cx: f32, cy: f32, cw: f32, ch: f32,
    browser: &BrowserState, fc: &FontCollection,
) {
    let center_x = cx + cw / 2.0;
    let center_y = cy + ch / 2.0;
    let bar_w = 240.0;
    let bar_x = center_x - bar_w / 2.0;
    let bar_y = center_y + 24.0;

    fill_rounded_rect_skia(canvas, bar_x, bar_y, bar_w, 4.0, 0x00_DA_DC_E0, 2.0, 1.0);
    let progress = ((browser.mouse_x.unsigned_abs() as f32) % (bar_w + 1.0)).min(bar_w);
    fill_rounded_rect_skia(canvas, bar_x, bar_y, progress.max(20.0), 4.0, COL_LOADING_BAR, 2.0, 1.0);

    let domain = extract_domain(&browser.url);
    let msg = format!("Lade {}...", domain);
    let tw = measure_text_width(fc, &msg, 14.0);
    draw_text_skia(canvas, fc, &msg, center_x - tw / 2.0, center_y, COL_MUTED, 14.0, false);
}

fn extract_domain(url: &str) -> &str {
    let stripped = url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    stripped.split('/').next().unwrap_or(stripped)
}

// ─── Scrollbar ────────────────────────────────────────────────────────────────

fn draw_scrollbar(canvas: &Canvas, win_w: f32, _win_h: f32, page: &LayoutBox, scroll: f32, content_h: f32) {
    let page_h = layout_max_bottom(page) as f32;
    if page_h <= content_h { return; }

    let max_scroll   = (page_h - content_h).max(1.0);
    let scroll_ratio = (scroll / max_scroll).clamp(0.0, 1.0);
    let track_h      = content_h;
    let thumb_h      = ((content_h / page_h) * track_h).max(24.0).min(track_h);
    let thumb_y      = HEADER_H + scroll_ratio * (track_h - thumb_h);

    // Track
    fill_rect_skia(canvas, win_w - 10.0, HEADER_H, 10.0, track_h, 0x00_F1_F3_F4, 1.0);
    // Thumb mit sanftem Hover-Effekt (TODO: echtes Hover-State)
    fill_rounded_rect_skia(canvas, win_w - 9.0, thumb_y + 2.0, 8.0, (thumb_h - 4.0).max(4.0),
                            0x00_BD_C1_C6, 4.0, 1.0);
}

// ─── Farbhilfsfunktionen ──────────────────────────────────────────────────────

fn lighten_u32(color: u32, amount: u32) -> u32 {
    let r = (((color >> 16) & 0xFF) + amount).min(0xFF);
    let g = (((color >>  8) & 0xFF) + amount).min(0xFF);
    let b = (( color        & 0xFF) + amount).min(0xFF);
    (r << 16) | (g << 8) | b
}

fn darken_u32(color: u32, amount: u32) -> u32 {
    let r = ((color >> 16) & 0xFF).saturating_sub(amount);
    let g = ((color >>  8) & 0xFF).saturating_sub(amount);
    let b = ( color        & 0xFF).saturating_sub(amount);
    (r << 16) | (g << 8) | b
}

// ─── Re-Export für draw_borders_skia ──────────────────────────────────────────
// (damit der Compiler den Pfad findet – in deinem Projekt musst du den
//  tatsächlichen Import-Pfad für ComputedStyle anpassen)
