// ┌─────────────────────────────────────────────────────────────┐
// │         Rust ↔ JavaScript Bridge  –  Boa Engine             │
// └─────────────────────────────────────────────────────────────┘
//
// Module-Übersicht:
//   1. basics    – Context, JS-Code ausführen, Werte lesen
//   2. native_fn – Rust-Funktion als JS-Funktion registrieren
//   3. objects   – Rust-Struct als JS-Objekt exponieren
//   4. callback  – JS ruft Rust, Rust ruft JS zurück (Callback)

mod basics;
mod native_fn;
mod objects;
mod callback;
wmod events;

fn main() {
    println!("╔══════════════════════════════════════════════════════════════╗");
    println!("║            Rust ↔ JavaScript Bridge  ·  Boa Engine           ║");
    println!("╚══════════════════════════════════════════════════════════════╝\n");

    println!("━━━  1. Basics: JS ausführen & Werte lesen  ━━━━━━━━━━━━━━━━━━");
    basics::run();

    println!("\n━━━  2. Native Fn: Rust-Funktion in JS  ━━━━━━━━━━━━━━━━━━━━━");
    native_fn::run();

    println!("\n━━━  3. Objects: Rust-Struct als JS-Objekt  ━━━━━━━━━━━━━━━━━");
    objects::run();

    println!("\n━━━  4. Callback: JS ruft Rust, Rust ruft JS  ━━━━━━━━━━━━━━━");
    callback::run();

    println!("\n━━━  7. Event-System  ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    events::run();
}