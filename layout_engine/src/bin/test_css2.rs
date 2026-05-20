fn main() {
    let css = "div.class#id > span { color: red; }";
    let sheet = layout_engine::cssom::parse_css(css);
    println!("{:#?}", sheet.rules);
}
