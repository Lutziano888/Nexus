use layout_engine::{
    dom::Node,
    cssom::{Stylesheet, Rule, Selector},
    layout::{build_layout_tree_with_viewport, LayoutBox},
};

use layout_engine::cssom::{
    Declaration::*,
    DisplayValue,
    FlexDirection,   // direkt, kein Alias nötig
    AlignItems,
};



pub const HEADER_H:    f32 = 46.0;   // Toolbar-Höhe (etwas mehr Platz für URL-Bar)
pub const STATUS_H:    f32 = 22.0;   // Statusleiste
pub const SIDEBAR_PCT: f32 = 0.0;    // Seitenleiste deaktiviert

pub fn get_user_agent_stylesheet() -> String {
    r#"
    /* ── Reset & Box Model ─────────────────────────────────────── */
    * { box-sizing: border-box; }
    html { min-height: 100%; background-color: #f9fafb; }
    body { 
        margin: 0; padding: 24px; 
        font-size: 16px; line-height: 1.6;
        font-family: 'Inter', 'Segoe UI', Roboto, Helvetica, Arial, sans-serif; 
        color: #1f2937; 
        background-color: #ffffff; 
    }

    /* ── Block-Elemente ────────────────────────────────────────── */
    html, body, div, section, article, aside, nav, main, header, footer,
    form, fieldset, details, summary, figure, figcaption,
    address, blockquote, pre, hr, canvas, video, audio {
        display: block;
    }

    /* ── Überschriften ─────────────────────────────────────────── */
    h1 { font-size: 2.5rem; margin: 0 0 1.5rem 0; font-weight: 800; color: #111827; letter-spacing: -0.025em; }
    h2 { font-size: 1.875rem; margin: 2rem 0 1rem 0; font-weight: 700; color: #1f2937; letter-spacing: -0.025em; }
    h3 { font-size: 1.5rem; margin: 1.5rem 0 0.75rem 0; font-weight: 600; color: #374151; }

    /* ── Absatz & Text ─────────────────────────────────────────── */
    p  { margin-bottom: 1.25rem; }
    a  { color: #2563eb; text-decoration: none; font-weight: 500; }
    a:hover { text-decoration: underline; color: #1d4ed8; }
    a:visited { color: #551a8b; }
    strong, b { font-weight: bold; }
    em, i     { font-style: italic; }
    small     { font-size: 13px; }
    sub       { font-size: 12px; }
    sup       { font-size: 12px; }
    code, kbd, samp, tt { font-family: monospace; font-size: 14px; }
    pre  { font-family: monospace; white-space: pre; margin-top: 16px; margin-bottom: 16px;
           padding: 12px; background-color: #f6f8fa; border-radius: 6px;
           border: 1px solid #d0d7de; overflow: auto; }
    blockquote {
        margin-top: 16px; margin-bottom: 16px;
        margin-left: 40px; margin-right: 40px;
        padding-left: 16px;
        border-left: 4px solid #d0d7de;
        color: #57606a;
    }
    hr { border: none; border-top: 1px solid #d0d7de; margin-top: 24px; margin-bottom: 24px; }
    abbr { text-decoration: underline dotted; cursor: help; }
    mark { background-color: #fff3cd; color: #1f1f1f; }

    /* ── Listen ────────────────────────────────────────────────── */
    ul, ol { margin-top: 16px; margin-bottom: 16px; padding-left: 40px; }
    ul ul, ul ol, ol ul, ol ol { margin-top: 0px; margin-bottom: 0px; }
    li { display: list-item; margin-bottom: 4px; }
    ul { list-style-type: disc; }
    ol { list-style-type: decimal; }
    ul ul  { list-style-type: circle; }
    ul ul ul { list-style-type: square; }
    dl { margin-top: 16px; margin-bottom: 16px; }
    dt { font-weight: bold; }
    dd { margin-left: 40px; }

    /* ── Formulare ─────────────────────────────────────────────── */
    input, textarea, select, button {
        font-size: 14px;
        font-family: sans-serif;
        border-radius: 4px;
        outline: none;
    }
    input, textarea, select {
        padding: 6px 10px;
        background-color: #ffffff;
        color: #1f1f1f;
        border: 1px solid #c4c7c5;
        border-radius: 4px;
    }
    input:focus, textarea:focus, select:focus {
        border-color: #0b57d0;
        box-shadow: 0px 0px 0px 3px rgba(11, 87, 208, 0.2);
    }
    input[type="text"], input[type="search"], input[type="email"],
    input[type="password"], input[type="url"], input[type="number"] {
        height: 36px;
    }
    input[type="search"] {
        border-radius: 20px;
        padding-left: 14px;
        padding-right: 14px;
    }
    input[type="checkbox"], input[type="radio"] {
        width: 16px;
        height: 16px;
        padding: 0px;
    }
    input[type="submit"], input[type="button"], input[type="reset"] {
        cursor: pointer;
        background-color: #0b57d0;
        color: #ffffff;
        border: none;
        padding: 8px 16px;
        height: 36px;
    }
    button {
        cursor: pointer;
        padding: 8px 16px;
        height: 36px;
        background-color: #0b57d0;
        color: #ffffff;
        border: none;
        border-radius: 4px;
        font-weight: bold;
    }
    button:hover { background-color: #0842a0; }
    select {
        height: 36px;
        padding-right: 24px;
        cursor: pointer;
    }
    textarea { padding: 8px 10px; resize: vertical; min-height: 80px; }
    label { cursor: pointer; }
    fieldset { border: 1px solid #d0d7de; border-radius: 4px; padding: 12px 16px; }
    legend { font-weight: bold; padding-left: 4px; padding-right: 4px; }

    /* ── Tabellen ──────────────────────────────────────────────── */
    table {
        border-collapse: collapse;
        margin-top: 16px;
        margin-bottom: 16px;
        width: 100%;
    }
    th, td {
        padding: 8px 12px;
        border: 1px solid #d0d7de;
        text-align: left;
    }
    th {
        font-weight: bold;
        background-color: #f6f8fa;
        color: #1f1f1f;
    }
    tr:nth-child(even) { background-color: #f6f8fa; }
    caption { font-weight: bold; margin-bottom: 8px; text-align: left; }

    /* ── Medien ────────────────────────────────────────────────── */
    img  { display: inline-block; max-width: 100%; height: auto; border: none; }
    svg  { display: inline-block; }
    video, audio, canvas, iframe, embed, object {
        display: inline-block;
        max-width: 100%;
    }
    figure { margin-top: 16px; margin-bottom: 16px; margin-left: 40px; margin-right: 40px; }
    figcaption { font-size: 13px; color: #57606a; margin-top: 4px; text-align: center; }

    /* ── Sonstiges ─────────────────────────────────────────────── */
    details > summary { cursor: pointer; font-weight: bold; }
    [hidden] { display: none; }

    /* ── Visuelle Tiefe & Abwechslung ──────────────────────────── */
    /* Sections, Articles, Aside bekommen eine leichte Karte */
    article, section, aside {
        background-color: #ffffff;
        border: 1px solid #e5e7eb;
        border-radius: 12px;
        padding: 20px 24px;
        margin-top: 20px;
        margin-bottom: 20px;
        box-shadow: 0 1px 2px rgba(15,23,42,0.06);
    }

    /* Aside / Infoboxen */
    aside {
        background-color: #f8f9fa;
        border-left: 4px solid #d2d7de;
        padding: 16px 20px;
        margin-top: 12px;
        margin-bottom: 12px;
        border-radius: 0 10px 10px 0;
    }

    /* H1 bekommt eine farbige Unterlinie */
    h1 {
        border-bottom: 3px solid #1a73e8;
        padding-bottom: 8px;
    }
    h2 {
        border-bottom: 1px solid #e8eaed;
        padding-bottom: 6px;
    }

    /* Flex-Row Boxen (unsere float→flex Konvertierung) visuell trennen */
    .nexus-flex-row {
        gap: 16px;
        margin-top: 12px;
        margin-bottom: 12px;
    }
    .nexus-td {
        background-color: #ffffff;
        border: 1px solid #e8eaed;
        border-radius: 6px;
        padding: 16px;
    }

    /* Wikipedia-spezifisch: Infoboxen & Hauptseiten-Boxen */
    .infobox, .wikitable {
        background-color: #ffffff;
        border: 1px solid #d1d5db;
        border-radius: 8px;
        padding: 1rem;
        margin: 1.5rem 0;
        font-size: 0.875rem;
        box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.1), 0 2px 4px -1px rgba(0, 0, 0, 0.06);
    }
    .mw-parser-output {
        width: 100%;
        max-width: 1080px;
        margin: 0 auto;
        padding: 0 14px;
        background-color: transparent;
    }
    .mw-headline {
        margin-top: 24px;
        margin-bottom: 12px;
        font-size: 1.2em;
    }
    .mw-parser-output > p,
    .mw-parser-output > ul,
    .mw-parser-output > ol,
    .mw-parser-output > table,
    .mw-parser-output > div {
        margin-bottom: 18px;
    }
    .mw-parser-output table {
        width: 100%;
        border-collapse: collapse;
    }
    .mw-parser-output table th,
    .mw-parser-output table td {
        padding: 10px;
        border: 1px solid #e5e7eb;
    }
    .mw-parser-output .toc {
        display: block;
        background: #ffffff;
        border: 1px solid #e5e7eb;
        border-radius: 10px;
        padding: 14px;
        box-shadow: 0 1px 2px rgba(15,23,42,0.05);
    }
    "#.to_string()
}

pub fn build_chrome_layout(win_w: u32, win_h: u32) -> LayoutBox {
    let w     = win_w as f32;
    let h     = win_h as f32;
    let main_h = (h - HEADER_H - STATUS_H).max(0.0);

    let btn_back   = Node::element("div", vec![("id", "nav-back")],   vec![]);
    let btn_fwd    = Node::element("div", vec![("id", "nav-fwd")],    vec![]);
    let btn_reload = Node::element("div", vec![("id", "nav-reload")], vec![]);
    let urlbar     = Node::element("div", vec![("id", "urlbar")],     vec![]);

    let header = Node::element(
        "div",
        vec![("id", "header")],
        vec![btn_back, btn_fwd, btn_reload, urlbar],
    );

    let content   = Node::element("div", vec![("id", "content")], vec![]);
    let statusbar = Node::element("div", vec![("id", "statusbar")], vec![]);

    let root = Node::element(
        "div",
        vec![("id", "root")],
        vec![header, content, statusbar],
    );

    let btn_size   = 28.0_f32;
    let btn_margin = 6.0_f32;
    let urlbar_x   = btn_margin + (btn_size + btn_margin) * 3.0;
    let urlbar_w   = w - urlbar_x - 8.0;
    let btn_y      = (HEADER_H - btn_size) / 2.0;

    let stylesheet = Stylesheet {
        rules: vec![
            Rule { selector: Selector::Id("root".into()),
                declarations: vec![Width(w), Height(h)] },
            Rule { selector: Selector::Id("header".into()),
                declarations: vec![
                    Width(w), Height(HEADER_H),
                    Display(DisplayValue::Flex),
                    FlexDirection(FlexDirection::Row),
                    AlignItems(AlignItems::Center),
                ]
            },
            Rule { selector: Selector::Id("nav-back".into()),
                declarations: vec![Width(btn_size), Height(btn_size),
                                   MarginLeft(btn_margin), MarginTop(btn_y)] },
            Rule { selector: Selector::Id("nav-fwd".into()),
                declarations: vec![Width(btn_size), Height(btn_size),
                                   MarginLeft(btn_margin + btn_size + btn_margin),
                                   MarginTop(btn_y)] },
            Rule { selector: Selector::Id("nav-reload".into()),
                declarations: vec![Width(btn_size), Height(btn_size),
                                   MarginLeft(btn_margin + (btn_size + btn_margin) * 2.0),
                                   MarginTop(btn_y)] },
            Rule { selector: Selector::Id("urlbar".into()),
                declarations: vec![Width(urlbar_w), Height(24.0),
                                   MarginLeft(urlbar_x),
                                   MarginTop((HEADER_H - 24.0) / 2.0)] },
            Rule { selector: Selector::Id("content".into()),
                declarations: vec![Width(w), Height(main_h)] },
            Rule { selector: Selector::Id("statusbar".into()),
                declarations: vec![Width(w), Height(STATUS_H)] },
            Rule {
                selector: Selector::Id("search-input".into()),
                declarations: vec![
                    Width(400.0),
                    Height(34.0),
                    PaddingLeft(15.0),
                    BackgroundColor(0x00_FFFFFF),
                    BorderColor(0x00_DF_E1_E5),
                    BorderWidth(1.0)
                ]
            }
        ],
    };

    build_layout_tree_with_viewport(&root, &stylesheet, 0.0, 0.0, w, h)
}