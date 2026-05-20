// ╔══════════════════════════════════════════════════════════════════════════╗
// ║   NexusCpp · Renderer  –  Main Process  (v0.4 – Fix: Hit-Test IDs)      ║
// ╚══════════════════════════════════════════════════════════════════════════╝

mod layout_bridge;
mod painter;
mod skia_painter;
mod color;
mod text_renderer;
mod page;
mod js_runtime;
mod image_cache;
mod svg_renderer;      // ← SVG Rendering für moderne Websites
mod web_storage;       // ← Cookies & LocalStorage
mod css_selector;      // ← CSS Selectors für präzise Element-Auswahl

use std::num::NonZeroU32;
use std::sync::{Arc, mpsc::{self, Receiver, TryRecvError}};
use std::thread;

use winit::{
    application::ApplicationHandler,
    dpi::LogicalSize,
    event::{ElementState, KeyEvent, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    window::{CursorIcon, Window, WindowId},
};
use softbuffer::{Context as SbContext, Surface};

use layout_bridge::{build_chrome_layout, HEADER_H, STATUS_H, SIDEBAR_PCT};
use layout_engine::layout::LayoutBox;
use painter::paint_layout;
use skia_painter::SkiaPainter;
use text_renderer::TextRenderer;
use page::{load_page, LoadedPage};

const WIN_W: u32 = 900;
const WIN_H: u32 = 650;

/// Berechnet die maximale untere Kante (y + height) aller Nodes im Layout-Baum.
/// Das ist die tatsächliche scrollbare Gesamthöhe des Inhalts.
fn layout_max_bottom(layout: &LayoutBox) -> i32 {
    let own_bottom = (layout.y + layout.height) as i32;
    layout.children.iter()
        .map(layout_max_bottom)
        .fold(own_bottom, i32::max)
}

enum NavResult {
    Ok(LoadedPage),
    Err(String),
}

/// Stabile ID: roher Zeiger auf den LayoutBox-Node
pub type NodePtr = usize;

#[derive(Clone, Debug, PartialEq)]
pub enum FocusedElement {
    None,
    UrlBar,
    PageInput { ptr: NodePtr },
}

pub struct BrowserState {
    pub url:           String,
    pub image_cache: image_cache::ImageCache,
    pub url_focused:   bool,
    pub status_text:   String,
    pub page_layout:   Option<LayoutBox>,
    pub is_loading:    bool,
    pub mouse_x:       i32,
    pub mouse_y:       i32,
    pub hovered_href:  Option<String>,
    pub focused:       FocusedElement,
    pub input_values:  std::collections::HashMap<NodePtr, String>,
    pub scroll_y:      i32,
    /// Momentum-Scrolling: aktuelle Scroll-Geschwindigkeit in px/frame
    pub scroll_vel:    f32,
}

impl BrowserState {
    fn new() -> Self {
        Self {
            url:           "https://".into(),
            image_cache: image_cache::ImageCache::new(),
            url_focused:   false,
            status_text:   "Bereit".into(),
            page_layout:   None,
            is_loading:    false,
            mouse_x:       0,
            mouse_y:       0,
            hovered_href:  None,
            focused:       FocusedElement::None,
            input_values:  std::collections::HashMap::new(),
            scroll_y:      0,
            scroll_vel:    0.0,
        }
    }
}

struct BrowserApp {
    window_state:   Option<WindowState>,
    browser:        BrowserState,
    text_renderer:  Option<TextRenderer>,
    nav_rx:         Option<Receiver<NavResult>>,
    window_arc:     Option<Arc<Window>>,
    win_w:          u32,
    win_h:          u32,
    chrome_layout:  Option<LayoutBox>,
}

struct WindowState {
    window:       Arc<Window>,
    surface:      Surface<Arc<Window>, Arc<Window>>,
    width:        u32,
    height:       u32,
    skia_painter: Option<SkiaPainter>,
}

impl BrowserApp {
    fn new() -> Self {
        Self {
            window_state:  None,
            browser:       BrowserState::new(),
            text_renderer: None,
            nav_rx:        None,
            window_arc:    None,
            win_w:         WIN_W,
            win_h:         WIN_H,
            chrome_layout: None,
        }
    }

    fn hit_test(&self, mx: i32, my: i32) -> HitResult {
        // 1. Check Chrome (URL bar, buttons) - simplified
        if (my as f32) < HEADER_H && mx > 120 {
            return HitResult::UrlBar;
        }

        // 2. Check Page Content
        let sidebar_w = (self.win_w as f32 * SIDEBAR_PCT).floor();
        
        // Transformiere Maus-Koordinaten in Page-Koordinaten:
        // - X: Abzüglich der Sidebar
        // - Y: Abzüglich des Headers + Aktueller Scroll-Stand
        let page_mx = mx as f32 - sidebar_w;
        let page_my = (my as f32 - HEADER_H) + self.browser.scroll_y as f32;

        if let Some(root) = &self.browser.page_layout {
            if let Some(found_box) = self.find_at_recursive(root, page_mx, page_my) {
                let tag = found_box.tag_name.to_lowercase();
                let ptr = found_box as *const LayoutBox as usize;

                match tag.as_str() {
                    "a" => {
                        if let Some(href) = found_box.attributes.get("href") {
                            if !href.is_empty() && !href.starts_with('#') {
                                return HitResult::Link(href.clone());
                            }
                        }
                    }
                    "input" | "textarea" | "select" => {
                        return HitResult::Input(ptr);
                    }
                    "button" => {
                        return HitResult::Button(ptr);
                    }
                    _ => {}
                }
            }
        }
        HitResult::None
    }

    fn find_at_recursive<'a>(&self, layout: &'a LayoutBox, mx: f32, my: f32) -> Option<&'a LayoutBox> {
        let inside = mx >= layout.x && mx <= (layout.x + layout.width) &&
                    my >= layout.y && my <= (layout.y + layout.height);

        if !inside { return None; }

        for child in layout.children.iter().rev() {
            if let Some(hit) = self.find_at_recursive(child, mx, my) {
                return Some(hit);
            }
        }
        Some(layout)
    }

    fn navigate(&mut self, url: String) {
        self.nav_rx = None;
        self.browser.is_loading    = true;
        self.browser.page_layout   = None;
        self.browser.status_text   = format!("Lade: {}", url);
        self.browser.url_focused   = false;
        self.browser.focused       = FocusedElement::None;
        self.browser.hovered_href  = None;
        self.browser.scroll_y      = 0;
        self.browser.input_values.clear();

        let (tx, rx) = mpsc::channel::<NavResult>();
        self.nav_rx = Some(rx);

        let win_w  = self.win_w;
        let win_h  = self.win_h;
        let window = self.window_arc.clone();

        thread::spawn(move || {
            println!("[Nav-Thread] Fetch: {}", url);
            let msg = match load_page(&url, win_w, win_h) {
                Ok(page)  => { println!("[Nav-Thread] OK: {}", page.title); NavResult::Ok(page) }
                Err(e)    => { println!("[Nav-Thread] Fehler: {}", e);      NavResult::Err(e.to_string()) }
            };
            let _ = tx.send(msg);
            if let Some(w) = window { w.request_redraw(); }
        });
    }

    fn poll_nav(&mut self) -> bool {
        let result = match &self.nav_rx {
            None     => return false,
            Some(rx) => match rx.try_recv() {
                Ok(r)                           => r,
                Err(TryRecvError::Empty)        => return false,
                Err(TryRecvError::Disconnected) => {
                    self.nav_rx = None;
                    self.browser.is_loading = false; // Fix: Reset loading state on disconnect
                    self.browser.status_text = "Fehler: Verbindung zum Lade-Thread verloren".into();
                    return true;
                }
            },
        };
        self.nav_rx = None;
        self.browser.is_loading = false;
        match result {
            NavResult::Ok(page) => {
                self.browser.status_text = format!("OK: {}", page.title);
                self.browser.page_layout = Some(page.layout);
            }
            NavResult::Err(e) => {
                self.browser.status_text = format!("Fehler: {}", e);
                self.browser.page_layout = None;
            }
        }
        true
    }
}

#[derive(Debug)]
enum HitResult {
    None,
    UrlBar,
    Link(String),
    Input(NodePtr),
    Button(NodePtr),
}

// kept for potential future use
#[allow(dead_code)]
fn tag_from_label(label: &str) -> String {
    label
        .trim_start_matches('<')
        .split(|c: char| c == '#' || c == '.' || c == '>' || c == ' ')
        .next()
        .unwrap_or("")
        .to_lowercase()
}

#[allow(dead_code)]
fn href_from_label(label: &str) -> Option<String> {
    if let Some(start) = label.find("href=\"") {
        let rest = &label[start + 6..];
        if let Some(end) = rest.find('"') {
            let href = rest[..end].to_string();
            if !href.is_empty() && !href.starts_with('#') {
                return Some(href);
            }
        }
    }
    None
}

impl ApplicationHandler for BrowserApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let attrs = Window::default_attributes()
            .with_title("NexusCpp Browser · Renderer v0.4")
            .with_inner_size(LogicalSize::new(WIN_W, WIN_H))
            .with_resizable(true);

        let window = Arc::new(
            event_loop.create_window(attrs).expect("Fenster konnte nicht erstellt werden")
        );

        let sb_ctx  = SbContext::new(window.clone()).expect("softbuffer Context fehlgeschlagen");
        let surface = Surface::new(&sb_ctx, window.clone()).expect("softbuffer Surface fehlgeschlagen");

        self.window_arc   = Some(window.clone());
        self.window_state = Some(WindowState {
            window,
            surface,
            width: WIN_W,
            height: WIN_H,
            skia_painter: Some(SkiaPainter::new(WIN_W, WIN_H)),
        });
        self.text_renderer = Some(TextRenderer::new(14.0));
        
        // Initialisiere TextMeasurer für Layout-Engine
        let measurer = std::sync::Arc::new(layout_engine::text_measure::DefaultTextMeasurer);
        layout_engine::text_measure::set_text_measurer(measurer);

        println!("[Renderer] Fenster erstellt  {}x{}px", WIN_W, WIN_H);
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if self.poll_nav() {
            if let Some(ws) = self.window_state.as_ref() {
                ws.window.request_redraw();
            }
        }
        event_loop.set_control_flow(ControlFlow::Wait);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        if self.poll_nav() {
            if let Some(ws) = self.window_state.as_ref() { ws.window.request_redraw(); }
        }

        match event {
            WindowEvent::CloseRequested => event_loop.exit(),

            WindowEvent::CursorMoved { position, .. } => {
                self.browser.mouse_x = position.x as i32;
                self.browser.mouse_y = position.y as i32;

                let hit = self.hit_test(self.browser.mouse_x, self.browser.mouse_y);
                let new_href = if let HitResult::Link(ref href) = hit {
                    Some(href.clone())
                } else { None };

                let cursor = match &hit {
                    HitResult::Link(_)  => CursorIcon::Pointer,
                    HitResult::Input(_) => CursorIcon::Text,
                    HitResult::UrlBar   => CursorIcon::Text,
                    _                   => CursorIcon::Default,
                };

                if self.browser.hovered_href != new_href {
                    self.browser.hovered_href = new_href.clone();
                    if let Some(href) = &new_href {
                        self.browser.status_text = href.clone();
                    } else if !self.browser.is_loading {
                        self.browser.status_text = "Bereit".into();
                    }
                    if let Some(ws) = self.window_state.as_ref() {
                        ws.window.set_cursor(cursor);
                        ws.window.request_redraw();
                    }
                }
            }

            WindowEvent::MouseInput { state: ElementState::Pressed, button: MouseButton::Left, .. } => {
                let mx = self.browser.mouse_x;
                let my = self.browser.mouse_y;
                let hit = self.hit_test(mx, my);

                match hit {
                    HitResult::UrlBar => {
                        self.browser.url_focused  = true;
                        self.browser.focused      = FocusedElement::UrlBar;
                        self.browser.status_text  = "URL eingeben, Enter druecken".into();
                    }
                    HitResult::Link(href) => {
                        let full_url = if href.starts_with("http") {
                            href
                        } else if href.starts_with('/') {
                            let base = extract_origin(&self.browser.url);
                            format!("{}{}", base, href)
                        } else if href.is_empty() || href.starts_with('#') {
                            self.browser.url.clone()
                        } else {
                            format!("https://{}", href)
                        };
                        self.browser.url = full_url.clone();
                        self.navigate(full_url);
                    }
                    HitResult::Input(ptr) => {
                        self.browser.focused     = FocusedElement::PageInput { ptr };
                        self.browser.url_focused = false;
                    }
                    HitResult::Button(_ptr) => {
                        let url = self.browser.url.clone();
                        self.browser.status_text = "Formular abgeschickt (kein JS)".into();
                        self.navigate(url);
                    }
                    HitResult::None => {
                        self.browser.url_focused = false;
                        self.browser.focused     = FocusedElement::None;
                    }
                }
                if let Some(ws) = self.window_state.as_ref() { ws.window.request_redraw(); }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                let dy = match delta {
                    winit::event::MouseScrollDelta::LineDelta(_, y)   => (y * 40.0) as i32,
                    winit::event::MouseScrollDelta::PixelDelta(pos)   => pos.y as i32,
                };
                let win_h = self.window_state.as_ref()
                    .map(|ws| ws.window.inner_size().height as i32)
                    .unwrap_or(WIN_H as i32);
                let viewport_h = (win_h - HEADER_H as i32 - STATUS_H as i32).max(1);
                let content_h  = self.browser.page_layout.as_ref()
                    .map(|l| layout_max_bottom(l)).unwrap_or(0);
                let max_scroll = (content_h - viewport_h).max(0);
                self.browser.scroll_y = (self.browser.scroll_y - dy).clamp(0, max_scroll);
                self.browser.scroll_vel = 0.0; // kein Nachroll
                if let Some(ws) = self.window_state.as_ref() { ws.window.request_redraw(); }
            }

            WindowEvent::KeyboardInput {
                event: KeyEvent { logical_key, state: ElementState::Pressed, .. }, ..
            } => {
                match logical_key {
                    Key::Named(NamedKey::Escape) => {
                        if self.browser.url_focused || self.browser.focused != FocusedElement::None {
                            self.browser.url_focused = false;
                            self.browser.focused     = FocusedElement::None;
                            self.browser.status_text = "Bereit".into();
                        } else {
                            event_loop.exit();
                        }
                    }

                    Key::Named(NamedKey::Enter) => {
                        match &self.browser.focused {
                            FocusedElement::UrlBar => {
                                let url = self.browser.url.clone();
                                self.navigate(url);
                            }
                            _ => {
                                if self.browser.url_focused {
                                    let url = self.browser.url.clone();
                                    self.navigate(url);
                                }
                            }
                        }
                    }

                    Key::Named(NamedKey::Backspace) => {
                        match self.browser.focused.clone() {
                            FocusedElement::UrlBar => { self.browser.url.pop(); }
                            FocusedElement::PageInput { ptr } => {
                                self.browser.input_values
                                    .entry(ptr).or_default().pop();
                            }
                            FocusedElement::None => {
                                if self.browser.url_focused { self.browser.url.pop(); }
                            }
                        }
                    }

                    Key::Named(NamedKey::F5) => {
                        if !self.browser.is_loading {
                            let url = self.browser.url.clone();
                            self.navigate(url);
                        }
                    }

                    Key::Character(c) => {
                        match self.browser.focused.clone() {
                            FocusedElement::UrlBar => { self.browser.url.push_str(&c); }
                            FocusedElement::PageInput { ptr } => {
                                self.browser.input_values
                                    .entry(ptr).or_default().push_str(&c);
                            }
                            FocusedElement::None => {
                                if self.browser.url_focused { self.browser.url.push_str(&c); }
                            }
                        }
                    }

                    _ => {}
                }

                if let Some(ws) = self.window_state.as_ref() { ws.window.request_redraw(); }
            }

            WindowEvent::Resized(size) => {
                let w = size.width.max(1);
                let h = size.height.max(1);
                self.win_w = w;
                self.win_h = h;
                self.chrome_layout = None; // Cache invalidieren
                if let Some(state) = self.window_state.as_mut() {
                    state.width  = w;
                    state.height = h;
                    state.surface
                        .resize(NonZeroU32::new(w).unwrap(), NonZeroU32::new(h).unwrap())
                        .expect("Surface-Resize fehlgeschlagen");
                    if let Some(painter) = state.skia_painter.as_mut() {
                        painter.resize(w, h);
                    }
                    state.window.request_redraw();
                }
            }

            WindowEvent::RedrawRequested => {
                if let Some(state) = self.window_state.as_mut() {
                    if let Some(tr) = &self.text_renderer {
                        let w = state.width;
                        let h = state.height;
                        if self.chrome_layout.is_none() {
                            self.chrome_layout = Some(build_chrome_layout(w, h));
                        }
                        if let Some(chrome) = &self.chrome_layout {
                            render(state, &self.browser, tr, chrome, self.window_arc.as_ref());
                        }
                    }
                    if self.browser.is_loading {
                        state.window.request_redraw();
                    }
                }
            }

            _ => {}
        }
    }
}

fn extract_origin(url: &str) -> String {
    if let Some(rest) = url.strip_prefix("https://").or_else(|| url.strip_prefix("http://")) {
        let scheme = if url.starts_with("https") { "https" } else { "http" };
        let host = rest.split('/').next().unwrap_or(rest);
        format!("{}://{}", scheme, host)
    } else {
        url.to_string()
    }
}

fn render(state: &mut WindowState, browser: &BrowserState, tr: &TextRenderer, chrome: &LayoutBox, window: Option<&Arc<Window>>) {
    let mut buffer = state.surface.buffer_mut().expect("Buffer-Lock fehlgeschlagen");
    if let Some(painter) = state.skia_painter.as_mut() {
        painter.paint_layout(&mut buffer, browser, window);
    } else {
        paint_layout(&mut buffer, state.width, state.height, chrome, browser, tr, window);
    }
    buffer.present().expect("Present fehlgeschlagen");
}

fn main() {
    let event_loop = EventLoop::new().expect("Event-Loop konnte nicht erstellt werden");
    event_loop.set_control_flow(ControlFlow::Wait);
    let mut app = BrowserApp::new();
    let _ = event_loop.run_app(&mut app);
}