// ─── Taffy Layout Bridge  v2.1 ───────────────────────────────────────────────
//
// Komplettes Rewrite: Rekursives Taffy-Layout mit korrektem Flexbox/Grid.
// Wird von layout.rs aufgerufen wenn display:flex oder display:grid erkannt.
//
// v2.1:
//   - CSS Grid Template Columns/Rows Unterstützung
//   - Grid Auto Columns/Rows und Grid Placement (Start/End)
//   - Aspect-Ratio Unterstützung
//
// v2.0 Fixes:
//   - Rekursive Taffy-Subtrees (verschachtelte Flex/Grid Container nutzen Taffy)
//   - min_size für Auto-Items (verhindert 0px bei fehlender measure-Funktion)
//   - flex_shrink Default korrekt (1.0, nicht 0.0)
//   - flex Shorthand korrekt gemäß CSS-Spec (flex: 1 → 1 1 0)
//   - flex-wrap aus CSS gelesen (nicht mehr hardcoded Wrap)
//   - align-self Unterstützung
//   - order wird von Taffy nativ gehandhabt (kein manueller Sort mehr)
//   - content_h wird korrekt an Taffy durchgereicht
//   - Doppel-Layout für finale LayoutBox eliminiert (kein layout_node_pub mehr)

use taffy::prelude::*;
use crate::dom::{Node, NodeType, ElementData};
use crate::cssom::{Stylesheet, DisplayValue, FlexWrap as CssFlexWrap, FlexDirection as CssFlexDir, JustifyContent as CssJC, AlignItems as CssAI};
use crate::style::{compute_style_with_ancestors, ComputedStyle};
use crate::layout::{LayoutBox, get_display, ElemDisplay, layout_block_children};
use crate::text_measure::get_text_measurer;

// ─── Öffentliche API ─────────────────────────────────────────────────────────

/// Layoutet einen Flex- oder Grid-Container mit Taffy.
/// Gibt fertig positionierte LayoutBox-Kinder zurück.
/// Unterstützt rekursiv verschachtelte Flex/Grid Container.
pub fn layout_flex_with_taffy(
    node:       &Node,
    sheet:      &Stylesheet,
    content_x:  f32,
    content_y:  f32,
    content_w:  f32,
    content_h:  f32,
    style:      &ComputedStyle,
    ancestors:  &[&ElementData],
) -> Vec<LayoutBox> {
    // In layout_taffy.rs, layout_flex_with_taffy ganz oben:
    println!("[TAFFY] called for {} children, content_w={}, node={:?}",
             node.children.len(), content_w,
             match &node.node_type {
                 NodeType::Element(e) => format!("<{} id={:?} class={:?}>",
                                                 e.tag_name, e.id(), e.class()),
                 _ => "text".into()
             }
    );
    let mut taffy = TaffyTree::<()>::new();

    // Sichtbare Kinder sammeln
    let visible_children: Vec<&Node> = node.children.iter()
        .filter(|c| is_visible_node(c, sheet, Some(style)))
        .collect();

    if visible_children.is_empty() {
        return vec![];
    }

    // ── Font-Größe für Text-Kinder bestimmen ────────────────────────────────
    let parent_font_size = style.font_size.unwrap_or(16.0);

    // ── Rekursiv Taffy-Nodes für alle Kinder erstellen ─────────────────────
    let child_ids: Vec<NodeId> = visible_children.iter()
        .map(|child| build_taffy_node(&mut taffy, child, sheet, content_w, parent_font_size, style, ancestors))
        .collect();

    // ── Container-Style erstellen ───────────────────────────────────────────
    let col_gap = style.column_gap.unwrap_or(style.gap);
    let row_gap = style.row_gap.unwrap_or(style.gap);
    let is_grid = style.display == Some(DisplayValue::Grid);
    let has_explicit_h = content_h > 0.0;

    let container_style = if is_grid {
        Style {
            display: Display::Grid,
            size: Size {
                width:  dim_length(content_w),
                height: if has_explicit_h { dim_length(content_h) } else { Dimension::Auto },
            },
            grid_template_columns: map_grid_template(&style.grid_template_columns).into_iter().map(Into::into).collect(),
            grid_template_rows:    map_grid_template(&style.grid_template_rows).into_iter().map(Into::into).collect(),
            grid_auto_columns:     map_grid_template(&style.grid_auto_columns),
            grid_auto_rows:        map_grid_template(&style.grid_auto_rows),
            gap: Size {
                width:  lp_length(col_gap),
                height: lp_length(row_gap),
            },
            padding: Rect::zero(),
            ..Default::default()
        }
    } else {
        Style {
            display: Display::Flex,
            flex_direction: map_flex_dir(&style.flex_direction),
            justify_content: Some(map_justify(&style.justify_content)),
            align_items: Some(map_align(&style.align_items)),
            flex_wrap: map_flex_wrap(&style.flex_wrap),
            size: Size {
                width:  dim_length(content_w),
                height: if has_explicit_h { dim_length(content_h) } else { Dimension::Auto },
            },
            gap: Size {
                width:  lp_length(col_gap),
                height: lp_length(row_gap),
            },
            padding: Rect::zero(),
            ..Default::default()
        }
    };

    let container_id = taffy
        .new_with_children(container_style, &child_ids)
        .expect("taffy container");

    // ── Layout berechnen ────────────────────────────────────────────────────
    taffy.compute_layout(
        container_id,
        Size {
            width:  AvailableSpace::Definite(content_w),
            height: if has_explicit_h {
                AvailableSpace::Definite(content_h)
            } else {
                AvailableSpace::MaxContent
            },
        },
    ).expect("taffy compute");

    // ── Ergebnisse extrahieren ──────────────────────────────────────────────
    extract_child_layouts(
        &taffy,
        &child_ids,
        &visible_children,
        sheet,
        content_x,
        content_y,
        style,
        ancestors,
    )
}

// ─── Rekursiver Taffy-Tree Aufbau ──────────────────────────────────────────

/// Erstellt rekursiv einen Taffy-Node für einen DOM-Knoten.
/// - Flex/Grid Container → Taffy Container mit rekursiven Kindern
/// - Block Container → Taffy Leaf mit Größenschätzung
/// - Text/Void Elemente → Taffy Leaf mit Größenschätzung
fn build_taffy_node(
    taffy: &mut TaffyTree<()>,
    node: &Node,
    sheet: &Stylesheet,
    parent_w: f32,
    parent_font_size: f32,
    parent_style: &ComputedStyle, // NEU: Eltern-ComputedStyle
    ancestors: &[&ElementData],
) -> NodeId {
    match &node.node_type {
        NodeType::Text(text) => {
            let trimmed = text.trim();
            let (w, h) = if trimmed.is_empty() {
                (0.0, 0.0)
            } else {
                let measurer = get_text_measurer();
                let est_w = measurer.measure_text_width(trimmed, parent_font_size).min(parent_w);
                let line_h = measurer.measure_text_height(parent_font_size);
                let lines = if est_w > parent_w && parent_w > 0.0 {
                    (measurer.measure_text_width(trimmed, parent_font_size) / parent_w).ceil() as u32
                } else { 1 };
                let est_h = line_h * lines as f32;
                (est_w, est_h)
            };
            let taffy_style = Style {
                size: Size {
                    width:  Dimension::Length(w),
                    height: Dimension::Length(h),
                },
                min_size: Size {
                    width:  Dimension::Length(w),
                    height: Dimension::Length(h),
                },
                ..Default::default()
            };
            taffy.new_leaf(taffy_style).expect("taffy text leaf")
        }

        NodeType::Element(elem) => {
            let tag = elem.tag_name.as_str();
            let child_style = compute_style_with_ancestors(node, sheet, Some(parent_style), ancestors);
            let disp = get_display(tag, &child_style);

            // Void-Tags (selbstschließend) → einfaches Leaf
            if is_void_tag(tag) {
                let attr_w = elem.attr("width").and_then(|v| v.parse::<f32>().ok());
                let attr_h = elem.attr("height").and_then(|v| v.parse::<f32>().ok());

                let w = child_style.width
                    .or(child_style.width_percent.map(|p| p * parent_w))
                    .or(attr_w)
                    .unwrap_or_else(|| {
                        if tag == "hr" { parent_w } else if tag == "img" { 100.0 } else { 120.0 }
                    });

                let h = child_style.height.or(attr_h).unwrap_or_else(|| {
                    if tag == "img" { w * 0.75 } // Fallback Aspect Ratio
                    else { default_height(tag, child_style.font_size.unwrap_or(16.0)) }
                });

                let taffy_style = build_leaf_style(&child_style, w, h);
                return taffy.new_leaf(taffy_style).expect("taffy void leaf");
            }

            // Flex/Grid Container → rekursiver Taffy-Subtree
            if disp == ElemDisplay::Flex || disp == ElemDisplay::Grid {
                let mut child_ancestors = vec![elem];
                child_ancestors.extend_from_slice(ancestors);
                return build_taffy_container(taffy, node, sheet, &child_style, parent_w, &child_ancestors);
            }

            // Block Container → Leaf mit flacher Größenschätzung (KEIN rekursiver Layout-Aufruf!)
            // layout_node_pub hier zu rufen würde bei Flex-Nachkommen Taffy erneut triggern
            // und zu exponentiellem Kaskadeneffekt führen.
            let font_size = child_style.font_size.unwrap_or(parent_font_size);
            let est_w = child_style.width
                .unwrap_or(parent_w)
                .min(child_style.max_width.unwrap_or(f32::MAX));

            // Höhe: explizit gesetzt > flache Kinder-Summe (1 Level, kein rekursiver Layout)
            let est_h = child_style.height.unwrap_or_else(|| {
                let kids_h: f32 = node.children.iter()
                    .filter(|c| is_visible_node(c, sheet, Some(&child_style)))
                    .map(|c| match &c.node_type {
                        NodeType::Text(t) => {
                            if t.trim().is_empty() { 0.0 }
                            else { get_text_measurer().measure_text_height(font_size) }
                        }
                        NodeType::Element(e) => {
                            let cs = compute_style_with_ancestors(c, sheet, Some(&child_style), ancestors);
                            let h = cs.height.unwrap_or_else(|| {
                                default_height(e.tag_name.as_str(), cs.font_size.unwrap_or(font_size))
                            });
                            h + cs.margin_top + cs.margin_bottom
                        }
                    })
                    .sum();
                kids_h.max(default_height(tag, font_size))
            });

            let taffy_style = build_leaf_style(&child_style, est_w, est_h);
            taffy.new_leaf(taffy_style).expect("taffy block leaf")
        }
    }
}

/// Erstellt einen Taffy-Container-Node mit rekursiven Kindern.
fn build_taffy_container(
    taffy: &mut TaffyTree<()>,
    node: &Node,
    sheet: &Stylesheet,
    child_style: &ComputedStyle,
    parent_w: f32,
    ancestors: &[&ElementData],
) -> NodeId {
    let visible_children: Vec<&Node> = node.children.iter()
        .filter(|c| is_visible_node(c, sheet, Some(child_style)))
        .collect();

    let child_font_size = child_style.font_size.unwrap_or(16.0);

    let effective_w = child_style.width.unwrap_or(parent_w)
        .min(child_style.max_width.unwrap_or(f32::MAX));

    let child_ids: Vec<NodeId> = visible_children.iter()
        .map(|c| build_taffy_node(taffy, c, sheet, effective_w, child_font_size, child_style, ancestors))
        .collect();

    let col_gap = child_style.column_gap.unwrap_or(child_style.gap);
    let row_gap = child_style.row_gap.unwrap_or(child_style.gap);
    let is_grid = child_style.display == Some(DisplayValue::Grid);

    let container_style = if is_grid {
        Style {
            display: Display::Grid,
            size: Size {
                width:  get_dim(child_style.width, child_style.width_percent),
                height: get_dim(child_style.height, child_style.height_percent),
            },
            grid_template_columns: map_grid_template(&child_style.grid_template_columns).into_iter().map(Into::into).collect(),
            grid_template_rows:    map_grid_template(&child_style.grid_template_rows).into_iter().map(Into::into).collect(),
            grid_auto_columns:     map_grid_template(&child_style.grid_auto_columns),
            grid_auto_rows:        map_grid_template(&child_style.grid_auto_rows),
            gap: Size {
                width:  lp_length(col_gap),
                height: lp_length(row_gap),
            },
            margin:  build_margin_rect(child_style),
            padding: Rect::zero(),  // Padding bereits in content_x/content_y verrechnet
            ..Default::default()
        }
    } else {
        Style {
            display: Display::Flex,
            flex_direction:  map_flex_dir(&child_style.flex_direction),
            justify_content: Some(map_justify(&child_style.justify_content)),
            align_items:     Some(map_align(&child_style.align_items)),
            flex_wrap:       map_flex_wrap(&child_style.flex_wrap),
            size: Size {
                width:  get_dim(child_style.width, child_style.width_percent),
                height: get_dim(child_style.height, child_style.height_percent),
            },
            gap: Size {
                width:  lp_length(col_gap),
                height: lp_length(row_gap),
            },
            margin:  build_margin_rect(child_style),
            padding: build_padding_rect(child_style),
            ..Default::default()
        }
    };

    if child_ids.is_empty() {
        taffy.new_leaf(container_style).expect("taffy empty container")
    } else {
        taffy.new_with_children(container_style, &child_ids).expect("taffy container")
    }
}

// ─── LayoutBox-Extraktion ──────────────────────────────────────────────────

/// Extrahiert LayoutBox-Kinder aus dem berechneten Taffy-Tree.
fn extract_child_layouts(
    taffy: &TaffyTree<()>,
    child_ids: &[NodeId],
    dom_children: &[&Node],
    sheet: &Stylesheet,
    parent_content_x: f32,
    parent_content_y: f32,
    parent_computed_style: &ComputedStyle, // NEU
    ancestors: &[&ElementData],
) -> Vec<LayoutBox> {
    child_ids.iter().zip(dom_children.iter())
        .map(|(id, node)| {
            let taffy_layout = taffy.layout(*id).expect("taffy layout");
            let tx = taffy_layout.location.x;
            let ty = taffy_layout.location.y;
            let tw = taffy_layout.size.width;
            let th = taffy_layout.size.height;

            extract_layout_box(taffy, *id, node, sheet,
                               parent_content_x + tx, parent_content_y + ty,
                               tw, th, Some(parent_computed_style), ancestors)
        })
        .collect()
}

/// Erstellt eine LayoutBox aus einem Taffy-Node + DOM-Node.
/// Behandelt rekursiv ob der Node ein Taffy-Container oder Leaf ist.
fn extract_layout_box(
    taffy: &TaffyTree<()>,
    node_id: NodeId,
    dom_node: &Node,
    sheet: &Stylesheet,
    abs_x: f32,
    abs_y: f32,
    abs_w: f32,
    abs_h: f32,
    parent_computed_style: Option<&ComputedStyle>, // NEU
    ancestors: &[&ElementData],
) -> LayoutBox {
    let style = compute_style_with_ancestors(dom_node, sheet, parent_computed_style, ancestors);
    let (tag_name, label, attributes) = extract_node_info(dom_node);

    // Prüfe ob dieser Node ein Taffy-Container ist (hat Taffy-Kinder)
    let taffy_children: Vec<NodeId> = taffy.children(node_id)
        .unwrap_or_default()
        .to_vec();
    let is_taffy_container = !taffy_children.is_empty();

    // Padding-Offset: Taffy's location/size beziehen sich auf den Border-Box
    // content_x = abs_x + padding_left, content_y = abs_y + padding_top
    let content_x = abs_x + style.padding_left;
    let content_y = abs_y + style.padding_top;
    let content_w = (abs_w - style.padding_left - style.padding_right).max(0.0);
    let _content_h = (abs_h - style.padding_top - style.padding_bottom).max(0.0);

    // Kinder extrahieren
    let children = if is_taffy_container {
        // Kinder sind im Taffy-Tree → rekursiv extrahieren
        let visible_dom: Vec<&Node> = dom_node.children.iter()
            .filter(|c| is_visible_node(c, sheet, Some(&style)))
            .collect();

        let mut child_ancestors = vec![];
        if let NodeType::Element(e) = &dom_node.node_type { child_ancestors.push(e); }
        child_ancestors.extend_from_slice(ancestors);

        taffy_children.iter().zip(visible_dom.iter())
            .map(|(cid, dn)| {
                let cl = taffy.layout(*cid).expect("taffy child layout");
                extract_layout_box(
                    taffy,
                    *cid,
                    dn,
                    sheet,
                    // Kinder-Position: Taffy berücksichtigt padding intern -> nicht nochmal addieren
                    abs_x + cl.location.x,
                    abs_y + cl.location.y,
                    cl.size.width,
                    cl.size.height,
                    Some(&style),
                    &child_ancestors,
                )
            })
            .collect()
    } else {
        // Leaf im Taffy-Tree → Block-Layout für DOM-Kinder verwenden
        extract_block_children(dom_node, sheet, content_x, content_y, content_w, &style, ancestors)
    };

    // Text für Text-Knoten
    let text = match &dom_node.node_type {
        NodeType::Text(t) => Some(t.clone()),
        _ => None,
    };

    // Box-Type bestimmen
    let tag = tag_name.as_str();
    let is_inline = is_inline_tag(tag);
    let box_type = if is_inline {
        crate::layout::BoxType::Inline
    } else {
        crate::layout::BoxType::Block
    };

    LayoutBox {
        x: abs_x,
        y: abs_y,
        width: abs_w,
        height: abs_h,
        style,
        box_type,
        label,
        tag_name,
        text,
        attributes,
        children,
        node: dom_node.clone(),
    }
}

/// Extrahiert Block-Layout-Kinder für einen Leaf-Node.
fn extract_block_children(
    node: &Node,
    sheet: &Stylesheet,
    content_x: f32,
    content_y: f32,
    content_w: f32,
    parent_computed_style: &ComputedStyle, // NEU
    ancestors: &[&ElementData],
) -> Vec<LayoutBox> {
    match &node.node_type {
        NodeType::Text(_) => vec![],
        NodeType::Element(_) => {
            // Nur layouten wenn das Element tatsächlich Kinder hat
            let has_layoutable_children = node.children.iter().any(|c| {
                match &c.node_type {
                    NodeType::Text(t) => !t.trim().is_empty(),
                    NodeType::Element(e) => {
                        let tag = e.tag_name.as_str();
                        !matches!(tag, "script" | "style" | "head" | "noscript" | "template")
                            && !is_void_tag(tag)
                    }
                }
            });

            if has_layoutable_children {
                let mut child_ancestors = vec![];
                if let NodeType::Element(e) = &node.node_type { child_ancestors.push(e); }
                child_ancestors.extend_from_slice(ancestors);
                layout_block_children(node, sheet, content_x, content_y, content_w, parent_computed_style, &child_ancestors)
            } else {
                vec![]
            }
        }
    }
}

// ─── Style-Hilfsfunktionen ──────────────────────────────────────────────────

/// Erstellt einen Taffy-Style für ein Leaf-Node.
fn build_leaf_style(child_style: &ComputedStyle, est_w: f32, est_h: f32) -> Style {
    let has_explicit_w = child_style.width.is_some();
    let has_explicit_h = child_style.height.is_some();

    Style {
        // Größe: explizit wenn gesetzt, sonst min_size als Hint
        size: Size {
            width:  get_dim(child_style.width, child_style.width_percent),
            height: get_dim(child_style.height, child_style.height_percent),
        },
        // NEU: AspectRatio
        aspect_ratio: child_style.aspect_ratio,
        // min_size: Content-Größe als Minimum (CSS-Standard für Flex-Items)
        // Nur setzen wenn KEINE explizite Größe, damit Auto-Items nicht 0px bekommen
        min_size: Size {
            width:  if has_explicit_w { Dimension::Length(0.0) } else { Dimension::Length(est_w.max(1.0)) },
            height: if has_explicit_h { Dimension::Length(0.0) } else { Dimension::Length(est_h.max(1.0)) },
        },
        // max_size: Wenn gesetzt
        max_size: Size {
            width:  if let Some(mw) = child_style.max_width { dim_length(mw) } else { Dimension::Auto },
            height: Dimension::Auto,
        },
        margin: build_margin_rect(child_style),
        padding: build_padding_rect(child_style),
        flex_grow:   child_style.flex_grow,
        flex_shrink: child_style.flex_shrink,
        flex_basis:  child_style.flex_basis
            .map(dim_length)
            .unwrap_or(Dimension::Auto),
        // align_self: Some(wert) überschreibt parent's align_items
        align_self: child_style.align_self.as_ref().map(map_align),

        // NEU: Grid Lines
            grid_column: Line {
                start: map_grid_line(child_style.grid_column_start),
                end:   map_grid_line(child_style.grid_column_end),
            },
            grid_row: Line {
                start: map_grid_line(child_style.grid_row_start),
                end:   map_grid_line(child_style.grid_row_end),
            },

        ..Default::default()
    }
}

/// Erstellt ein Taffy margin Rect aus ComputedStyle.
fn build_margin_rect(style: &ComputedStyle) -> Rect<LengthPercentageAuto> {
    Rect {
        left:   lpa_length(style.margin_left),
        right:  lpa_length(style.margin_right),
        top:    lpa_length(style.margin_top),
        bottom: lpa_length(style.margin_bottom),
    }
}

/// Erstellt ein Taffy padding Rect aus ComputedStyle.
fn build_padding_rect(style: &ComputedStyle) -> Rect<LengthPercentage> {
    Rect {
        left:   lp_length(style.padding_left),
        right:  lp_length(style.padding_right),
        top:    lp_length(style.padding_top),
        bottom: lp_length(style.padding_bottom),
    }
}

// ─── Mapping-Hilfsfunktionen ────────────────────────────────────────────────

fn map_grid_template(v: &[crate::cssom::GridTemplateValue]) -> Vec<NonRepeatedTrackSizingFunction> {
    v.iter().map(|val| {
        match val {
            crate::cssom::GridTemplateValue::Length(l)  => length(*l),
            crate::cssom::GridTemplateValue::Percent(p) => percent(*p),
            crate::cssom::GridTemplateValue::Flex(f)    => fr(*f),
            crate::cssom::GridTemplateValue::Auto       => auto(),
            crate::cssom::GridTemplateValue::MinContent => min_content(),
            crate::cssom::GridTemplateValue::MaxContent => max_content(),
        }
    }).collect()
}

fn map_grid_line(v: Option<i32>) -> GridPlacement {
    match v {
        Some(n) => GridPlacement::from_line_index(n as i16),
        None    => GridPlacement::Auto,
    }
}

fn map_flex_dir(d: &CssFlexDir) -> FlexDirection {
    match d {
        CssFlexDir::Row           => FlexDirection::Row,
        CssFlexDir::RowReverse    => FlexDirection::RowReverse,
        CssFlexDir::Column        => FlexDirection::Column,
        CssFlexDir::ColumnReverse => FlexDirection::ColumnReverse,
    }
}

fn map_justify(j: &CssJC) -> JustifyContent {
    match j {
        CssJC::FlexStart    => JustifyContent::FlexStart,
        CssJC::FlexEnd      => JustifyContent::FlexEnd,
        CssJC::Center       => JustifyContent::Center,
        CssJC::SpaceBetween => JustifyContent::SpaceBetween,
        CssJC::SpaceAround  => JustifyContent::SpaceAround,
        CssJC::SpaceEvenly  => JustifyContent::SpaceEvenly,
    }
}

fn map_align(a: &CssAI) -> AlignItems {
    match a {
        CssAI::FlexStart => AlignItems::FlexStart,
        CssAI::FlexEnd   => AlignItems::FlexEnd,
        CssAI::Center    => AlignItems::Center,
        CssAI::Stretch   => AlignItems::Stretch,
        CssAI::Baseline  => AlignItems::Baseline,
    }
}

fn map_flex_wrap(w: &CssFlexWrap) -> FlexWrap {
    match w {
        CssFlexWrap::NoWrap     => FlexWrap::NoWrap,
        CssFlexWrap::Wrap       => FlexWrap::Wrap,
        CssFlexWrap::WrapReverse => FlexWrap::WrapReverse,
    }
}

// ─── Taffy-Typ-Helfer ────────────────────────────────────────────────────────
// Taffy 0.5 verwendet drei verschiedene Längen-Typen je nach Kontext:
//   Dimension             → size, min_size, max_size, flex_basis
//   LengthPercentageAuto   → margin
//   LengthPercentage       → padding, gap, border

fn get_dim(fixed: Option<f32>, percent: Option<f32>) -> Dimension {
    if let Some(f) = fixed { Dimension::Length(f) }
    else if let Some(p) = percent { Dimension::Percent(p) }
    else { Dimension::Auto }
}

#[inline] fn dim_length(v: f32) -> Dimension {
    Dimension::Length(v)
}

#[inline] fn lpa_length(v: f32) -> LengthPercentageAuto {
    LengthPercentageAuto::Length(v)
}

#[inline] fn lp_length(v: f32) -> LengthPercentage {
    LengthPercentage::Length(v)
}

// ─── Hilfsfunktionen ────────────────────────────────────────────────────────

fn is_visible_node(node: &Node, sheet: &Stylesheet, parent_computed_style: Option<&ComputedStyle>) -> bool {
    match &node.node_type {
        NodeType::Text(t) => !t.trim().is_empty(),
        NodeType::Element(e) => {
            let tag = e.tag_name.as_str();
            if matches!(tag, "script" | "style" | "head" | "noscript" | "template" | "link" | "meta") {
                return false;
            }
            // display: none aus CSS prüfen
            let style = compute_style_with_ancestors(node, sheet, parent_computed_style, &[]);
            if style.is_hidden() {
                return false;
            }
            if tag == "input" {
                let itype = e.attr("type").unwrap_or("text").trim().to_lowercase();
                if matches!(itype.as_str(), "hidden" | "reset" | "image" | "checkbox" | "radio") {
                    return false;
                }
            }
            true
        }
    }
}

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
        "track" | "wbr"
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
        _                          => 20.0,
    }
}

fn extract_node_info(node: &Node) -> (String, String, std::collections::HashMap<String, String>) {
    match &node.node_type {
        NodeType::Text(text) => {
            let trimmed = text.trim();
            let label = if trimmed.is_empty() {
                String::new()
            } else {
                let display: String = trimmed.chars().take(60).collect();
                format!("\"{}{}\"", display, if trimmed.len() > 60 { "…" } else { "" })
            };
            (String::new(), label, std::collections::HashMap::new())
        }
        NodeType::Element(e) => {
            let tag = e.tag_name.clone();
            let id   = e.id().map(|v| format!("#{}", v)).unwrap_or_default();
            let cls  = e.class().map(|v| format!(".{}", v)).unwrap_or_default();
            let href = e.attr("href").map(|v| format!(" href=\"{}\"", v)).unwrap_or_default();
            let ph   = e.attr("placeholder").map(|v| format!(" placeholder=\"{}\"", v)).unwrap_or_default();
            let itype = e.attr("type").map(|v| format!(" type=\"{}\"", v)).unwrap_or_default();
            let value = e.attr("value").map(|v| format!(" value=\"{}\"", v)).unwrap_or_default();
            let label = format!("<{}{}{}{}{}{}{}>", tag, id, cls, href, ph, itype, value);
            let attrs: std::collections::HashMap<String, String> = e.attributes.iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect();
            (tag, label, attrs)
        }
    }
}