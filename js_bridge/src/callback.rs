use boa_engine::{
    js_string, Context, JsArgs, JsResult, JsValue,
    NativeFunction, Source,
    object::builtins::JsFunction,
    property::Attribute,
};

fn on_layout_complete(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let callback_val = args.get_or_undefined(0);
    if !callback_val.is_callable() {
        println!("  [Rust] Kein Callback übergeben.");
        return Ok(JsValue::undefined());
    }
    println!("  [Rust] Layout wird berechnet...");
    // FIX: js_string!(...) für Property-Keys
    let layout_result = boa_engine::object::ObjectInitializer::new(ctx)
        .property(js_string!("duration_ms"), 42_u32, Attribute::all())
        .property(js_string!("nodeCount"),   3_u32,  Attribute::all())
        .property(js_string!("success"),     true,   Attribute::all())
        .build();
    let cb = JsFunction::from_object(callback_val.as_object().unwrap().clone()).unwrap();
    cb.call(&JsValue::undefined(), &[JsValue::from(layout_result)], ctx)?;
    Ok(JsValue::undefined())
}

pub fn run() {
    let mut ctx = Context::default();

    // print_result zuerst registrieren (wird im JS-Callback gebraucht)
    ctx.global_object().set(
        js_string!("print_result"),
        NativeFunction::from_fn_ptr(|_, args, ctx| -> JsResult<JsValue> {
            let msg = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
            println!("  [JS → Rust]  {}", msg);
            Ok(JsValue::undefined())
        }).to_js_function(ctx.realm()),
        false, &mut ctx,
    ).unwrap();

    ctx.global_object().set(
        js_string!("onLayoutComplete"),
        NativeFunction::from_fn_ptr(on_layout_complete).to_js_function(ctx.realm()),
        false, &mut ctx,
    ).unwrap();

    // JS übergibt benannten Callback
    ctx.eval(Source::from_bytes(r#"
        function handleLayoutDone(result) {
            if (result.success) {
                print_result("Layout fertig!  Dauer: " + result.duration_ms + "ms  |  Knoten: " + result.nodeCount);
            }
        }
        onLayoutComplete(handleLayoutDone);
    "#)).expect("JS-Fehler");

    // JS übergibt anonymen Callback
    ctx.eval(Source::from_bytes(r#"
        onLayoutComplete(function(result) {
            print_result("Anonymer Callback – Knoten: " + result.nodeCount + ", Dauer: " + result.duration_ms + "ms");
        });
    "#)).expect("JS-Fehler");

    // Rust holt JS-Funktion aus globalem Scope und ruft sie direkt auf
    ctx.eval(Source::from_bytes(r#"
        function computeBoundingBox(nodes) {
            const maxX = Math.max(...nodes.map(n => n.x + n.width));
            const maxY = Math.max(...nodes.map(n => n.y + n.height));
            return { maxX, maxY };
        }
    "#)).expect("JS-Fehler");

    let compute_fn_val = ctx.global_object().get(js_string!("computeBoundingBox"), &mut ctx).unwrap();
    let compute_fn = JsFunction::from_object(compute_fn_val.as_object().unwrap().clone()).unwrap();

    let nodes_js = ctx.eval(Source::from_bytes(r#"
        [
          { x: 0,  y: 0,   width: 800, height: 80 },
          { x: 16, y: 96,  width: 768, height: 40 },
          { x: 0,  y: 152, width: 800, height: 60 }
        ]
    "#)).unwrap();

    let bbox = compute_fn.call(&JsValue::undefined(), &[nodes_js], &mut ctx).unwrap();
    let bbox_obj = bbox.as_object().unwrap();
    // FIX: js_string!(...)
    let max_x = bbox_obj.get(js_string!("maxX"), &mut ctx).unwrap().as_number().unwrap();
    let max_y = bbox_obj.get(js_string!("maxY"), &mut ctx).unwrap().as_number().unwrap();
    println!("  [Rust ruft JS auf]  BoundingBox: {}×{}px", max_x, max_y);
}