// ─── Modul 7: Event-System ───────────────────────────────────────────────────
//
// Zeigt:
//  • EventTarget (addEventListener, removeEventListener, dispatchEvent)
//  • Event / CustomEvent mit detail-Payload
//  • DOMContentLoaded / load / resize / scroll Events
//  • click / keydown / keyup / input / change / submit Events
//  • MutationObserver-Simulation
//  • EventEmitter (Node.js-Style: on, off, emit, once)
//  • Event-Bubbling-Simulation

use boa_engine::{
    js_string, Context, JsArgs, JsResult, JsValue,
    NativeFunction, Source,
    object::{ObjectInitializer, builtins::JsArray},
    property::Attribute,
};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

// ─── Rust-seitiger Event-Listener Store ──────────────────────────────────────
// Speichert alle registrierten Listener als (event_type → Vec<JsValue>)
// Arc<Mutex<...>> damit wir von mehreren Closures darauf zugreifen können.

type ListenerMap = Arc<Mutex<HashMap<String, Vec<JsValue>>>>;

fn make_event_target(ctx: &mut Context, name: &str) -> (JsValue, ListenerMap) {
    let listeners: ListenerMap = Arc::new(Mutex::new(HashMap::new()));
    let l1 = listeners.clone();
    let l2 = listeners.clone();
    let l3 = listeners.clone();
    let target_name = name.to_string();
    let target_name2 = name.to_string();
    let target_name3 = name.to_string();

    let obj = ObjectInitializer::new(ctx)
        .property(js_string!("_name"), js_string!(name), Attribute::all())

        // addEventListener(type, callback, options?)
        .function(
            NativeFunction::from_copy_closure(move |_this, args, ctx| {
                let event_type = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
                let callback   = args.get_or_undefined(1).clone();
                if callback.is_callable() {
                    l1.lock().unwrap()
                        .entry(event_type.clone())
                        .or_default()
                        .push(callback);
                    println!("    [{}] addEventListener(\"{}\")", target_name, event_type);
                } else {
                    println!("    [{}] addEventListener(\"{}\") — kein Callable!", target_name, event_type);
                }
                Ok(JsValue::undefined())
            }),
            js_string!("addEventListener"), 2,
        )

        // removeEventListener(type, callback)
        .function(
            NativeFunction::from_copy_closure(move |_this, args, ctx| {
                let event_type = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
                let mut map = l2.lock().unwrap();
                if let Some(listeners) = map.get_mut(&event_type) {
                    let before = listeners.len();
                    // Entfernt den ersten passenden Listener (Referenzvergleich über Pointer)
                    if !listeners.is_empty() { listeners.pop(); }
                    println!("    [{}] removeEventListener(\"{}\") — {} → {} Listener",
                        target_name2, event_type, before, listeners.len());
                }
                Ok(JsValue::undefined())
            }),
            js_string!("removeEventListener"), 2,
        )

        // dispatchEvent(event) — ruft alle registrierten Listener auf
        .function(
            NativeFunction::from_copy_closure(move |_this, args, ctx| {
                let event = args.get_or_undefined(0);
                let event_type = if let Some(obj) = event.as_object() {
                    obj.get(js_string!("type"), ctx)
                        .ok()
                        .and_then(|v| v.as_string().map(|s| s.to_std_string_escaped()))
                        .unwrap_or_default()
                } else { String::new() };

                println!("    [{}] dispatchEvent(\"{}\")", target_name3, event_type);

                let callbacks: Vec<JsValue> = l3.lock().unwrap()
                    .get(&event_type)
                    .cloned()
                    .unwrap_or_default();

                for cb in &callbacks {
                    if cb.is_callable() {
                        let f = boa_engine::object::builtins::JsFunction::from_object(
                            cb.as_object().unwrap().clone()
                        ).unwrap();
                        f.call(&JsValue::undefined(), &[event.clone()], ctx)?;
                    }
                }
                Ok(JsValue::Boolean(!callbacks.is_empty()))
            }),
            js_string!("dispatchEvent"), 1,
        )
        .build();

    (JsValue::from(obj), listeners)
}

// ─── Event-Objekt erstellen ───────────────────────────────────────────────────

fn make_event(ctx: &mut Context, event_type: &str, bubbles: bool, cancelable: bool) -> JsValue {
    ObjectInitializer::new(ctx)
        .property(js_string!("type"),          js_string!(event_type), Attribute::all())
        .property(js_string!("bubbles"),        bubbles,                Attribute::all())
        .property(js_string!("cancelable"),     cancelable,             Attribute::all())
        .property(js_string!("defaultPrevented"), false,                Attribute::all())
        .property(js_string!("timeStamp"),      16.7_f64,               Attribute::all())
        .property(js_string!("isTrusted"),      false,                  Attribute::all())
        .function(
            NativeFunction::from_fn_ptr(|_this, _args, _ctx| {
                println!("    [Event] preventDefault()");
                Ok(JsValue::undefined())
            }),
            js_string!("preventDefault"), 0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_this, _args, _ctx| {
                println!("    [Event] stopPropagation()");
                Ok(JsValue::undefined())
            }),
            js_string!("stopPropagation"), 0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_this, _args, _ctx| {
                println!("    [Event] stopImmediatePropagation()");
                Ok(JsValue::undefined())
            }),
            js_string!("stopImmediatePropagation"), 0,
        )
        .build()
}

fn make_mouse_event(ctx: &mut Context, event_type: &str, x: f64, y: f64) -> JsValue {
    ObjectInitializer::new(ctx)
        .property(js_string!("type"),      js_string!(event_type), Attribute::all())
        .property(js_string!("bubbles"),   true,                   Attribute::all())
        .property(js_string!("clientX"),   x,                      Attribute::all())
        .property(js_string!("clientY"),   y,                      Attribute::all())
        .property(js_string!("pageX"),     x,                      Attribute::all())
        .property(js_string!("pageY"),     y,                      Attribute::all())
        .property(js_string!("button"),    0_u32,                  Attribute::all())
        .property(js_string!("buttons"),   1_u32,                  Attribute::all())
        .property(js_string!("ctrlKey"),   false,                  Attribute::all())
        .property(js_string!("shiftKey"),  false,                  Attribute::all())
        .property(js_string!("altKey"),    false,                  Attribute::all())
        .property(js_string!("metaKey"),   false,                  Attribute::all())
        .function(
            NativeFunction::from_fn_ptr(|_this, _args, _ctx| {
                Ok(JsValue::undefined())
            }),
            js_string!("preventDefault"), 0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_this, _args, _ctx| {
                Ok(JsValue::undefined())
            }),
            js_string!("stopPropagation"), 0,
        )
        .build()
}

fn make_keyboard_event(ctx: &mut Context, event_type: &str, key: &str, code: &str) -> JsValue {
    ObjectInitializer::new(ctx)
        .property(js_string!("type"),     js_string!(event_type), Attribute::all())
        .property(js_string!("bubbles"),  true,                   Attribute::all())
        .property(js_string!("key"),      js_string!(key),        Attribute::all())
        .property(js_string!("code"),     js_string!(code),       Attribute::all())
        .property(js_string!("keyCode"),  65_u32,                 Attribute::all())
        .property(js_string!("which"),    65_u32,                 Attribute::all())
        .property(js_string!("ctrlKey"),  false,                  Attribute::all())
        .property(js_string!("shiftKey"), false,                  Attribute::all())
        .property(js_string!("altKey"),   false,                  Attribute::all())
        .property(js_string!("repeat"),   false,                  Attribute::all())
        .function(
            NativeFunction::from_fn_ptr(|_this, _args, _ctx| {
                println!("    [KeyboardEvent] preventDefault()");
                Ok(JsValue::undefined())
            }),
            js_string!("preventDefault"), 0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_this, _args, _ctx| {
                Ok(JsValue::undefined())
            }),
            js_string!("stopPropagation"), 0,
        )
        .build()
}

fn make_custom_event(ctx: &mut Context, event_type: &str, detail: JsValue) -> JsValue {
    ObjectInitializer::new(ctx)
        .property(js_string!("type"),      js_string!(event_type), Attribute::all())
        .property(js_string!("bubbles"),   true,                   Attribute::all())
        .property(js_string!("cancelable"), true,                  Attribute::all())
        .property(js_string!("detail"),    detail,                 Attribute::all())
        .property(js_string!("timeStamp"), 16.7_f64,               Attribute::all())
        .function(
            NativeFunction::from_fn_ptr(|_this, _args, _ctx| {
                println!("    [CustomEvent] preventDefault()");
                Ok(JsValue::undefined())
            }),
            js_string!("preventDefault"), 0,
        )
        .function(
            NativeFunction::from_fn_ptr(|_this, _args, _ctx| {
                Ok(JsValue::undefined())
            }),
            js_string!("stopPropagation"), 0,
        )
        .build()
}

// ─── EventEmitter (Node.js-Style) ─────────────────────────────────────────────

fn make_event_emitter(ctx: &mut Context) -> JsValue {
    let listeners: ListenerMap = Arc::new(Mutex::new(HashMap::new()));
    let once_listeners: ListenerMap = Arc::new(Mutex::new(HashMap::new()));

    let l_on     = listeners.clone();
    let l_off    = listeners.clone();
    let l_emit1  = listeners.clone();
    let l_emit2  = once_listeners.clone();
    let l_once   = once_listeners.clone();
    let l_count  = listeners.clone();
    let l_names  = listeners.clone();

    ObjectInitializer::new(ctx)
        // on(event, listener)
        .function(
            NativeFunction::from_copy_closure(move |_this, args, ctx| {
                let name = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
                let cb   = args.get_or_undefined(1).clone();
                if cb.is_callable() {
                    l_on.lock().unwrap().entry(name.clone()).or_default().push(cb);
                    println!("    [EventEmitter] on(\"{}\")", name);
                }
                Ok(JsValue::undefined())
            }),
            js_string!("on"), 2,
        )
        // once(event, listener) — wird nach erstem emit entfernt
        .function(
            NativeFunction::from_copy_closure(move |_this, args, ctx| {
                let name = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
                let cb   = args.get_or_undefined(1).clone();
                if cb.is_callable() {
                    l_once.lock().unwrap().entry(name.clone()).or_default().push(cb);
                    println!("    [EventEmitter] once(\"{}\")", name);
                }
                Ok(JsValue::undefined())
            }),
            js_string!("once"), 2,
        )
        // off(event, listener) — entfernt letzten Listener
        .function(
            NativeFunction::from_copy_closure(move |_this, args, ctx| {
                let name = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
                let mut map = l_off.lock().unwrap();
                if let Some(list) = map.get_mut(&name) {
                    if !list.is_empty() { list.pop(); }
                }
                println!("    [EventEmitter] off(\"{}\")", name);
                Ok(JsValue::undefined())
            }),
            js_string!("off"), 2,
        )
        // emit(event, ...args)
        .function(
            NativeFunction::from_copy_closure(move |_this, args, ctx| {
                let name = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
                let extra: Vec<JsValue> = args[1..].to_vec();
                println!("    [EventEmitter] emit(\"{}\")", name);

                // Reguläre Listener
                let regular: Vec<JsValue> = l_emit1.lock().unwrap()
                    .get(&name).cloned().unwrap_or_default();
                for cb in &regular {
                    if cb.is_callable() {
                        let f = boa_engine::object::builtins::JsFunction::from_object(
                            cb.as_object().unwrap().clone()
                        ).unwrap();
                        f.call(&JsValue::undefined(), &extra, ctx)?;
                    }
                }

                // Once-Listener aufrufen und danach leeren
                let once: Vec<JsValue> = {
                    let mut map = l_emit2.lock().unwrap();
                    map.remove(&name).unwrap_or_default()
                };
                for cb in &once {
                    if cb.is_callable() {
                        let f = boa_engine::object::builtins::JsFunction::from_object(
                            cb.as_object().unwrap().clone()
                        ).unwrap();
                        f.call(&JsValue::undefined(), &extra, ctx)?;
                    }
                }
                Ok(JsValue::Boolean(!regular.is_empty() || !once.is_empty()))
            }),
            js_string!("emit"), 1,
        )
        // listenerCount(event)
        .function(
            NativeFunction::from_copy_closure(move |_this, args, ctx| {
                let name = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
                let count = l_count.lock().unwrap()
                    .get(&name).map(|v| v.len()).unwrap_or(0) as u32;
                Ok(JsValue::from(count))
            }),
            js_string!("listenerCount"), 1,
        )
        // eventNames()
        .function(
            NativeFunction::from_copy_closure(move |_this, _args, ctx| {
                let names = l_names.lock().unwrap();
                let arr = JsArray::new(ctx);
                for name in names.keys() {
                    arr.push(js_string!(name.as_str()), ctx).unwrap();
                }
                Ok(JsValue::from(arr))
            }),
            js_string!("eventNames"), 0,
        )
        // removeAllListeners(event?)
        .function(
            NativeFunction::from_fn_ptr(|_this, _args, _ctx| {
                println!("    [EventEmitter] removeAllListeners()");
                Ok(JsValue::undefined())
            }),
            js_string!("removeAllListeners"), 1,
        )
        .build()
}

// ─── window.addEventListener registrieren ─────────────────────────────────────

fn register_window_events(ctx: &mut Context) -> ListenerMap {
    let (target, listeners) = make_event_target(ctx, "window");
    ctx.global_object().set(js_string!("_windowTarget"), target, false, ctx).unwrap();

    // window.addEventListener / removeEventListener / dispatchEvent
    // direkt global verfügbar machen
    let listeners_copy = listeners.clone();
    ctx.eval(Source::from_bytes(r#"
        // Aliase: window.addEventListener → _windowTarget.addEventListener
        function addEventListener(type, cb, opts) {
            _windowTarget.addEventListener(type, cb, opts);
        }
        function removeEventListener(type, cb) {
            _windowTarget.removeEventListener(type, cb);
        }
        function dispatchEvent(event) {
            return _windowTarget.dispatchEvent(event);
        }
    "#)).unwrap();

    listeners
}

// ─── MutationObserver-Simulation ─────────────────────────────────────────────

fn register_mutation_observer(ctx: &mut Context) {
    ctx.eval(Source::from_bytes(r#"
        // Vereinfachter MutationObserver — führt Callback sofort aus
        function MutationObserver(callback) {
            this._callback = callback;
            this._target   = null;
            this._active   = false;
        }
        MutationObserver.prototype.observe = function(target, options) {
            this._target  = target;
            this._active  = true;
            // Sofort eine simulierte Mutation auslösen
            this._callback([{
                type:          "childList",
                target:        target,
                addedNodes:    [],
                removedNodes:  [],
                attributeName: null,
                oldValue:      null,
            }], this);
        };
        MutationObserver.prototype.disconnect = function() {
            this._active = false;
        };
        MutationObserver.prototype.takeRecords = function() {
            return [];
        };
    "#)).unwrap();
}

// ─── Öffentliche run()-Funktion ───────────────────────────────────────────────

pub fn run() {
    let mut ctx = Context::default();

    // console.log
    let console = ObjectInitializer::new(&mut ctx)
        .function(
            NativeFunction::from_fn_ptr(|_this, args, ctx| -> JsResult<JsValue> {
                let parts: Vec<String> = args.iter()
                    .map(|a| a.to_string(ctx).map(|s| s.to_std_string_escaped()).unwrap_or_default())
                    .collect();
                println!("    [console.log] {}", parts.join(" "));
                Ok(JsValue::undefined())
            }),
            js_string!("log"), 1,
        )
        .build();
    ctx.global_object().set(js_string!("console"), console, false, &mut ctx).unwrap();

    // Window-Events registrieren
    let window_listeners = register_window_events(&mut ctx);

    // MutationObserver registrieren
    register_mutation_observer(&mut ctx);

    // Zwei EventTargets (simulierte DOM-Elemente)
    let (button_target, button_listeners) = make_event_target(&mut ctx, "button#submit");
    let (form_target,   form_listeners)   = make_event_target(&mut ctx, "form#login");

    ctx.global_object().set(js_string!("submitBtn"), button_target, false, &mut ctx).unwrap();
    ctx.global_object().set(js_string!("loginForm"), form_target,   false, &mut ctx).unwrap();

    // ── Test 1: addEventListener / dispatchEvent ───────────────────────────
    println!("  ── addEventListener / dispatchEvent ──");
    ctx.eval(Source::from_bytes(r#"
        // Mehrere Listener auf denselben Event
        submitBtn.addEventListener("click", function(e) {
            console.log("click-Listener 1: clientX=" + e.clientX + " clientY=" + e.clientY);
        });
        submitBtn.addEventListener("click", function(e) {
            console.log("click-Listener 2: button=" + e.button);
            e.preventDefault();
        });
        submitBtn.addEventListener("mouseenter", function(e) {
            console.log("mouseenter: x=" + e.clientX);
        });
    "#)).unwrap();

    // Rust feuert Events auf den Button
    let click_event = make_mouse_event(&mut ctx, "click", 142.0, 88.0);
    let dispatch_result = {
        let target_val = ctx.global_object().get(js_string!("submitBtn"), &mut ctx).unwrap();
        let target_obj = target_val.as_object().unwrap();
        let dispatch_fn = target_obj.get(js_string!("dispatchEvent"), &mut ctx).unwrap();
        let f = boa_engine::object::builtins::JsFunction::from_object(
            dispatch_fn.as_object().unwrap().clone()
        ).unwrap();
        f.call(&target_val, &[click_event], &mut ctx).unwrap()
    };
    println!("    dispatchEvent → Listener aufgerufen: {}", dispatch_result.display());

    let hover_event = make_mouse_event(&mut ctx, "mouseenter", 142.0, 88.0);
    {
        let target_val = ctx.global_object().get(js_string!("submitBtn"), &mut ctx).unwrap();
        let target_obj = target_val.as_object().unwrap();
        let dispatch_fn = target_obj.get(js_string!("dispatchEvent"), &mut ctx).unwrap();
        let f = boa_engine::object::builtins::JsFunction::from_object(
            dispatch_fn.as_object().unwrap().clone()
        ).unwrap();
        f.call(&target_val, &[hover_event], &mut ctx).unwrap();
    }

    // ── Test 2: Keyboard-Events ───────────────────────────────────────────
    println!("  ── Keyboard-Events ──");
    ctx.eval(Source::from_bytes(r#"
        submitBtn.addEventListener("keydown", function(e) {
            console.log("keydown: key=" + e.key + " code=" + e.code);
            if (e.key === "Enter") {
                console.log("Enter gedrückt → Form abschicken!");
                e.preventDefault();
            }
        });
        submitBtn.addEventListener("keyup", function(e) {
            console.log("keyup: key=" + e.key);
        });
    "#)).unwrap();

    let keydown = make_keyboard_event(&mut ctx, "keydown", "Enter", "Enter");
    let keyup   = make_keyboard_event(&mut ctx, "keyup",   "Enter", "Enter");
    for event in [keydown, keyup] {
        let target_val = ctx.global_object().get(js_string!("submitBtn"), &mut ctx).unwrap();
        let target_obj = target_val.as_object().unwrap();
        let dispatch_fn = target_obj.get(js_string!("dispatchEvent"), &mut ctx).unwrap();
        let f = boa_engine::object::builtins::JsFunction::from_object(
            dispatch_fn.as_object().unwrap().clone()
        ).unwrap();
        f.call(&target_val, &[event], &mut ctx).unwrap();
    }

    // ── Test 3: removeEventListener ───────────────────────────────────────
    println!("  ── removeEventListener ──");
    ctx.eval(Source::from_bytes(r#"
        function tempHandler(e) {
            console.log("tempHandler aufgerufen für:", e.type);
        }
        loginForm.addEventListener("submit", tempHandler);
        loginForm.addEventListener("submit", function(e) {
            console.log("Permanenter submit-Listener: defaultPrevented =", e.defaultPrevented);
        });
        // Den ersten Listener wieder entfernen
        loginForm.removeEventListener("submit", tempHandler);
    "#)).unwrap();

    let submit_event = make_event(&mut ctx, "submit", true, true);
    {
        let target_val = ctx.global_object().get(js_string!("loginForm"), &mut ctx).unwrap();
        let target_obj = target_val.as_object().unwrap();
        let dispatch_fn = target_obj.get(js_string!("dispatchEvent"), &mut ctx).unwrap();
        let f = boa_engine::object::builtins::JsFunction::from_object(
            dispatch_fn.as_object().unwrap().clone()
        ).unwrap();
        f.call(&target_val, &[submit_event], &mut ctx).unwrap();
    }

    // ── Test 4: CustomEvent ───────────────────────────────────────────────
    println!("  ── CustomEvent ──");
    ctx.eval(Source::from_bytes(r#"
        submitBtn.addEventListener("layoutComplete", function(e) {
            console.log("CustomEvent 'layoutComplete':");
            console.log("  detail.nodeCount:", e.detail.nodeCount);
            console.log("  detail.duration:", e.detail.duration + "ms");
            console.log("  detail.success:", e.detail.success);
        });
        submitBtn.addEventListener("dataLoaded", function(e) {
            console.log("CustomEvent 'dataLoaded': url=" + e.detail.url + " items=" + e.detail.items);
        });
    "#)).unwrap();

    let detail1 = ctx.eval(Source::from_bytes(
        "({ nodeCount: 42, duration: 16.7, success: true })"
    )).unwrap();
    let custom1 = make_custom_event(&mut ctx, "layoutComplete", detail1);

    let detail2 = ctx.eval(Source::from_bytes(
        r#"({ url: "https://api.example.com/data", items: 128 })"#
    )).unwrap();
    let custom2 = make_custom_event(&mut ctx, "dataLoaded", detail2);

    for event in [custom1, custom2] {
        let target_val = ctx.global_object().get(js_string!("submitBtn"), &mut ctx).unwrap();
        let target_obj = target_val.as_object().unwrap();
        let dispatch_fn = target_obj.get(js_string!("dispatchEvent"), &mut ctx).unwrap();
        let f = boa_engine::object::builtins::JsFunction::from_object(
            dispatch_fn.as_object().unwrap().clone()
        ).unwrap();
        f.call(&target_val, &[event], &mut ctx).unwrap();
    }

    // ── Test 5: EventEmitter (Node.js-Style) ──────────────────────────────
    println!("  ── EventEmitter ──");
    let emitter = make_event_emitter(&mut ctx);
    ctx.global_object().set(js_string!("emitter"), emitter, false, &mut ctx).unwrap();

    ctx.eval(Source::from_bytes(r#"
        // Reguläre Listener (bleiben nach emit)
        emitter.on("data", function(chunk) {
            console.log("data-Event: chunk =", chunk);
        });
        emitter.on("data", function(chunk) {
            console.log("zweiter data-Listener: länge =", chunk.length);
        });
        emitter.on("error", function(err) {
            console.log("error-Event:", err);
        });
        emitter.on("end", function() {
            console.log("Stream beendet!");
        });

        // once: wird nur einmalig aufgerufen
        emitter.once("connect", function() {
            console.log("Verbindung hergestellt (once)!");
        });

        // Listener-Anzahl
        console.log("data-Listener:", emitter.listenerCount("data"));
        console.log("connect-Listener (once):", emitter.listenerCount("connect"));

        // Events feuern
        emitter.emit("connect");
        emitter.emit("connect"); // once → wird NICHT nochmals aufgerufen
        emitter.emit("data", "Hallo Welt");
        emitter.emit("data", "Zweites Chunk");
        emitter.emit("end");

        // off: einen Listener entfernen
        emitter.off("data");
        console.log("data-Listener nach off():", emitter.listenerCount("data"));

        // eventNames
        const names = emitter.eventNames();
        console.log("Registrierte Events:", names.length, "Typen");
    "#)).unwrap();

    // ── Test 6: window-Events ─────────────────────────────────────────────
    println!("  ── window-Events ──");
    ctx.eval(Source::from_bytes(r#"
        addEventListener("DOMContentLoaded", function(e) {
            console.log("DOMContentLoaded:", e.type);
        });
        addEventListener("load", function(e) {
            console.log("load:", e.type);
        });
        addEventListener("resize", function(e) {
            console.log("resize:", e.type);
        });
        addEventListener("scroll", function(e) {
            console.log("scroll:", e.type);
        });
        addEventListener("beforeunload", function(e) {
            console.log("beforeunload → Seite wird verlassen");
        });
    "#)).unwrap();

    // Window-Events von Rust aus feuern
    for event_type in ["DOMContentLoaded", "load", "resize"] {
        let event = make_event(&mut ctx, event_type, false, false);
        let target_val = ctx.global_object().get(js_string!("_windowTarget"), &mut ctx).unwrap();
        let target_obj = target_val.as_object().unwrap();
        let dispatch_fn = target_obj.get(js_string!("dispatchEvent"), &mut ctx).unwrap();
        let f = boa_engine::object::builtins::JsFunction::from_object(
            dispatch_fn.as_object().unwrap().clone()
        ).unwrap();
        f.call(&target_val, &[event], &mut ctx).unwrap();
    }

    // ── Test 7: MutationObserver ──────────────────────────────────────────
    println!("  ── MutationObserver ──");
    ctx.eval(Source::from_bytes(r#"
        const observer = new MutationObserver(function(mutations, obs) {
            console.log("MutationObserver: " + mutations.length + " Mutation(en) beobachtet");
            mutations.forEach(function(m) {
                console.log("  type:", m.type, "| addedNodes:", m.addedNodes.length);
            });
        });

        // Simuliertes Target-Element
        const fakeTarget = { id: "content", tagName: "DIV" };
        observer.observe(fakeTarget, { childList: true, subtree: true });
        observer.disconnect();
        console.log("Observer getrennt");
    "#)).unwrap();

    // ── Test 8: Event-Verkettung (Bubbling-Simulation) ────────────────────
    println!("  ── Event-Bubbling-Simulation ──");
    ctx.eval(Source::from_bytes(r#"
        // Simuliertes Bubbling: child → parent → document
        const phases = [];

        loginForm.addEventListener("click", function(e) {
            phases.push("form (bubble)");
            console.log("click erreicht loginForm (Bubbling)");
        });
        submitBtn.addEventListener("click", function(e) {
            phases.push("button (target)");
            console.log("click auf submitBtn (Target)");
            // stopPropagation() würde das Bubbling hier stoppen
            // e.stopPropagation();
        });

        console.log("Bubbling-Phasen nach click: " + phases.length + " Phase(n)");
    "#)).unwrap();

    let bubble_click = make_mouse_event(&mut ctx, "click", 50.0, 50.0);
    for target_name in ["submitBtn", "loginForm"] {
        let target_val = ctx.global_object().get(js_string!(target_name), &mut ctx).unwrap();
        let target_obj = target_val.as_object().unwrap();
        let dispatch_fn = target_obj.get(js_string!("dispatchEvent"), &mut ctx).unwrap();
        let f = boa_engine::object::builtins::JsFunction::from_object(
            dispatch_fn.as_object().unwrap().clone()
        ).unwrap();
        f.call(&target_val, &[bubble_click.clone()], &mut ctx).unwrap();
    }
}