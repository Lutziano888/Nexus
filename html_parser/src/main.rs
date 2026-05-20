use html_parser::{parse, print_tree};


// ============================================================
//  main – Demo
// ============================================================

fn main() {
    let html = r#"
        <!DOCTYPE html>
        <html lang="de">
          <head>
            <meta charset="UTF-8"/>
            <title>Meine Seite</title>
          </head>
          <body>
            <!-- Haupt-Inhalt -->
            <h1 class="title">Willkommen beim Rust HTML-Parser</h1>
            <p id="intro">Dies ist ein <em>einfacher</em> rekursiver Parser.</p>
            <p>Er unterstützt Attribute, Kommentare und void-Elemente wie <br/>.</p>
          </body>
        </html>
    "#;

    println!("=== DOM-Baum ===\n");
    let dom = parse(html);
    print_tree(&dom, 0);
}