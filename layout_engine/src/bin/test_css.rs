fn main() {
    let css = "div { width: 50%; height: 100%; color: rgb(10, 20, 30); background-color: #AABBCC; opacity: 0.5; }";
    let opts = lightningcss::stylesheet::ParserOptions::default();
    let sheet = lightningcss::stylesheet::StyleSheet::parse(css, opts).unwrap();
    println!("{:#?}", sheet.rules);
}
