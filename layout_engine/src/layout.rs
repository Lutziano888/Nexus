use crate::dom::{Node, NodeType, ElementData};
use crate::cssom::{Stylesheet, TextAlignValue, DisplayValue, PositionValue, JustifyContent};
use crate::style::{compute_style_with_ancestors, ComputedStyle};
use crate::layout_taffy::layout_flex_with_taffy;
use crate::text_measure::get_text_measurer;

// ─── Datenstrukturen ──────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct LayoutBox {
    pub x: f32,
    pub y: f32,
    pub width: f32,
    pub height: f32,
    pub style: ComputedStyle,
    pub box_type: BoxType,
    pub label: String,
    pub children: Vec<LayoutBox>,
    // NEU: Node-Daten für den Painter
    pub tag_name:   String,
    pub text:       Option<String>,
    pub attributes: std::collections::HashMap<String, String>,
    pub node: Node,
}

#[derive(Debug, Clone, PartialEq)]
pub enum BoxType {
    Block,
    Inline,
    Anonymous,
}

// ─── Öffentliche API ──────────────────────────────────────────────────────────

pub fn build_layout_tree(
    node:       &Node,
    stylesheet: &Stylesheet,
    origin_x:   f32,
    origin_y:   f32,
    viewport_w: f32,
) -> LayoutBox {
    build_layout_tree_with_viewport(node, stylesheet, origin_x, origin_y, viewport_w, 900.0)
}

pub fn build_layout_tree_with_viewport(
    node:       &Node,
    stylesheet: &Stylesheet,
    origin_x:   f32,
    origin_y:   f32,
    viewport_w: f32,
    viewport_h: f32,
) -> LayoutBox {
    let mut root = layout_node(node, stylesheet, origin_x, origin_y, viewport_w, None, &[]);
    reposition_fixed(&mut root, origin_x, origin_y, viewport_w, viewport_h);
    root
}

// ─── Tag-Klassifikation ───────────────────────────────────────────────────────

fn is_inline_tag(tag: &str) -> bool {
    matches!(tag,
        "a" | "span" | "em" | "strong" | "b" | "i" | "u" | "small" |
        "sub" | "sup" | "abbr" | "cite" | "code" | "kbd" | "s" |
        "label" | "time" | "mark" | "q" | "bdi" | "bdo"
    )
}

fn is_void_tag(tag: &str) -> bool {
    matches!(tag,
        "br" | "hr" | "img" | "input" | "meta" | "link" |
        "area" | "base" | "col" | "embed" | "param" | "source" |
        "track" | "wbr" | "path" | "svg" | "image"
    )
}

fn default_height(tag: &str, font_size: f32) -> f32 {
    let lh = font_size * 1.4;
    match tag {
        "h1"                       => font_size * 2.0 + 16.0,
        "h2"                       => font_size * 1.5 + 12.0,
        "h3" | "h4" | "h5" | "h6" => font_size * 1.25 + 8.0,
        "p" | "li"                 => lh,
        "a" | "span"               => lh,
        "td" | "th"                => lh + 12.0,
        "tr"                       => lh + 12.0,
        "input"                    => 36.0,
        "button"                   => 36.0,
        "select" | "textarea"      => 36.0,
        _                          => 0.0,
    }
}

/// Ermittelt ob ein Element inline-level ist (auch durch display-Style)
pub fn get_display(tag: &str, style: &ComputedStyle) -> ElemDisplay {
    if let Some(d) = &style.display {
        match d {
            DisplayValue::None        => return ElemDisplay::None,
            DisplayValue::Flex        => return ElemDisplay::Flex,
            DisplayValue::Grid        => return ElemDisplay::Grid,
            DisplayValue::Inline      => return ElemDisplay::Inline,
            DisplayValue::InlineBlock => return ElemDisplay::InlineBlock,
            DisplayValue::Block       => return ElemDisplay::Block,
            // FIX D: DisplayValue::Other deckt display:list-item ab.
            // Für <li> ist das normalerweise Block — aber wir lassen den
            // Tag-Default unten entscheiden statt hier Block zu erzwingen.
            DisplayValue::Other       => {}
        }
    }
    // Browser-Defaults
    if tag == "input" || tag == "button" || tag == "select" || tag == "textarea" {
        return ElemDisplay::InlineBlock;
    }
    // FIX D: <li> ist standardmäßig Block (list-item), aber wenn das CSS
    // explizit display:inline setzt kommt das oben bereits als Inline zurück.
    // Hier kein Sonderfall nötig — is_inline_tag entscheidet den Rest.
    if is_inline_tag(tag) {
        ElemDisplay::Inline
    } else {
        ElemDisplay::Block
    }
}

#[derive(PartialEq, Clone, Copy)]
pub enum ElemDisplay { Block, Inline, InlineBlock, Flex, Grid, None }

// ─── Interne Implementierung ──────────────────────────────────────────────────

/// Öffentliche Wrapper-Funktion für layout_taffy.rs
pub fn layout_node_pub(
    node:     &Node,
    sheet:    &Stylesheet,
    parent_x: f32,
    parent_y: f32,
    parent_w: f32,
    parent_computed_style: Option<&ComputedStyle>,
) -> LayoutBox {
    layout_node(node, sheet, parent_x, parent_y, parent_w, parent_computed_style, &[])
}

fn layout_node(
    node:     &Node,
    sheet:    &Stylesheet,
    parent_x: f32,
    parent_y: f32,
    parent_w: f32,
    parent_computed_style: Option<&ComputedStyle>,
    ancestors: &[&ElementData],
) -> LayoutBox {
    match &node.node_type {
        NodeType::Text(text) => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return empty_box(parent_x, parent_y, BoxType::Anonymous, String::new());
            }

            // font_size aus parent-Style erben (h1 → 32px, p → 16px, small → 13px …)
            let font_px = parent_computed_style
                .and_then(|s| s.font_size)
                .unwrap_or(16.0)
                .clamp(8.0, 72.0);

            let measurer   = get_text_measurer();
            let raw_w      = measurer.measure_text_width(trimmed, font_px);
            // Breite des Text-Nodes = echter gemessener Wert, gecappt auf parent_w
            let estimated_w = raw_w.min(parent_w.max(1.0));
            let line_h      = measurer.measure_text_height(font_px);

            // Zeilenzahl schätzen: wie oft passt raw_w in parent_w?
            let lines = if raw_w > parent_w && parent_w > 1.0 {
                (raw_w / parent_w).ceil() as u32
            } else {
                1
            };
            let estimated_h = line_h * lines as f32;

            // Style vom Elternteil erben damit der Painter font_size / color kennt
            let inherited_style = parent_computed_style.cloned().unwrap_or_default();

            LayoutBox {
                x: parent_x, y: parent_y,
                width: estimated_w, height: estimated_h,
                style:    inherited_style,
                box_type: BoxType::Anonymous,
                label:    format!("\"{}\"", truncate(trimmed, 60)),
                tag_name: String::new(),
                text:     Some(text.clone()),
                attributes: std::collections::HashMap::new(),
                children: vec![],
                node: node.clone(),
            }
        }

        NodeType::Element(elem) => {
            let tag   = elem.tag_name.as_str();
            let style = compute_style_with_ancestors(node, sheet, parent_computed_style, ancestors);
            let label = format_label(&elem.tag_name, node);

            let disp = get_display(tag, &style);
            if style.is_hidden() {
                return empty_box(parent_x, parent_y, BoxType::Anonymous, label);
            }

            if matches!(tag, "noscript" | "script" | "style") {
                return empty_box(parent_x, parent_y, BoxType::Anonymous, label);
            }

            // input type=hidden etc. nicht rendern
            if tag == "input" {
                let itype = elem.attr("type").unwrap_or("text").trim().to_lowercase();
                if matches!(itype.as_str(),
                    "hidden" | "reset" | "button" | "image" | "checkbox" | "radio"
                ) {
                    return empty_box(parent_x, parent_y, BoxType::Anonymous, label);
                }
            }

            // Void-Tags (self-closing)
            if is_void_tag(tag) {
                let h = style.height.unwrap_or_else(|| {
                    if tag == "img" {
                        let attr_h = elem.attr("height").and_then(|v| v.parse::<f32>().ok());
                        let fallback_w = style.width
                            .or_else(|| elem.attr("width").and_then(|v| v.parse::<f32>().ok()))
                            .unwrap_or_else(|| parent_w.min(300.0));
                        attr_h.unwrap_or(fallback_w * 0.6)
                    } else {
                        default_height(tag, style.font_size.unwrap_or(16.0))
                    }
                });
                let w = style.width.unwrap_or_else(|| {
                    if tag == "input" || tag == "button" { style.width.unwrap_or(0.0) }
                    else if tag == "hr" { parent_w }
                    else if tag == "img" {
                        elem.attr("width").and_then(|v| v.parse::<f32>().ok())
                            .unwrap_or_else(|| parent_w.min(300.0))
                    }
                    else { 0.0 }
                });
                LayoutBox {
                    x: parent_x + style.margin_left,
                    y: parent_y + style.margin_top,
                    width: w,
                    height: h.max(if tag == "hr" { 1.0 } else { 0.0 }),
                    style, box_type: BoxType::Inline, label,
                    tag_name: elem.tag_name.clone(),
                    text: None,
                    attributes: elem.attributes.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                    children: vec![],
                    node: node.clone(),
                }
            } else {
                let font_size = style.font_size.unwrap_or(16.0);
                let is_inline = disp == ElemDisplay::Inline;
                let is_inline_block = disp == ElemDisplay::InlineBlock;
                let is_flex = disp == ElemDisplay::Flex;
                let is_grid = disp == ElemDisplay::Grid;

                // ── Breite berechnen ─────────────────────────────────────────
                // %-Werte jetzt gegen den echten parent_w auflösen
                let resolved_w = style.width_percent.map(|p| p * parent_w).or(style.width);
                let explicit_w = resolved_w;
                let raw_w = explicit_w.unwrap_or(parent_w);
                let capped_w = if let Some(mw) = style.max_width { raw_w.min(mw) } else { raw_w };
                let padding_h = style.padding_left + style.padding_right;
                let margin_h  = style.margin_left  + style.margin_right;
                // content_width ist der Platz für Kinder (Padding schon abgezogen)
                let content_width = if style.box_sizing_border {
                    // border-box: width enthält padding → netto = w - padding
                    (capped_w - padding_h).max(0.0)
                } else {
                    (capped_w - margin_h - padding_h).max(0.0)
                };

                // ── Position ─────────────────────────────────────────────────
                let mut box_x = parent_x + style.margin_left;
                let box_y     = parent_y + style.margin_top;

                // margin: auto → zentrieren
                if style.margin_left_auto && style.margin_right_auto {
                    let box_w = content_width + padding_h;
                    let gap = parent_w - box_w - margin_h;
                    if gap > 0.0 { box_x = parent_x + gap / 2.0; }
                }

                let content_x = box_x + style.padding_left;
                let content_y = box_y + style.padding_top;

                // ── Kinder layouten ──────────────────────────────────────────
                // Ancestor-Pfad für Kinder: aktuelles Element vorne anhängen
                let mut child_ancestors: Vec<&ElementData> = Vec::with_capacity(ancestors.len() + 1);
                child_ancestors.push(elem);
                child_ancestors.extend_from_slice(ancestors);

                let (children, taffy_h) = if is_flex || is_grid {
                    let kids = layout_flex_with_taffy(
                        node, sheet, content_x, content_y,
                        content_width,
                        style.height.unwrap_or(0.0),
                        &style,
                        &child_ancestors,
                    );
                    let h = kids.iter()
                        .filter(|c| c.style.position != PositionValue::Absolute
                            && c.style.position != PositionValue::Fixed)
                        .map(|c| c.y - content_y + c.height + c.style.margin_bottom)
                        .fold(0.0_f32, f32::max);
                    (kids, Some(h))
                } else {
                    (layout_block_children(node, sheet, content_x, content_y, content_width, &style, &child_ancestors), None)
                };

                // ── Höhe berechnen ───────────────────────────────────────────
                // Resolve height_percent gegen parent_w (kein parent_h verfügbar, pragmatisch)
                let explicit_h_px = style.height
                    .or_else(|| style.height_percent.map(|p| p * parent_w));

                let content_height = explicit_h_px.unwrap_or_else(|| {
                    // Flex/Grid: Taffy-Höhe direkt verwenden
                    if let Some(th) = taffy_h {
                        return th.max(default_height(tag, font_size));
                    }
                    let bottom = children.iter()
                        .filter(|c| c.style.position != PositionValue::Absolute
                            && c.style.position != PositionValue::Fixed)
                        .map(|c| c.y + c.height + c.style.margin_bottom)
                        .fold(content_y, f32::max);
                    let ch = bottom - content_y;
                    if ch > 0.0 { ch } else { default_height(tag, font_size) }
                });

                // ── Breite für Inline-Elemente anpassen ──────────────────────
                let final_content_w = if explicit_w.is_none() && (is_inline || is_inline_block) {
                    let children_total: f32 = children.iter()
                        .map(|c| c.width + c.style.margin_left + c.style.margin_right)
                        .sum();
                    children_total.min(parent_w)
                } else {
                    content_width
                };

                // container_w = was nach außen gemeldet wird.
                // Wenn explicit_w gesetzt: direkt übernehmen (kein extra padding drauf).
                // border-box: explicit_w schließt padding ein.
                // content-box: explicit_w ist nur content → +padding.
                let container_w = if let Some(ew) = explicit_w {
                    if style.box_sizing_border {
                        ew  // border-box: ew ist already total width
                    } else {
                        ew + padding_h  // content-box: total = content + padding
                    }
                } else {
                    final_content_w + padding_h
                };

                // container_h = was nach außen gemeldet wird.
                // content_height ist entweder style.height (explizit) oder aus Kindern berechnet.
                // Wenn style.height explizit gesetzt: Padding nur bei content-box addieren.
                let container_h = if let Some(explicit_h) = style.height.or_else(|| style.height_percent.map(|p| p * parent_w)) {
                    if style.box_sizing_border {
                        explicit_h  // border-box: schließt padding bereits ein
                    } else {
                        explicit_h + style.padding_top + style.padding_bottom  // content-box
                    }
                } else {
                    content_height + style.padding_top + style.padding_bottom
                };

                let box_type = if is_inline || is_inline_block {
                    BoxType::Inline
                } else {
                    BoxType::Block
                };

                // Absolute/Fixed Kinder positionieren
                let mut final_children = children;
                for child in final_children.iter_mut() {
                    if child.style.position == PositionValue::Absolute
                        || child.style.position == PositionValue::Fixed {
                        if let Some(l) = child.style.left   { child.x = box_x + l; }
                        else if let Some(r) = child.style.right {
                            let ref_w = if is_flex { content_width + padding_h } else { container_w };
                            child.x = box_x + ref_w - child.width - r;
                        }
                        if let Some(t) = child.style.top    { child.y = box_y + t; }
                        else if let Some(b) = child.style.bottom {
                            child.y = box_y + container_h - child.height - b;
                        }
                    }
                }

                LayoutBox {
                    x: box_x, y: box_y,
                    width: container_w, height: container_h,
                    style, box_type, label,
                    tag_name: elem.tag_name.clone(),
                    text: None,
                    attributes: elem.attributes.iter().map(|(k, v)| (k.clone(), v.clone())).collect(),
                    children: final_children,
                    node: node.clone(),
                }
            } // close else
        } // close Element arm
    } // close match
} // close function

#[allow(unused_assignments)]
pub fn layout_block_children(
    node:      &Node,
    sheet:     &Stylesheet,
    content_x: f32,
    content_y: f32,
    content_w: f32,
    parent_computed_style: &ComputedStyle,
    ancestors: &[&ElementData],
) -> Vec<LayoutBox> {
    let mut cursor_y = content_y;
    let mut children: Vec<LayoutBox> = vec![];
    let style = parent_computed_style;

    let mut line_x    = content_x;
    let mut line_h    = 0.0_f32;
    let mut line_items: Vec<LayoutBox> = vec![];

    macro_rules! flush_line {
        () => {
            if !line_items.is_empty() {
                let row_w: f32 = line_items.iter()
                    .map(|b| b.width + b.style.margin_left + b.style.margin_right)
                    .sum();
                let offset = match &style.text_align {
                    TextAlignValue::Center => ((content_w - row_w) / 2.0).max(0.0),
                    TextAlignValue::Right  => (content_w - row_w).max(0.0),
                    TextAlignValue::Left   => 0.0,
                };
                for b in line_items.iter_mut() { b.x += offset; }
                children.append(&mut line_items);
                cursor_y += line_h;
                line_x    = content_x;
                line_h    = 0.0;
            }
        }
    }

    for child in &node.children {
        let (child_disp, child_pos) = match &child.node_type {
            NodeType::Text(t) => {
                if t.trim().is_empty() { continue; }
                (ElemDisplay::Inline, PositionValue::Static)
            }
            NodeType::Element(e) => {
                let cs = compute_style_with_ancestors(child, sheet, Some(parent_computed_style), ancestors);
                let pos = cs.position.clone();
                let d = get_display(&e.tag_name, &cs);
                (d, pos)
            }
        };

        if child_disp == ElemDisplay::None { continue; }

        if child_pos == PositionValue::Absolute || child_pos == PositionValue::Fixed {
            let cb = layout_node(child, sheet, content_x, cursor_y, content_w, Some(parent_computed_style), ancestors);
            children.push(cb);
            continue;
        }

        let is_inline_level = matches!(child_disp, ElemDisplay::Inline | ElemDisplay::InlineBlock);
        let is_text = matches!(child.node_type, NodeType::Text(_));

        if is_inline_level || is_text {
            let cb = layout_node(child, sheet, line_x, cursor_y, content_w, Some(parent_computed_style), ancestors);
            let adv = cb.width + cb.style.margin_left + cb.style.margin_right;

            if !line_items.is_empty() && line_x + cb.width > content_x + content_w + 1.0 {
                flush_line!();
                // FIX B: line_x ist jetzt wieder content_x — neues Layout mit korrekter Position
                let cb2 = layout_node(child, sheet, line_x, cursor_y, content_w, Some(parent_computed_style), ancestors);
                let adv2 = cb2.width + cb2.style.margin_left + cb2.style.margin_right;
                line_h  = line_h.max(cb2.height + cb2.style.margin_top + cb2.style.margin_bottom);
                line_x += adv2;
                line_items.push(cb2);
            } else {
                line_h  = line_h.max(cb.height + cb.style.margin_top + cb.style.margin_bottom);
                line_x += adv;
                line_items.push(cb);
            }
        } else {
            flush_line!();
            // FIX C: Vertikales Margin-Collapsing (max statt Addition)
            let prev_mb = children.last().map(|c: &LayoutBox| c.style.margin_bottom).unwrap_or(0.0);
            let child_mt = match &child.node_type {
                NodeType::Element(_) => {
                    let cs = compute_style_with_ancestors(child, sheet, Some(parent_computed_style), ancestors);
                    cs.margin_top
                }
                _ => 0.0,
            };
            let collapsed = prev_mb.max(child_mt);
            if !children.is_empty() {
                cursor_y += collapsed - prev_mb;
            }
            let cb = layout_node(child, sheet, content_x, cursor_y, content_w, Some(parent_computed_style), ancestors);
            cursor_y = cb.y + cb.height + cb.style.margin_bottom;
            children.push(cb);
        }
    }
    flush_line!();
    children
}

#[allow(dead_code, unused_variables)]
fn layout_flex_children(
    node:      &Node,
    sheet:     &Stylesheet,
    content_x: f32,
    content_y: f32,
    content_w: f32,
    style:     &ComputedStyle,
) -> Vec<LayoutBox> { vec![] }

#[allow(dead_code)]
fn justify_offsets(_jc: &JustifyContent, _free: f32, _count: usize) -> (f32, f32) { (0.0, 0.0) }

fn empty_box(x: f32, y: f32, bt: BoxType, label: String) -> LayoutBox {
    LayoutBox {
        x, y, width: 0.0, height: 0.0,
        style: ComputedStyle::default(),
        box_type: bt, label,
        tag_name: String::new(), text: None, attributes: std::collections::HashMap::new(),
        children: vec![],
        node: Node::text(""),
    }
}

fn truncate(s: &str, max: usize) -> String {
    let s = s.replace('\n', " ");
    if s.len() <= max { s } else { format!("{}…", &s[..max]) }
}

fn format_label(tag: &str, node: &Node) -> String {
    if let NodeType::Element(e) = &node.node_type {
        let id    = e.id().map(|v|    format!("#{}", v)).unwrap_or_default();
        let cls   = e.class().map(|v| format!(".{}", v)).unwrap_or_default();
        let href  = e.attr("href").map(|v|        format!(" href=\"{}\"", v)).unwrap_or_default();
        let ph    = e.attr("placeholder").map(|v| format!(" placeholder=\"{}\"", v)).unwrap_or_default();
        let itype = e.attr("type").map(|v|        format!(" type=\"{}\"", v)).unwrap_or_default();
        let value = e.attr("value").map(|v|       format!(" value=\"{}\"", v)).unwrap_or_default();
        format!("<{}{}{}{}{}{}{}>", tag, id, cls, href, ph, itype, value)
    } else {
        tag.to_string()
    }
}

fn reposition_fixed(node: &mut LayoutBox, vx: f32, vy: f32, vw: f32, vh: f32) {
    for child in node.children.iter_mut() {
        if child.style.position == PositionValue::Fixed {
            if let Some(l) = child.style.left   { child.x = vx + l; }
            else if let Some(r) = child.style.right { child.x = vx + vw - child.width - r; }
            if let Some(t) = child.style.top    { child.y = vy + t; }
            else if let Some(b) = child.style.bottom { child.y = vy + vh - child.height - b; }
        }
        reposition_fixed(child, vx, vy, vw, vh);
    }
}

impl LayoutBox {
    pub fn print(&self, depth: usize) {
        let indent = "  ".repeat(depth);
        let conn   = if depth == 0 { "┌" } else { "├" };
        println!("{}{} {} [{:?}]", indent, conn, self.label, self.box_type);
        println!("{}    x={:.1}  y={:.1}  w={:.1}  h={:.1}",
                 indent, self.x, self.y, self.width, self.height);
        for child in &self.children { child.print(depth + 1); }
    }
}