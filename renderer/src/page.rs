// ─── Page Pipeline  v0.4  (Fix: externe CSS + inline styles) ─────────────────

use html_parser::{Node as HtmlNode, Element as HtmlElement};
use layout_engine::{
    dom::Node as LayoutNode,
    cssom::{Stylesheet, Rule, Selector, parse_css},
    cssom::Declaration::*,
    layout::{build_layout_tree_with_viewport, LayoutBox},
};
use crate::layout_bridge::{HEADER_H, STATUS_H, SIDEBAR_PCT};
use crate::js_runtime::JsRuntime;

// Thread-local base URL damit convert_element relative Bild-URLs auflösen kann
thread_local! {
    static BASE_URL: std::cell::RefCell<String> = std::cell::RefCell::new(String::new());
}

fn set_base_url(url: &str) {
    BASE_URL.with(|b| *b.borrow_mut() = url.to_string());
}

fn get_base_url() -> String {
    BASE_URL.with(|b| b.borrow().clone())
}
pub struct LoadedPage {
    pub layout: LayoutBox,
    pub title:  String,
}

/// Browser-Default-CSS (Chrome/Blink-ähnlich, massiv erweitert)
fn get_default_css() -> String {
    r#"
    :root {
      --nx-primary: #1a73e8;
      --nx-bg: #ffffff;
      --nx-border: #dadce0;
      --nx-radius: 8px;
    }
    *, *::before, *::after { box-sizing: border-box; }
    html, body { min-height: 100%; width: 100%; margin: 0; padding: 0; background-color: var(--nx-bg); }
    html { display: block; }
    body { font-family: system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif; font-size: 16px; color: #202124; line-height: 1.6; margin: 0; max-width: none; display: block; }

    /* ===== HEADINGS ===== */
    h1 { font-size: 2em; font-weight: bold; line-height: 1.25; margin-top: 0.67em; margin-bottom: 0.67em; display: block; }
    h2 { font-size: 1.5em; font-weight: bold; line-height: 1.3; margin-top: 0.83em; margin-bottom: 0.83em; display: block; }
    h3 { font-size: 1.17em; font-weight: bold; line-height: 1.4; margin-top: 1em; margin-bottom: 1em; display: block; }
    h4 { font-size: 1em; font-weight: bold; line-height: 1.4; margin-top: 1.33em; margin-bottom: 1.33em; display: block; }
    h5 { font-size: 0.83em; font-weight: bold; line-height: 1.4; margin-top: 1.67em; margin-bottom: 1.67em; display: block; }
    h6 { font-size: 0.67em; font-weight: bold; line-height: 1.4; margin-top: 2.33em; margin-bottom: 2.33em; display: block; }

    /* ===== TEXT ===== */
    p { margin-top: 1em; margin-bottom: 1em; display: block; }
    strong, b { font-weight: bold; }
    em, i { font-style: italic; }
    u { text-decoration: underline; }
    s, del { text-decoration: line-through; }
    small { font-size: 80%; }
    sub { vertical-align: sub; font-size: smaller; }
    sup { vertical-align: super; font-size: smaller; }
    mark { background-color: #ffff00; color: #000000; }
    ins { text-decoration: underline; color: #000000; background-color: #ffffe0; }
    abbr[title] { text-decoration: underline dotted; cursor: help; }

    /* ===== LINKS ===== */
    a { color: var(--nx-primary); text-decoration: none; cursor: pointer; }
    a:hover { text-decoration: underline; }

    /* ===== LISTS ===== */
    ul, ol { margin-top: 1em; margin-bottom: 1em; padding-left: 40px; display: block; color: #3c4043; }
    li { display: list-item; }
    ul ul, ol ul, ul ol, ol ol { margin-top: 0; margin-bottom: 0; }
    dl { margin-top: 1em; margin-bottom: 1em; display: block; }
    dt { font-weight: bold; }
    dd { margin-left: 40px; }

    /* ===== CODE & PRE ===== */
    code, samp, kbd { font-family: monospace, monospace; font-size: 1em; padding: 2px 4px; background-color: #f1f3f4; color: #202124; }
    pre { font-family: monospace, monospace; font-size: 13px; white-space: pre; margin-top: 1em; margin-bottom: 1em; padding: 16px; display: block; background-color: #282a2e; color: #e8eaed; overflow: auto; }
    pre code { background-color: transparent; color: inherit; padding: 0; font-size: inherit; }

    /* ===== TABLES ===== */
    table { margin-top: 1em; margin-bottom: 1em; border-collapse: separate; border-spacing: 2px; display: table; color: #3c4043; }
    thead { display: table-header-group; vertical-align: middle; border-color: inherit; }
    tbody { display: table-row-group; vertical-align: middle; border-color: inherit; }
    tfoot { display: table-footer-group; vertical-align: middle; border-color: inherit; }
    tr { display: table-row; vertical-align: inherit; border-color: inherit; }
    th { font-weight: bold; text-align: center; padding: 8px 12px; background-color: #f1f3f4; display: table-cell; vertical-align: inherit; }
    td { padding: 6px 12px; display: table-cell; vertical-align: inherit; }
    caption { text-align: center; padding: 8px; display: table-caption; }
    col { display: table-column; }
    colgroup { display: table-column-group; }
    /* Table borders */
    table, th, td { border-color: #dadce0; border-width: 0; }

    /* ===== FORMS ===== */
    form { display: block; margin-top: 0; margin-bottom: 1em; }
    fieldset { display: block; margin: 0 2px; padding: 0.35em 0.75em 0.625em; border: 2px groove #c0c0c0; }
    legend { display: block; padding: 0 2px; max-width: 100%; color: inherit; }
    label { cursor: default; }

    input, textarea, select, button { font-family: inherit; font-size: inherit; color: inherit; }
    button { 
      background: linear-gradient(to bottom, #ffffff, #f1f3f4);
      border: 1px solid var(--nx-border);
      border-radius: var(--nx-radius);
      padding: 8px 16px;
      cursor: pointer;
    }
    button:hover { background: #f1f3f4; }
    input[type="text"], input[type="password"], input[type="email"],
    input[type="search"], input[type="url"], input[type="number"], textarea, select {
        border: 1px solid #c4c7c5; background-color: #ffffff;
        padding: 6px 8px; margin: 0; border-radius: 2px;
        box-sizing: border-box; display: inline-block;
    }
    input[type="checkbox"], input[type="radio"] { margin: 3px 3px 3px 4px; }
    input[type="button"], input[type="submit"], input[type="reset"] {
        background-color: #1558ab; color: #ffffff; border: 1px solid #1558ab;
        padding: 8px 16px; cursor: pointer; border-radius: 4px; display: inline-block;
    }
    button, input[type="button"]:hover, input[type="submit"]:hover {
        background-color: #0b57d0; border-color: #0b57d0;
    }
    textarea { overflow: auto; vertical-align: top; resize: vertical; }
    select { padding: 6px 24px 6px 8px; appearance: menulist; }
    option { font-family: sans-serif; }

    /* ===== BLOCKQUOTE & CITE ===== */
    blockquote { margin-top: 1em; margin-bottom: 1em; margin-left: 40px; margin-right: 40px; padding: 12px 16px; display: block; background-color: #f8f9fa; border-left: 4px solid #1558ab; color: #3c4043; }
    q { quotes: '"' '"' "'" "'"; }
    cite { font-style: italic; color: #5f6368; }

    /* ===== RULES & BREAKS ===== */
    hr { border-style: inset; border-width: 1px; margin: 0.5em auto; border-color: #dadce0; display: block; }
    br { display: inline; }

    /* ===== MEDIA ===== */
    img, embed, object, video { display: inline-block; max-width: 100%; height: auto; }
    figure { display: block; margin: 1em 40px; }
    figcaption { display: block; text-align: center; font-size: 0.9em; color: #5f6368; margin-top: 8px; }

    /* ===== STRUCTURAL ===== */
    header, nav, main, section, article, aside, footer, address,
    details, summary, dialog {
        display: block;
    }
    nav { margin-top: 1em; margin-bottom: 1em; }
    main { margin-top: 0; margin-bottom: 1em; }
    article { margin-top: 1em; margin-bottom: 1em; }
    aside { margin-top: 1em; margin-bottom: 1em; }
    footer { margin-top: 1em; margin-bottom: 1em; color: #5f6368; font-size: 0.9em; border-top: 1px solid #dadce0; padding-top: 16px; }
    address { font-style: italic; margin-top: 1em; margin-bottom: 1em; }
    details { display: block; margin-top: 1em; margin-bottom: 1em; }
    summary { display: list-item; cursor: pointer; font-weight: bold; }
    dialog { position: absolute; left: 0; right: 0; width: fit-content; height: fit-content; margin: auto; border: 1px solid #c0c0c0; padding: 1em; background-color: #ffffff; }

    /* ===== SEMANTIC ===== */
    div { display: block; }
    span { display: inline; }
    center { display: block; text-align: center; }
    hr { display: block; unicode-bidi: embed; margin: 0.5em auto; border-style: inset; border-width: 1px; }
    noscript { display: none; }

    /* ===== METER & PROGRESS ===== */
    progress { display: inline-block; vertical-align: baseline; }
    meter { display: inline-block; vertical-align: baseline; }

    /* ===== LIST ITEMS ===== */
    li { margin-bottom: 4px; display: list-item; }
    ul li { list-style-type: disc; }
    ul ul li { list-style-type: circle; }
    ul ul ul li { list-style-type: square; }
    ol li { list-style-type: decimal; }

    /* ===== HEAD & META (should be hidden) ===== */
    head { display: none; }
    title { display: none; }
    meta { display: none; }
    link { display: none; }
    style { display: none; }
    script { display: none; }
    "#.to_string()
}

#[derive(Debug)]
pub enum PageError {
    Network(String),
    EmptyBody,
}

impl std::fmt::Display for PageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PageError::Network(e) => write!(f, "Netzwerkfehler: {}", e),
            PageError::EmptyBody  => write!(f, "Leere Antwort"),
        }
    }
}

fn decode_entities(s: &str) -> String {
    let s = s
        .replace("&middot;", "·").replace("&nbsp;",   " ")
        .replace("&amp;",    "&").replace("&lt;",     "<")
        .replace("&gt;",     ">").replace("&quot;",   "\"")
        .replace("&apos;",   "'").replace("&copy;",   "©")
        .replace("&reg;",    "®").replace("&trade;",  "™")
        .replace("&mdash;",  "—").replace("&ndash;",  "–")
        .replace("&laquo;",  "«").replace("&raquo;",  "»")
        .replace("&hellip;", "…").replace("&euro;",   "€")
        .replace("&#160;",   " ");

    let mut result = String::with_capacity(s.len());
    let mut chars  = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' && chars.peek() == Some(&'#') {
            chars.next();
            let mut num_str = String::new();
            while let Some(&d) = chars.peek() {
                if d.is_ascii_digit() { num_str.push(d); chars.next(); }
                else { break; }
            }
            if chars.peek() == Some(&';') { chars.next(); }
            if let Ok(n) = num_str.parse::<u32>() {
                if let Some(decoded) = char::from_u32(n) { result.push(decoded); continue; }
            }
            result.push('&'); result.push('#'); result.push_str(&num_str);
        } else { result.push(c); }
    }
    result
}

/// Inline <style>-Tags extrahieren
fn extract_style_css(node: &HtmlNode) -> String {
    match node {
        HtmlNode::Text(_) => String::new(),
        HtmlNode::Element(e) => {
            let mut css = String::new();
            if e.tag_name == "style" {
                for child in &e.children {
                    if let HtmlNode::Text(t) = child { css.push_str(t); }
                }
            }
            for child in &e.children { css.push_str(&extract_style_css(child)); }
            css
        }
    }
}

// 2. extract_scripts Funktion hinzufügen (nach extract_style_css)
fn extract_scripts(node: &HtmlNode) -> Vec<String> {
    match node {
        HtmlNode::Text(_) => vec![],
        HtmlNode::Element(e) => {
            let mut out = vec![];
            if e.tag_name == "script" && !e.attributes.contains_key("src") {
                for child in &e.children {
                    if let HtmlNode::Text(t) = child { out.push(t.clone()); }
                }
            }
            for child in &e.children { out.extend(extract_scripts(child)); }
            out
        }
    }
}

/// Externe CSS-Links (<link rel="stylesheet">) sammeln
fn extract_css_links(node: &HtmlNode, base_url: &str) -> Vec<String> {
    match node {
        HtmlNode::Text(_) => vec![],
        HtmlNode::Element(e) => {
            let mut links = vec![];
            if e.tag_name == "link" {
                let rel = e.attributes.get("rel").map(|s| s.as_str()).unwrap_or("");
                let as_attr = e.attributes.get("as").map(|s| s.as_str()).unwrap_or("");
                let is_stylesheet_link = rel.contains("stylesheet")
                    || (rel.contains("preload") && as_attr == "style");
                if is_stylesheet_link {
                    if let Some(href) = e.attributes.get("href") {
                        let href = href.as_str();
                        // Keine data:-URIs, keine reinen Schriftarten-Domains (fonts.googleapis.com etc.)
                        let is_font_only = href.contains("fonts.googleapis.com")
                            || href.contains("fonts.gstatic.com")
                            || href.ends_with(".woff")
                            || href.ends_with(".woff2")
                            || href.ends_with(".ttf");
                        if !href.starts_with("data:") && !is_font_only {
                            let href = href.replace("&amp;", "&");
                            let full = resolve_url(base_url, &href);
                            links.push(full);
                        }
                    }
                }
            }
            for child in &e.children {
                links.extend(extract_css_links(child, base_url));
            }
            links
        }
    }
}

/// Inline style="..." Attribute zu echtem CSS konvertieren
fn extract_inline_styles(node: &mut HtmlNode, counter: &mut usize) -> String {
    match node {
        HtmlNode::Text(_) => String::new(),
        HtmlNode::Element(e) => {
            let mut css = String::new();
            if let Some(style_val) = e.attributes.get("style").cloned() {
                if !style_val.is_empty() {
                    // Eindeutigen data-nexus-id vergeben
                    let id = format!("nexus-inline-{}", counter);
                    e.attributes.insert("data-nexus-id".to_string(), id.clone());
                    *counter += 1;
                    css.push_str(&format!("[data-nexus-id=\"{}\"] {{ {} }}\n", id, style_val));
                }
            }
            for child in &mut e.children {
                css.push_str(&extract_inline_styles(child, counter));
            }
            css
        }
    }
}

fn replace_viewport_units(css: &str, vw: f32, vh: f32) -> String {
    let mut result = String::with_capacity(css.len());
    let mut token  = String::new();
    let flush = |tok: &mut String, res: &mut String| {
        if tok.is_empty() { return; }
        let out = if tok.ends_with("vh") {
            tok[..tok.len()-2].trim().parse::<f32>()
                .map(|v| format!("{:.1}px", v * vh / 100.0))
                .unwrap_or_else(|_| tok.clone())
        } else if tok.ends_with("vw") {
            tok[..tok.len()-2].trim().parse::<f32>()
                .map(|v| format!("{:.1}px", v * vw / 100.0))
                .unwrap_or_else(|_| tok.clone())
        } else { tok.clone() };
        res.push_str(&out);
        tok.clear();
    };
    for c in css.chars() {
        if c.is_alphanumeric() || c == '.' || c == '-' { token.push(c); }
        else { flush(&mut token, &mut result); result.push(c); }
    }
    flush(&mut token, &mut result);
    result
}

/// CSS-Filter: entfernt problematische Konstrukte und normalisiert Layout-Properties
/// die unsere Engine nicht unterstützt (float, position:absolute, display:table-cell etc.)
/// zu block-kompatiblen Äquivalenten.
fn sanitize_page_css(css: &str) -> String {
    // Da wir jetzt Taffy (Flex/Grid) nutzen, müssen wir keine Properties mehr entfernen.
    // lightningcss und unser CSSOM ignorieren nicht unterstützte Properties ohnehin sicher.
    css.to_string()
}

/// URL auflösen (relativ → absolut)
fn resolve_url(base: &str, href: &str) -> String {
    if href.starts_with("http://") || href.starts_with("https://") {
        return href.to_string();
    }
    // data:, javascript:, mailto: niemals als relative URL behandeln
    if href.starts_with("data:") || href.starts_with("javascript:") || href.starts_with("mailto:") {
        return href.to_string();
    }
    if href.starts_with("//") {
        let scheme = if base.starts_with("https") { "https" } else { "http" };
        return format!("{}:{}", scheme, href);
    }
    // Origin ermitteln
    let origin = if let Some(rest) = base.strip_prefix("https://").or_else(|| base.strip_prefix("http://")) {
        let scheme = if base.starts_with("https") { "https" } else { "http" };
        let host = rest.split('/').next().unwrap_or(rest);
        format!("{}://{}", scheme, host)
    } else {
        base.to_string()
    };

    if href.starts_with('/') {
        format!("{}{}", origin, href)
    } else {
        // Relativer Pfad: Basis-Verzeichnis ermitteln
        // Wir suchen den letzten Slash, aber ignorieren die im Protokoll (http://)
        let path_start = base.find("//").map(|i| i + 2).unwrap_or(0);
        let last_slash = base[path_start..].rfind('/');
        
        let base_dir = if let Some(i) = last_slash {
            &base[..path_start + i]
        } else {
            base
        };
        
        format!("{}/{}", base_dir.trim_end_matches('/'), href)
    }
}

pub fn load_page(url: &str, win_w: u32, win_h: u32) -> Result<LoadedPage, PageError> {
    let url = normalize_url(url);
    let active_url = url.clone();
    // Basis-URL setzen damit convert_element relative Bild-URLs auflösen kann
    set_base_url(&active_url);

    let response = if active_url.starts_with("file://") {
        // Lokale Datei laden
        let path = active_url.strip_prefix("file://").unwrap_or(&active_url);
        let content = std::fs::read_to_string(path)
            .map_err(|e| PageError::Network(format!("Datei konnte nicht gelesen werden: {}", e)))?;
        
        network_fetch::response::FetchResponse {
            url: active_url.clone(),
            status_code: 200,
            headers: std::collections::HashMap::new(),
            body: content,
            content_type: "text/html".to_string(),
        }
    } else {
        // Netzwerk-Request
        network_fetch::fetch_blocking(&active_url)
            .map_err(|e| PageError::Network(e.to_string()))?
    };

    if response.body.is_empty() { return Err(PageError::EmptyBody); }

    let mut html_dom = html_parser::parse(&response.body);
    let title    = extract_title(&html_dom).unwrap_or_else(|| url.clone());

    // Wir klonen hier nur einmal, um JS eine Arbeitskopie zu geben.
    // Das Original nutzen wir für das Layout.
    let scripts = extract_scripts(&html_dom);
    let mut js = JsRuntime::new(html_dom.clone());
    for src in scripts { js.run_script(&src); }

    // 1) Styles extrahieren
    let mut css_accum = extract_style_css(&html_dom);
    
    // 1.5) Inline-Attribute style="..." verarbeiten (erfordert mut html_dom)
    let mut inline_counter = 0;
    css_accum.push_str(&extract_inline_styles(&mut html_dom, &mut inline_counter));

    let inline_css = sanitize_page_css(&css_accum);

    // 2) Externe CSS-Links laden (Parallel)
    let css_links = extract_css_links(&html_dom, &url);
    let mut external_css = String::new();
    
    if !css_links.is_empty() {
        let rt = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|e| PageError::Network(format!("Tokio-Runtime Fehler: {}", e)))?;

        let fetches = css_links.iter().take(10).map(|link| {
            let link = link.clone();
            async move {
                if link.starts_with("file://") { return None; }
                println!("[CSS] Lade: {}", link);
                match network_fetch::fetch(&link).await {
                    Ok(r) if !r.body.is_empty() => Some(r.body),
                    Ok(_) => { println!("[CSS] Leer: {}", link); None }
                    Err(e) => { println!("[CSS] Fehler {}: {}", link, e); None }
                }
            }
        });

        let results = rt.block_on(futures::future::join_all(fetches));
        for body in results.into_iter().flatten() {
            let sanitized = sanitize_page_css(&body);
            external_css.push_str(&sanitized);
            external_css.push('\n');
        }
    }

    let content_h_px = (win_h as f32 - HEADER_H - STATUS_H).max(100.0);
    let content_w_px = win_w as f32;

    let mut combined = String::new();
    combined.push_str(&get_default_css());
    combined.push_str(&external_css);
    combined.push_str(&inline_css);
    let combined = replace_viewport_units(&combined, content_w_px, content_h_px);

    let css_rules  = parse_css(&combined);
    
    let render_root: &HtmlNode = &html_dom;
    let layout_dom = convert_node(render_root);
    println!("[DBG render_root tag={} id={:?} children={}]",
             match render_root {
                 HtmlNode::Element(e) => format!("{}", e.tag_name),
                 _ => "text".into()
             },
             match render_root {
                 HtmlNode::Element(e) => e.attributes.get("id"),
                 _ => None
             },
             match render_root {
                 HtmlNode::Element(e) => e.children.len(),
                 _ => 0
             }
    );
    let layout     = build_page_layout(layout_dom, win_w, win_h, css_rules);

    Ok(LoadedPage { layout, title })
}

fn normalize_url(url: &str) -> String {
    let url = url.trim();
    if url.starts_with("http://") || url.starts_with("https://") || url.starts_with("file://") {
        url.to_string()
    } else if url.contains(r":\") || url.starts_with('/') {
        format!("file://{}", url)
    } else {
        format!("https://{}", url)
    }
}


// ─── Content Scoring (generischer Readability-Algorithmus) ─────────────────────────────

fn count_text_len(elem: &HtmlElement) -> usize {
    let mut len = 0;
    for child in &elem.children {
        match child {
            HtmlNode::Text(t) => len += t.trim().len(),
            HtmlNode::Element(e) => len += count_text_len(e),
        }
    }
    len
}

/// Sucht zuerst nach bekannten Content-Selektoren (ID/Class), dann per Score.
/// Priorität: spezifische IDs > spezifische Classes > generischer Score
fn find_main_content<'a>(node: &'a HtmlNode, _threshold: i32) -> Option<&'a HtmlElement> {
    // Bekannte Content-IDs/Classes in Prioritätsreihenfolge
    // Wird direkt gesucht, umgeht den Score-Algorithmus
    let priority_ids    = ["mw-content-text", "bodyContent", "content-text",
        "article", "main-content", "page-content"];
    let priority_classes = ["mw-parser-output", "article-body", "entry-content",
        "post-content", "article-content"];

    // Schritt 1: Direktsuche nach Prioritäts-IDs
    for id in &priority_ids {
        if let Some(found) = find_by_id(node, id) {
            println!("[Content] Direkttreffer id={}", id);
            return Some(found);
        }
    }

    // Schritt 2: Direktsuche nach Prioritäts-Classes
    for cls in &priority_classes {
        if let Some(found) = find_by_class(node, cls) {
            println!("[Content] Direkttreffer class={}", cls);
            return Some(found);
        }
    }

    // Schritt 3: Fallback — generischer Score-Algorithmus
    find_by_score(node, 50)
}

fn find_by_id<'a>(node: &'a HtmlNode, target_id: &str) -> Option<&'a HtmlElement> {
    match node {
        HtmlNode::Text(_) => None,
        HtmlNode::Element(e) => {
            if e.attributes.get("id").map(|s| s.as_str()) == Some(target_id) {
                return Some(e);
            }
            for child in &e.children {
                if let Some(found) = find_by_id(child, target_id) {
                    return Some(found);
                }
            }
            None
        }
    }
}

fn find_by_tag<'a>(node: &'a HtmlNode, tag: &str) -> Option<&'a HtmlElement> {
    match node {
        HtmlNode::Text(_) => None,
        HtmlNode::Element(e) => {
            if e.tag_name == tag { return Some(e); }
            for child in &e.children {
                if let Some(found) = find_by_tag(child, tag) { return Some(found); }
            }
            None
        }
    }
}

fn find_by_class<'a>(node: &'a HtmlNode, target_class: &str) -> Option<&'a HtmlElement> {
    match node {
        HtmlNode::Text(_) => None,
        HtmlNode::Element(e) => {
            let classes = e.attributes.get("class").map(|s| s.as_str()).unwrap_or("");
            if classes.split_whitespace().any(|c| c == target_class) {
                return Some(e);
            }
            for child in &e.children {
                if let Some(found) = find_by_class(child, target_class) {
                    return Some(found);
                }
            }
            None
        }
    }
}

fn content_score(elem: &HtmlElement) -> i32 {
    let mut score: i32 = 0;

    score += match elem.tag_name.as_str() {
        "article" | "main" => 50,
        "section"          => 20,
        "nav" | "header" | "footer" | "aside" => -50,
        "ul" | "ol"        => -20,
        _                  =>  0,
    };

    let role = elem.attributes.get("role").map(|s| s.as_str()).unwrap_or("");
    score += match role {
        "main"                 =>  80,
        "article"              =>  50,
        "navigation" | "nav"   => -80,
        "banner" | "complementary" | "contentinfo" => -40,
        _                      =>   0,
    };

    let id    = elem.attributes.get("id").map(|s| s.as_str()).unwrap_or("");
    let class = elem.attributes.get("class").map(|s| s.as_str()).unwrap_or("");
    let combined = format!("{} {}", id, class).to_lowercase();

    for kw in &["mw-navigation", "mw-head", "mw-panel", "vector-menu",
        "vector-sidebar", "vector-header", "vector-page-tools",
        "vector-variants", "mw-portlet", "catlinks", "noprint",
        "printfooter", "nav", "menu", "sidebar", "toolbar",
        "breadcrumb", "toc", "share", "social", "comment"] {
        if combined.contains(kw) { score -= 60; }
    }

    let text_len = count_text_len(elem);
    score += (text_len / 100).min(150) as i32;

    score
}

fn find_by_score<'a>(node: &'a HtmlNode, threshold: i32) -> Option<&'a HtmlElement> {
    match node {
        HtmlNode::Text(_) => None,
        HtmlNode::Element(e) => {
            let score = content_score(e);
            let mut best: Option<(&HtmlElement, i32)> = if score >= threshold {
                Some((e, score))
            } else {
                None
            };
            for child in &e.children {
                if let Some(candidate) = find_by_score(child, threshold) {
                    let cs = content_score(candidate);
                    if cs > best.map(|(_, s)| s).unwrap_or(i32::MIN) + 10 {
                        best = Some((candidate, cs));
                    }
                }
            }
            best.map(|(e, _)| e)
        }
    }
}

fn convert_node(node: &HtmlNode) -> LayoutNode {
    match node {
        HtmlNode::Text(t) => {
            let d = decode_entities(t);
            if d.trim().is_empty() { LayoutNode::text("") } else { LayoutNode::text(&d) }
        }
        HtmlNode::Element(e) => convert_element(e),
    }
}

fn is_noise_element(elem: &HtmlElement) -> bool {
    let id    = elem.attributes.get("id").map(|s| s.as_str()).unwrap_or("");
    let _class = elem.attributes.get("class").map(|s| s.as_str()).unwrap_or("");

    // Exakte IDs die wirklich nur störende Overlays sind
    let noise_ids = [
        "siteNotice", "centralNotice", "mw-cookiewarning",
        "mw-site-toolbar",
        // Wikipedia Vector-Skin Navigation und Footer
        "mw-navigation", "mw-head", "mw-panel",
        "mw-sidebar-button", "vector-user-links",
        "p-lang-btn", "vector-page-tools-landm",
        "vector-main-menu-landmark", "vector-sticky-header",
        "footer", "catlinks",
    ];
    for noise_id in &noise_ids {
        if id == *noise_id { return true; }
    }
    if elem.attributes.get("role").map(|s| s.as_str()) == Some("navigation") {
        return true;
    }

    /* 
    // Diese Filter entfernen zu viel von der echten Wikipedia-Struktur
    let noise_ids_aggressive = [
        "mw-head-base", "mw-page-base", "p-lang-btn", "vector-page-tools", 
        "vector-toc", "left-navigation", "right-navigation", "mw-sidebar-button",
        "contentSub", "siteSub", "p-search", "p-tb", "p-coll-print_export", 
        "p-wikibase-otherprojects", "p-namespaces", "p-views", "p-variants",
    ];
    */

    false
}

/// Liest den `float`-Wert aus einem inline style-Attribut aus
fn get_float_value(elem: &HtmlElement) -> Option<&str> {
    let style = elem.attributes.get("style").map(|s| s.as_str()).unwrap_or("");
    for part in style.split(';') {
        let part = part.trim();
        if let Some(rest) = part.strip_prefix("float") {
            let val = rest.trim_start_matches([':',' ']).trim();
            if val == "left" || val == "right" { return Some(val); }
        }
    }
    None
}

fn convert_element(elem: &HtmlElement) -> LayoutNode {
    // Diese Tags werden komplett ignoriert
    if matches!(elem.tag_name.as_str(),
        "script" | "head" | "meta" | "noscript" | "template" | "svg" | "path" | "image" | "style"
    ) { return LayoutNode::text(""); }

    if elem.tag_name == "link" {
        return LayoutNode::text("");
    }

    if is_noise_element(elem) {
        return LayoutNode::text("");
    }

    // ── display:none im inline style → nicht rendern ─────────────────────
    // Auch hidden-Attribut berücksichtigen
    if elem.attributes.get("hidden").is_some() {
        return LayoutNode::text("");
    }
    if let Some(style_val) = elem.attributes.get("style") {
        // Einfacher Check: enthält "display" und "none" (mit beliebig viel Whitespace)
        let s = style_val.replace(' ', "").replace('\t', "").to_lowercase();
        if s.contains("display:none") {
            return LayoutNode::text("");
        }
    }

    // ── Bekannte "versteckte bis JS es aufklappt" Klassen filtern ─────────
    // Wikipedia und viele andere Sites nutzen diese Klassen für Inhalte die
    // normalerweise via JS aufgeklappt werden. Da unser JS-Runtime das nicht
    // tut, würden diese Elemente sichtbar sein obwohl sie es nicht sein sollten —
    // oder umgekehrt: Navigations-Dropdown-Inhalte erscheinen immer ausgeklappt.
    {
        let class = elem.attributes.get("class").map(|s| s.as_str()).unwrap_or("");
        let classes: Vec<&str> = class.split_whitespace().collect();

        // Klassen die "immer versteckt bis JS" bedeuten
        let always_hidden_classes = [
            // Wikipedia collapsible — nur wirklich versteckte
            "mw-collapsible-content",
            "mw-collapsed",
            // Generisch
            "dropdown-menu",
            "submenu",
            "sub-menu",
            "collapse",
            "js-hidden",
            "is-hidden",
        ];

        // "collapse" ohne "show" → Bootstrap collapse, normalerweise versteckt
        // Ausnahme: wenn "show" oder "in" zusätzlich gesetzt → sichtbar
        let has_bootstrap_collapse = classes.contains(&"collapse");
        let has_show = classes.contains(&"show") || classes.contains(&"in");

        for hidden_cls in &always_hidden_classes {
            if *hidden_cls == "collapse" {
                // Bootstrap: nur versteckt wenn "show" NICHT gesetzt
                if has_bootstrap_collapse && !has_show {
                    return LayoutNode::text("");
                }
            } else if classes.iter().any(|c| c == hidden_cls) {
                return LayoutNode::text("");
            }
        }

        // aria-hidden="true" nur bei dekorativen Elementen ohne echten Text anwenden.
        // Pauschal alle aria-hidden-Elemente zu verstecken entfernt zu viel Wikipedia-Inhalt.
        if elem.attributes.get("aria-hidden").map(|s| s.as_str()) == Some("true") {
            let tag = elem.tag_name.as_str();
            if matches!(tag, "span" | "i" | "svg" | "button") && count_text_len(elem) < 10 {
                return LayoutNode::text("");
            }
        }
    }

    // ── Tabellen-Layout → Flex konvertieren ──────────────────────────────
    // <table> → div.nexus-flex-col  (stapelt Zeilen)
    // <tr>    → div.nexus-flex-row  (Zellen nebeneinander)
    // <td>/<th> → div.nexus-flex-item
    // So werden Tabellen die als Layout-Grid dienen korrekt nebeneinander gerendert.
    match elem.tag_name.as_str() {
        "table" | "tbody" | "thead" | "tfoot" => {
            // Alle Zeilen sammeln, thead/tbody/tfoot transparent durchreichen
            let rows = collect_table_rows(elem);
            let row_nodes: Vec<LayoutNode> = rows.iter().map(|tr| convert_tr(tr)).collect();
            return LayoutNode::element("div", vec![("class", "nexus-table")], row_nodes);
        }
        "tr" => return convert_tr(elem),
        "td" | "th" => {
            let mut attrs: Vec<(&str, &str)> = elem.attributes.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
            attrs.push(("class", "nexus-td"));
            let children: Vec<LayoutNode> = elem.children.iter().map(convert_node).collect();
            return LayoutNode::element("div", attrs, children);
        }
        _ => {}
    }

    let base = get_base_url();

    // URL-Attribute auflösen (src für img, href für a)
    let attrs_owned: Vec<(String, String)> = elem.attributes.iter().map(|(k, v)| {
        let resolved = match k.as_str() {
            "src" => {
                // Lazy-loading: src ist oft data:-Platzhalter, echtes Bild in data-src
                if v.starts_with("data:") || v.is_empty() {
                    if let Some(real) = elem.attributes.get("data-src") {
                        resolve_url(&base, real)
                    } else if let Some(real) = elem.attributes.get("data-lazy-src") {
                        resolve_url(&base, real)
                    } else {
                        String::new()
                    }
                } else {
                    resolve_url(&base, v)
                }
            }
            "href" if elem.tag_name == "a" => resolve_url(&base, v),
            _ => v.clone(),
        };
        (k.clone(), resolved)
    }).collect();
    let attrs: Vec<(&str, &str)> = attrs_owned.iter()
        .map(|(k, v)| (k.as_str(), v.as_str()))
        .collect();

    // Float-Kinder → Flex-Row
    let has_float_children = elem.children.iter().any(|n| {
        matches!(n, HtmlNode::Element(e) if get_float_value(e).is_some())
    });

    if has_float_children {
        let mut result_children: Vec<LayoutNode> = Vec::new();
        let mut float_group: Vec<LayoutNode> = Vec::new();

        for child in &elem.children {
            let is_float = matches!(child, HtmlNode::Element(e) if get_float_value(e).is_some());
            if is_float {
                float_group.push(convert_node(child));
            } else {
                if !float_group.is_empty() {
                    let group = std::mem::take(&mut float_group);
                    result_children.push(make_flex_row(group));
                }
                result_children.push(convert_node(child));
            }
        }
        if !float_group.is_empty() {
            result_children.push(make_flex_row(float_group));
        }
        return LayoutNode::element(&elem.tag_name, attrs, result_children);
    }

    let children: Vec<LayoutNode> = elem.children.iter().map(convert_node).collect();
    LayoutNode::element(&elem.tag_name, attrs, children)
}

/// Sammelt alle <tr>-Elemente aus einem table/tbody/thead/tfoot rekursiv
fn collect_table_rows<'a>(elem: &'a HtmlElement) -> Vec<&'a HtmlElement> {
    let mut rows = Vec::new();
    for child in &elem.children {
        match child {
            HtmlNode::Element(e) => {
                if e.tag_name == "tr" {
                    rows.push(e);
                } else if matches!(e.tag_name.as_str(), "tbody"|"thead"|"tfoot"|"colgroup"|"caption") {
                    rows.extend(collect_table_rows(e));
                }
            }
            HtmlNode::Text(_) => {}
        }
    }
    rows
}

/// Konvertiert ein <tr> in einen Flex-Row-div
fn convert_tr(tr: &HtmlElement) -> LayoutNode {
    let cells: Vec<LayoutNode> = tr.children.iter().filter_map(|child| {
        match child {
            HtmlNode::Element(e) if matches!(e.tag_name.as_str(), "td"|"th") => {
                let colspan: usize = e.attributes.get("colspan")
                    .and_then(|v| v.parse().ok()).unwrap_or(1).max(1);
                let children: Vec<LayoutNode> = e.children.iter().map(convert_node).collect();
                // Bei colspan > 1: nexus-td-span-N für breitere Zellen
                let class = if colspan > 1 {
                    Box::leak(format!("nexus-td nexus-td-span-{}", colspan).into_boxed_str()) as &str
                } else {
                    "nexus-td"
                };
                Some(LayoutNode::element("div", vec![("class", class)], children))
            }
            _ => None,
        }
    }).collect();

    if cells.is_empty() { return LayoutNode::text(""); }

    // Einzeilige Tabelle mit nur einer Zelle → kein flex nötig
    if cells.len() == 1 {
        return LayoutNode::element("div", vec![("class", "nexus-tr-single")], cells);
    }

    LayoutNode::element("div", vec![("class", "nexus-flex-row")], cells)
}

/// Wickelt eine Liste von Knoten in einen display:flex; flex-direction:row Container
fn make_flex_row(children: Vec<LayoutNode>) -> LayoutNode {
    // Kinder in gleichbreite flex-items wrappen
    let wrapped: Vec<LayoutNode> = children.into_iter().map(|child| {
        LayoutNode::element("div", vec![("class", "nexus-flex-item")], vec![child])
    }).collect();
    LayoutNode::element("div", vec![("class", "nexus-flex-row")], wrapped)
}

fn extract_title(node: &HtmlNode) -> Option<String> {
    match node {
        HtmlNode::Element(e) => {
            if e.tag_name == "title" {
                for child in &e.children {
                    if let HtmlNode::Text(t) = child { return Some(t.trim().to_string()); }
                }
            }
            for child in &e.children { if let Some(t) = extract_title(child) { return Some(t); } }
            None
        }
        HtmlNode::Text(_) => None,
    }
}

fn build_page_layout(page_root: LayoutNode, win_w: u32, win_h: u32, extra_css: Stylesheet) -> LayoutBox {
    let w = win_w as f32;
    let h = win_h as f32;
    let main_h = (h - HEADER_H - STATUS_H).max(0.0);
    let sidebar_w = (w * SIDEBAR_PCT).floor();
    let content_w = w - sidebar_w;
    let effective_w = content_w;
    let h_offset = 0.0_f32;
    let pad = 32.0_f32;

    // Wikipedia-Hauptseiten-Blöcke und allgemeine Float-Reset-Regeln.
    // Da unsere Engine kein float/grid kann, erzwingen wir display:block mit
    // anständigem Padding und einer dezenten Trennlinie für die Abschnitte.
    let compat_css = r#"
        /* Float-to-Flex */
        .nexus-flex-row {
            display: flex;
            flex-direction: row;
            flex-wrap: wrap;
            width: 100%;
            margin-bottom: 12px;
            align-items: flex-start;
            justify-content: stretch;
            gap: 14px;
        }
        .nexus-flex-item {
            flex: 1 1 240px;
            padding: 12px;
            box-sizing: border-box;
            min-width: 0;
            background-color: #ffffff;
            border: 1px solid #e5e7eb;
            border-radius: 10px;
            box-shadow: 0 1px 2px rgba(15,23,42,0.05);
        }

        /* Tabellen → Flex */
        .nexus-table {
            display: block;
            width: 100%;
            margin-top: 12px;
            margin-bottom: 12px;
        }
        .nexus-tr-single {
            display: block;
            width: 100%;
            padding: 6px 0;
        }
        .nexus-td {
            flex: 1 1 180px;
            padding: 12px;
            box-sizing: border-box;
            min-width: 0;
            background-color: #ffffff;
            border: 1px solid #e5e7eb;
            border-radius: 8px;
        }
        .nexus-td-span-2 { flex: 2; }
        .nexus-td-span-3 { flex: 3; }
        .nexus-td-span-4 { flex: 4; }

        .mw-parser-output {
            display: block;
            width: 100%;
            max-width: 1080px;
            margin: 0 auto;
            padding: 0 14px;
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

        /* Wikipedia Hauptseiten-Sektionen */
        #mp-upper, #mp-middle, #mp-lower,
        #mp-left, #mp-right, #mp-bottom,
        .mp-box, .mainpage-box {
            display: block;
            width: 100%;
            margin-bottom: 24px;
            padding: 18px;
            border: 1px solid #dfe3e8;
            border-radius: 10px;
            box-sizing: border-box;
            background-color: #ffffff;
            box-shadow: 0 1px 2px rgba(15,23,42,0.04);
        }
    "#;

    let mut rules = vec![
        Rule { selector: Selector::Tag("html".into()), declarations: vec![Width(effective_w), Height(main_h)] },
        Rule { selector: Selector::Tag("body".into()), declarations: vec![PaddingLeft(pad), PaddingRight(pad)] },
    ];
    rules.extend(extra_css.rules);

    // compat_css als zusätzliche Stylesheet-Regeln parsen und anhängen
    let compat_parsed = parse_css(compat_css);
    rules.extend(compat_parsed.rules);

    let stylesheet = Stylesheet { rules };
    build_layout_tree_with_viewport(&page_root, &stylesheet, sidebar_w + h_offset, HEADER_H, effective_w, main_h)
}