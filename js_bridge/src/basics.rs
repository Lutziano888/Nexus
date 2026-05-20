// ─── Modul 1: Grundlagen ─────────────────────────────────────────────────────
//
// Zeigt:
//  • Einen Boa-Context erstellen
//  • JavaScript-Code als String ausführen
//  • Den Rückgabewert nach Rust konvertieren

use boa_engine::{js_string, Context, Source};

pub fn run() {
    let mut ctx = Context::default();

    // ── Einfacher Ausdruck ────────────────────────────────────────────────────
    let result = ctx.eval(Source::from_bytes("2 + 2")).expect("JS-Fehler");
    println!("  2 + 2  =  {}", result.display());

    // ── String ────────────────────────────────────────────────────────────────
    let result = ctx.eval(Source::from_bytes(r#"
        const greeting = "Hallo aus JavaScript!";
        greeting.toUpperCase();
    "#)).expect("JS-Fehler");

    let as_str = result.as_string().unwrap().to_std_string_escaped();
    println!("  JS-String nach Rust: \"{}\"", as_str);

    // ── Objekt-Property lesen ─────────────────────────────────────────────────
    let result = ctx.eval(Source::from_bytes(r#"
        ({ name: "NexusCpp", version: 1, ready: true })
    "#)).expect("JS-Fehler");

    println!("  JS-Objekt-Typ: {:?}", result.type_of());

    let obj = result.as_object().unwrap();
    // FIX: &str → js_string!(...)
    let name = obj
        .get(js_string!("name"), &mut ctx)
        .unwrap()
        .as_string()
        .unwrap()
        .to_std_string_escaped();
    println!("  obj.name = \"{}\"", name);
}