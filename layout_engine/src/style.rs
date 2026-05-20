use crate::dom::{Node, NodeType};
use crate::cssom::{Stylesheet, Selector, Declaration, DisplayValue, FontWeightValue,
                   TextDecorationValue, TextAlignValue, PositionValue, FlexDirection,
                   JustifyContent, AlignItems, FlexWrap, parse_declarations,
                   VisibilityValue, OverflowValue, WhiteSpaceValue, GridTemplateValue};

/// Berechnete Stilwerte für einen einzelnen Knoten.
#[derive(Debug, Clone)]
pub struct ComputedStyle {
    pub width:             Option<f32>,
    pub height:            Option<f32>,
    /// Prozentwert 0.0–1.0 für width, falls CSS `width: X%` gesetzt hat.
    /// Wird im Layout gegen den tatsächlichen parent_w aufgelöst.
    pub width_percent:     Option<f32>,
    /// Prozentwert 0.0–1.0 für height, falls CSS `height: X%` gesetzt hat.
    pub height_percent:    Option<f32>,
    pub max_width:         Option<f32>,
    pub margin_top:        f32,
    pub margin_right:      f32,
    pub margin_bottom:     f32,
    pub margin_left:       f32,
    pub margin_left_auto:  bool,
    pub margin_right_auto: bool,
    pub padding_top:       f32,
    pub padding_right:     f32,
    pub padding_bottom:    f32,
    pub padding_left:      f32,
    pub color:             Option<u32>,
    pub background_color:  Option<u32>,
    pub font_size:         Option<f32>,
    pub display:           Option<DisplayValue>,
    pub font_weight:       Option<FontWeightValue>,
    pub text_decoration:   Option<TextDecorationValue>,
    pub border_color:      Option<u32>,
    pub border_width:      Option<f32>,
    pub border_radius:     Option<f32>,
    // NEU
    pub text_align:        TextAlignValue,
    pub line_height:       Option<f32>,
    pub letter_spacing:    Option<f32>,
    // Position
    pub position:          PositionValue,
    pub top:               Option<f32>,
    pub left:              Option<f32>,
    pub right:             Option<f32>,
    pub bottom:            Option<f32>,
    // Flexbox
    pub flex_direction:    FlexDirection,
    pub justify_content:   JustifyContent,
    pub align_items:       AlignItems,
    // NEU: align-self
    pub align_self:        Option<AlignItems>,
    pub flex_grow:         f32,
    // FIX: CSS-Standard sagt flex-shrink: 1 ist der Default!
    pub flex_shrink:       f32,
    pub flex_basis:        Option<f32>,
    // NEU: flex-wrap
    pub flex_wrap:         FlexWrap,
    pub gap:               f32,
    pub column_gap:        Option<f32>,
    pub row_gap:           Option<f32>,
    pub order:             i32,

    // NEU: Grid
    pub grid_template_columns: Vec<GridTemplateValue>,
    pub grid_template_rows:    Vec<GridTemplateValue>,
    pub grid_auto_columns:     Vec<GridTemplateValue>,
    pub grid_auto_rows:        Vec<GridTemplateValue>,
    pub grid_column_start:     Option<i32>,
    pub grid_column_end:       Option<i32>,
    pub grid_row_start:        Option<i32>,
    pub grid_row_end:          Option<i32>,

    // NEU: aspect-ratio
    pub aspect_ratio:      Option<f32>,

    pub box_sizing_border: bool, // true = border-box
    // NEU: visibility
    pub visibility:        VisibilityValue,
    // NEU: opacity
    pub opacity:           f32,
    // NEU: overflow
    pub overflow:          OverflowValue,
    // NEU: z-index
    pub z_index:           Option<i32>,
    // NEU: white-space
    pub white_space:       WhiteSpaceValue,
    // NEU: font-family
    pub font_family:       Option<String>,
    // NEU: background-image none
    pub background_image_none: bool,
    // NEU: box-shadow
    pub box_shadow:        Option<(f32, f32, f32, u32)>, // (offset_x, offset_y, blur, color)
    // NEU: border per Seite (None = erbt von border_color/border_width)
    pub border_top_width:    Option<f32>,
    pub border_right_width:  Option<f32>,
    pub border_bottom_width: Option<f32>,
    pub border_left_width:   Option<f32>,
    pub border_top_color:    Option<u32>,
    pub border_right_color:  Option<u32>,
    pub border_bottom_color: Option<u32>,
    pub border_left_color:   Option<u32>,
    // NEU: cursor
    pub cursor_pointer:    bool,
    // NEU: border-radius per Ecke (None = erbt von border_radius)
    pub border_top_left_radius:     Option<f32>,
    pub border_top_right_radius:    Option<f32>,
    pub border_bottom_left_radius:  Option<f32>,
    pub border_bottom_right_radius: Option<f32>,
    // NEU: background-gradient
    pub background_gradient: Option<(u32, u32)>,
}

impl Default for ComputedStyle {
    fn default() -> Self {
        Self {
            width: None,
            height: None,
            width_percent: None,
            height_percent: None,
            max_width: None,
            margin_top: 0.0,
            margin_right: 0.0,
            margin_bottom: 0.0,
            margin_left: 0.0,
            margin_left_auto: false,
            margin_right_auto: false,
            padding_top: 0.0,
            padding_right: 0.0,
            padding_bottom: 0.0,
            padding_left: 0.0,
            color: None,
            background_color: None,
            font_size: None,
            display: None,
            font_weight: None,
            text_decoration: None,
            border_color: None,
            border_width: None,
            border_radius: None,
            text_align: TextAlignValue::default(),
            line_height: None,
            letter_spacing: None,
            position: PositionValue::default(),
            top: None,
            left: None,
            right: None,
            bottom: None,
            flex_direction: FlexDirection::default(),
            justify_content: JustifyContent::default(),
            align_items: AlignItems::default(),
            align_self: None,
            flex_grow: 0.0,
            flex_shrink: 1.0, // CSS-Standard Default ist 1
            flex_basis: None,
            flex_wrap: FlexWrap::default(),
            gap: 0.0,
            column_gap: None,
            row_gap: None,
            order: 0,
            grid_template_columns: vec![],
            grid_template_rows: vec![],
            grid_auto_columns: vec![],
            grid_auto_rows: vec![],
            grid_column_start: None,
            grid_column_end: None,
            grid_row_start: None,
            grid_row_end: None,
            aspect_ratio: None,
            box_sizing_border: false,
            visibility: VisibilityValue::default(),
            opacity: 1.0, // WICHTIG: Default muss 1.0 (sichtbar) sein!
            overflow: OverflowValue::default(),
            z_index: None,
            white_space: WhiteSpaceValue::default(),
            font_family: None,
            background_image_none: false,
            box_shadow: None,
            border_top_width: None,
            border_right_width: None,
            border_bottom_width: None,
            border_left_width: None,
            border_top_color: None,
            border_right_color: None,
            border_bottom_color: None,
            border_left_color: None,
            cursor_pointer: false,
            border_top_left_radius: None,
            border_top_right_radius: None,
            border_bottom_left_radius: None,
            border_bottom_right_radius: None,
            background_gradient: None,
        }
    }
}

impl ComputedStyle {
    pub fn horizontal_spacing(&self) -> f32 {
        self.margin_left + self.margin_right + self.padding_left + self.padding_right
    }
    pub fn is_hidden(&self) -> bool {
        self.display == Some(DisplayValue::None) || self.visibility == VisibilityValue::Hidden
    }
    /// Effektive Zeilenhöhe (fallback auf font_size × 1.4 oder 20px)
    pub fn effective_line_height(&self) -> f32 {
        self.line_height.unwrap_or_else(|| {
            self.font_size.map(|fs| fs * 1.4).unwrap_or(20.0)
        })
    }
    // NEU: Platzhalter für Opacity (verhindert den Compiler-Fehler in painter.rs)
    pub fn effective_opacity(&self) -> f32 {
        self.opacity
    }
    /// Prüft ob das Element sichtbar ist (display:none ODER visibility:hidden)
    pub fn is_visible(&self) -> bool {
        !self.is_hidden() && self.visibility == VisibilityValue::Visible
    }

    /// Effektiver border-radius: einheitlich oder 0
    pub fn effective_border_radius(&self) -> f32 {
        self.border_radius.unwrap_or(0.0)
    }

    /// Border-Farbe für eine Seite (fällt auf border_color zurück)
    pub fn border_color_top(&self)    -> Option<u32> { self.border_top_color.or(self.border_color) }
    pub fn border_color_right(&self)  -> Option<u32> { self.border_right_color.or(self.border_color) }
    pub fn border_color_bottom(&self) -> Option<u32> { self.border_bottom_color.or(self.border_color) }
    pub fn border_color_left(&self)   -> Option<u32> { self.border_left_color.or(self.border_color) }

    /// Border-Breite für eine Seite (fällt auf border_width zurück)
    pub fn border_width_top(&self)    -> f32 { self.border_top_width.or(self.border_width).unwrap_or(0.0) }
    pub fn border_width_right(&self)  -> f32 { self.border_right_width.or(self.border_width).unwrap_or(0.0) }
    pub fn border_width_bottom(&self) -> f32 { self.border_bottom_width.or(self.border_width).unwrap_or(0.0) }
    pub fn border_width_left(&self)   -> f32 { self.border_left_width.or(self.border_width).unwrap_or(0.0) }

    /// Border-Radius für jede Ecke (fällt auf border_radius zurück)
    pub fn border_radius_tl(&self) -> f32 { self.border_top_left_radius.or(self.border_radius).unwrap_or(0.0) }
    pub fn border_radius_tr(&self) -> f32 { self.border_top_right_radius.or(self.border_radius).unwrap_or(0.0) }
    pub fn border_radius_bl(&self) -> f32 { self.border_bottom_left_radius.or(self.border_radius).unwrap_or(0.0) }
    pub fn border_radius_br(&self) -> f32 { self.border_bottom_right_radius.or(self.border_radius).unwrap_or(0.0) }
}

/// Wendet erbliche Eigenschaften vom Eltern-Style auf den aktuellen Style an.
fn inherit_properties(style: &mut ComputedStyle, parent_style: Option<&ComputedStyle>) {
    if let Some(parent) = parent_style {
        if style.color.is_none() {
            style.color = parent.color;
        }
        if style.font_size.is_none() {
            style.font_size = parent.font_size;
        }
        if style.font_weight.is_none() {
            style.font_weight = parent.font_weight.clone();
        }
        if style.line_height.is_none() {
            style.line_height = parent.line_height;
        }
        // text-align ist immer gesetzt (Default), aber es ist erblich
        // wir überschreiben nur, wenn der aktuelle Wert der Default ist
        if style.text_align == TextAlignValue::Left { // Default-Wert
            style.text_align = parent.text_align.clone();
        }
        if style.letter_spacing.is_none() {
            style.letter_spacing = parent.letter_spacing;
        }
        // NEU: visibility ist erblich (CSS Standard)
        if style.visibility == VisibilityValue::Visible { // Default-Wert
            style.visibility = parent.visibility.clone();
        }
        // NEU: white-space ist erblich
        if style.white_space == WhiteSpaceValue::Normal { // Default-Wert
            style.white_space = parent.white_space.clone();
        }
        // NEU: font-family ist erblich
        if style.font_family.is_none() {
            style.font_family = parent.font_family.clone();
        }
    }
}

/// Prüft ob ein SimpleSelector auf ein Element passt
fn simple_matches(sel: &crate::cssom::SimpleSelector, e: &crate::dom::ElementData) -> bool {
    use crate::cssom::SimpleSelector;
    match sel {
        SimpleSelector::Universal       => true,
        SimpleSelector::Tag(tag)        => e.tag_name == *tag,
        SimpleSelector::Id(id)          => e.id() == Some(id.as_str()),
        SimpleSelector::Class(cls)      => e.class()
            .map(|c| c.split_whitespace().any(|p| p == cls.as_str()))
            .unwrap_or(false),
        SimpleSelector::Attribute { name, value } => {
            if let Some(attr_val) = e.attributes.iter().find(|(k,_)| k == name).map(|(_,v)| v) {
                match value {
                    None      => true,
                    Some(val) => attr_val == val,
                }
            } else { false }
        }
        SimpleSelector::PseudoClass(_) | SimpleSelector::PseudoElement(_) => false,
    }
}

/// Prüft ob ein Selector auf ein Element passt.
/// ancestors: Liste der Vorfahren von nächstem Elternteil bis zur Wurzel (für Descendant/Child).
fn selector_matches_with_ancestors(
    selector: &Selector,
    node: &Node,
    ancestors: &[&crate::dom::ElementData],
) -> bool {
    match &node.node_type {
        NodeType::Text(_) => false,
        NodeType::Element(e) => match selector {
            Selector::Universal         => true,
            Selector::Tag(tag)          => e.tag_name == *tag,
            Selector::Id(id)            => e.id() == Some(id.as_str()),
            Selector::Class(cls)        => e.class()
                .map(|c| c.split_whitespace().any(|p| p == cls.as_str()))
                .unwrap_or(false),
            Selector::Attribute { name, value } => {
                if let Some(attr_val) = e.attributes.iter().find(|(k,_)| k == name).map(|(_,v)| v) {
                    match value { None => true, Some(val) => attr_val == val }
                } else { false }
            }
            Selector::Compound(parts)   => parts.iter().all(|p| simple_matches(p, e)),
            Selector::List(sels)        =>
                sels.iter().any(|s| selector_matches_with_ancestors(s, node, ancestors)),
            Selector::Child(parent_sel, child_sel) => {
                // Das aktuelle Element muss child_sel matchen
                // UND das direkte Elternteil muss parent_sel matchen
                let child_node = Node { node_type: NodeType::Element(e.clone()), children: vec![] };
                if !selector_matches_with_ancestors(child_sel, &child_node, ancestors) {
                    return false;
                }
                // Direkt-Elternteil = erstes Element in ancestors
                if let Some(parent_elem) = ancestors.first() {
                    let parent_node = Node {
                        node_type: NodeType::Element((*parent_elem).clone()),
                        children: vec![],
                    };
                    selector_matches_with_ancestors(parent_sel, &parent_node, &ancestors[1..])
                } else { false }
            }
            Selector::Descendant(ancestor_sel, child_sel) => {
                // Das aktuelle Element muss child_sel matchen
                let child_node = Node { node_type: NodeType::Element(e.clone()), children: vec![] };
                if !selector_matches_with_ancestors(child_sel, &child_node, ancestors) {
                    return false;
                }
                // Irgendein Vorfahre muss ancestor_sel matchen
                for (i, anc) in ancestors.iter().enumerate() {
                    let anc_node = Node {
                        node_type: NodeType::Element((*anc).clone()),
                        children: vec![],
                    };
                    if selector_matches_with_ancestors(ancestor_sel, &anc_node, &ancestors[i+1..]) {
                        return true;
                    }
                }
                false
            }
        },
    }
}

#[allow(dead_code)]
fn selector_matches(selector: &Selector, node: &Node) -> bool {
    // Ohne Ancestor-Kontext: nur für einfache Selektoren (Compound, List, Tag, Id, Class)
    // Descendant/Child können ohne ancestors nicht korrekt geprüft werden —
    // selector_matches_with_ancestors wird von compute_style aufgerufen.
    selector_matches_with_ancestors(selector, node, &[])
}

fn apply_declarations(style: &mut ComputedStyle, decls: &[Declaration]) {
    for decl in decls {
        match decl {
            Declaration::Width(v)           => { style.width = Some(*v); style.width_percent = None; }
            Declaration::Height(v)          => { style.height = Some(*v); style.height_percent = None; }
            Declaration::WidthPercent(p)    => { style.width_percent = Some(*p); style.width = None; }
            Declaration::HeightPercent(p)   => { style.height_percent = Some(*p); style.height = None; }
            Declaration::MaxWidth(v)        => style.max_width = Some(*v),
            Declaration::MarginTop(v)       => style.margin_top = *v,
            Declaration::MarginBottom(v)    => style.margin_bottom = *v,
            Declaration::MarginLeft(v)      => { style.margin_left = *v; style.margin_left_auto = false; }
            Declaration::MarginRight(v)     => { style.margin_right = *v; style.margin_right_auto = false; }
            Declaration::MarginLeftAuto     => style.margin_left_auto = true,
            Declaration::MarginRightAuto    => style.margin_right_auto = true,
            Declaration::PaddingTop(v)      => style.padding_top = *v,
            Declaration::PaddingRight(v)    => style.padding_right = *v,
            Declaration::PaddingBottom(v)   => style.padding_bottom = *v,
            Declaration::PaddingLeft(v)     => style.padding_left = *v,
            Declaration::Color(c)           => style.color = Some(*c),
            Declaration::BackgroundColor(c) => style.background_color = Some(*c),
            Declaration::FontSize(v)        => style.font_size = Some(*v),
            Declaration::Display(d)         => style.display = Some(d.clone()),
            Declaration::FontWeight(w)      => style.font_weight = Some(w.clone()),
            Declaration::TextDecoration(t)  => style.text_decoration = Some(t.clone()),
            Declaration::BorderColor(c)     => style.border_color = Some(*c),
            Declaration::BorderWidth(v)     => style.border_width = Some(*v),
            Declaration::BorderRadius(v)    => style.border_radius = Some(*v),
            Declaration::TextAlign(ta)      => style.text_align = ta.clone(),
            Declaration::LineHeight(v)      => style.line_height = Some(*v),
            Declaration::LetterSpacing(v)   => style.letter_spacing = Some(*v),
            // Position
            Declaration::Position(p)        => style.position = p.clone(),
            Declaration::Top(v)             => style.top = Some(*v),
            Declaration::Left(v)            => style.left = Some(*v),
            Declaration::Right(v)           => style.right = Some(*v),
            Declaration::Bottom(v)          => style.bottom = Some(*v),
            // Flexbox
            Declaration::FlexDirection(d)   => style.flex_direction = d.clone(),
            Declaration::JustifyContent(j)  => style.justify_content = j.clone(),
            Declaration::AlignItems(a)      => style.align_items = a.clone(),
            // NEU: align-self
            Declaration::AlignSelf(a)       => style.align_self = Some(a.clone()),
            Declaration::FlexGrow(v)        => style.flex_grow = *v,
            Declaration::FlexShrink(v)      => style.flex_shrink = *v,
            Declaration::FlexBasis(v)       => style.flex_basis = Some(*v),
            // NEU: flex-wrap
            Declaration::FlexWrap(w)        => style.flex_wrap = w.clone(),
            Declaration::Gap(v)             => style.gap = *v,
            Declaration::ColumnGap(v)       => style.column_gap = Some(*v),
            Declaration::RowGap(v)          => style.row_gap = Some(*v),
            Declaration::Order(v)           => style.order = *v,
            Declaration::BoxSizingBorder    => style.box_sizing_border = true,
            // NEU: visibility
            Declaration::Visibility(v)      => style.visibility = v.clone(),
            // NEU: opacity
            Declaration::Opacity(o)          => style.opacity = *o,
            // NEU: overflow
            Declaration::Overflow(o)         => style.overflow = o.clone(),
            // NEU: z-index
            Declaration::ZIndex(z)           => style.z_index = Some(*z),
            // NEU: white-space
            Declaration::WhiteSpace(ws)      => style.white_space = ws.clone(),
            // NEU: font-family
            Declaration::FontFamily(f)       => style.font_family = Some(f.clone()),
            // NEU: background-image: none
            Declaration::BackgroundImageNone => style.background_image_none = true,
            // NEU: box-shadow
            Declaration::BoxShadow { offset_x, offset_y, blur, color } =>
                style.box_shadow = Some((*offset_x, *offset_y, *blur, *color)),
            // NEU: border per Seite
            Declaration::BorderTopWidth(v)    => style.border_top_width    = Some(*v),
            Declaration::BorderRightWidth(v)  => style.border_right_width  = Some(*v),
            Declaration::BorderBottomWidth(v) => style.border_bottom_width = Some(*v),
            Declaration::BorderLeftWidth(v)   => style.border_left_width   = Some(*v),
            Declaration::BorderTopColor(c)    => style.border_top_color    = Some(*c),
            Declaration::BorderRightColor(c)  => style.border_right_color  = Some(*c),
            Declaration::BorderBottomColor(c) => style.border_bottom_color = Some(*c),
            Declaration::BorderLeftColor(c)   => style.border_left_color   = Some(*c),
            // NEU: cursor
            Declaration::CursorPointer        => style.cursor_pointer = true,
            // NEU: border-radius per Ecke
            Declaration::BorderTopLeftRadius(v)     => style.border_top_left_radius     = Some(*v),
            Declaration::BorderTopRightRadius(v)    => style.border_top_right_radius    = Some(*v),
            Declaration::BorderBottomLeftRadius(v)  => style.border_bottom_left_radius  = Some(*v),
            Declaration::BorderBottomRightRadius(v) => style.border_bottom_right_radius = Some(*v),
            // Outline (für jetzt: ignoriert, kein Feld nötig)
            Declaration::OutlineWidth(_) | Declaration::OutlineColor(_) => {}

            // NEU: Grid
            Declaration::GridTemplateColumns(v) => style.grid_template_columns = v.clone(),
            Declaration::GridTemplateRows(v)    => style.grid_template_rows = v.clone(),
            Declaration::GridAutoColumns(v)     => style.grid_auto_columns = v.clone(),
            Declaration::GridAutoRows(v)        => style.grid_auto_rows = v.clone(),
            Declaration::GridColumnStart(v)     => style.grid_column_start = Some(*v),
            Declaration::GridColumnEnd(v)       => style.grid_column_end = Some(*v),
            Declaration::GridRowStart(v)        => style.grid_row_start = Some(*v),
            Declaration::GridRowEnd(v)          => style.grid_row_end = Some(*v),

            // NEU: AspectRatio
            Declaration::AspectRatio(v)         => style.aspect_ratio = Some(*v),
            // NEU: Background-Gradient
            Declaration::BackgroundImageGradient { c1, c2 } =>
                style.background_gradient = Some((*c1, *c2)),
        }
    }
}

/// Berechnet den Style für einen Knoten:
/// 0. Browser-Defaults (UA Stylesheet)
/// 1. Stylesheet-Regeln (Tag, Class, Id, Compound, Descendant, Child)
/// 2. Inline-Style-Attribut
///
/// ancestors: Liste der Vorfahren-ElementData von direktem Elternteil bis Wurzel.
/// Wird für Descendant/Child-Selektoren benötigt.
pub fn compute_style(node: &Node, stylesheet: &Stylesheet, parent_style: Option<&ComputedStyle>) -> ComputedStyle {
    compute_style_with_ancestors(node, stylesheet, parent_style, &[])
}

pub fn compute_style_with_ancestors(
    node: &Node,
    stylesheet: &Stylesheet,
    parent_style: Option<&ComputedStyle>,
    ancestors: &[&crate::dom::ElementData],
) -> ComputedStyle {
    let mut style = ComputedStyle::default();

    // 0. Erbliche Eigenschaften vom Eltern-Knoten übernehmen
    inherit_properties(&mut style, parent_style);

    // 1. Browser-Defaults (UA Stylesheet)
    if let NodeType::Element(e) = &node.node_type {
        apply_ua_defaults(&mut style, &e.tag_name);
    }

    // 1.5 Spezialfall Flex-Items: Kinder von Flex-Containern verhalten sich wie Blocks
    if let Some(parent) = parent_style {
        if parent.display == Some(DisplayValue::Flex) {
            if style.display == Some(DisplayValue::Inline) || style.display.is_none() {
                style.display = Some(DisplayValue::Block);
            }
        }
    }

    // 2. Stylesheet-Regeln (externe, interne <style>) nach Spezifität sortiert anwenden
    let mut matching: Vec<&crate::cssom::Rule> = stylesheet.rules.iter()
        .filter(|rule| selector_matches_with_ancestors(&rule.selector, node, ancestors))
        .collect();
    matching.sort_by_key(|r| r.selector.specificity());
    for rule in matching {
        apply_declarations(&mut style, &rule.declarations);
    }

    // 3. Inline-Style-Attribute (style="...") anwenden (höchste Priorität)
    if let NodeType::Element(e) = &node.node_type {
        if let Some(inline) = e.attr("style") {
            let decls = parse_declarations(inline);
            apply_declarations(&mut style, &decls);
        }
    }

    style
}

/// Minimaler UA-Stylesheet: Browser-Defaults für gängige Tags
fn apply_ua_defaults(style: &mut ComputedStyle, tag: &str) {
    match tag {
        "body" => {
            style.margin_top    = 8.0;
            style.margin_right  = 8.0;
            style.margin_bottom = 8.0;
            style.margin_left   = 8.0;
            style.font_size     = Some(16.0);
        }
        "h1" => {
            style.font_size     = Some(32.0);
            style.font_weight   = Some(FontWeightValue::Bold);
            style.margin_top    = 21.0;
            style.margin_bottom = 21.0;
            style.line_height   = Some(40.0);
        }
        "h2" => {
            style.font_size     = Some(24.0);
            style.font_weight   = Some(FontWeightValue::Bold);
            style.margin_top    = 19.0;
            style.margin_bottom = 19.0;
            style.line_height   = Some(32.0);
        }
        "h3" => {
            style.font_size     = Some(18.0);
            style.font_weight   = Some(FontWeightValue::Bold);
            style.margin_top    = 16.0;
            style.margin_bottom = 16.0;
            style.line_height   = Some(26.0);
        }
        "h4" => {
            style.font_size     = Some(16.0);
            style.font_weight   = Some(FontWeightValue::Bold);
            style.margin_top    = 14.0;
            style.margin_bottom = 4.0;
            style.line_height   = Some(22.0);
        }
        "h5" => {
            style.font_size     = Some(14.0);
            style.font_weight   = Some(FontWeightValue::Bold);
            style.margin_top    = 12.0;
            style.margin_bottom = 4.0;
            style.line_height   = Some(20.0);
        }
        "h6" => {
            style.font_size     = Some(13.0);
            style.font_weight   = Some(FontWeightValue::Bold);
            style.margin_top    = 10.0;
            style.margin_bottom = 4.0;
            style.line_height   = Some(18.0);
        }
        "p" => {
            style.margin_top    = 16.0;
            style.margin_bottom = 16.0;
            style.line_height   = Some(24.0);
            style.font_size     = Some(16.0);
        }
        "ul" | "ol" => {
            style.margin_top    = 16.0;
            style.margin_bottom = 16.0;
            style.padding_left  = 40.0;
        }
        "li" => {
            style.margin_bottom = 4.0;
            style.line_height   = Some(22.0);
        }
        "a" => {
            style.color           = Some(0x00_18_58_AB);
            style.text_decoration = Some(TextDecorationValue::Underline);
            style.cursor_pointer  = true;
        }
        "strong" | "b" => {
            style.font_weight = Some(FontWeightValue::Bold);
        }
        "small" => {
            style.font_size = Some(12.0);
        }
        "hr" => {
            style.margin_top    = 8.0;
            style.margin_bottom = 8.0;
            style.border_color  = Some(0x00_DA_DC_E0);
            style.border_width  = Some(1.0);
        }
        "th" => {
            style.font_weight    = Some(FontWeightValue::Bold);
            style.padding_top    = 6.0;
            style.padding_bottom = 6.0;
            style.padding_left   = 8.0;
            style.padding_right  = 8.0;
        }
        "td" => {
            style.padding_top    = 6.0;
            style.padding_bottom = 6.0;
            style.padding_left   = 8.0;
            style.padding_right  = 8.0;
        }
        "button" | "input" | "select" | "textarea" => {
            style.border_radius  = Some(4.0);
            style.border_color   = Some(0x00_C4_C7_C5);
            style.border_width   = Some(1.0);
            // button bekommt mehr padding + cursor
            if tag == "button" {
                style.padding_top    = 8.0;
                style.padding_bottom = 8.0;
                style.padding_left   = 16.0;
                style.padding_right  = 16.0;
                style.cursor_pointer = true;
            } else {
                style.padding_top    = 4.0;
                style.padding_bottom = 4.0;
                style.padding_left   = 8.0;
                style.padding_right  = 8.0;
            }
        }
        "pre" | "code" | "samp" | "kbd" => {
            style.font_size      = Some(13.0);
            style.padding_top    = 8.0;
            style.padding_bottom = 8.0;
            style.padding_left   = 12.0;
            style.padding_right  = 12.0;
        }
        "blockquote" => {
            style.margin_left    = 40.0;
            style.margin_right   = 40.0;
            style.margin_top     = 16.0;
            style.margin_bottom  = 16.0;
            style.padding_left   = 16.0;
        }
        _ => {}
    }
}