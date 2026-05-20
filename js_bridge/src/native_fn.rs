use boa_engine::{
    js_string, Context, JsArgs, JsNativeError, JsResult, JsValue, NativeFunction, Source,
};

fn print_to_console(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let msg = args.get_or_undefined(0).to_string(ctx)?.to_std_string_escaped();
    println!("  [JS → Rust]  print_to_console: \"{}\"", msg);
    Ok(JsValue::undefined())
}

fn add_numbers(_this: &JsValue, args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let a = args.get_or_undefined(0).to_number(ctx)?;
    let b = args.get_or_undefined(1).to_number(ctx)?;
    if a.is_nan() || b.is_nan() {
        return Err(JsNativeError::typ().with_message("add_numbers erwartet zwei Zahlen").into());
    }
    Ok(JsValue::from(a + b))
}

fn get_rust_config(_this: &JsValue, _args: &[JsValue], ctx: &mut Context) -> JsResult<JsValue> {
    let features = ["html_parser", "network_fetch", "layout_engine", "js_bridge"];
    let obj = boa_engine::object::ObjectInitializer::new(ctx)
        .property(js_string!("project"),      js_string!("NexusCpp"), boa_engine::property::Attribute::all())
        .property(js_string!("version"),      1_u32,                  boa_engine::property::Attribute::all())
        .property(js_string!("featureCount"), features.len() as u32,  boa_engine::property::Attribute::all())
        .build();
    Ok(JsValue::from(obj))
}

pub fn run() {
    let mut ctx = Context::default();
    let global = ctx.global_object();

    global.set(js_string!("print_to_console"),
               NativeFunction::from_fn_ptr(print_to_console).to_js_function(ctx.realm()),
               false, &mut ctx).unwrap();

    global.set(js_string!("addNumbers"),
               NativeFunction::from_fn_ptr(add_numbers).to_js_function(ctx.realm()),
               false, &mut ctx).unwrap();

    global.set(js_string!("getRustConfig"),
               NativeFunction::from_fn_ptr(get_rust_config).to_js_function(ctx.realm()),
               false, &mut ctx).unwrap();

    ctx.eval(Source::from_bytes(r#"
        print_to_console("Ich bin JavaScript und rufe Rust auf!");
        const summe = addNumbers(17, 25);
        print_to_console("17 + 25 = " + summe);
        const config = getRustConfig();
        print_to_console("Projekt: " + config.project + "  |  Version: " + config.version + "  |  Module: " + config.featureCount);
    "#)).expect("JS-Fehler");

    let result = ctx.eval(Source::from_bytes("addNumbers(100, 23)")).unwrap();
    println!("  Rust liest Rückgabewert: {}", result.display());
}