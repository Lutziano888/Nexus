mod dom;
mod cssom;
mod layout;
mod layout_taffy;
mod style;
mod text_measure;

use dom::{Node, NodeType};
use cssom::{Stylesheet, Rule, Selector, Declaration::{self, *}};
use layout::build_layout_tree;

fn main() {
    // ─── DOM aufbauen ──────────────────────────────────────────────────────────
    //
    //  <div id="root">          (800 × auto, kein padding)
    //    <div id="header">      (100 % Breite, Höhe 80 px)
    //    <div id="main">        (100 % Breite, padding 16 px)
    //      <p id="para">        (100 % Breite, Höhe 40 px)
    //    <div id="footer">      (100 % Breite, Höhe 60 px)

    let para = Node::element(
        "p",
        vec![("id", "para")],
        vec![Node::text("Hello, Layout Engine!")],
    );

    let main_div = Node::element(
        "div",
        vec![("id", "main")],
        vec![para],
    );

    let header = Node::element("div", vec![("id", "header")], vec![]);
    let footer = Node::element("div", vec![("id", "footer")], vec![]);

    let root = Node::element(
        "div",
        vec![("id", "root")],
        vec![header, main_div, footer],
    );

    // ─── CSSOM aufbauen ───────────────────────────────────────────────────────
    let stylesheet = Stylesheet {
        rules: vec![
            Rule {
                selector: Selector::Id("root".into()),
                declarations: vec![
                    Height(600.0),
                ],
            },
            Rule {
                selector: Selector::Id("header".into()),
                declarations: vec![
                    Height(80.0),
                ],
            },
            Rule {
                selector: Selector::Id("main".into()),
                declarations: vec![
                    PaddingTop(16.0),
                    PaddingRight(16.0),
                    PaddingBottom(16.0),
                    PaddingLeft(16.0),
                ],
            },
            Rule {
                selector: Selector::Id("para".into()),
                declarations: vec![
                    Height(40.0),
                ],
            },
            Rule {
                selector: Selector::Id("footer".into()),
                declarations: vec![
                    Height(60.0),
                ],
            },
        ],
    };

    // ─── Layout berechnen ─────────────────────────────────────────────────────
    let viewport_w = 1280.0_f32; // TODO: echten Fensterwert einsetzen
    let layout_tree = build_layout_tree(&root, &stylesheet, 0.0, 0.0, viewport_w);

    println!("╔══════════════════════════════════════════════════════════╗");
    println!("║              LAYOUT ENGINE  ·  Layout Tree               ║");
    println!("╚══════════════════════════════════════════════════════════╝");
    println!();
    layout_tree.print(0);
}
