// js_runtime.rs  (neue Datei)
use boa_engine::{js_string, Context, JsArgs, JsValue, NativeFunction, Source, object::ObjectInitializer, property::Attribute};
use html_parser::Node as HtmlNode;
use boa_engine::object::builtins::JsArray;

pub struct JsRuntime {
    ctx: Context,
}

impl JsRuntime {
    pub fn new(root_node: HtmlNode) -> Self {
        let mut ctx = Context::default();
        let root_ref: &'static HtmlNode = Box::leak(Box::new(root_node));
        register_dom_methods(&mut ctx, root_ref);
        Self { ctx }
    }

    pub fn run_script(&mut self, src: &str) {
        if let Err(e) = self.ctx.eval(Source::from_bytes(src)) {
            eprintln!("[JS] Fehler: {}", e);
        }
    }
}

fn register_dom_methods(ctx: &mut Context, root: &'static HtmlNode) {
    // console.log → println
    let console = ObjectInitializer::new(ctx)
        .function(
            NativeFunction::from_fn_ptr(|_, args, ctx| {
                let msg = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
                println!("[JS console.log] {}", msg);
                Ok(JsValue::undefined())
            }),
            js_string!("log"), 1,
        )
        .build();

    ctx.global_object()
        .set(js_string!("console"), console, false, ctx)
        .unwrap();

    // window Objekt
    let window = ObjectInitializer::new(ctx)
        .property(js_string!("innerWidth"), 1280, Attribute::all())
        .property(js_string!("innerHeight"), 800, Attribute::all())
        .function(
            NativeFunction::from_fn_ptr(|_, args, ctx| {
                let msg = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
                println!("[JS Alert] {}", msg);
                Ok(JsValue::undefined())
            }),
            js_string!("alert"), 1
        )
        .function(
            NativeFunction::from_fn_ptr(|_, args, ctx| {
                let callback = args.get_or_undefined(0);
                if callback.is_callable() {
                    let cb = boa_engine::object::builtins::JsFunction::from_object(callback.as_object().unwrap().clone()).unwrap();
                    let _ = cb.call(&JsValue::undefined(), &[], ctx);
                }
                Ok(JsValue::from(1))
            }),
            js_string!("setTimeout"), 2
        )
        .build();

    let location = ObjectInitializer::new(ctx)
        .property(js_string!("href"), js_string!("https://nexus.browser"), Attribute::all())
        .property(js_string!("protocol"), js_string!("https:"), Attribute::all())
        .property(js_string!("host"), js_string!("nexus.browser"), Attribute::all())
        .build();

    window.set(js_string!("location"), location, false, ctx).unwrap();
    ctx.global_object().set(js_string!("window"), window.clone(), false, ctx).unwrap();
    
    // Globale Aliase
    ctx.global_object().set(js_string!("alert"), window.get(js_string!("alert"), ctx).unwrap(), false, ctx).unwrap();
    ctx.global_object().set(js_string!("setTimeout"), window.get(js_string!("setTimeout"), ctx).unwrap(), false, ctx).unwrap();

    // Wir erstellen die JS-Elemente für body/html VOR document_init,
    // da document_init eine exklusive mutable Referenz auf ctx hält.
    let body_js = find_tag_in_node(root, "body").map(|el| create_js_element(el, ctx));
    let html_js = find_tag_in_node(root, "html").map(|el| create_js_element(el, ctx));

    // document
    let mut document_init = ObjectInitializer::new(ctx);
    document_init
        .function(
            NativeFunction::from_copy_closure(move |_this, args, ctx| {
                let r = root; // Explizit die Referenz nutzen
                let id = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
                if let Some(el) = r.find_by_id(&id) {
                    Ok(JsValue::from(create_js_element(el, ctx)))
                } else {
                    Ok(JsValue::null())
                }
            }),
            js_string!("getElementById"),
            1,
        )
        .function(
            NativeFunction::from_copy_closure(move |_this, args, ctx| {
                let tag = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
                // Erstellt ein leeres virtuelles Element für die JS-Seite
                let el_raw = Box::leak(Box::new(html_parser::Element {
                    tag_name: tag,
                    attributes: std::collections::HashMap::new(),
                    children: Vec::new(),
                }));
                Ok(JsValue::from(create_js_element(el_raw, ctx)))
            }),
            js_string!("createElement"), 1
        )
        .property(js_string!("onload"), JsValue::null(), Attribute::all())
        .function(
            NativeFunction::from_copy_closure(move |_this, args, ctx| {
                let r = root;
                let sel = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
                if let Some(el) = find_tag_in_node(r, &sel) {
                    Ok(JsValue::from(create_js_element(el, ctx)))
                } else {
                    Ok(JsValue::null())
                }
            }),
            js_string!("querySelector"), 1,
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, ctx| {
                let arr = JsArray::new(ctx);
                Ok(JsValue::from(arr))
            }),
            js_string!("querySelectorAll"), 1
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined())),
            js_string!("addEventListener"), 2
        )
        .property(js_string!("title"), js_string!("Nexus Browser"), Attribute::all());

    if let Some(body) = body_js {
        document_init.property(js_string!("body"), body, Attribute::all());
    }
    if let Some(html) = html_js {
        document_init.property(js_string!("documentElement"), html, Attribute::all());
    }

    let document = document_init.build();

    ctx.global_object()
        .set(js_string!("document"), document, false, ctx)
        .unwrap();
}

fn create_js_element(el: &'static html_parser::Element, ctx: &mut Context) -> boa_engine::JsObject {
    ObjectInitializer::new(ctx)
        .property(js_string!("tagName"), js_string!(el.tag_name.clone()), Attribute::all())
        .property(js_string!("textContent"), js_string!(collect_text(el)), Attribute::all())
        .property(js_string!("innerHTML"), js_string!(""), Attribute::WRITABLE | Attribute::ENUMERABLE | Attribute::CONFIGURABLE)
        .property(js_string!("id"), js_string!(el.attributes.get("id").cloned().unwrap_or_default()), Attribute::all())
        .property(js_string!("className"), js_string!(el.attributes.get("class").cloned().unwrap_or_default()), Attribute::all())
         .property(js_string!("classList"),
            // Erstelle ein neues Context-Objekt für ObjectInitializer, um den mutable borrow Fehler zu vermeiden
            ObjectInitializer::new(&mut Context::default())
                .function(NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined())), js_string!("add"), 1)
                .function(NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined())), js_string!("remove"), 1)
                .function(NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::from(false))), js_string!("contains"), 1)
                .build(),
            Attribute::READONLY
        )
        .property(js_string!("style"), ObjectInitializer::new(&mut Context::default()).build(), Attribute::all())
        .function(
            NativeFunction::from_fn_ptr(|_, args, _| {
                let child = args.get_or_undefined(0);
                Ok(child.clone())
            }),
            js_string!("appendChild"), 1
        )
        .function(
            NativeFunction::from_copy_closure(move |_this, args, ctx| {
                let attr_name = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
                // Wir greifen auf die Attribute des Elements zu
                Ok(el.attributes.get(&attr_name)
                    .map(|v| JsValue::from(js_string!(v.clone())))
                    .unwrap_or(JsValue::null()))
            }),
            js_string!("getAttribute"), 1
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined())),
            js_string!("setAttribute"), 2
        )
        .function(
            NativeFunction::from_fn_ptr(|_, _, _| Ok(JsValue::undefined())),
            js_string!("addEventListener"), 2
        )
        .build()
}

fn find_tag_in_node<'a>(node: &'a HtmlNode, tag: &str) -> Option<&'a html_parser::Element> {
    match node {
        HtmlNode::Element(e) => {
            if e.tag_name == tag { return Some(e); }
            for child in &e.children {
                if let Some(found) = find_tag_in_node(child, tag) { return Some(found); }
            }
            None
        }
        HtmlNode::Text(_) => None,
    }
}

fn collect_text(el: &html_parser::Element) -> String {
    let mut text = String::new();
    collect_text_recursive(el, &mut text);
    text
}

fn collect_text_recursive(el: &html_parser::Element, text: &mut String) {
    // Performance-Capping für riesige Seiten
    if text.len() > 10000 { return; } 

    for child in &el.children {
        match child {
            html_parser::Node::Text(t) => { text.push_str(t); text.push(' '); },
            html_parser::Node::Element(e) => collect_text_recursive(e, text),
        }
    }
}