use softbuffer::Buffer;
use std::sync::Arc;
use winit::window::Window;

use layout_engine::layout::LayoutBox;
use crate::text_renderer::TextRenderer;
use crate::BrowserState;
use crate::layout_bridge::{HEADER_H, STATUS_H};
use crate::FocusedElement;
use crate::NodePtr;
use crate::image_cache::ImageCache;
use crate::image_cache::DecodedImage;
use crate::svg_renderer::SvgCache;

// ─── Farbpalette ──────────────────────────────────────────────────────────────
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

// ─── Hilfsfunktionen für Node-Zugriff ────────────────────────────────────────

fn layout_max_bottom(layout: &LayoutBox) -> i32 {
    let own = (layout.y + layout.height) as i32;
    layout.children.iter().map(layout_max_bottom).fold(own, i32::max)
}

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

fn lighten(color: u32, amount: u32) -> u32 {
    let r = (((color >> 16) & 0xFF) + amount).min(0xFF);
    let g = (((color >>  8) & 0xFF) + amount).min(0xFF);
    let b = (( color        & 0xFF) + amount).min(0xFF);
    (r << 16) | (g << 8) | b
}

// ─── Öffentliche API ──────────────────────────────────────────────────────────

pub fn paint_layout(
    buffer:  &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    win_w:   u32,
    win_h:   u32,
    _chrome: &LayoutBox,
    browser: &BrowserState,
    tr:      &TextRenderer,
    window:  Option<&Arc<Window>>,
) {
    buffer.fill(COL_TOOLBAR_BG);

    let content_x = 0_i32;
    let content_y = HEADER_H as i32;
    let content_w = win_w as i32;
    let content_h = (win_h as i32 - HEADER_H as i32 - STATUS_H as i32).max(0);

    if let Some(page) = &browser.page_layout {
        fill_rect(buffer, win_w, win_h, content_x, content_y, content_w, content_h, COL_PAGE_BG);
        let scroll = browser.scroll_y;
        let page_clip_x = page.x as i32;
        let page_clip_w = page.width as i32;
        let svg_cache = Arc::new(std::sync::Mutex::new(SvgCache::new(50 * 1024 * 1024)));
        let ctx = RenderCtx {
            win_w, win_h,
            clip_x: page_clip_x, clip_y: content_y,
            clip_w: page_clip_w, clip_h: content_h,
            container_w: page_clip_w,
            scroll,
            mouse_x: browser.mouse_x, mouse_y: browser.mouse_y,
            focused: &browser.focused,
            input_values: &browser.input_values,
            hovered_href: browser.hovered_href.as_deref(),
            image_cache: &browser.image_cache,
            svg_cache: &svg_cache,
            window,
        };
        paint_page_box(buffer, tr, page, &ctx);

        // position:fixed Elemente nochmal ohne Scroll-Offset rendern
        paint_fixed_children(buffer, tr, page, &ctx);

    } else if browser.is_loading {
        fill_rect(buffer, win_w, win_h, content_x, content_y, content_w, content_h, 0x00_F8_F9_FA);
        draw_spinner(buffer, win_w, win_h, content_x, content_y, content_w, content_h, tr, browser);
    } else {
        fill_rect(buffer, win_w, win_h, content_x, content_y, content_w, content_h, COL_PAGE_BG);
        draw_welcome_page(buffer, win_w, win_h, content_x, content_y, content_w, content_h, tr);
    }

    draw_browser_chrome(buffer, win_w, win_h, browser, tr);
}

// ─── Render-Kontext ───────────────────────────────────────────────────────────

struct RenderCtx<'a> {
    win_w: u32, win_h: u32,
    clip_x: i32, clip_y: i32, clip_w: i32, clip_h: i32,
    container_w: i32,
    scroll: i32,
    mouse_x: i32, mouse_y: i32,
    focused: &'a FocusedElement,
    input_values: &'a std::collections::HashMap<NodePtr, String>,
    hovered_href: Option<&'a str>,
    image_cache: &'a ImageCache,
    svg_cache: &'a std::sync::Arc<std::sync::Mutex<SvgCache>>,
    window: Option<&'a Arc<Window>>,
}

impl<'a> RenderCtx<'a> {
    fn is_input_focused(&self, ptr: NodePtr) -> bool {
        matches!(self.focused, FocusedElement::PageInput { ptr: p } if *p == ptr)
    }
    fn input_value(&self, ptr: NodePtr) -> &str {
        self.input_values.get(&ptr).map(|s| s.as_str()).unwrap_or("")
    }
    fn fixed_ctx(&self) -> RenderCtx<'_> {
        RenderCtx {
            scroll: 0,
            ..*self
        }
    }
}

// ─── position:fixed Rendering ────────────────────────────────────────────────

fn paint_fixed_children(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    tr: &TextRenderer,
    layout: &LayoutBox,
    ctx: &RenderCtx<'_>,
) {
    use layout_engine::cssom::PositionValue;
    if layout.style.position == PositionValue::Fixed {
        let fixed_ctx = ctx.fixed_ctx();
        paint_page_box(buffer, tr, layout, &fixed_ctx);
        return;
    }
    for child in &layout.children {
        paint_fixed_children(buffer, tr, child, ctx);
    }
}

// ─── Seiten-Rendering ─────────────────────────────────────────────────────────

fn paint_page_box(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    tr: &TextRenderer,
    layout: &LayoutBox,
    ctx: &RenderCtx<'_>,
) {
    use layout_engine::cssom::PositionValue;

    if layout.style.is_hidden() {
        return;
    }

    // position:fixed → bereits in paint_fixed_children ohne Scroll gerendert
    if layout.style.position == PositionValue::Fixed {
        return;
    }

    let x = layout.x as i32;
    let y = layout.y as i32 - ctx.scroll;
    let w = layout.width  as i32;
    let h = layout.height as i32;

    let opacity = layout.style.effective_opacity();
    if opacity <= 0.0 { return; }

    let in_clip = y < ctx.clip_y + ctx.clip_h && y + h.max(1) > ctx.clip_y
        && x < ctx.clip_x + ctx.clip_w && x + w.max(1) > ctx.clip_x;

    let tag = node_tag(layout);

    if w == 0 && h == 0 {
        for child in &layout.children {
            paint_page_box(buffer, tr, child, ctx);
        }
        return;
    }

    if in_clip {
        // ── box-shadow ────────────────────────────────────────────────────
        if let Some((ox, oy, blur, sc)) = layout.style.box_shadow {
            draw_box_shadow(buffer, ctx.win_w, ctx.win_h, x, y, w, h, ox, oy, blur, sc);
        }

        // ── Hintergrundfarbe / Gradient ───────────────────────────────────
        let s = &layout.style;
        if let Some((c1, c2)) = s.background_gradient {
            fill_linear_gradient(buffer, ctx.win_w, ctx.win_h, x, y, w, h, c1, c2);
        } else if let Some(bg) = s.background_color {
            let color = apply_opacity(bg, opacity);
            let radius = s.effective_border_radius() as i32;
            if radius > 0 {
                fill_rounded_rect(buffer, ctx.win_w, ctx.win_h, x, y, w, h, color, radius);
            } else {
                fill_rect(buffer, ctx.win_w, ctx.win_h, x, y, w, h, color);
            }
        }

        let is_hovered = ctx.mouse_x >= x && ctx.mouse_x < x + w.max(1)
            && ctx.mouse_y >= y && ctx.mouse_y < y + h.max(1);
        let ptr = layout as *const LayoutBox as usize;

        match tag {
            "input" | "textarea" => {
                let focused = ctx.is_input_focused(ptr);
                let border  = if focused { COL_INPUT_FOCUS }
                    else if is_hovered { 0x00_80_86_8B }
                    else { layout.style.border_color.unwrap_or(COL_INPUT_BORDER) };

                let bw = w.max(200);
                let bh = h.max(if tag == "textarea" { 80 } else { 36 });
                let radius = layout.style.effective_border_radius() as i32;

                if radius > 0 {
                    fill_rounded_rect(buffer, ctx.win_w, ctx.win_h, x, y, bw, bh, COL_INPUT_BG, radius);
                } else {
                    fill_rect(buffer, ctx.win_w, ctx.win_h, x, y, bw, bh, COL_INPUT_BG);
                }
                let border_w = if focused { 2 } else { 1 };
                draw_rect_outline_width(buffer, ctx.win_w, ctx.win_h, x, y, bw, bh, border, border_w);

                // Focus-Glow
                if focused {
                    blend_rect(buffer, ctx.win_w, ctx.win_h, x-1, y-1, bw+2, bh+2, 0.071, 0.345, 0.816, 0.18);
                }

                let value       = ctx.input_value(ptr);
                let placeholder = node_attr(layout, "placeholder").to_string();
                let font_sz     = layout.style.font_size;
                let text_h      = tr.measure_height("X", bw, font_sz);
                let text_y      = y + (bh - text_h).max(0) / 2;

                if value.is_empty() && !placeholder.is_empty() {
                    tr.draw(buffer, ctx.win_w, ctx.win_h, &placeholder, x+10, text_y, COL_URL_PLACEHOLDER, font_sz);
                } else {
                    tr.draw(buffer, ctx.win_w, ctx.win_h, value, x+10, text_y, COL_INPUT_TEXT, font_sz);
                    if focused {
                        let cx = x + 10 + tr.measure_width(value, font_sz);
                        fill_rect(buffer, ctx.win_w, ctx.win_h, cx, y+6, 2, bh-12, COL_CURSOR);
                    }
                }
            }

            "select" => {
                let bw = w.max(150);
                fill_rect(buffer, ctx.win_w, ctx.win_h, x, y, bw, h, COL_SELECT_BG);
                draw_rect_outline(buffer, ctx.win_w, ctx.win_h, x, y, bw, h, COL_INPUT_BORDER);
                fill_rect(buffer, ctx.win_w, ctx.win_h, x+bw-28, y, 28, h, 0x00_F1_F3_F4);
                fill_rect(buffer, ctx.win_w, ctx.win_h, x+bw-28, y, 1, h, COL_INPUT_BORDER);
                tr.draw(buffer, ctx.win_w, ctx.win_h, "▾", x+bw-20, y+(h-14)/2, COL_MUTED, None);
            }

            "th" => {
                fill_rect(buffer, ctx.win_w, ctx.win_h, x, y, w, h, COL_TABLE_HEADER);
                draw_rect_outline(buffer, ctx.win_w, ctx.win_h, x, y, w, h, COL_STATUS_BORDER);
            }
            "td" => {
                draw_rect_outline(buffer, ctx.win_w, ctx.win_h, x, y, w, h, 0x00_E8_EA_ED);
            }

            "button" | "submit" => {
                let bw = w.max(80);
                let bh = h.max(36);
                let radius = layout.style.effective_border_radius() as i32;
                let bg_color = layout.style.background_color
                    .unwrap_or(if is_hovered { COL_BTN_PAGE_HOVER } else { COL_BTN_PAGE_BG });
                let bg = if is_hovered && layout.style.background_color.is_some() {
                    // Leichtes Abdunkeln beim Hover wenn benutzerdefinierte Farbe
                    darken(bg_color, 15)
                } else if is_hovered {
                    COL_BTN_PAGE_HOVER
                } else {
                    bg_color
                };

                // Schatten
                blend_rect(buffer, ctx.win_w, ctx.win_h, x+2, y+2, bw, bh, 0.0, 0.0, 0.0, 0.08);
                if radius > 0 {
                    fill_rounded_rect(buffer, ctx.win_w, ctx.win_h, x, y, bw, bh, bg, radius);
                } else {
                    fill_rounded_rect(buffer, ctx.win_w, ctx.win_h, x, y, bw, bh, bg, 4);
                }
                // Highlight-Linie oben
                let eff_r = if radius > 0 { radius } else { 4 };
                fill_rect(buffer, ctx.win_w, ctx.win_h, x+eff_r, y, bw-eff_r*2, 1, lighten(bg, 20));
            }

            "code" | "samp" | "kbd" => {
                fill_rounded_rect(buffer, ctx.win_w, ctx.win_h, x, y, w.max(20), h.max(18), COL_CODE_BG, 3);
                draw_rounded_rect_outline(buffer, ctx.win_w, ctx.win_h, x, y, w.max(20), h.max(18), 0x00_C4_C7_C5, 3);
            }
            "pre" => {
                fill_rounded_rect(buffer, ctx.win_w, ctx.win_h, x, y, w, h.max(20), 0x00_28_2A_2E, 6);
                draw_rounded_rect_outline(buffer, ctx.win_w, ctx.win_h, x, y, w, h.max(20), 0x00_3C_40_43, 6);
            }

            "blockquote" => {
                fill_rect(buffer, ctx.win_w, ctx.win_h, x, y, w, h, 0x00_F8_F9_FA);
                fill_rect(buffer, ctx.win_w, ctx.win_h, x, y, 4, h, COL_BLOCKQUOTE);
            }

            "hr" => {
                fill_rect(buffer, ctx.win_w, ctx.win_h, x, y + h/2, w, 1, COL_STATUS_BORDER);
            }

            "img" => {
                let iw = w.max(60);
                let ih = h.max(40);
                let src = node_attr(layout, "src");
                let drawn = if !src.is_empty() {
                    let window_clone = ctx.window.cloned();
                    let img = ctx.image_cache.get_or_load_with_callback(src, move || {
                        if let Some(ref w) = window_clone {
                            w.request_redraw();
                        }
                    });
                    if let Some(img) = img {
                        blit_image(buffer, ctx.win_w, ctx.win_h, x, y, iw, ih, &img);
                        true
                    } else { false }
                } else { false };

                if !drawn {
                    fill_rect(buffer, ctx.win_w, ctx.win_h, x, y, iw, ih, 0x00_F1_F3_F4);
                    draw_rect_outline(buffer, ctx.win_w, ctx.win_h, x, y, iw, ih, COL_STATUS_BORDER);
                    // Kleines Bild-Icon
                    let icon_x = x + (iw - 20) / 2;
                    let icon_y = y + (ih - 14) / 2;
                    fill_rect(buffer, ctx.win_w, ctx.win_h, icon_x, icon_y, 20, 14, 0x00_DA_DC_E0);
                    fill_rect(buffer, ctx.win_w, ctx.win_h, icon_x+2, icon_y+3, 5, 5, 0x00_B0_B8_C0);
                    let lbl = node_attr(layout, "alt");
                    if !lbl.is_empty() {
                        tr.draw(buffer, ctx.win_w, ctx.win_h, lbl,
                                x + 2, y + ih - 16, COL_MUTED, Some(11.0));
                    }
                }
            }

            "svg" => {
                let iw = (w.max(30)) as u32;
                let ih = (h.max(30)) as u32;
                
                // Versuche SVG von src Attribut zu laden
                let src = node_attr(layout, "src");
                if !src.is_empty() {
                    if let Ok(mut svg_cache) = ctx.svg_cache.lock() {
                        let window_clone = ctx.window.cloned();
                        match std::fs::read_to_string(src) {
                            Ok(svg_data) => {
                                if let Some(_png_bytes) = svg_cache.render_svg(src, &svg_data, iw, ih) {
                                    // PNG wurde gepuffert - in nächster Runde rendern
                                    eprintln!("[SVG] Rendered: {} ({}x{})", src, iw, ih);
                                }
                            }
                            Err(e) => {
                                eprintln!("[SVG] Error loading {}: {}", src, e);
                            }
                        }
                    }
                } else {
                    // Inline SVG: Platzhalter anzeigen
                    fill_rect(buffer, ctx.win_w, ctx.win_h, x, y, iw as i32, ih as i32, 0x00_E0_E0_E0);
                }
            }

            _ => {
                // CSS-Borders pro Seite
                if w >= 2 && h >= 2 {
                    let s = &layout.style;
                    let radius = s.effective_border_radius() as i32;

                    // Hintergrundfarbe (nochmal falls gradient nicht greift)
                    if s.background_gradient.is_none() {
                        if let Some(bg) = s.background_color {
                            let color = apply_opacity(bg, opacity);
                            if radius > 0 {
                                fill_rounded_rect(buffer, ctx.win_w, ctx.win_h, x, y, w, h, color, radius);
                            } else {
                                fill_rect(buffer, ctx.win_w, ctx.win_h, x, y, w, h, color);
                            }
                        }
                    }

                    draw_borders(buffer, ctx.win_w, ctx.win_h, x, y, w, h,
                                 s.border_width_top(),    s.border_color_top(),
                                 s.border_width_right(),  s.border_color_right(),
                                 s.border_width_bottom(), s.border_color_bottom(),
                                 s.border_width_left(),   s.border_color_left(),
                    );
                }
            }
        }

        // ── Text-Knoten ──────────────────────────────────────────────────
        if is_text_node(layout) {
            let raw  = node_text(layout);
            let text = raw.replace('\u{00A0}', " ");
            let text = text.trim();
            if !text.is_empty() {
                let col      = layout.style.color.unwrap_or(COL_PAGE_TEXT);
                let text_col = if tag == "pre" { 0x00_E8_EA_ED } else { col };
                let font_sz  = layout.style.font_size;
                let line_h   = font_sz.unwrap_or(16.0) * 1.4;
                let text_y   = y + 2;

                let container_right = ctx.clip_x + ctx.container_w;
                let max_w = (container_right - x).max(50);

                use layout_engine::cssom::TextAlignValue;
                let text_x = match layout.style.text_align {
                    TextAlignValue::Center => {
                        let tw = tr.measure_width(text, font_sz);
                        let offset = ((max_w - tw) / 2).max(0);
                        x + offset
                    }
                    TextAlignValue::Right => {
                        let tw = tr.measure_width(text, font_sz);
                        let offset = (max_w - tw - 4).max(0);
                        x + offset
                    }
                    _ => x,
                };

                // Unterstreichung für Links
                if matches!(tag, "a" | "u") || layout.style.text_decoration
                    .as_ref()
                    .map(|td| matches!(td, layout_engine::cssom::TextDecorationValue::Underline))
                    .unwrap_or(false)
                {
                    let tw = tr.measure_width(text, font_sz);
                    fill_rect(buffer, ctx.win_w, ctx.win_h, text_x, y + line_h as i32 - 2, tw, 1, text_col);
                }

                tr.draw_wrapped(buffer, ctx.win_w, ctx.win_h, text, text_x, text_y, max_w, text_col, font_sz);
            }
        }

        // ── H1 Unterstreichung ────────────────────────────────────────────
        if tag == "h1" && h > 4 {
            fill_rect(buffer, ctx.win_w, ctx.win_h, x, y + h - 2, w, 1, 0x00_DA_DC_E0);
        }
        // ── H2 Unterstreichung ────────────────────────────────────────────
        if tag == "h2" && h > 4 {
            fill_rect(buffer, ctx.win_w, ctx.win_h, x, y + h - 1, w, 1, 0x00_E8_EA_ED);
        }
    }

    // ── Kinder rendern ────────────────────────────────────────────────────
    // Für position:absolute / :fixed Kinder keinen clip-kontext vererben
    let is_block = !is_text_node(layout) && w > 50;
    let child_container_w = if is_block { w } else { ctx.container_w };
    let child_clip_x      = if is_block { x } else { ctx.clip_x };

    let child_ctx = RenderCtx {
        win_w: ctx.win_w, win_h: ctx.win_h,
        clip_x: child_clip_x, clip_y: ctx.clip_y,
        clip_w: ctx.clip_w, clip_h: ctx.clip_h,
        container_w: child_container_w,
        scroll: ctx.scroll,
        mouse_x: ctx.mouse_x, mouse_y: ctx.mouse_y,
        focused: ctx.focused,
        input_values: ctx.input_values,
        hovered_href: ctx.hovered_href,
        image_cache: ctx.image_cache,
        svg_cache: ctx.svg_cache,
        window: ctx.window,
    };

    // Inline-Kinder als zusammenhängenden Fließtext rendern
    if tag != "a" && tag != "button" && tag != "pre"
        && !matches!(tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6")
        && in_clip
        && has_only_inline_children(layout)
        && !layout.children.is_empty()
    {
        paint_inline_children(buffer, tr, layout, &child_ctx);
        return;
    }

    for child in &layout.children {
        let child_tag = node_tag(child);
        if tag == "a" {
            paint_link_child(buffer, tr, child, &child_ctx);
        } else if matches!(child_tag, "h1" | "h2" | "h3" | "h4" | "h5" | "h6") {
            paint_heading_child(buffer, tr, child, &child_ctx);
        } else if tag == "button" || tag == "submit" {
            paint_button_child(buffer, tr, child, &child_ctx);
        } else if tag == "pre" {
            paint_pre_child(buffer, tr, child, &child_ctx);
        } else {
            paint_page_box(buffer, tr, child, &child_ctx);
        }
    }
}

// ─── Inline-Kinder Rendering ──────────────────────────────────────────────────
//
// FIX v1.0: Rendert Inline-Kinder mit korrektem x-Offset (padding des Parents),
// respektiert font_size jedes Kind-Nodes, rendert Links farbig.

fn has_only_inline_children(layout: &LayoutBox) -> bool {
    const INLINE_TAGS: &[&str] = &[
        "a", "span", "b", "i", "em", "strong", "code", "small",
        "sup", "sub", "abbr", "cite", "q", "mark", "u", "s",
        "del", "ins", "kbd", "samp", "var", "time", "label",
    ];

    if layout.children.is_empty() { return false; }

    let mut link_count = 0usize;
    let mut text_char_count = 0usize;
    let mut has_block = false;

    for child in &layout.children {
        let t = node_tag(child);
        if child.text.is_some() {
            text_char_count += child.text.as_deref().unwrap_or("").trim().len();
        } else if INLINE_TAGS.contains(&t) {
            if t == "a" { link_count += 1; }
            for grandchild in &child.children {
                if let Some(txt) = &grandchild.text {
                    text_char_count += txt.trim().len();
                }
            }
        } else {
            has_block = true;
            break;
        }
    }

    if has_block { return false; }
    if link_count > 8 && text_char_count < 20 { return false; }
    true
}

/// Inline-Kinder: Text korrekt am layout.x-Position plus padding rendern.
/// Jedes Kind bringt seine eigene font_size und color mit.
fn paint_inline_children(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    tr: &TextRenderer,
    layout: &LayoutBox,
    ctx: &RenderCtx<'_>,
) {
    // Startposition: padding_left des Parents berücksichtigen
    let base_x = layout.x as i32 + layout.style.padding_left as i32;
    let base_y = layout.y as i32 - ctx.scroll + layout.style.padding_top as i32 + 2;
    let max_w  = (layout.width as i32 - layout.style.padding_left as i32
                  - layout.style.padding_right as i32).max(50);

    // Alle Inline-Kinder als ein zusammenhängender Text-Block,
    // aber mit Tag-spezifischer Formatierung
    paint_inline_box(buffer, tr, layout, ctx, base_x, base_y, max_w);
}

/// Rendert einen Inline-Knoten und seine Kinder.
/// Gibt die X-Position nach dem gerenderten Inhalt zurück.
fn paint_inline_box(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    tr: &TextRenderer,
    layout: &LayoutBox,
    ctx: &RenderCtx<'_>,
    start_x: i32,
    base_y: i32,
    max_w: i32,
) -> i32 {
    let tag = node_tag(layout);
    let is_link = tag == "a";
    let _is_bold = matches!(tag, "b" | "strong");
    let is_code = matches!(tag, "code" | "kbd" | "samp");

    // Basis-Textfarbe aus Style, mit Tag-Überschreibungen
    let text_color = if is_link {
        let href = node_attr(layout, "href");
        let hovered = ctx.hovered_href == Some(href);
        if hovered { COL_LINK_HOVER } else { COL_LINK }
    } else {
        layout.style.color.unwrap_or(COL_PAGE_TEXT)
    };

    let font_sz = layout.style.font_size;

    // code-Hintergrund
    if is_code {
        let text = collect_text_of(layout);
        let tw = tr.measure_width(text.trim(), font_sz);
        let th = tr.measure_height(text.trim(), tw.max(1), font_sz);
        fill_rounded_rect(buffer, ctx.win_w, ctx.win_h, start_x - 2, base_y - 2, tw + 4, th + 4, COL_CODE_BG, 3);
    }

    if is_text_node(layout) {
        let raw  = node_text(layout);
        let text = raw.replace('\u{00A0}', " ");
        let text = text.trim();
        if !text.is_empty() {
            let tw = tr.measure_width(text, font_sz);
            tr.draw_wrapped(buffer, ctx.win_w, ctx.win_h, text, start_x, base_y, max_w, text_color, font_sz);

            // Unterstreichung für Links
            if is_link && tw > 0 {
                let lh = font_sz.unwrap_or(16.0) * 1.4;
                fill_rect(buffer, ctx.win_w, ctx.win_h, start_x, base_y + lh as i32 - 2, tw, 1, text_color);
            }

            return start_x + tw;
        }
        return start_x;
    }

    // Kinder inline rendern
    let mut cx = start_x;
    for child in &layout.children {
        let child_color = if is_link {
            text_color // Link-Farbe vererben
        } else {
            child.style.color.unwrap_or(text_color)
        };
        // child mit überschriebener Farbe rendern
        cx = paint_inline_box_colored(buffer, tr, child, ctx, cx, base_y, max_w, child_color);
    }
    cx
}

/// Wie paint_inline_box, aber mit explizit überschriebener Textfarbe (für Links).
fn paint_inline_box_colored(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    tr: &TextRenderer,
    layout: &LayoutBox,
    ctx: &RenderCtx<'_>,
    start_x: i32,
    base_y: i32,
    max_w: i32,
    color_override: u32,
) -> i32 {
    let font_sz = layout.style.font_size;
    let text_color = layout.style.color.unwrap_or(color_override);

    if is_text_node(layout) {
        let raw  = node_text(layout);
        let text = raw.replace('\u{00A0}', " ");
        let text = text.trim();
        if !text.is_empty() {
            let tw = tr.measure_width(text, font_sz);
            tr.draw_wrapped(buffer, ctx.win_w, ctx.win_h, text, start_x, base_y, max_w, text_color, font_sz);
            return start_x + tw;
        }
        return start_x;
    }

    let tag = node_tag(layout);
    let is_code = matches!(tag, "code" | "kbd");
    if is_code {
        let txt = collect_text_of(layout);
        let tw = tr.measure_width(txt.trim(), font_sz);
        let th = tr.measure_height(txt.trim(), tw.max(1), font_sz);
        fill_rounded_rect(buffer, ctx.win_w, ctx.win_h, start_x - 2, base_y - 2, tw + 4, th + 4, COL_CODE_BG, 3);
    }

    let mut cx = start_x;
    for child in &layout.children {
        cx = paint_inline_box_colored(buffer, tr, child, ctx, cx, base_y, max_w, text_color);
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

// ─── Link-Rendering ───────────────────────────────────────────────────────────

fn paint_link_child(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>, tr: &TextRenderer,
    layout: &LayoutBox, ctx: &RenderCtx<'_>,
) {
    if is_text_node(layout) {
        let x = layout.x as i32;
        let y = layout.y as i32 - ctx.scroll;
        let raw  = node_text(layout);
        let text = raw.replace('\u{00A0}', " ");
        let text = text.trim();
        if !text.is_empty() {
            let font_sz = layout.style.font_size;
            let tw = tr.measure_width(text, font_sz);
            let hovered = ctx.mouse_x >= x && ctx.mouse_x < x+tw+4
                && ctx.mouse_y >= y && ctx.mouse_y < y+20;
            let color = if hovered { COL_LINK_HOVER } else { COL_LINK };
            let lh = font_sz.unwrap_or(16.0) * 1.4;
            tr.draw(buffer, ctx.win_w, ctx.win_h, text, x+2, y+3, color, font_sz);
            if tw > 0 {
                fill_rect(buffer, ctx.win_w, ctx.win_h, x+2, y + lh as i32 - 2, tw, 1, color);
            }
        }
    }
    for child in &layout.children {
        paint_link_child(buffer, tr, child, ctx);
    }
}

// ─── Heading-Rendering ────────────────────────────────────────────────────────

fn paint_heading_child(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>, tr: &TextRenderer,
    layout: &LayoutBox, ctx: &RenderCtx<'_>,
) {
    let tag   = node_tag(layout);
    let color = layout.style.color.unwrap_or(match tag {
        "h1"        => COL_H1,
        "h2"        => COL_H2,
        "h3" | "h4" => 0x00_30_31_34_u32,
        _           => COL_PAGE_TEXT,
    });

    if is_text_node(layout) {
        let x    = layout.x as i32;
        let y    = layout.y as i32 - ctx.scroll;
        let raw  = node_text(layout);
        let text = raw.replace('\u{00A0}', " ");
        let text = text.trim();
        if !text.is_empty() {
            let font_sz = layout.style.font_size;
            let container_right = ctx.clip_x + ctx.container_w;
            let max_w = (container_right - x).max(50);
            tr.draw_wrapped(buffer, ctx.win_w, ctx.win_h, text, x, y + 2, max_w, color, font_sz);
        }
    }

    for child in &layout.children {
        paint_heading_child(buffer, tr, child, ctx);
    }
}

// ─── Button-Rendering ─────────────────────────────────────────────────────────

fn paint_button_child(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>, tr: &TextRenderer,
    layout: &LayoutBox, ctx: &RenderCtx<'_>,
) {
    if is_text_node(layout) {
        let x    = layout.x as i32;
        let y    = layout.y as i32 - ctx.scroll;
        let raw  = node_text(layout);
        let text = raw.replace('\u{00A0}', " ");
        let text = text.trim();
        if !text.is_empty() {
            let font_sz = layout.style.font_size;
            let color = layout.style.color.unwrap_or(COL_BTN_PAGE_TEXT);
            tr.draw(buffer, ctx.win_w, ctx.win_h, text, x+8, y+11, color, font_sz);
        }
    }
    for child in &layout.children {
        paint_button_child(buffer, tr, child, ctx);
    }
}

// ─── Pre-Rendering ────────────────────────────────────────────────────────────

fn paint_pre_child(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>, tr: &TextRenderer,
    layout: &LayoutBox, ctx: &RenderCtx<'_>,
) {
    if is_text_node(layout) {
        let x    = layout.x as i32;
        let y    = layout.y as i32 - ctx.scroll;
        let raw  = node_text(layout);
        // Pre: Leerzeichen und Newlines beibehalten
        let text = raw.trim_matches('\n');
        if !text.is_empty() {
            let font_sz = layout.style.font_size.or(Some(14.0));
            let max_w = (layout.width as i32).max(100);
            tr.draw_wrapped(buffer, ctx.win_w, ctx.win_h, text, x+12, y+8, max_w - 24, 0x00_E8_EA_ED, font_sz);
        }
    }
    for child in &layout.children {
        paint_pre_child(buffer, tr, child, ctx);
    }
}

// ─── Willkommensseite ─────────────────────────────────────────────────────────

fn draw_welcome_page(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    win_w: u32, win_h: u32, cx: i32, cy: i32, cw: i32, ch: i32, tr: &TextRenderer,
) {
    let center_y = cy + ch / 2;
    let logo_x   = cx + cw / 2;
    let logo_y   = center_y - 70;

    // Logo-Kreis
    fill_rounded_rect(buffer, win_w, win_h, logo_x - 28, logo_y - 28, 56, 56, 0x00_E8_F0_FE, 28);
    draw_rounded_rect_outline(buffer, win_w, win_h, logo_x - 28, logo_y - 28, 56, 56, 0x00_18_58_AB, 28);
    let n_tw = tr.measure_width("N", Some(20.0));
    tr.draw(buffer, win_w, win_h, "N", logo_x - n_tw/2, logo_y - 10, 0x00_18_58_AB, Some(20.0));

    let title = "NexusBrowser";
    let tw = tr.measure_width(title, Some(18.0));
    tr.draw(buffer, win_w, win_h, title, cx + (cw - tw) / 2, center_y - 20, 0x00_20_21_24, Some(18.0));

    let sub = "URL eingeben und Enter drücken";
    let stw = tr.measure_width(sub, Some(13.0));
    tr.draw(buffer, win_w, win_h, sub, cx + (cw - stw) / 2, center_y + 8, COL_MUTED, Some(13.0));

    fill_rect(buffer, win_w, win_h, cx + cw/4, center_y + 32, cw/2, 1, 0x00_DA_DC_E0);

    let tips = ["F5 = Neu laden", "ESC = Fokus aufheben", "Scroll = Seite scrollen"];
    for (i, tip) in tips.iter().enumerate() {
        let tw = tr.measure_width(tip, Some(12.0));
        tr.draw(buffer, win_w, win_h, tip, cx+(cw-tw)/2, center_y + 46 + (i as i32 * 20), COL_MUTED, Some(12.0));
    }
}

// ─── Lade-Spinner ─────────────────────────────────────────────────────────────

fn draw_spinner(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    win_w: u32, win_h: u32, content_x: i32, content_y: i32,
    content_w: i32, content_h: i32, tr: &TextRenderer, browser: &BrowserState,
) {
    let cx    = content_x + content_w / 2;
    let cy    = content_y + content_h / 2;
    let bar_w = 240;
    let bar_x = cx - bar_w / 2;
    let bar_y = cy + 24;

    fill_rounded_rect(buffer, win_w, win_h, bar_x, bar_y, bar_w, 4, 0x00_DA_DC_E0, 2);
    let progress = (((browser.mouse_x.unsigned_abs() as u32) % (bar_w as u32 + 1)) as i32).min(bar_w);
    fill_rounded_rect(buffer, win_w, win_h, bar_x, bar_y, progress.max(20), 4, COL_LOADING_BAR, 2);

    let domain = extract_domain(&browser.url);
    let msg    = format!("Lade {}...", domain);
    let tw     = tr.measure_width(&msg, None);
    tr.draw(buffer, win_w, win_h, &msg, cx - tw/2, cy, COL_MUTED, None);
}

fn extract_domain(url: &str) -> &str {
    let stripped = url.strip_prefix("https://")
        .or_else(|| url.strip_prefix("http://"))
        .unwrap_or(url);
    stripped.split('/').next().unwrap_or(stripped)
}

// ─── Chrome-Rendering ─────────────────────────────────────────────────────────

fn draw_browser_chrome(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    win_w: u32, win_h: u32, browser: &BrowserState, tr: &TextRenderer,
) {
    // Toolbar Hintergrund
    fill_rect(buffer, win_w, win_h, 0, 0, win_w as i32, HEADER_H as i32, COL_TOOLBAR_BG);
    fill_rect(buffer, win_w, win_h, 0, 0, win_w as i32, 1, lighten(COL_TOOLBAR_BG, 12));
    fill_rect(buffer, win_w, win_h, 0, HEADER_H as i32 - 2, win_w as i32, 2, COL_TOOLBAR_BOTTOM);

    draw_nav_buttons(buffer, win_w, win_h, tr);
    draw_url_bar(buffer, win_w, win_h, browser, tr);

    // Statusleiste
    let sy = (win_h as i32 - STATUS_H as i32).max(0);
    fill_rect(buffer, win_w, win_h, 0, sy, win_w as i32, STATUS_H as i32, COL_STATUS_BG);
    fill_rect(buffer, win_w, win_h, 0, sy, win_w as i32, 1, COL_STATUS_BORDER);

    let status_text = if let Some(href) = &browser.hovered_href {
        format!("  {}", href)
    } else {
        format!("  {}", browser.status_text)
    };
    tr.draw(buffer, win_w, win_h, &status_text, 6, sy + (STATUS_H as i32 - 14) / 2, COL_STATUS_TEXT, Some(12.0));

    if browser.is_loading {
        tr.draw(buffer, win_w, win_h, "⟳ Laden...", win_w as i32 - 80, (HEADER_H as i32 - 14) / 2, 0x00_8A_B4_F8, Some(12.0));
    }

    // Scrollbar
    if let Some(page) = &browser.page_layout {
        let content_h  = (win_h as i32 - HEADER_H as i32 - STATUS_H as i32).max(1);
        let page_h = layout_max_bottom(page);
        if page_h > content_h {
            let max_scroll   = (page_h - content_h).max(1);
            let scroll_ratio = (browser.scroll_y as f32 / max_scroll as f32).clamp(0.0, 1.0);
            let track_h  = content_h;
            let thumb_h  = ((content_h as f32 / page_h as f32) * track_h as f32)
                               .max(24.0).min(track_h as f32) as i32;
            let thumb_y  = HEADER_H as i32
                + (scroll_ratio * (track_h - thumb_h) as f32) as i32;
            fill_rect(buffer, win_w, win_h, win_w as i32 - 10, HEADER_H as i32, 10, track_h, 0x00_F1_F3_F4);
            fill_rounded_rect(buffer, win_w, win_h, win_w as i32 - 9, thumb_y + 2, 8, (thumb_h - 4).max(4), 0x00_BD_C1_C6, 4);
        }
    }
}

fn draw_nav_buttons(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>, win_w: u32, win_h: u32, tr: &TextRenderer,
) {
    let btn: i32 = 32;
    let mg:  i32 = 5;
    let by = (HEADER_H as i32 - btn) / 2;
    let buttons: &[(i32, &str)] = &[
        (mg,               "←"),
        (mg + btn + mg,    "→"),
        (mg + (btn+mg)*2,  "↻"),
    ];
    for (bx, label) in buttons {
        fill_rounded_rect(buffer, win_w, win_h, *bx+1, by+1, btn-2, btn-2, COL_BTN_BG, 6);
        let tw = tr.measure_width(label, None);
        tr.draw(buffer, win_w, win_h, label, bx + (btn-tw)/2, by + (btn-14)/2, COL_BTN_TEXT, None);
    }
}

fn draw_url_bar(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    win_w: u32, win_h: u32, browser: &BrowserState, tr: &TextRenderer,
) {
    let btn:   i32 = 32;
    let mg:    i32 = 5;
    let bar_h: i32 = 30;
    let bar_x  = mg + (btn + mg) * 3 + 4;
    let bar_y  = (HEADER_H as i32 - bar_h) / 2;
    let bar_w  = (win_w as i32 - bar_x - mg - 4).max(0);
    if bar_w == 0 { return; }

    let focused = browser.url_focused || matches!(browser.focused, FocusedElement::UrlBar);
    let bg      = if focused { COL_URL_BG_FOCUS } else { COL_URL_BG };
    let border  = if focused { COL_URL_BORDER } else { 0x00_5F_63_68 };

    fill_rounded_rect(buffer, win_w, win_h, bar_x, bar_y, bar_w, bar_h, bg, 15);
    draw_rounded_rect_outline(buffer, win_w, win_h, bar_x, bar_y, bar_w, bar_h, border, 15);

    let text_y = bar_y + (bar_h - 14) / 2;

    if browser.url.is_empty() {
        tr.draw(buffer, win_w, win_h, "URL eingeben...", bar_x + 12, text_y, COL_URL_PLACEHOLDER, Some(13.0));
    } else {
        // Protokoll-Teil grau, Rest weiß
        let url = &browser.url;
        if let Some(sep) = url.find("://") {
            let proto = &url[..sep+3];
            let rest  = &url[sep+3..];
            let pw = tr.measure_width(proto, Some(13.0));
            tr.draw(buffer, win_w, win_h, proto, bar_x + 12, text_y, COL_URL_PLACEHOLDER, Some(13.0));
            tr.draw(buffer, win_w, win_h, rest,  bar_x + 12 + pw, text_y, COL_URL_TEXT, Some(13.0));
        } else {
            tr.draw(buffer, win_w, win_h, url, bar_x + 12, text_y, COL_URL_TEXT, Some(13.0));
        }
        if focused {
            let tw = tr.measure_width(url, Some(13.0));
            fill_rect(buffer, win_w, win_h, bar_x + 12 + tw, text_y, 2, 16, COL_URL_BORDER);
        }
    }
}

// ─── Box-Shadow ───────────────────────────────────────────────────────────────

fn draw_box_shadow(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    win_w: u32, win_h: u32,
    x: i32, y: i32, w: i32, h: i32,
    ox: f32, oy: f32, blur: f32, color: u32,
) {
    let steps = (blur as i32).max(1);
    let r = ((color >> 16) & 0xFF) as f32 / 255.0;
    let g = ((color >>  8) & 0xFF) as f32 / 255.0;
    let b = ( color        & 0xFF) as f32 / 255.0;
    for i in 0..=steps {
        let t = i as f32 / steps as f32;
        let alpha = (1.0 - t) * 0.15;
        let expand = i;
        blend_rect(
            buffer, win_w, win_h,
            x + ox as i32 - expand, y + oy as i32 - expand,
            w + expand * 2, h + expand * 2,
            r, g, b, alpha,
        );
    }
}

// ─── Zeichenprimitive ─────────────────────────────────────────────────────────

fn blend_rect(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    win_w: u32, win_h: u32,
    x: i32, y: i32, w: i32, h: i32,
    fr: f32, fg: f32, fb: f32, alpha: f32,
) {
    if w <= 0 || h <= 0 || alpha <= 0.0 { return; }
    let x0 = (x as i64).clamp(0, win_w as i64) as u32;
    let y0 = (y as i64).clamp(0, win_h as i64) as u32;
    let x1 = ((x as i64 + w as i64).clamp(0, win_w as i64)) as u32;
    let y1 = ((y as i64 + h as i64).clamp(0, win_h as i64)) as u32;
    if x0 >= x1 || y0 >= y1 { return; }
    let buf_len = buffer.len();
    let inv = 1.0 - alpha;
    for row in y0..y1 {
        for col in x0..x1 {
            let idx = (row * win_w + col) as usize;
            if idx >= buf_len { break; }
            let bg  = buffer[idx];
            let br  = ((bg >> 16) & 0xFF) as f32 / 255.0;
            let bg_ = ((bg >>  8) & 0xFF) as f32 / 255.0;
            let bb  = ( bg        & 0xFF) as f32 / 255.0;
            let nr = ((fr * alpha + br * inv) * 255.0) as u32;
            let ng = ((fg * alpha + bg_ * inv) * 255.0) as u32;
            let nb = ((fb * alpha + bb * inv) * 255.0) as u32;
            buffer[idx] = (nr << 16) | (ng << 8) | nb;
        }
    }
}

fn draw_borders(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    win_w: u32, win_h: u32,
    x: i32, y: i32, w: i32, h: i32,
    top_w: f32,    top_c: Option<u32>,
    right_w: f32,  right_c: Option<u32>,
    bottom_w: f32, bottom_c: Option<u32>,
    left_w: f32,   left_c: Option<u32>,
) {
    if let Some(c) = top_c    { if top_w > 0.0    { fill_rect(buffer, win_w, win_h, x, y, w, top_w as i32, c); } }
    if let Some(c) = right_c  { if right_w > 0.0  { fill_rect(buffer, win_w, win_h, x+w-right_w as i32, y, right_w as i32, h, c); } }
    if let Some(c) = bottom_c { if bottom_w > 0.0 { fill_rect(buffer, win_w, win_h, x, y+h-bottom_w as i32, w, bottom_w as i32, c); } }
    if let Some(c) = left_c   { if left_w > 0.0   { fill_rect(buffer, win_w, win_h, x, y, left_w as i32, h, c); } }
}

pub fn fill_rect(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    win_w: u32, win_h: u32, x: i32, y: i32, w: i32, h: i32, color: u32,
) {
    if w <= 0 || h <= 0 { return; }
    let x0 = (x as i64).clamp(0, win_w as i64) as u32;
    let y0 = (y as i64).clamp(0, win_h as i64) as u32;
    let x1 = ((x as i64 + w as i64).clamp(0, win_w as i64)) as u32;
    let y1 = ((y as i64 + h as i64).clamp(0, win_h as i64)) as u32;
    if x0 >= x1 || y0 >= y1 { return; }
    let buf_len = buffer.len();
    for row in y0..y1 {
        let start = (row * win_w + x0) as usize;
        let end   = (row * win_w + x1) as usize;
        if start >= buf_len { break; }
        let end = end.min(buf_len);
        if start >= end { continue; }
        buffer[start..end].fill(color);
    }
}

fn draw_rect_outline(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    win_w: u32, win_h: u32, x: i32, y: i32, w: i32, h: i32, color: u32,
) {
    draw_rect_outline_width(buffer, win_w, win_h, x, y, w, h, color, 1);
}

fn draw_rect_outline_width(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    win_w: u32, win_h: u32, x: i32, y: i32, w: i32, h: i32, color: u32, bw: i32,
) {
    if w <= 0 || h <= 0 { return; }
    fill_rect(buffer, win_w, win_h, x,         y,         w, bw, color);
    fill_rect(buffer, win_w, win_h, x,         y + h - bw, w, bw, color);
    fill_rect(buffer, win_w, win_h, x,         y,         bw, h, color);
    fill_rect(buffer, win_w, win_h, x + w - bw, y,        bw, h, color);
}

pub fn fill_rounded_rect(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    win_w: u32, win_h: u32,
    x: i32, y: i32, w: i32, h: i32,
    color: u32, radius: i32,
) {
    if w <= 0 || h <= 0 { return; }
    if radius <= 0 {
        fill_rect(buffer, win_w, win_h, x, y, w, h, color);
        return;
    }
    let r = radius.min(w / 2).min(h / 2).max(0);
    if r == 0 {
        fill_rect(buffer, win_w, win_h, x, y, w, h, color);
        return;
    }
    let r_f = r as f32;
    let r_sq = r_f * r_f + r_f;

    fill_rect(buffer, win_w, win_h, x + r, y,         w - 2*r, h,         color);
    fill_rect(buffer, win_w, win_h, x,     y + r,     r,       h - 2*r,   color);
    fill_rect(buffer, win_w, win_h, x+w-r, y + r,     r,       h - 2*r,   color);

    let corners = [
        (x + r,     y + r,     true,  true),
        (x + w - r - 1, y + r,     false, true),
        (x + r,     y + h - r - 1, true,  false),
        (x + w - r - 1, y + h - r - 1, false, false),
    ];

    for (cx, cy, _left, _top) in &corners {
        for dy in -r..=r {
            for dx in -r..=r {
                let dist_sq = (dx * dx + dy * dy) as f32;
                if dist_sq <= r_sq {
                    let px = cx + dx;
                    let py = cy + dy;
                    if px >= 0 && px < win_w as i32 && py >= 0 && py < win_h as i32 {
                        let idx = (py as u32 * win_w + px as u32) as usize;
                        if idx < buffer.len() {
                            buffer[idx] = color;
                        }
                    }
                }
            }
        }
    }
}

fn draw_rounded_rect_outline(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    win_w: u32, win_h: u32,
    x: i32, y: i32, w: i32, h: i32, color: u32, radius: i32,
) {
    if w <= 0 || h <= 0 { return; }
    let r = radius.min(w/2).min(h/2).max(0);
    fill_rect(buffer, win_w, win_h, x+r,   y,       w-2*r, 1, color);
    fill_rect(buffer, win_w, win_h, x+r,   y+h-1,   w-2*r, 1, color);
    fill_rect(buffer, win_w, win_h, x,     y+r,     1, h-2*r, color);
    fill_rect(buffer, win_w, win_h, x+w-1, y+r,     1, h-2*r, color);
    for i in 0..r {
        let t = i as f32 / r as f32;
        let dx = (r as f32 * (1.0 - (1.0 - t*t).sqrt())) as i32;
        fill_rect(buffer, win_w, win_h, x+dx, y+i, 1, 1, color);
        fill_rect(buffer, win_w, win_h, x+w-1-dx, y+i, 1, 1, color);
        fill_rect(buffer, win_w, win_h, x+dx, y+h-1-i, 1, 1, color);
        fill_rect(buffer, win_w, win_h, x+w-1-dx, y+h-1-i, 1, 1, color);
    }
}

fn fill_linear_gradient(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    win_w: u32, win_h: u32,
    x: i32, y: i32, w: i32, h: i32,
    c1: u32, c2: u32,
) {
    if w <= 0 || h <= 0 { return; }
    let r1 = ((c1 >> 16) & 0xFF) as f32;
    let g1 = ((c1 >>  8) & 0xFF) as f32;
    let b1 = ( c1        & 0xFF) as f32;
    let r2 = ((c2 >> 16) & 0xFF) as f32;
    let g2 = ((c2 >>  8) & 0xFF) as f32;
    let b2 = ( c2        & 0xFF) as f32;

    for row in 0..h {
        let t = row as f32 / (h - 1).max(1) as f32;
        let r = (r1 + (r2 - r1) * t) as u32;
        let g = (g1 + (g2 - g1) * t) as u32;
        let b = (b1 + (b2 - b1) * t) as u32;
        let color = (r << 16) | (g << 8) | b;
        fill_rect(buffer, win_w, win_h, x, y + row, w, 1, color);
    }
}

fn apply_opacity(color: u32, opacity: f32) -> u32 {
    if opacity >= 1.0 { return color; }
    let r = ((color >> 16) & 0xFF) as f32;
    let g = ((color >>  8) & 0xFF) as f32;
    let b = ( color        & 0xFF) as f32;
    let r_new = (r * opacity + 255.0 * (1.0 - opacity)) as u32;
    let g_new = (g * opacity + 255.0 * (1.0 - opacity)) as u32;
    let b_new = (b * opacity + 255.0 * (1.0 - opacity)) as u32;
    (r_new << 16) | (g_new << 8) | b_new
}

fn darken(color: u32, amount: u32) -> u32 {
    let r = ((color >> 16) & 0xFF).saturating_sub(amount);
    let g = ((color >>  8) & 0xFF).saturating_sub(amount);
    let b = ( color        & 0xFF).saturating_sub(amount);
    (r << 16) | (g << 8) | b
}

fn blit_image(
    buffer: &mut Buffer<'_, Arc<Window>, Arc<Window>>,
    win_w: u32, win_h: u32,
    dst_x: i32, dst_y: i32,
    dst_w: i32, dst_h: i32,
    img: &DecodedImage,
) {
    if dst_w <= 0 || dst_h <= 0 { return; }
    let src_w = img.width as i32;
    let src_h = img.height as i32;
    if src_w == 0 || src_h == 0 { return; }

    for dy in 0..dst_h {
        let sy = (dy * src_h / dst_h).clamp(0, src_h - 1) as u32;
        let py = dst_y + dy;
        if py < 0 || py >= win_h as i32 { continue; }
        for dx in 0..dst_w {
            let sx = (dx * src_w / dst_w).clamp(0, src_w - 1) as u32;
            let px = dst_x + dx;
            if px < 0 || px >= win_w as i32 { continue; }
            let idx = (py as u32 * win_w + px as u32) as usize;
            if idx >= buffer.len() { continue; }
            let src_idx = (sy * img.width + sx) as usize;
            if src_idx >= img.pixels.len() { continue; }
            buffer[idx] = img.pixels[src_idx];
        }
    }
}