/// Das CSS Object Model: eine geordnete Liste von Regeln.
#[derive(Debug, Clone)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

/// Eine einzelne CSS-Regel: Selector + Deklarationen.
#[derive(Debug, Clone)]
pub struct Rule {
    pub selector: Selector,
    pub declarations: Vec<Declaration>,
}

/// Ein einfacher Selector-Baustein (ohne Kombinator)
#[derive(Debug, Clone, PartialEq)]
pub enum SimpleSelector {
    Tag(String),
    Id(String),
    Class(String),
    Attribute { name: String, value: Option<String> },
    PseudoClass(String),
    PseudoElement(String),
    Universal,
}

/// CSS-Spezifität als (a, b, c) Tupel – höher = gewinnt.
/// a = ID-Selektoren
/// b = Klassen, Attribute, Pseudo-Klassen
/// c = Typ-Selektoren, Pseudo-Elemente
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Specificity(pub u32, pub u32, pub u32);

impl std::ops::Add for Specificity {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Specificity(self.0 + other.0, self.1 + other.1, self.2 + other.2)
    }
}

/// Unterstützte Selektoren (erweiterbar).
#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    // Einfache Selektoren
    Id(String),
    Class(String),
    Tag(String),
    Universal,
    Attribute { name: String, value: Option<String> },
    // Compound: mehrere SimpleSelectors auf dasselbe Element — ".foo.bar", "div.active#id"
    Compound(Vec<SimpleSelector>),
    // Kombinator-Selektoren
    Descendant(Box<Selector>, Box<Selector>),  // "div span" — irgendein Vorfahre
    Child(Box<Selector>, Box<Selector>),        // "div > span" — direktes Elternteil
    // Komma-Liste: "h1, h2, h3"
    List(Vec<Selector>),
}

impl Selector {
    /// CSS-Spezifität berechnen – höher = gewinnt
    pub fn specificity(&self) -> Specificity {
        match self {
            Selector::Id(_)             => Specificity(1, 0, 0),
            Selector::Class(_)          => Specificity(0, 1, 0),
            Selector::Attribute { .. }  => Specificity(0, 1, 0),
            Selector::Tag(_)            => Specificity(0, 0, 1),
            Selector::Universal         => Specificity(0, 0, 0),
            Selector::Compound(parts)   => {
                let mut ids = 0u32; let mut cls = 0u32; let mut tags = 0u32;
                for p in parts {
                    match p {
                        SimpleSelector::Id(_)          => ids += 1,
                        SimpleSelector::Class(_)       => cls += 1,
                        SimpleSelector::Attribute { .. }=> cls += 1,
                        SimpleSelector::PseudoClass(_)  => cls += 1,
                        SimpleSelector::Tag(_)         => tags += 1,
                        SimpleSelector::PseudoElement(_) => tags += 1,
                        SimpleSelector::Universal      => {}
                    }
                }
                Specificity(ids, cls, tags)
            }
            Selector::Descendant(a, b) | Selector::Child(a, b) => {
                a.specificity() + b.specificity()
            }
            Selector::List(sels) => {
                // Maximale Spezifität aus der Liste (für Sortierung)
                sels.iter().map(|s| s.specificity()).max().unwrap_or_default()
            }
        }
    }
}

/// Farbwert-Parsing: "rgb(r,g,b)", "#rrggbb", "#rgb", benannte Farben
pub fn parse_color(s: &str) -> Option<u32> {
    let s = s.trim();
    if s.starts_with('#') {
        let hex = &s[1..];
        if hex.len() == 6 {
            return u32::from_str_radix(hex, 16).ok();
        } else if hex.len() == 3 {
            let r = u8::from_str_radix(&hex[0..1].repeat(2), 16).ok()?;
            let g = u8::from_str_radix(&hex[1..2].repeat(2), 16).ok()?;
            let b = u8::from_str_radix(&hex[2..3].repeat(2), 16).ok()?;
            return Some(((r as u32) << 16) | ((g as u32) << 8) | b as u32);
        }
    }
    if s.starts_with("rgb(") && s.ends_with(')') {
        let inner = &s[4..s.len()-1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() == 3 {
            let r = parts[0].trim().parse::<u32>().ok()?;
            let g = parts[1].trim().parse::<u32>().ok()?;
            let b = parts[2].trim().parse::<u32>().ok()?;
            return Some((r << 16) | (g << 8) | b);
        }
    }
    if s.starts_with("rgba(") && s.ends_with(')') {
        let inner = &s[5..s.len()-1];
        let parts: Vec<&str> = inner.split(',').collect();
        if parts.len() >= 3 {
            let r = parts[0].trim().parse::<u32>().ok()?;
            let g = parts[1].trim().parse::<u32>().ok()?;
            let b = parts[2].trim().parse::<u32>().ok()?;
            return Some((r << 16) | (g << 8) | b);
        }
    }
    // Benannte Farben
    match s {
        "black"       => Some(0x000000), "white"       => Some(0xFFFFFF),
        "red"         => Some(0xFF0000), "green"       => Some(0x008000),
        "blue"        => Some(0x0000FF), "yellow"      => Some(0xFFFF00),
        "orange"      => Some(0xFF8C00), "purple"      => Some(0x800080),
        "pink"        => Some(0xFF69B4), "gray"        => Some(0x808080),
        "grey"        => Some(0x808080), "silver"      => Some(0xC0C0C0),
        "navy"        => Some(0x000080), "teal"        => Some(0x008080),
        "cyan"        => Some(0x00FFFF), "magenta"     => Some(0xFF00FF),
        "brown"       => Some(0xA52A2A), "lime"        => Some(0x00FF00),
        "maroon"      => Some(0x800000), "olive"       => Some(0x808000),
        "coral"       => Some(0xFF6347), "salmon"      => Some(0xFA8072),
        "gold"        => Some(0xFFD700), "indigo"      => Some(0x4B0082),
        "violet"      => Some(0xEE82EE), "transparent" => Some(0xFFFFFF),
        "darkgray"    => Some(0xA9A9A9), "darkgrey"    => Some(0xA9A9A9),
        "lightgray"   => Some(0xD3D3D3), "lightgrey"   => Some(0xD3D3D3),
        "whitesmoke"  => Some(0xF5F5F5), "gainsboro"   => Some(0xDCDCDC),
        "crimson"     => Some(0xDC143C), "tomato"      => Some(0xFF6347),
        "steelblue"   => Some(0x4682B4), "dodgerblue"  => Some(0x1E90FF),
        "deepskyblue" => Some(0x00BFFF), "royalblue"   => Some(0x4169E1),
        "slategray"   => Some(0x708090), "dimgray"     => Some(0x696969),
        _ => None,
    }
}

/// Pixelwert parsen: "16px", "1em" (≈16px), "1.5rem", "10pt"
/// parent_width: falls Some(w), werden %-Angaben aufgelöst
pub fn parse_px(s: &str) -> Option<f32> {
    parse_px_with_parent(s, None)
}

pub fn parse_px_with_parent(s: &str, parent_width: Option<f32>) -> Option<f32> {
    let s = s.trim();
    if let Some(v) = s.strip_suffix("px") {
        return v.trim().parse().ok();
    }
    if let Some(v) = s.strip_suffix("em") {
        return v.trim().parse::<f32>().ok().map(|f| f * 16.0);
    }
    // "rem" muss vor "em" geprüft werden
    if let Some(v) = s.strip_suffix("rem") {
        return v.trim().parse::<f32>().ok().map(|f| f * 16.0);
    }
    if let Some(v) = s.strip_suffix("pt") {
        return v.trim().parse::<f32>().ok().map(|f| f * 1.333);
    }
    if let Some(v) = s.strip_suffix("vh") {
        return v.trim().parse::<f32>().ok().map(|f| f * 6.5);
    }
    if let Some(v) = s.strip_suffix("vw") {
        return v.trim().parse::<f32>().ok().map(|f| f * 9.0);
    }
    if let Some(v) = s.strip_suffix('%') {
        // Prozent nur auflösen wenn parent_width bekannt
        if let Some(pw) = parent_width {
            return v.trim().parse::<f32>().ok().map(|f| f / 100.0 * pw);
        }
        return None;
    }
    // reine Zahl (px implizit)
    s.parse().ok()
}

/// Eine einzelne CSS-Eigenschaft mit Wert.
#[derive(Debug, Clone)]
pub enum Declaration {
    Width(f32),
    Height(f32),
    /// width als Prozentwert (0.0–1.0), wird im Layout gegen parent_w aufgelöst
    WidthPercent(f32),
    /// height als Prozentwert (0.0–1.0), wird im Layout gegen parent_h aufgelöst
    HeightPercent(f32),
    MarginTop(f32),
    MarginRight(f32),
    MarginBottom(f32),
    MarginLeft(f32),
    MarginLeftAuto,
    MarginRightAuto,
    PaddingTop(f32),
    PaddingRight(f32),
    PaddingBottom(f32),
    PaddingLeft(f32),
    Color(u32),
    BackgroundColor(u32),
    FontSize(f32),
    Display(DisplayValue),
    FontWeight(FontWeightValue),
    TextDecoration(TextDecorationValue),
    BorderColor(u32),
    BorderWidth(f32),
    // NEU
    TextAlign(TextAlignValue),
    LineHeight(f32),
    MaxWidth(f32),
    BorderRadius(f32),
    BorderTopLeftRadius(f32),
    BorderTopRightRadius(f32),
    BorderBottomLeftRadius(f32),
    BorderBottomRightRadius(f32),
    LetterSpacing(f32),
    // Flexbox
    Position(PositionValue),
    Top(f32),
    Left(f32),
    Right(f32),
    Bottom(f32),
    FlexDirection(FlexDirection),
    JustifyContent(JustifyContent),
    AlignItems(AlignItems),
    // NEU: align-self (gleiche Werte wie align-items)
    AlignSelf(AlignItems),
    FlexGrow(f32),
    FlexShrink(f32),
    FlexBasis(f32),
    // NEU: flex-wrap
    FlexWrap(FlexWrap),
    Gap(f32),
    ColumnGap(f32),
    RowGap(f32),
    Order(i32),
    BoxSizingBorder, // box-sizing: border-box
    // NEU: visibility
    Visibility(VisibilityValue),
    // NEU: opacity
    Opacity(f32),
    // NEU: overflow
    Overflow(OverflowValue),
    // NEU: z-index
    ZIndex(i32),
    // NEU: white-space
    WhiteSpace(WhiteSpaceValue),
    // NEU: font-family (als String, vereinfacht)
    FontFamily(String),
    // NEU: background-image (nur none/gradient linear für jetzt)
    BackgroundImageNone,
    BackgroundImageGradient { c1: u32, c2: u32 },
    // NEU: box-shadow (vereinfacht: offset-x, offset-y, blur, color)
    BoxShadow { offset_x: f32, offset_y: f32, blur: f32, color: u32 },
    // NEU: border per Seite
    BorderTopWidth(f32),
    BorderRightWidth(f32),
    BorderBottomWidth(f32),
    BorderLeftWidth(f32),
    BorderTopColor(u32),
    BorderRightColor(u32),
    BorderBottomColor(u32),
    BorderLeftColor(u32),
    // NEU: outline für Focus-Rings
    OutlineWidth(f32),
    OutlineColor(u32),
    // NEU: cursor
    CursorPointer,
    // NEU: Grid-Properties
    GridTemplateColumns(Vec<GridTemplateValue>),
    GridTemplateRows(Vec<GridTemplateValue>),
    GridAutoColumns(Vec<GridTemplateValue>),
    GridAutoRows(Vec<GridTemplateValue>),
    GridColumnStart(i32),
    GridColumnEnd(i32),
    GridRowStart(i32),
    GridRowEnd(i32),
    // NEU: aspect-ratio
    AspectRatio(f32),
}

#[derive(Debug, Clone, PartialEq)]
pub enum GridTemplateValue {
    Length(f32),
    Percent(f32),
    Flex(f32), // fr-Einheit
    Auto,
    MinContent,
    MaxContent,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DisplayValue { Block, Inline, InlineBlock, Flex, Grid, None, Other }

#[derive(Debug, Clone, PartialEq)]
pub enum FontWeightValue { Normal, Bold }

#[derive(Debug, Clone, PartialEq)]
pub enum TextDecorationValue { None, Underline }

// NEU: text-align
#[derive(Debug, Clone, PartialEq, Default)]
pub enum TextAlignValue { #[default] Left, Center, Right }

// NEU: position
#[derive(Debug, Clone, PartialEq, Default)]
pub enum PositionValue { #[default] Static, Relative, Absolute, Fixed, Sticky }

// NEU: flex-direction
#[derive(Debug, Clone, PartialEq, Default)]
pub enum FlexDirection { #[default] Row, Column, RowReverse, ColumnReverse }

// NEU: justify-content
#[derive(Debug, Clone, PartialEq, Default)]
pub enum JustifyContent { #[default] FlexStart, FlexEnd, Center, SpaceBetween, SpaceAround, SpaceEvenly }

// NEU: align-items
#[derive(Debug, Clone, PartialEq, Default)]
pub enum AlignItems { #[default] Stretch, FlexStart, FlexEnd, Center, Baseline }

// NEU: flex-wrap
#[derive(Debug, Clone, PartialEq, Default)]
pub enum FlexWrap { #[default] NoWrap, Wrap, WrapReverse }

// NEU: visibility
#[derive(Debug, Clone, PartialEq, Default)]
pub enum VisibilityValue { #[default] Visible, Hidden, Collapse }

// NEU: overflow
#[derive(Debug, Clone, PartialEq, Default)]
pub enum OverflowValue { #[default] Visible, Hidden, Scroll, Auto }

// NEU: white-space
#[derive(Debug, Clone, PartialEq, Default)]
pub enum WhiteSpaceValue { #[default] Normal, NoWrap, Pre, PreWrap, PreLine, Nowrap }

// impl Declaration: parse_property wird nicht mehr genutzt (lightningcss übernimmt)
// Der Block existiert noch für layout_bridge.rs Kompatibilität.

// ─── CSS-Text-Parser (lightningcss) ──────────────────────────────────────────

// ─── CSS Custom Property (var()) Auflösung ───────────────────────────────────
//
// Strategie: Zwei-Pass-Ansatz
// Pass 1: Alle --custom-property Definitionen aus :root und * sammeln
// Pass 2: var(--x) im CSS-Text durch ihre Werte ersetzen
// Danach erst lightningcss parsen → kein var() mehr im AST

/// Extrahiert alle --custom-property: value; Definitionen aus CSS-Text.
/// Sucht in :root { } und * { } Blöcken.
fn collect_custom_properties(css: &str) -> std::collections::HashMap<String, String> {
    let mut vars = std::collections::HashMap::new();

    // Alle --name: value; Paare finden (auch außerhalb von :root)
    let mut i = 0;
    let bytes = css.as_bytes();
    let len = css.len();

    while i < len {
        // "--" suchen
        if i + 2 < len && bytes[i] == b'-' && bytes[i+1] == b'-' {
            // Name lesen bis ':'
            let name_start = i;
            while i < len && bytes[i] != b':' && bytes[i] != b'{' && bytes[i] != b'}' {
                i += 1;
            }
            if i >= len || bytes[i] != b':' { continue; }
            let name = css[name_start..i].trim().to_string();
            if !name.starts_with("--") { i += 1; continue; }
            i += 1; // skip ':'

            // Value lesen bis ';' oder '}'
            let val_start = i;
            let mut depth = 0i32;
            while i < len {
                match bytes[i] {
                    b'(' => { depth += 1; i += 1; }
                    b')' => { depth -= 1; i += 1; }
                    b';' if depth == 0 => { i += 1; break; }
                    b'}' if depth == 0 => break,
                    _ => { i += 1; }
                }
            }
            let value = css[val_start..i.saturating_sub(1)].trim().to_string();
            if !value.is_empty() {
                vars.entry(name).or_insert(value);
            }
        } else {
            i += 1;
        }
    }
    vars
}

/// Ersetzt var(--name) und var(--name, fallback) rekursiv im CSS-Text.
/// Maximal 8 Durchläufe um verschachtelte vars aufzulösen.
fn resolve_vars(css: &str, vars: &std::collections::HashMap<String, String>) -> String {
    if vars.is_empty() || !css.contains("var(") { return css.to_string(); }

    let mut result = css.to_string();
    for _ in 0..8 {
        if !result.contains("var(") { break; }
        let mut out = String::with_capacity(result.len());
        let mut chars = result.char_indices().peekable();

        while let Some((i, c)) = chars.next() {
            // "var(" suchen
            if c == 'v' && result[i..].starts_with("var(") {
                // Überspringe "var("
                for _ in 0..3 { chars.next(); }
                
                // Inhalt bis passende ')' lesen
                let content_start = i + 4;
                let mut depth = 1i32;
                let mut content_end = content_start;
                
                while let Some((ci, cc)) = chars.next() {
                    if cc == '(' { depth += 1; }
                    else if cc == ')' { depth -= 1; }
                    if depth == 0 { content_end = ci; break; }
                }
                
                let content = result[content_start..content_end].trim();

                // name und optionalen fallback trennen
                let (name, fallback) = if let Some(comma) = content.find(',') {
                    (content[..comma].trim(), Some(content[comma+1..].trim()))
                } else {
                    (content, None)
                };

                if let Some(value) = vars.get(name) {
                    out.push_str(value);
                } else if let Some(fb) = fallback {
                    out.push_str(fb);
                } else {
                    // unbekannte Variable → transparent/inherit als Fallback
                    out.push_str("transparent");
                }
            } else {
                out.push(c);
            }
        }
        result = out;
    }
    result
}


pub fn parse_css(css: &str) -> Stylesheet {
    // Pass 1: Custom Properties sammeln
    let vars = collect_custom_properties(css);

    // Pass 2: var() auflösen
    let resolved = if vars.is_empty() {
        css.to_string()
    } else {
        resolve_vars(css, &vars)
    };

    // Pass 3: Gradients werden jetzt nativ in lc_declarations_to_ours verarbeitet
    parse_css_resolved(&resolved)
}

/// Parst bereits aufgelöstes CSS (kein var() mehr drin)
fn parse_css_resolved(css: &str) -> Stylesheet {
    use lightningcss::stylesheet::{StyleSheet, ParserOptions};

    let opts = ParserOptions::default();
    let Ok(sheet) = StyleSheet::parse(css, opts) else {
        return Stylesheet { rules: vec![] };
    };

    let mut rules = vec![];
    parse_rules(&sheet.rules.0, &mut rules);
    Stylesheet { rules }
}

/// Rekursive Regel-Verarbeitung: Style-Regeln + @media/@supports/etc.
fn parse_rules(css_rules: &[lightningcss::rules::CssRule], out: &mut Vec<Rule>) {
    use lightningcss::rules::CssRule;
    for rule in css_rules {
        match rule {
            CssRule::Style(style_rule) => {
                let declarations = lc_declarations_to_ours(&style_rule.declarations);
                if declarations.is_empty() { continue; }
                for selector_list in style_rule.selectors.0.iter() {
                    if let Some(selector) = lc_selector_to_ours(selector_list) {
                        out.push(Rule { selector, declarations: declarations.clone() });
                    }
                }
            }
            // @media Regeln rekursiv auswerten
            // @media Regeln rekursiv auswerten
            CssRule::Media(media_rule) => {
                use lightningcss::media_query::MediaType;
                let media = &media_rule.query.media_queries;
                // Strategie: @media print ablehnen, alles andere (screen, all, min-width, etc.) annehmen.
                // Wir können keine echten Viewport-Bedingungen auswerten, aber "annehmen" ist
                // besser als "ablehnen" — Wikipedia's gesamtes CSS steckt in @media-Blöcken.
                let applies = media.is_empty() || media.iter().any(|mq| {
                    !matches!(mq.media_type, MediaType::Print)
                });
                if applies {
                    parse_rules(&media_rule.rules.0, out);
                }
            }
            // @supports Regeln (ignorieren, aber keine Panik)
            CssRule::Supports(_) => {
                // Hier könnte man die Supports-Bedingung prüfen, für jetzt: ignorieren
            }
            _ => {}
        }
    }
}

// ── lightningcss Selector → unser Selector ────────────────────────────────
fn lc_selector_to_ours(sel: &lightningcss::selector::Selector) -> Option<Selector> {
    use lightningcss::selector::Component;

    // lightningcss liefert Selektoren als flache Liste von Components.
    // Kombinatoren (Descendant, Child) sind eigene Components zwischen den Teilen.
    // Wir bauen daraus einen Baum auf.
    //
    // Beispiel ".foo > span":
    //   [Class("foo"), Combinator(Child), LocalName("span")]
    // → Child(Compound([Class("foo")]), Compound([Tag("span")]))

    let mut components: Vec<_> = sel.iter().collect();
    components.reverse();
    if components.is_empty() { return None; }

    // Segmente an Kombinatoren aufteilen
    // Jedes Segment ist eine Liste von SimpleSelectors
    // Zwischen Segmenten steht ein Kombinator
    #[derive(Clone, Debug)]
    enum Comb { Descendant, Child }

    let mut segments: Vec<Vec<SimpleSelector>> = vec![vec![]];
    let mut combinators: Vec<Comb> = vec![];

    for comp in &components {
        match comp {
            Component::Combinator(c) => {
                use lightningcss::selector::Combinator as LC;
                let comb = match c {
                    LC::Child           => Comb::Child,
                    LC::Descendant      => Comb::Descendant,
                    // Sibling-Kombinatoren (~, +) behandeln wir wie Descendant (approximation)
                    _                   => Comb::Descendant,
                };
                combinators.push(comb);
                segments.push(vec![]);
            }
            Component::ID(id) => {
                segments.last_mut()?.push(SimpleSelector::Id(id.to_string()));
            }
            Component::Class(cls) => {
                segments.last_mut()?.push(SimpleSelector::Class(cls.to_string()));
            }
            Component::LocalName(ln) => {
                let name = ln.lower_name.to_string();
                if name != "*" {
                    segments.last_mut()?.push(SimpleSelector::Tag(name));
                }
            }
            Component::AttributeInNoNamespace { local_name, value, .. } => {
                segments.last_mut()?.push(SimpleSelector::Attribute {
                    name: local_name.to_string(),
                    value: Some(value.to_string()),
                });
            }
            Component::AttributeInNoNamespaceExists { local_name, .. } => {
                segments.last_mut()?.push(SimpleSelector::Attribute {
                    name: local_name.to_string(),
                    value: None,
                });
            }
            Component::ExplicitUniversalType | Component::ExplicitAnyNamespace => {
                segments.last_mut()?.push(SimpleSelector::Universal);
            }
            // Pseudo-Klassen (:hover, :nth-child etc.) und Negationen (:not())
            // werden komplett ignoriert, da wir sie nicht korrekt auswerten können.
            // Ein falsches Matching ist schlimmer als gar kein Matching.
            // TODO: Später gezielte Unterstützung für einfache Pseudo-Klassen.
            Component::NonTSPseudoClass(_) |
            Component::PseudoElement(_) |
            Component::Negation(_) |
            Component::Root |
            Component::Empty => {}
            _ => {}
        }
    }

    // Segment → Selector konvertieren
    fn seg_to_sel(seg: Vec<SimpleSelector>) -> Option<Selector> {
        if seg.len() == 1 {
            match seg.into_iter().next().unwrap() {
                SimpleSelector::Tag(t)          => Some(Selector::Tag(t)),
                SimpleSelector::Id(id)          => Some(Selector::Id(id)),
                SimpleSelector::Class(cls)      => Some(Selector::Class(cls)),
                SimpleSelector::Universal       => Some(Selector::Universal),
                SimpleSelector::Attribute { name, value } => Some(Selector::Attribute { name, value }),
                _ => None,
            }
        } else if seg.len() > 1 {
            Some(Selector::Compound(seg))
        } else {
            None
        }
    }

    // Segmente von links nach rechts zu einem Baum falten
    let mut result = seg_to_sel(segments.remove(0))?;
    for (comb, seg) in combinators.into_iter().zip(segments.into_iter()) {
        let right = seg_to_sel(seg)?;
        result = match comb {
            Comb::Child      => Selector::Child(Box::new(result), Box::new(right)),
            Comb::Descendant => Selector::Descendant(Box::new(result), Box::new(right)),
        };
    }

    Some(result)
}

// ── lightningcss Declarations → unsere Declarations ───────────────────────
fn lc_declarations_to_ours(
    block: &lightningcss::declaration::DeclarationBlock,
) -> Vec<Declaration> {
    use lightningcss::properties::Property;
    use lightningcss::values::length::LengthPercentageOrAuto;

    let mut out = vec![];

    // Default UnitContext für das Parsen von Deklarationen.
    // parent_width wird auf viewport_width gesetzt damit %-Werte nicht lautlos
    // verschwinden. Der tatsächliche parent_w wird im Layout dann nochmal korrekt
    // angewendet — wir speichern %-Werte als WidthPercent/HeightPercent.
    let unit_context = UnitContext {
        root_font_size: 16.0,
        current_font_size: 16.0,
        viewport_width: 1280.0,
        viewport_height: 800.0,
        parent_width: Some(1280.0), // Fallback, damit lc_lp nicht None zurückgibt
    };

    // Alle Properties (normal + important) verarbeiten
    let all_props = block.declarations.iter()
        .chain(block.important_declarations.iter());

    for prop in all_props {
        match prop {
            Property::Width(w)  => {
                // Prozentwerte als WidthPercent speichern, damit das Layout sie
                // korrekt gegen den echten parent_w auflösen kann.
                if let Some(pct) = lc_size_percent(w) {
                    out.push(Declaration::WidthPercent(pct));
                } else if let Some(v) = lc_length_pct(w, &unit_context) {
                    out.push(Declaration::Width(v));
                }
            }
            Property::Height(h) => {
                if let Some(pct) = lc_size_percent(h) {
                    out.push(Declaration::HeightPercent(pct));
                } else if let Some(v) = lc_length_pct(h, &unit_context) {
                    out.push(Declaration::Height(v));
                }
            }
            Property::MaxWidth(mw) => { if let Some(v) = lc_max_size(mw, &unit_context) { out.push(Declaration::MaxWidth(v)); } }

            Property::MarginTop(m)    => if let Some(v) = lc_lpa(m, &unit_context) { out.push(Declaration::MarginTop(v)); },
            Property::MarginBottom(m) => if let Some(v) = lc_lpa(m, &unit_context) { out.push(Declaration::MarginBottom(v)); },
            Property::MarginLeft(m)   => match m {
                LengthPercentageOrAuto::Auto => out.push(Declaration::MarginLeftAuto),
                _ => if let Some(v) = lc_lpa(m, &unit_context) { out.push(Declaration::MarginLeft(v)); }
            },
            Property::MarginRight(m)  => match m {
                LengthPercentageOrAuto::Auto => out.push(Declaration::MarginRightAuto),
                _ => if let Some(v) = lc_lpa(m, &unit_context) { out.push(Declaration::MarginRight(v)); }
            },
            Property::Margin(m) => {
                match &m.top    { LengthPercentageOrAuto::Auto => out.push(Declaration::MarginLeftAuto), _ => if let Some(v) = lc_lpa(&m.top, &unit_context)    { out.push(Declaration::MarginTop(v)); } }
                match &m.bottom { LengthPercentageOrAuto::Auto => {}, _ => if let Some(v) = lc_lpa(&m.bottom, &unit_context) { out.push(Declaration::MarginBottom(v)); } }
                match &m.left   { LengthPercentageOrAuto::Auto => out.push(Declaration::MarginLeftAuto),  _ => if let Some(v) = lc_lpa(&m.left, &unit_context)   { out.push(Declaration::MarginLeft(v)); } }
                match &m.right  { LengthPercentageOrAuto::Auto => out.push(Declaration::MarginRightAuto), _ => if let Some(v) = lc_lpa(&m.right, &unit_context)  { out.push(Declaration::MarginRight(v)); } }
            }

            Property::PaddingTop(p)    => if let Some(v) = lc_lpa(p, &unit_context) { out.push(Declaration::PaddingTop(v)); },
            Property::PaddingBottom(p) => if let Some(v) = lc_lpa(p, &unit_context) { out.push(Declaration::PaddingBottom(v)); },
            Property::PaddingLeft(p)   => if let Some(v) = lc_lpa(p, &unit_context) { out.push(Declaration::PaddingLeft(v)); },
            Property::PaddingRight(p)  => if let Some(v) = lc_lpa(p, &unit_context) { out.push(Declaration::PaddingRight(v)); },
            Property::Padding(p) => {
                if let Some(v) = lc_lpa(&p.top, &unit_context)    { out.push(Declaration::PaddingTop(v)); }
                if let Some(v) = lc_lpa(&p.bottom, &unit_context) { out.push(Declaration::PaddingBottom(v)); }
                if let Some(v) = lc_lpa(&p.left, &unit_context)   { out.push(Declaration::PaddingLeft(v)); }
                if let Some(v) = lc_lpa(&p.right, &unit_context)  { out.push(Declaration::PaddingRight(v)); }
            }

            Property::Color(c)           => if let Some(v) = lc_color(c) { out.push(Declaration::Color(v)); },
            Property::BackgroundColor(c) => if let Some(v) = lc_color(c) { out.push(Declaration::BackgroundColor(v)); },
            Property::Background(bg) => {
                if let Some(first) = bg.first() {
                    if let Some(v) = lc_color(&first.color) { out.push(Declaration::BackgroundColor(v)); }
                }
            }

            Property::FontSize(fs) => if let Some(v) = lc_font_size(fs, &unit_context) { out.push(Declaration::FontSize(v)); },
            Property::FontWeight(fw) => {
                use lightningcss::properties::font::FontWeight as LcFW;
                let bold = match fw {
                    LcFW::Bolder => true,
                    LcFW::Absolute(w) => {
                        use lightningcss::properties::font::AbsoluteFontWeight;
                        matches!(w, AbsoluteFontWeight::Weight(n) if *n >= 600.0)
                            || matches!(w, AbsoluteFontWeight::Bold)
                    }
                    _ => false,
                };
                out.push(Declaration::FontWeight(if bold { FontWeightValue::Bold } else { FontWeightValue::Normal }));
            }

            Property::LineHeight(lh) => {
                use lightningcss::properties::font::LineHeight;
                match lh {
                    LineHeight::Normal => out.push(Declaration::LineHeight(16.0 * 1.2)),
                    LineHeight::Number(n) => out.push(Declaration::LineHeight(*n * 16.0)),
                    LineHeight::Length(lp) => if let Some(v) = lc_lp(lp, &unit_context) { out.push(Declaration::LineHeight(v)); }
                }
            }

            Property::Display(d) => {
                use lightningcss::properties::display::{Display as LcDisplay, DisplayKeyword, DisplayInside, DisplayOutside};
                let dv = match d {
                    LcDisplay::Keyword(kw) => match kw {
                        DisplayKeyword::None  => DisplayValue::None,
                        _ => DisplayValue::Other,
                    },
                    LcDisplay::Pair(pair) => match (&pair.outside, &pair.inside) {
                        (DisplayOutside::Block, DisplayInside::Flex(_))  => DisplayValue::Flex,
                        (DisplayOutside::Inline, DisplayInside::Flex(_)) => DisplayValue::Flex,
                        (DisplayOutside::Block, DisplayInside::Grid)  => DisplayValue::Grid,
                        (DisplayOutside::Inline, DisplayInside::Grid) => DisplayValue::Grid,
                        (DisplayOutside::Inline, DisplayInside::FlowRoot) => DisplayValue::InlineBlock,
                        (DisplayOutside::Block, _)                    => DisplayValue::Block,
                        (DisplayOutside::Inline, DisplayInside::Flow) => DisplayValue::Inline,
                        _ => DisplayValue::Other,
                    },
                };
                out.push(Declaration::Display(dv));
            }

            Property::TextAlign(ta) => {
                use lightningcss::properties::text::TextAlign;
                let v = match ta {
                    TextAlign::Center => TextAlignValue::Center,
                    TextAlign::Right | TextAlign::End => TextAlignValue::Right,
                    _ => TextAlignValue::Left,
                };
                out.push(Declaration::TextAlign(v));
            }

            Property::TextDecoration(td, _) => {
                use lightningcss::properties::text::TextDecorationLine;
                if td.line.contains(TextDecorationLine::Underline) {
                    out.push(Declaration::TextDecoration(TextDecorationValue::Underline));
                } else {
                    out.push(Declaration::TextDecoration(TextDecorationValue::None));
                }
            }

            Property::BorderColor(bc) => {
                if let Some(v) = lc_color(&bc.top) { out.push(Declaration::BorderColor(v)); }
            }
            Property::BorderWidth(bw) => {
                if let Some(v) = lc_border_width(&bw.top, &unit_context) { out.push(Declaration::BorderWidth(v)); }
            }
            Property::BorderRadius(br, _) => {
                if let Some(v) = lc_lp(&br.top_left.0, &unit_context) { out.push(Declaration::BorderRadius(v)); }
            }

            Property::Position(p) => {
                use lightningcss::properties::position::Position as LcPos;
                let pv = match p {
                    LcPos::Relative => PositionValue::Relative,
                    LcPos::Absolute => PositionValue::Absolute,
                    LcPos::Fixed    => PositionValue::Fixed,
                    LcPos::Sticky(_) => PositionValue::Sticky,
                    _               => PositionValue::Static,
                };
                out.push(Declaration::Position(pv));
            }
            Property::Top(v)    => if let Some(v) = lc_lpa(v, &unit_context) { out.push(Declaration::Top(v)); },
            Property::Left(v)   => if let Some(v) = lc_lpa(v, &unit_context) { out.push(Declaration::Left(v)); },
            Property::Right(v)  => if let Some(v) = lc_lpa(v, &unit_context) { out.push(Declaration::Right(v)); },
            Property::Bottom(v) => if let Some(v) = lc_lpa(v, &unit_context) { out.push(Declaration::Bottom(v)); },

            Property::FlexDirection(fd, _) => {
                use lightningcss::properties::flex::FlexDirection as LcFD;
                let v = match fd {
                    LcFD::Column        => FlexDirection::Column,
                    LcFD::RowReverse    => FlexDirection::RowReverse,
                    LcFD::ColumnReverse => FlexDirection::ColumnReverse,
                    _                   => FlexDirection::Row,
                };
                out.push(Declaration::FlexDirection(v));
            }
            Property::FlexWrap(fw, _) => {
                use lightningcss::properties::flex::FlexWrap as LcFW;
                let v = match fw {
                    LcFW::Wrap        => FlexWrap::Wrap,
                    LcFW::WrapReverse => FlexWrap::WrapReverse,
                    _                 => FlexWrap::NoWrap,
                };
                out.push(Declaration::FlexWrap(v));
            }
            Property::FlexGrow(v, _)   => out.push(Declaration::FlexGrow(*v)),
            Property::FlexShrink(v, _) => out.push(Declaration::FlexShrink(*v)),
            Property::FlexBasis(fb, _) => {
                if let Some(v) = lc_lpa(fb, &unit_context) { out.push(Declaration::FlexBasis(v)); }
            }
            Property::Flex(f, _) => {
                out.push(Declaration::FlexGrow(f.grow));
                out.push(Declaration::FlexShrink(f.shrink));
                if let Some(v) = lc_lpa(&f.basis, &unit_context) { out.push(Declaration::FlexBasis(v)); }
            }

            Property::JustifyContent(jc, _) => {
                use lightningcss::properties::align::JustifyContent as LcJC;
                use lightningcss::properties::align::ContentDistribution;
                let v = match jc {
                    LcJC::ContentDistribution(ContentDistribution::SpaceBetween) => JustifyContent::SpaceBetween,
                    LcJC::ContentDistribution(ContentDistribution::SpaceAround)  => JustifyContent::SpaceAround,
                    LcJC::ContentDistribution(ContentDistribution::SpaceEvenly)  => JustifyContent::SpaceEvenly,
                    LcJC::ContentPosition { value: pos, .. } => {
                        use lightningcss::properties::align::ContentPosition;
                        match pos {
                            ContentPosition::Center    => JustifyContent::Center,
                            ContentPosition::FlexEnd | ContentPosition::End => JustifyContent::FlexEnd,
                            _ => JustifyContent::FlexStart,
                        }
                    }
                    _ => JustifyContent::FlexStart,
                };
                out.push(Declaration::JustifyContent(v));
            }
            Property::AlignItems(ai, _) => {
                if let Some(v) = lc_align_items(ai) { out.push(Declaration::AlignItems(v)); }
            }
            Property::AlignSelf(asi, _) => {
                use lightningcss::properties::align::AlignSelf as LcAS;
                if let LcAS::SelfPosition { value: pos, .. } = asi {
                    use lightningcss::properties::align::SelfPosition;
                    let v = match pos {
                        SelfPosition::Center   => AlignItems::Center,
                        SelfPosition::FlexEnd | SelfPosition::End => AlignItems::FlexEnd,
                        SelfPosition::FlexStart | SelfPosition::Start => AlignItems::FlexStart,
                        _ => AlignItems::Stretch,
                    };
                    out.push(Declaration::AlignSelf(v));
                }
            }
            Property::Gap(g) => {
                if let Some(v) = lc_gap(&g.row, &unit_context) { out.push(Declaration::Gap(v)); }
                if let Some(col) = lc_gap(&g.column, &unit_context) { out.push(Declaration::ColumnGap(col)); }
            }
            Property::RowGap(v)    => if let Some(v) = lc_gap(v, &unit_context) { out.push(Declaration::RowGap(v)); },
            Property::ColumnGap(v) => if let Some(v) = lc_gap(v, &unit_context) { out.push(Declaration::ColumnGap(v)); },

            Property::Order(v, _) => out.push(Declaration::Order(*v as i32)),

            Property::BoxSizing(bs, _) => {
                use lightningcss::properties::size::BoxSizing;
                if matches!(bs, BoxSizing::BorderBox) { out.push(Declaration::BoxSizingBorder); }
            }

            // NEU: visibility
            Property::Visibility(v) => {
                use lightningcss::properties::display::Visibility as LcVis;
                let vv = match v {
                    LcVis::Visible => VisibilityValue::Visible,
                    LcVis::Hidden  => VisibilityValue::Hidden,
                    LcVis::Collapse => VisibilityValue::Collapse,
                };
                out.push(Declaration::Visibility(vv));
            }

            // NEU: opacity
            Property::Opacity(o) => {
                out.push(Declaration::Opacity(o.0 as f32));
            }

            // NEU: overflow
            Property::Overflow(o) => {
                use lightningcss::properties::overflow::OverflowKeyword;
                let ov = match o.x {
                    OverflowKeyword::Visible => OverflowValue::Visible,
                    OverflowKeyword::Hidden  | OverflowKeyword::Clip => OverflowValue::Hidden,
                    OverflowKeyword::Scroll  => OverflowValue::Scroll,
                    OverflowKeyword::Auto    => OverflowValue::Auto,
                };
                out.push(Declaration::Overflow(ov));
            }
            Property::OverflowX(o) | Property::OverflowY(o) => {
                use lightningcss::properties::overflow::OverflowKeyword;
                let ov = match o {
                    OverflowKeyword::Visible => OverflowValue::Visible,
                    OverflowKeyword::Hidden  | OverflowKeyword::Clip => OverflowValue::Hidden,
                    OverflowKeyword::Scroll  => OverflowValue::Scroll,
                    OverflowKeyword::Auto    => OverflowValue::Auto,
                };
                out.push(Declaration::Overflow(ov));
            }

            // NEU: z-index
            Property::ZIndex(z) => {
                // z ist &ZIndex, enhalt Option<i32>
                let z_opt: &Option<i32> = unsafe { &*(z as *const _ as *const Option<i32>) };
                if let Some(z_val) = z_opt {
                    out.push(Declaration::ZIndex(*z_val as i32));
                }
            }

            // NEU: white-space
            Property::WhiteSpace(ws) => {
                use lightningcss::properties::text::WhiteSpace as LcWS;
                let wsv = match ws {
                    LcWS::Normal   => WhiteSpaceValue::Normal,
                    LcWS::NoWrap  => WhiteSpaceValue::NoWrap,
                    LcWS::Pre     => WhiteSpaceValue::Pre,
                    LcWS::PreWrap => WhiteSpaceValue::PreWrap,
                    LcWS::PreLine => WhiteSpaceValue::PreLine,
                    _              => WhiteSpaceValue::Normal,
                };
                out.push(Declaration::WhiteSpace(wsv));
            }

            // NEU: font-family (vereinfacht: nur ersten Font-Namen)
            Property::FontFamily(ff) => {
                use lightningcss::properties::font::FontFamily as LcFF;
                if let Some(first) = ff.first() {
                    let name = match first {
                        LcFF::FamilyName(f) => {
                            use lightningcss::traits::ToCss;
                            let mut dest = String::new();
                            let mut printer = lightningcss::printer::Printer::new(&mut dest, lightningcss::printer::PrinterOptions::default());
                            let _ = f.to_css(&mut printer);
                            dest.trim_matches('"').trim_matches('\'').to_string()
                        }
                        _ => "sans-serif".to_string(),
                    };
                    out.push(Declaration::FontFamily(name));
                }
            }

            // NEU: background-image (none erkennen + gradient Fallback-Farbe)
            Property::BackgroundImage(bi) => {
                use lightningcss::values::image::Image;
                use lightningcss::values::gradient::Gradient;
                for img in bi.iter() {
                    match img {
                        Image::None => { out.push(Declaration::BackgroundImageNone); }
                        Image::Gradient(grad) => {
                            if let Gradient::Linear(linear) = &**grad {
                                use lightningcss::values::gradient::GradientItem;

                                let color_stops: Vec<_> = linear.items.iter()
                                    .filter_map(|item| match item {
                                        GradientItem::ColorStop(stop) => Some(stop),
                                        _ => None,
                                    })
                                    .collect();

                                if color_stops.len() >= 2 {
                                    let c1 = lc_color(&color_stops.first().unwrap().color);
                                    let c2 = lc_color(&color_stops.last().unwrap().color);
                                    if let (Some(c1), Some(c2)) = (c1, c2) {
                                        out.push(Declaration::BackgroundImageGradient { c1, c2 });
                                    }
                                } else if let Some(stop) = color_stops.first() {
                                    if let Some(c) = lc_color(&stop.color) {
                                        out.push(Declaration::BackgroundColor(c));
                                    }
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }

            // NEU: box-shadow
            Property::BoxShadow(shadows, _) => {
                for shadow in shadows.iter() {
                    if shadow.inset { continue; } // inset ignorieren
                    let ox = match &shadow.x_offset {
                        lightningcss::values::length::Length::Value(lv) => lc_length_value(lv, &unit_context).unwrap_or(0.0),
                        _ => 0.0,
                    };
                    let oy = match &shadow.y_offset {
                        lightningcss::values::length::Length::Value(lv) => lc_length_value(lv, &unit_context).unwrap_or(0.0),
                        _ => 0.0,
                    };
                    let blur = match &shadow.blur {
                        lightningcss::values::length::Length::Value(lv) => lc_length_value(lv, &unit_context).unwrap_or(0.0),
                        _ => 0.0,
                    };
                    if let Some(col) = lc_color(&shadow.color) {
                        out.push(Declaration::BoxShadow { offset_x: ox, offset_y: oy, blur, color: col });
                    }
                    break; // nur ersten Shadow
                }
            }

            // NEU: border-top/right/bottom/left Breiten und Farben einzeln
            Property::BorderTopWidth(bw) => {
                if let Some(v) = lc_border_width(bw, &unit_context) { out.push(Declaration::BorderTopWidth(v)); }
            }
            Property::BorderRightWidth(bw) => {
                if let Some(v) = lc_border_width(bw, &unit_context) { out.push(Declaration::BorderRightWidth(v)); }
            }
            Property::BorderBottomWidth(bw) => {
                if let Some(v) = lc_border_width(bw, &unit_context) { out.push(Declaration::BorderBottomWidth(v)); }
            }
            Property::BorderLeftWidth(bw) => {
                if let Some(v) = lc_border_width(bw, &unit_context) { out.push(Declaration::BorderLeftWidth(v)); }
            }
            Property::BorderTopColor(c) => {
                if let Some(v) = lc_color(c) { out.push(Declaration::BorderTopColor(v)); }
            }
            Property::BorderRightColor(c) => {
                if let Some(v) = lc_color(c) { out.push(Declaration::BorderRightColor(v)); }
            }
            Property::BorderBottomColor(c) => {
                if let Some(v) = lc_color(c) { out.push(Declaration::BorderBottomColor(v)); }
            }
            Property::BorderLeftColor(c) => {
                if let Some(v) = lc_color(c) { out.push(Declaration::BorderLeftColor(v)); }
            }

            // NEU: border-radius per Ecke
            Property::BorderTopLeftRadius(br, _) => {
                if let Some(v) = lc_lp(&br.0, &unit_context) { out.push(Declaration::BorderTopLeftRadius(v)); }
            }
            Property::BorderTopRightRadius(br, _) => {
                if let Some(v) = lc_lp(&br.0, &unit_context) { out.push(Declaration::BorderTopRightRadius(v)); }
            }
            Property::BorderBottomLeftRadius(br, _) => {
                if let Some(v) = lc_lp(&br.0, &unit_context) { out.push(Declaration::BorderBottomLeftRadius(v)); }
            }
            Property::BorderBottomRightRadius(br, _) => {
                if let Some(v) = lc_lp(&br.0, &unit_context) { out.push(Declaration::BorderBottomRightRadius(v)); }
            }

            // NEU: cursor: pointer
            Property::Cursor(cursor) => {
                use lightningcss::properties::ui::CursorKeyword;
                if matches!(cursor.keyword, CursorKeyword::Pointer) {
                    out.push(Declaration::CursorPointer);
                }
            }

            Property::AspectRatio(ar) => {
                if let Some(ratio) = &ar.ratio {
                    out.push(Declaration::AspectRatio(ratio.0));
                }
            }

            Property::GridTemplateColumns(gtc) => {
                if let lightningcss::properties::grid::TrackSizing::TrackList(tl) = gtc {
                    out.push(Declaration::GridTemplateColumns(lc_track_list_to_values(tl, &unit_context)));
                }
            }
            Property::GridTemplateRows(gtr) => {
                if let lightningcss::properties::grid::TrackSizing::TrackList(tl) = gtr {
                    out.push(Declaration::GridTemplateRows(lc_track_list_to_values(tl, &unit_context)));
                }
            }
            Property::GridAutoColumns(gac) => {
                out.push(Declaration::GridAutoColumns(lc_track_size_list_to_values(gac, &unit_context)));
            }
            Property::GridAutoRows(gar) => {
                out.push(Declaration::GridAutoRows(lc_track_size_list_to_values(gar, &unit_context)));
            }

            Property::GridColumn(gc) => {
                if let Some(v) = lc_grid_line(&gc.start) { out.push(Declaration::GridColumnStart(v)); }
                if let Some(v) = lc_grid_line(&gc.end) { out.push(Declaration::GridColumnEnd(v)); }
            }
            Property::GridRow(gr) => {
                if let Some(v) = lc_grid_line(&gr.start) { out.push(Declaration::GridRowStart(v)); }
                if let Some(v) = lc_grid_line(&gr.end) { out.push(Declaration::GridRowEnd(v)); }
            }

            Property::Unparsed(unparsed) => {
                // Normalerweise schon durch var()-Preprocessing aufgelöst.
                // Falls noch vorhanden: ignorieren (kein panic)
                let _ = unparsed;
            }

            _ => {} // alle anderen Properties ignorieren
        }
    }
    out
}

// ── Konvertierungshelfer ───────────────────────────────────────────────────

pub struct UnitContext {
    pub root_font_size: f32,
    pub current_font_size: f32,
    pub viewport_width: f32,
    pub viewport_height: f32,
    pub parent_width: Option<f32>,
}

fn lc_length_value(lv: &lightningcss::values::length::LengthValue, unit_context: &UnitContext) -> Option<f32> {
    use lightningcss::values::length::LengthValue;
    match lv {
        LengthValue::Px(v)  => Some(*v),
        LengthValue::Em(v)  => Some(v * unit_context.current_font_size),
        LengthValue::Rem(v) => Some(v * unit_context.root_font_size),
        LengthValue::Pt(v)  => Some(v * 1.333), // PT ist immer noch 1.333px pro Pt
        LengthValue::Vw(v)  => Some(v / 100.0 * unit_context.viewport_width),
        LengthValue::Vh(v)  => Some(v / 100.0 * unit_context.viewport_height),
        _ => None,
    }
}

fn lc_length(l: &lightningcss::values::length::Length, unit_context: &UnitContext) -> Option<f32> {
    use lightningcss::values::length::Length;
    match l {
        Length::Value(lv) => lc_length_value(lv, unit_context),
        Length::Calc(_) => None,
    }
}

fn lc_lp(lp: &lightningcss::values::length::LengthPercentage, unit_context: &UnitContext) -> Option<f32> {
    use lightningcss::values::percentage::DimensionPercentage;
    match lp {
        DimensionPercentage::Dimension(lv) => lc_length_value(lv, unit_context),
        DimensionPercentage::Calc(_calc) => {
            // Fallback: wenn parent_width bekannt ist, nimm 100% davon
            // (besser als 0)
            unit_context.parent_width.map(|pw| pw * 0.9)
        }
        DimensionPercentage::Percentage(p) => {
            if p.0 == 0.0 {
                Some(0.0)
            } else {
                unit_context.parent_width.map(|pw| p.0 * pw)
            }
        },
    }
}

fn lc_lpa(lpa: &lightningcss::values::length::LengthPercentageOrAuto, unit_context: &UnitContext) -> Option<f32> {
    use lightningcss::values::length::LengthPercentageOrAuto;
    match lpa {
        LengthPercentageOrAuto::Auto       => None,
        LengthPercentageOrAuto::LengthPercentage(lp) => lc_lp(lp, unit_context),
    }
}

fn lc_length_pct(
    s: &lightningcss::properties::size::Size, unit_context: &UnitContext
) -> Option<f32> {
    use lightningcss::properties::size::Size;
    match s {
        Size::LengthPercentage(lp) => lc_lp(lp, unit_context),
        _ => None,
    }
}

fn lc_max_size(s: &lightningcss::properties::size::MaxSize, unit_context: &UnitContext) -> Option<f32> {
    use lightningcss::properties::size::MaxSize;
    match s {
        MaxSize::LengthPercentage(lp) => lc_lp(lp, unit_context),
        _ => None,
    }
}

fn lc_color(c: &lightningcss::values::color::CssColor) -> Option<u32> {
    use lightningcss::values::color::CssColor;
    // Erst direkt versuchen
    if let CssColor::RGBA(rgba) = c {
        let r = rgba.red as u32;
        let g = rgba.green as u32;
        let b = rgba.blue as u32;
        return Some((r << 16) | (g << 8) | b);
    }
    // Dann über to_rgb() konvertieren (oklch, lab, etc.)
    if let Ok(CssColor::RGBA(rgba)) = c.to_rgb() {
        let r = rgba.red as u32;
        let g = rgba.green as u32;
        let b = rgba.blue as u32;
        return Some((r << 16) | (g << 8) | b);
    }
    None
}

fn lc_font_size(fs: &lightningcss::properties::font::FontSize, unit_context: &UnitContext) -> Option<f32> {
    use lightningcss::properties::font::FontSize;
    match fs {
        FontSize::Length(lp) => lc_lp(lp, unit_context),
        FontSize::Absolute(abs) => {
            use lightningcss::properties::font::AbsoluteFontSize;
            Some(match abs {
                AbsoluteFontSize::XXSmall => 9.0,
                AbsoluteFontSize::XSmall  => 10.0,
                AbsoluteFontSize::Small   => 13.0,
                AbsoluteFontSize::Medium  => 16.0,
                AbsoluteFontSize::Large   => 18.0,
                AbsoluteFontSize::XLarge  => 24.0,
                AbsoluteFontSize::XXLarge => 32.0,
                AbsoluteFontSize::XXXLarge => 48.0,
            })
        }
        _ => None,
    }
}

fn lc_border_width(bw: &lightningcss::properties::border::BorderSideWidth, unit_context: &UnitContext) -> Option<f32> {
    use lightningcss::properties::border::BorderSideWidth;
    match bw {
        BorderSideWidth::Length(l) => lc_length(l, unit_context),
        BorderSideWidth::Thin   => Some(1.0),
        BorderSideWidth::Medium => Some(3.0),
        BorderSideWidth::Thick  => Some(5.0),
    }
}

/// Gibt den Prozentwert (0.0–1.0) zurück wenn Size eine Prozentangabe ist, sonst None.
fn lc_size_percent(s: &lightningcss::properties::size::Size) -> Option<f32> {
    use lightningcss::properties::size::Size;
    use lightningcss::values::percentage::DimensionPercentage;
    if let Size::LengthPercentage(lp) = s {
        if let DimensionPercentage::Percentage(p) = lp {
            if p.0 > 0.0 {
                return Some(p.0);
            }
        }
    }
    None
}

fn lc_gap(g: &lightningcss::properties::align::GapValue, unit_context: &UnitContext) -> Option<f32> {
    use lightningcss::properties::align::GapValue;
    match g {
        GapValue::LengthPercentage(lp) => lc_lp(lp, unit_context),
        GapValue::Normal => None,
    }
}

fn lc_align_items(ai: &lightningcss::properties::align::AlignItems) -> Option<AlignItems> {
    use lightningcss::properties::align::{AlignItems as LcAI, SelfPosition};
    match ai {
        LcAI::Normal | LcAI::Stretch => Some(AlignItems::Stretch),
        LcAI::BaselinePosition(_)     => Some(AlignItems::Baseline),
        LcAI::SelfPosition { value: pos, .. } => Some(match pos {
            SelfPosition::Center              => AlignItems::Center,
            SelfPosition::FlexEnd | SelfPosition::End => AlignItems::FlexEnd,
            SelfPosition::FlexStart | SelfPosition::Start => AlignItems::FlexStart,
            _ => AlignItems::Stretch,
        }),
    }
}

fn lc_track_list_to_values(
    tl: &lightningcss::properties::grid::TrackList,
    unit_context: &UnitContext
) -> Vec<GridTemplateValue> {
    let mut out = vec![];
    for item in &tl.items {
        if let lightningcss::properties::grid::TrackListItem::TrackSize(ts) = item {
            out.push(lc_track_size(ts, unit_context));
        }
    }
    out
}

fn lc_track_size_list_to_values(
    tsl: &lightningcss::properties::grid::TrackSizeList,
    unit_context: &UnitContext
) -> Vec<GridTemplateValue> {
    tsl.0.iter().map(|ts| lc_track_size(ts, unit_context)).collect()
}

fn lc_track_size(ts: &lightningcss::properties::grid::TrackSize, unit_context: &UnitContext) -> GridTemplateValue {
    use lightningcss::properties::grid::TrackSize;
    use lightningcss::values::percentage::DimensionPercentage;

    // lightningcss TrackSize ist MinMax { min, max } oder ein einzelner TrackBreadth
    // Je nach Version wird TrackSize direkt als TrackBreadth behandelt oder als MinMax-Wrapper
    // Wir matchen auf TrackBreadth-Varianten via den MinMax-min-Wert als Fallback
    match ts {
        TrackSize::TrackBreadth(breadth) => lc_track_breadth(breadth, unit_context),
        TrackSize::MinMax { min: _, max } => lc_track_breadth(max, unit_context),
        TrackSize::FitContent(lp) => {
            match lp {
                DimensionPercentage::Dimension(lv) => {
                    GridTemplateValue::Length(lc_length_value(lv, unit_context).unwrap_or(0.0))
                }
                DimensionPercentage::Percentage(p) => GridTemplateValue::Percent(p.0),
                _ => GridTemplateValue::Auto,
            }
        }
    }
}

fn lc_track_breadth(breadth: &lightningcss::properties::grid::TrackBreadth, unit_context: &UnitContext) -> GridTemplateValue {
    use lightningcss::properties::grid::TrackBreadth;
    use lightningcss::values::percentage::DimensionPercentage;

    match breadth {
        TrackBreadth::Length(lp) => {
            match lp {
                DimensionPercentage::Dimension(lv) => {
                    GridTemplateValue::Length(lc_length_value(lv, unit_context).unwrap_or(0.0))
                }
                DimensionPercentage::Percentage(p) => GridTemplateValue::Percent(p.0),
                _ => GridTemplateValue::Auto,
            }
        }
        TrackBreadth::Flex(f) => GridTemplateValue::Flex(*f),
        TrackBreadth::Auto => GridTemplateValue::Auto,
        TrackBreadth::MinContent => GridTemplateValue::MinContent,
        TrackBreadth::MaxContent => GridTemplateValue::MaxContent,
    }
}

fn lc_grid_line(gl: &lightningcss::properties::grid::GridLine) -> Option<i32> {
    use lightningcss::properties::grid::GridLine;
    match gl {
        GridLine::Line { index: v, name: _ } => Some(*v as i32),
        _ => None,
    }
}

/// Inline-Style parsen (für style="...") mit optionaler var()-Auflösung
pub fn parse_declarations(decl_str: &str) -> Vec<Declaration> {
    parse_declarations_with_vars(decl_str, None)
}

pub fn parse_declarations_with_vars(
    decl_str: &str,
    vars: Option<&std::collections::HashMap<String, String>>,
) -> Vec<Declaration> {
    let resolved = if let Some(v) = vars {
        resolve_vars(decl_str, v)
    } else {
        decl_str.to_string()
    };
    let wrapped = format!("*{{{}}}", resolved);
    let sheet = parse_css_resolved(&wrapped);
    sheet.rules.into_iter().flat_map(|r| r.declarations).collect()
}

// ─── CSS Cascade & Spezifitäts-Auflösung ────────────────────────────────────
//
// Implementiert den CSS-Cascade-Algorithmus nach Spec:
//   1. !important schlägt alle normalen Declarations
//   2. Höhere Spezifität gewinnt bei gleicher Importance
//   3. Spätere source_order gewinnt bei Gleichstand (later wins)
//
// Verwendung:
//   let matched = vec![
//       MatchedRule { declarations: &ua_decls,     specificity: Specificity(0,0,1), source_order: 0,  important: false },
//       MatchedRule { declarations: &author_decls,  specificity: Specificity(0,1,0), source_order: 5,  important: false },
//       MatchedRule { declarations: &inline_decls,  specificity: Specificity(1,0,0), source_order: 99, important: false },
//   ];
//   let computed: ComputedStyle = cascade(matched);
//   // computed.get(&PropertyKey::Color) → gewinnende Color-Declaration

/// Eine passende Regel mit Spezifität und Stylesheet-Position.
/// `source_order` = Index der Regel im Stylesheet (0-basiert, höher = später).
/// `important` = true wenn die Regel aus einem !important-Block stammt.
#[derive(Debug, Clone)]
pub struct MatchedRule<'a> {
    pub declarations: &'a [Declaration],
    pub specificity:  Specificity,
    pub source_order: usize,
    pub important:    bool,
}

/// Property-Schlüssel: Discriminant von `Declaration` ohne Wert.
/// Wird als HashMap-Key in `ComputedStyle` verwendet.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum PropertyKey {
    Width, Height, WidthPercent, HeightPercent,
    MarginTop, MarginRight, MarginBottom, MarginLeft,
    MarginLeftAuto, MarginRightAuto,
    PaddingTop, PaddingRight, PaddingBottom, PaddingLeft,
    Color, BackgroundColor, FontSize, Display,
    FontWeight, TextDecoration, BorderColor, BorderWidth,
    TextAlign, LineHeight, MaxWidth,
    BorderRadius,
    BorderTopLeftRadius, BorderTopRightRadius,
    BorderBottomLeftRadius, BorderBottomRightRadius,
    LetterSpacing, Position,
    Top, Left, Right, Bottom,
    FlexDirection, JustifyContent, AlignItems, AlignSelf,
    FlexGrow, FlexShrink, FlexBasis, FlexWrap,
    Gap, ColumnGap, RowGap, Order,
    BoxSizingBorder, Visibility, Opacity, Overflow,
    ZIndex, WhiteSpace, FontFamily, BackgroundImageNone,
    BoxShadow, OutlineWidth, OutlineColor, CursorPointer,
    BorderTopWidth, BorderRightWidth, BorderBottomWidth, BorderLeftWidth,
    BorderTopColor, BorderRightColor, BorderBottomColor, BorderLeftColor,
    GridTemplateColumns, GridTemplateRows,
    GridAutoColumns, GridAutoRows,
    GridColumnStart, GridColumnEnd, GridRowStart, GridRowEnd,
    AspectRatio, BackgroundImageGradient,
}

impl Declaration {
    /// Gibt den Property-Schlüssel zurück (Discriminant ohne Wert).
    pub fn key(&self) -> PropertyKey {
        match self {
            Declaration::Width(_)                   => PropertyKey::Width,
            Declaration::Height(_)                  => PropertyKey::Height,
            Declaration::WidthPercent(_)            => PropertyKey::WidthPercent,
            Declaration::HeightPercent(_)           => PropertyKey::HeightPercent,
            Declaration::MarginTop(_)               => PropertyKey::MarginTop,
            Declaration::MarginRight(_)             => PropertyKey::MarginRight,
            Declaration::MarginBottom(_)            => PropertyKey::MarginBottom,
            Declaration::MarginLeft(_)              => PropertyKey::MarginLeft,
            Declaration::MarginLeftAuto             => PropertyKey::MarginLeftAuto,
            Declaration::MarginRightAuto            => PropertyKey::MarginRightAuto,
            Declaration::PaddingTop(_)              => PropertyKey::PaddingTop,
            Declaration::PaddingRight(_)            => PropertyKey::PaddingRight,
            Declaration::PaddingBottom(_)           => PropertyKey::PaddingBottom,
            Declaration::PaddingLeft(_)             => PropertyKey::PaddingLeft,
            Declaration::Color(_)                   => PropertyKey::Color,
            Declaration::BackgroundColor(_)         => PropertyKey::BackgroundColor,
            Declaration::FontSize(_)                => PropertyKey::FontSize,
            Declaration::Display(_)                 => PropertyKey::Display,
            Declaration::FontWeight(_)              => PropertyKey::FontWeight,
            Declaration::TextDecoration(_)          => PropertyKey::TextDecoration,
            Declaration::BorderColor(_)             => PropertyKey::BorderColor,
            Declaration::BorderWidth(_)             => PropertyKey::BorderWidth,
            Declaration::TextAlign(_)               => PropertyKey::TextAlign,
            Declaration::LineHeight(_)              => PropertyKey::LineHeight,
            Declaration::MaxWidth(_)                => PropertyKey::MaxWidth,
            Declaration::BorderRadius(_)            => PropertyKey::BorderRadius,
            Declaration::BorderTopLeftRadius(_)     => PropertyKey::BorderTopLeftRadius,
            Declaration::BorderTopRightRadius(_)    => PropertyKey::BorderTopRightRadius,
            Declaration::BorderBottomLeftRadius(_)  => PropertyKey::BorderBottomLeftRadius,
            Declaration::BorderBottomRightRadius(_) => PropertyKey::BorderBottomRightRadius,
            Declaration::LetterSpacing(_)           => PropertyKey::LetterSpacing,
            Declaration::Position(_)                => PropertyKey::Position,
            Declaration::Top(_)                     => PropertyKey::Top,
            Declaration::Left(_)                    => PropertyKey::Left,
            Declaration::Right(_)                   => PropertyKey::Right,
            Declaration::Bottom(_)                  => PropertyKey::Bottom,
            Declaration::FlexDirection(_)           => PropertyKey::FlexDirection,
            Declaration::JustifyContent(_)          => PropertyKey::JustifyContent,
            Declaration::AlignItems(_)              => PropertyKey::AlignItems,
            Declaration::AlignSelf(_)               => PropertyKey::AlignSelf,
            Declaration::FlexGrow(_)                => PropertyKey::FlexGrow,
            Declaration::FlexShrink(_)              => PropertyKey::FlexShrink,
            Declaration::FlexBasis(_)               => PropertyKey::FlexBasis,
            Declaration::FlexWrap(_)                => PropertyKey::FlexWrap,
            Declaration::Gap(_)                     => PropertyKey::Gap,
            Declaration::ColumnGap(_)               => PropertyKey::ColumnGap,
            Declaration::RowGap(_)                  => PropertyKey::RowGap,
            Declaration::Order(_)                   => PropertyKey::Order,
            Declaration::BoxSizingBorder            => PropertyKey::BoxSizingBorder,
            Declaration::Visibility(_)              => PropertyKey::Visibility,
            Declaration::Opacity(_)                 => PropertyKey::Opacity,
            Declaration::Overflow(_)                => PropertyKey::Overflow,
            Declaration::ZIndex(_)                  => PropertyKey::ZIndex,
            Declaration::WhiteSpace(_)              => PropertyKey::WhiteSpace,
            Declaration::FontFamily(_)              => PropertyKey::FontFamily,
            Declaration::BackgroundImageNone        => PropertyKey::BackgroundImageNone,
            Declaration::BoxShadow { .. }           => PropertyKey::BoxShadow,
            Declaration::BorderTopWidth(_)          => PropertyKey::BorderTopWidth,
            Declaration::BorderRightWidth(_)        => PropertyKey::BorderRightWidth,
            Declaration::BorderBottomWidth(_)       => PropertyKey::BorderBottomWidth,
            Declaration::BorderLeftWidth(_)         => PropertyKey::BorderLeftWidth,
            Declaration::BorderTopColor(_)          => PropertyKey::BorderTopColor,
            Declaration::BorderRightColor(_)        => PropertyKey::BorderRightColor,
            Declaration::BorderBottomColor(_)       => PropertyKey::BorderBottomColor,
            Declaration::BorderLeftColor(_)         => PropertyKey::BorderLeftColor,
            Declaration::OutlineWidth(_)            => PropertyKey::OutlineWidth,
            Declaration::OutlineColor(_)            => PropertyKey::OutlineColor,
            Declaration::CursorPointer              => PropertyKey::CursorPointer,
            Declaration::GridTemplateColumns(_)     => PropertyKey::GridTemplateColumns,
            Declaration::GridTemplateRows(_)        => PropertyKey::GridTemplateRows,
            Declaration::GridAutoColumns(_)         => PropertyKey::GridAutoColumns,
            Declaration::GridAutoRows(_)            => PropertyKey::GridAutoRows,
            Declaration::GridColumnStart(_)         => PropertyKey::GridColumnStart,
            Declaration::GridColumnEnd(_)           => PropertyKey::GridColumnEnd,
            Declaration::GridRowStart(_)            => PropertyKey::GridRowStart,
            Declaration::GridRowEnd(_)              => PropertyKey::GridRowEnd,
            Declaration::AspectRatio(_)             => PropertyKey::AspectRatio,
            Declaration::BackgroundImageGradient { .. } => PropertyKey::BackgroundImageGradient,
        }
    }
}

/// Vollständig aufgelöste Computed-Style-Map: Property → gewinnende Declaration.
pub type ComputedStyle = std::collections::HashMap<PropertyKey, Declaration>;

/// CSS-Cascade: Ermittelt für jede Property die gewinnende Declaration.
///
/// `matched` sollte in Stylesheet-Reihenfolge übergeben werden
/// (source_order = 0 ist die erste Regel im Stylesheet).
///
/// Cascade-Priorität (höher gewinnt):
///   1. !important  (important: true schlägt important: false)
///   2. Spezifität  (Specificity(1,0,0) > Specificity(0,1,0))
///   3. Reihenfolge (source_order 10 > source_order 5)
pub fn cascade(matched: Vec<MatchedRule<'_>>) -> ComputedStyle {
    struct Winner {
        decl:         Declaration,
        important:    bool,
        specificity:  Specificity,
        source_order: usize,
    }

    let mut winners: std::collections::HashMap<PropertyKey, Winner> =
        std::collections::HashMap::new();

    for rule in &matched {
        for decl in rule.declarations {
            let key = decl.key();

            let beats_current = match winners.get(&key) {
                None => true,
                Some(current) => match (rule.important, current.important) {
                    // !important schlägt normal, egal welche Spezifität
                    (true,  false) => true,
                    (false, true)  => false,
                    // Gleiche Importance: höhere Spezifität gewinnt
                    _ => {
                        if rule.specificity != current.specificity {
                            rule.specificity > current.specificity
                        } else {
                            // Tiebreaker: spätere Regel gewinnt (later wins)
                            rule.source_order >= current.source_order
                        }
                    }
                },
            };

            if beats_current {
                winners.insert(key, Winner {
                    decl:         decl.clone(),
                    important:    rule.important,
                    specificity:  rule.specificity,
                    source_order: rule.source_order,
                });
            }
        }
    }

    winners.into_iter().map(|(k, w)| (k, w.decl)).collect()
}

/// Wendet den Cascade auf ein Stylesheet an, gegeben die Indizes der passenden
/// Regeln und optionale Inline-Styles.
///
/// # Parameter
/// - `stylesheet`: Das Gesamt-Stylesheet
/// - `rule_indices`: Indizes der Regeln die auf das Element matchen (in source_order)
/// - `inline_style`: Inline style="..." Declarations (gewinnen immer)
/// - `important_inline`: !important Inline-Declarations (höchste Priorität überhaupt)
pub fn cascade_stylesheet<'a>(
    stylesheet: &'a Stylesheet,
    rule_indices: &[usize],
    inline_style: Option<&'a [Declaration]>,
    important_inline: Option<&'a [Declaration]>,
) -> ComputedStyle {
    let mut matched: Vec<MatchedRule<'a>> = rule_indices
        .iter()
        .map(|&i| {
            let rule = &stylesheet.rules[i];
            // !important-Declarations aus lightningcss landen bereits in
            // `block.important_declarations` und wurden in lc_declarations_to_ours
            // zusammengeführt. Für feinere Kontrolle hier important: false setzen
            // und die Trennung später einbauen.
            MatchedRule {
                declarations: &rule.declarations,
                specificity:  rule.selector.specificity(),
                source_order: i,
                important:    false,
            }
        })
        .collect();

    // Inline-Styles: Spezifität (1,0,0,0) — schlagen alle Author-Selektoren.
    // Wir modellieren das als sehr hohe Spezifität + maximale source_order.
    if let Some(inline) = inline_style {
        matched.push(MatchedRule {
            declarations: inline,
            specificity:  Specificity(1_000, 0, 0),
            source_order: usize::MAX - 1,
            important:    false,
        });
    }

    // !important Inline-Styles: höchste Priorität überhaupt.
    if let Some(imp) = important_inline {
        matched.push(MatchedRule {
            declarations: imp,
            specificity:  Specificity(1_000, 0, 0),
            source_order: usize::MAX,
            important:    true,
        });
    }

    cascade(matched)
}