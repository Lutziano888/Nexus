/// Nexus Browser — Servo-powered, kein WebView
///
/// Architektur:
///   winit (Fenster + Input)
///     └── ServoBuilder → Servo (SpiderMonkey JS, WebRender GPU)
///           └── WebViewBuilder → WebView (ein Tab)
///                 └── NexusDelegate (Events: laden, malen, navigation)

use std::rc::Rc;
use std::cell::{Cell, RefCell};
use std::sync::Arc;

use dpi::PhysicalSize;
use euclid::Point2D;
use log::info;
use url::Url;
use winit::{
    application::ApplicationHandler,
    event::{ElementState, MouseButton, MouseScrollDelta, WindowEvent},
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop, EventLoopProxy},
    keyboard::{Key, NamedKey},
    window::{Window, WindowId},
};

use servo::{
    DeviceIndependentPixel, DevicePixel,
    EventLoopWaker,
    RenderingContext,
    Servo, ServoBuilder,
    WebView, WebViewBuilder,
    WebViewDelegate,
    LoadStatus,
    input_events::{
        KeyboardEvent, MouseButtonEvent, MouseMoveEvent, WheelEvent,
        InputEvent, MouseButtonAction, WheelDelta, WheelMode,
    },
    WebViewPoint,
};
use euclid::Scale;
use keyboard_types::{KeyState, Key as KeyboardKey};

// ─── Waker ───────────────────────────────────────────────────────────[...]

struct NexusWaker(EventLoopProxy<WakerEvent>);

#[derive(Debug, Clone)]
struct WakerEvent;

impl EventLoopWaker for NexusWaker {
    fn wake(&self) {
        let _ = self.0.send_event(WakerEvent);
    }

    fn clone_box(&self) -> Box<dyn EventLoopWaker> {
        Box::new(NexusWaker(self.0.clone()))
    }
}

// ─── Delegate ──────────────────────────────────────────────────────────[...]

struct NexusDelegate {
    current_url: RefCell<String>,
    load_complete: Cell<bool>,
    frame_ready: Cell<bool>,
    page_title: RefCell<Option<String>>,
}

impl NexusDelegate {
    fn new(initial_url: &str) -> Self {
        Self {
            current_url: RefCell::new(initial_url.to_string()),
            load_complete: Cell::new(false),
            frame_ready: Cell::new(false),
            page_title: RefCell::new(None),
        }
    }
}

impl WebViewDelegate for NexusDelegate {
    // WebView kommt by value (nicht &WebView) laut Trait
    fn notify_new_frame_ready(&self, webview: WebView) {
        self.frame_ready.set(true);
        webview.paint();
    }

    fn notify_load_status_changed(&self, _webview: WebView, status: LoadStatus) {
        match status {
            LoadStatus::Started => info!("[Nexus] Seite lädt..."),
            LoadStatus::Complete => {
                info!("[Nexus] Seite fertig");
                self.load_complete.set(true);
            }
            LoadStatus::HeadParsed => {}
        }
    }

    fn notify_url_changed(&self, _webview: WebView, url: Url) {
        *self.current_url.borrow_mut() = url.to_string();
        info!("[Nexus] URL: {}", url);
    }
}

// ─── App ───────────────────────────────────────────────────────────[...]

struct NexusApp {
    window: Option<Arc<Window>>,
    servo: Option<Servo>,
    webview: Option<WebView>,
    delegate: Option<Rc<NexusDelegate>>,
    rendering_context: Option<Rc<dyn RenderingContext>>,
    start_url: String,
    mouse_pos: (f64, f64),
    proxy: EventLoopProxy<WakerEvent>,
}

impl NexusApp {
    fn new(proxy: EventLoopProxy<WakerEvent>, start_url: String) -> Self {
        Self {
            window: None,
            servo: None,
            webview: None,
            delegate: None,
            rendering_context: None,
            start_url,
            mouse_pos: (0.0, 0.0),
            proxy,
        }
    }

    fn navigate(&self, url_str: &str) {
        let url = Url::parse(url_str)
            .or_else(|_| Url::parse(&format!("https://{}", url_str)))
            .unwrap_or_else(|_| {
                let query = urlencoding_simple(url_str);
                Url::parse(&format!("https://duckduckgo.com/?q={}", query)).unwrap()
            });

        if let Some(wv) = &self.webview {
            info!("[Nexus] Navigiere zu: {}", url);
            wv.load(url);
        }
    }

    fn tick(&mut self) {
        if let Some(servo) = &mut self.servo {
            servo.spin_event_loop();
        }

        if let Some(delegate) = &self.delegate {
            if delegate.frame_ready.get() {
                delegate.frame_ready.set(false);
                if let Some(ctx) = &self.rendering_context {
                    ctx.present();
                }
                if let Some(window) = &self.window {
                    let title = delegate
                        .page_title
                        .borrow()
                        .clone()
                        .unwrap_or_else(|| "Nexus".to_string());
                    window.set_title(&title);
                }
            }
        }
    }
}

impl ApplicationHandler<WakerEvent> for NexusApp {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = Arc::new(
            event_loop
                .create_window(
                    winit::window::Window::default_attributes()
                        .with_title("Nexus")
                        .with_inner_size(winit::dpi::LogicalSize::new(1280u32, 800u32)),
                )
                .expect("Fenster konnte nicht erstellt werden"),
        );

        let size = window.inner_size();
        let scale = window.scale_factor();

        // RenderingContext mit DisplayHandle + WindowHandle
        use raw_window_handle::HasDisplayHandle;
        use raw_window_handle::HasWindowHandle;
        let display_handle = window.display_handle().unwrap();
        let window_handle = window.window_handle().unwrap();

        let rendering_context = Rc::new(
            servo::WindowRenderingContext::new(
                display_handle,
                window_handle,
                PhysicalSize::new(size.width, size.height),
            )
            .expect("RenderingContext konnte nicht erstellt werden"),
        );

        let waker = Box::new(NexusWaker(self.proxy.clone()));

        // ServoBuilder.build() takes no arguments
        let servo = ServoBuilder::default()
            .event_loop_waker(waker)
            .build();

        let start_url = Url::parse(&self.start_url)
            .unwrap_or_else(|_| Url::parse("https://duckduckgo.com").unwrap());

        let delegate = Rc::new(NexusDelegate::new(&self.start_url));

        // Use Scale for hidpi_scale_factor with correct unit types
        let hidpi_scale: Scale<f32, DeviceIndependentPixel, DevicePixel> = Scale::new(scale as f32);

        // WebViewBuilder requires rendering_context by value as Rc<dyn RenderingContext>
        let webview = WebViewBuilder::new(&servo, Rc::clone(&rendering_context) as Rc<dyn RenderingContext>)
            .url(start_url)
            .hidpi_scale_factor(hidpi_scale)
            .delegate(Rc::clone(&delegate) as Rc<dyn WebViewDelegate>)
            .build();

        webview.resize(PhysicalSize::new(size.width, size.height));

        info!("[Nexus] Browser gestartet — Engine: Servo 0.1.0");

        self.window = Some(window);
        self.rendering_context = Some(rendering_context);
        self.servo = Some(servo);
        self.delegate = Some(delegate);
        self.webview = Some(webview);

        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                info!("[Nexus] Fenster wird geschlossen");
                self.servo.take();
                event_loop.exit();
            }

            WindowEvent::Resized(new_size) => {
                if let Some(ctx) = &self.rendering_context {
                    ctx.resize(PhysicalSize::new(new_size.width, new_size.height));
                }
                if let Some(wv) = &self.webview {
                    wv.resize(PhysicalSize::new(new_size.width, new_size.height));
                }
            }

            WindowEvent::ScaleFactorChanged { scale_factor, .. } => {
                if let Some(wv) = &self.webview {
                    let hidpi: Scale<f32, DeviceIndependentPixel, DevicePixel> = Scale::new(scale_factor as f32);
                    wv.set_hidpi_scale_factor(hidpi);
                }
            }

            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_pos = (position.x, position.y);
                if let Some(wv) = &self.webview {
                    wv.notify_input_event(InputEvent::MouseMove(MouseMoveEvent {
                        point: WebViewPoint::Device(Point2D::new(
                            position.x as f32,
                            position.y as f32,
                        )),
                        is_compatibility_event_for_touch: false,
                    }));
                }
            }

            WindowEvent::MouseInput { state, button, .. } => {
                if let Some(wv) = &self.webview {
                    let btn = match button {
                        MouseButton::Left => servo::input_events::MouseButton::Left,
                        MouseButton::Right => servo::input_events::MouseButton::Right,
                        MouseButton::Middle => servo::input_events::MouseButton::Middle,
                        _ => return,
                    };
                    let action = if state == ElementState::Pressed {
                        MouseButtonAction::Down
                    } else {
                        MouseButtonAction::Up
                    };
                    wv.notify_input_event(InputEvent::MouseButton(MouseButtonEvent {
                        button: btn,
                        action,
                        point: WebViewPoint::Device(Point2D::new(
                            self.mouse_pos.0 as f32,
                            self.mouse_pos.1 as f32,
                        )),
                    }));
                }
            }

            WindowEvent::MouseWheel { delta, .. } => {
                if let Some(wv) = &self.webview {
                    let (dx, dy) = match delta {
                        MouseScrollDelta::LineDelta(x, y) => (x * 60.0, y * 60.0),
                        MouseScrollDelta::PixelDelta(pos) => (pos.x as f32, pos.y as f32),
                    };
                    // WheelDelta is a struct with x, y, z fields and a mode
                    wv.notify_input_event(InputEvent::Wheel(WheelEvent {
                        delta: WheelDelta { x: dx as f64, y: dy as f64, z: 0.0, mode: WheelMode::DeltaPixel },
                        point: WebViewPoint::Device(Point2D::new(
                            self.mouse_pos.0 as f32,
                            self.mouse_pos.1 as f32,
                        )),
                    }));
                }
            }

            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed {
                    match &event.logical_key {
                        Key::Named(NamedKey::GoBack) => {
                            if let Some(wv) = &self.webview {
                                wv.go_back(1);
                                return;
                            }
                        }
                        Key::Named(NamedKey::F5) => {
                            if let Some(wv) = &self.webview {
                                wv.reload();
                                return;
                            }
                        }
                        _ => {}
                    }
                }

                if let Some(wv) = &self.webview {
                    // Convert winit Key to keyboard_types Key
                    let key_state = if event.state == ElementState::Pressed {
                        KeyState::Down
                    } else {
                        KeyState::Up
                    };
                    
                    // Convert winit::Key to keyboard_types::Key
                    let kb_key = convert_key(&event.logical_key);
                    
                    let kb_event = KeyboardEvent::from_state_and_key(key_state, kb_key);
                    wv.notify_input_event(InputEvent::Keyboard(kb_event));
                }
            }

            WindowEvent::RedrawRequested => {
                self.tick();
            }

            _ => {}
        }
    }

    fn user_event(&mut self, _event_loop: &ActiveEventLoop, _event: WakerEvent) {
        self.tick();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        self.tick();
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }
}

// ─── Hilfsfunktionen ───────────────────────────────────────────────────────[...]

fn urlencoding_simple(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            ' ' => '+'.to_string(),
            c if c.is_alphanumeric() || "-_.~".contains(c) => c.to_string(),
            c => format!("%{:02X}", c as u32),
        })
        .collect()
}

/// Convert winit::keyboard::Key to keyboard_types::Key
fn convert_key(key: &Key) -> KeyboardKey {
    match key {
        Key::Named(named_key) => match named_key {
            NamedKey::ArrowDown => KeyboardKey::Character("ArrowDown".to_string()),
            NamedKey::ArrowLeft => KeyboardKey::Character("ArrowLeft".to_string()),
            NamedKey::ArrowRight => KeyboardKey::Character("ArrowRight".to_string()),
            NamedKey::ArrowUp => KeyboardKey::Character("ArrowUp".to_string()),
            NamedKey::Enter => KeyboardKey::Character("Enter".to_string()),
            NamedKey::Tab => KeyboardKey::Character("Tab".to_string()),
            NamedKey::Escape => KeyboardKey::Character("Escape".to_string()),
            NamedKey::Backspace => KeyboardKey::Character("Backspace".to_string()),
            NamedKey::Delete => KeyboardKey::Character("Delete".to_string()),
            NamedKey::Space => KeyboardKey::Character(" ".to_string()),
            _ => KeyboardKey::Character("Unidentified".to_string()),
        },
        Key::Character(ch) => KeyboardKey::Character(ch.to_string()),
        _ => KeyboardKey::Character("Unidentified".to_string()),
    }
}

// ─── main ──────────────────────────────────────────────────────────[...]

fn main() {
    env_logger::Builder::from_env(
        env_logger::Env::default().default_filter_or("nexus=info,warn"),
    )
    .init();

    let start_url = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "https://duckduckgo.com".to_string());

    info!("=== Nexus Browser === Engine: Servo 0.1.0");
    info!("Start-URL: {}", start_url);

    let event_loop = EventLoop::<WakerEvent>::with_user_event()
        .build()
        .expect("Event-Loop Fehler");

    let proxy = event_loop.create_proxy();
    let mut app = NexusApp::new(proxy, start_url);

    event_loop.run_app(&mut app).expect("Event-Loop Fehler");
}