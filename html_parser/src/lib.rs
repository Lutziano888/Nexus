use std::collections::HashMap;

// ============================================================
//  DOM-Datenstrukturen
// ============================================================

/// Ein einzelner Knoten im DOM-Baum.
#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    /// Ein HTML-Element: <tag attr="wert">...</tag>
    Element(Element),
    /// Reiner Text zwischen Tags
    Text(String),
}

impl Node {
    /// Sucht rekursiv nach einem Element mit der angegebenen ID.
    pub fn find_by_id(&self, id: &str) -> Option<&Element> {
        match self {
            Node::Element(e) => {
                if e.attributes.get("id").map(|s| s.as_str()) == Some(id) {
                    return Some(e);
                }
                for child in &e.children {
                    if let Some(found) = child.find_by_id(id) { return Some(found); }
                }
                None
            }
            Node::Text(_) => None,
        }
    }
}

/// Repräsentiert ein HTML-Element mit Tag-Name, Attributen und Kindern.
#[derive(Debug, Clone, PartialEq)]
pub struct Element {
    pub tag_name:   String,
    pub attributes: HashMap<String, String>,
    pub children:   Vec<Node>,
}

impl Element {
    fn new(tag_name: impl Into<String>) -> Self {
        Self {
            tag_name:   tag_name.into(),
            attributes: HashMap::new(),
            children:   Vec::new(),
        }
    }
}

// ============================================================
//  Tokenizer
// ============================================================

/// Rohe Token, die der Tokenizer erzeugt.
#[derive(Debug, Clone, PartialEq)]
enum Token {
    /// <tag key="val" …>
    OpenTag  { name: String, attrs: HashMap<String, String>, self_closing: bool },
    /// </tag>
    CloseTag { name: String },
    /// Beliebiger Text zwischen Tags
    Text     (String),
}

struct Tokenizer<'a> {
    input: &'a [u8],
    pos:   usize,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input: input.as_bytes(), pos: 0 }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn consume(&mut self) -> Option<u8> {
        let ch = self.input.get(self.pos).copied();
        if ch.is_some() { self.pos += 1; }
        ch
    }

    fn consume_while(&mut self, pred: impl Fn(u8) -> bool) -> String {
        let start = self.pos;
        while let Some(c) = self.peek() {
            if pred(c) { self.pos += 1; } else { break; }
        }
        String::from_utf8_lossy(&self.input[start..self.pos]).into_owned()
    }

    fn skip_whitespace(&mut self) {
        self.consume_while(|c| c.is_ascii_whitespace());
    }

    /// Liest einen Bezeichner (Tag-Name, Attribut-Name).
    fn read_ident(&mut self) -> String {
        self.consume_while(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_' || c == b':')
    }

    /// Liest einen Attributwert: "wert" oder 'wert' oder unquotiertes Wort.
    fn read_attr_value(&mut self) -> String {
        match self.peek() {
            Some(b'"') | Some(b'\'') => {
                let quote = self.consume().unwrap();
                let val = self.consume_while(|c| c != quote);
                self.consume(); // schließendes Anführungszeichen
                val
            }
            _ => self.consume_while(|c| !c.is_ascii_whitespace() && c != b'>'),
        }
    }

    /// Liest alle Attribute eines öffnenden Tags.
    fn read_attributes(&mut self) -> (HashMap<String, String>, bool) {
        let mut attrs = HashMap::new();
        loop {
            self.skip_whitespace();
            match self.peek() {
                None | Some(b'>') => { self.consume(); return (attrs, false); }
                Some(b'/') => {
                    self.consume();
                    self.consume(); // '>'
                    return (attrs, true);
                }
                _ => {}
            }
            let name = self.read_ident().to_lowercase();
            if name.is_empty() { self.consume(); continue; } // kaputtes Markup überspringen
            self.skip_whitespace();
            let value = if self.peek() == Some(b'=') {
                self.consume(); // '='
                self.skip_whitespace();
                self.read_attr_value()
            } else {
                String::new() // boolesches Attribut: checked, disabled …
            };
            attrs.insert(name, value);
        }
    }

    /// Tokenisiert das gesamte Dokument in eine Liste von Token.
    fn tokenize(mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some(c) = self.peek() {
            if c == b'<' {
                self.consume(); // '<'
                match self.peek() {
                    // Kommentar: <!-- … -->
                    Some(b'!') => {
                        self.consume();
                        if self.input.get(self.pos..self.pos+2) == Some(b"--") {
                            self.pos += 2;
                            // Sicherstellen, dass wir nicht über das Ende des Buffers lesen
                            while self.pos + 3 <= self.input.len() 
                                && &self.input[self.pos..self.pos+3] != b"-->" {
                                self.pos += 1;
                            }
                            if self.pos + 3 <= self.input.len() {
                                self.pos += 3; // Schließendes '-->' überspringen
                            }
                        } else {
                            // DOCTYPE oder andere Direktiven überspringen
                            self.consume_while(|c| c != b'>');
                            self.consume();
                        }
                    }
                    // Schließendes Tag: </tag>
                    Some(b'/') => {
                        self.consume();
                        let name = self.read_ident().to_lowercase();
                        self.consume_while(|c| c != b'>');
                        self.consume();
                        tokens.push(Token::CloseTag { name });
                    }
                    // Öffnendes Tag: <tag …>
                    _ => {
                        let name = self.read_ident().to_lowercase();
                        if name.is_empty() {
                            // '<' gefolgt von Nicht-Bezeichner → als Text behandeln
                            tokens.push(Token::Text("<".into()));
                            continue;
                        }
                        let (attrs, self_closing) = self.read_attributes();
                        let tag_name = name.clone();
                        tokens.push(Token::OpenTag { name, attrs, self_closing });

                        // Spezialfall: <script> und <style> – Inhalt als rohen Text lesen
                        if !self_closing && (tag_name == "script" || tag_name == "style") {
                            let end_tag = format!("</{}", tag_name);
                            let start = self.pos;
                            while self.pos + end_tag.len() <= self.input.len() {
                                if self.input[self.pos..].get(..end_tag.len())
                                    .map(|b| b.eq_ignore_ascii_case(end_tag.as_bytes()))
                                    .unwrap_or(false) 
                                {
                                    break;
                                }
                                self.pos += 1;
                            }
                            let text = String::from_utf8_lossy(&self.input[start..self.pos]).trim().to_string();
                            if !text.is_empty() {
                                tokens.push(Token::Text(text));
                            }
                        }
                    }
                }
            } else {
                // Text-Knoten
                let text = self.consume_while(|c| c != b'<');
                let trimmed = text.trim().to_string();
                if !trimmed.is_empty() {
                    tokens.push(Token::Text(trimmed));
                }
            }
        }
        tokens
    }
}

// ============================================================
//  Rekursiver Descent-Parser (Token → DOM-Baum)
// ============================================================

struct Parser {
    tokens: Vec<Token>,
    pos:    usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn consume(&mut self) -> Option<Token> {
        let tok = self.tokens.get(self.pos).cloned();
        if tok.is_some() { self.pos += 1; }
        tok
    }

    /// Tags, die keinen Body haben (void elements).
    fn is_void(tag: &str) -> bool {
        matches!(tag, "area"|"base"|"br"|"col"|"embed"|"hr"|"img"|"input"
                     |"link"|"meta"|"param"|"source"|"track"|"wbr")
    }

    /// Parst Kinder-Knoten, bis ein schließendes Tag für `parent_tag` kommt
    /// oder der Token-Strom endet.
    fn parse_children(&mut self, parent_tag: &str) -> Vec<Node> {
        let mut children = Vec::new();
        loop {
            match self.peek() {
                None => break,
                Some(Token::CloseTag { name }) if name == parent_tag => {
                    self.consume();
                    break;
                }
                // Schließendes Tag, das nicht zu uns gehört → Fehler-Toleranz:
                // einfach abbrechen, ohne zu konsumieren (Elternteil räumt auf).
                Some(Token::CloseTag { .. }) => break,
                Some(Token::Text(_)) => {
                    if let Some(Token::Text(t)) = self.consume() {
                        children.push(Node::Text(t));
                    }
                }
                Some(Token::OpenTag { .. }) => {
                    if let Some(node) = self.parse_element() {
                        children.push(node);
                    }
                }
            }
        }
        children
    }

    /// Parst ein einzelnes Element (ohne vorheriges OpenTag zu konsumieren).
    fn parse_element(&mut self) -> Option<Node> {
        if let Some(Token::OpenTag { name, attrs, self_closing }) = self.consume() {
            let mut elem = Element::new(&name);
            elem.attributes = attrs;

            if !self_closing && !Self::is_void(&name) {
                elem.children = self.parse_children(&name);
            }
            Some(Node::Element(elem))
        } else {
            None
        }
    }

    /// Einstiegspunkt: parst alle Top-Level-Knoten.
    fn parse_document(&mut self) -> Node {
        // Erstelle eine synthetische Wurzel, falls kein <html>-Tag vorhanden ist.
        let children = self.parse_children("__root__");

        // Wenn genau ein Element auf oberster Ebene vorhanden ist, gib es direkt zurück.
        if children.len() == 1 {
            if let Node::Element(_) = &children[0] {
                return children[0].clone();
            }
        }

        // Andernfalls wickle alles in ein künstliches <document>-Element.
        Node::Element(Element {
            tag_name:   "document".into(),
            attributes: HashMap::new(),
            children,
        })
    }
}

// ============================================================
//  Öffentliche API
// ============================================================

/// Wandelt einen HTML-String in einen DOM-Baum um.
///
/// # Beispiel
/// ```
/// use html_parser::parse;
/// let dom = parse("<html><body><h1>Hallo</h1></body></html>");
/// ```
pub fn parse(html: &str) -> Node {
    let tokens = Tokenizer::new(html).tokenize();
    Parser::new(tokens).parse_document()
}

// ============================================================
//  Hilfsfunktion: Pretty-Print des DOM-Baums
// ============================================================

pub fn print_tree(node: &Node, depth: usize) {
    let indent = "  ".repeat(depth);
    match node {
        Node::Text(t) => println!("{indent}\"{}\"", t),
        Node::Element(e) => {
            let attrs: Vec<String> = e.attributes
                .iter()
                .map(|(k, v)| if v.is_empty() { k.clone() } else { format!("{k}=\"{v}\"") })
                .collect();
            let attr_str = if attrs.is_empty() { String::new() } else { format!(" {}", attrs.join(" ")) };
            println!("{indent}<{}{}>", e.tag_name, attr_str);
            for child in &e.children {
                print_tree(child, depth + 1);
            }
            println!("{indent}</{}>", e.tag_name);
        }
    }
}

// ============================================================
//  Tests
// ============================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn elem(node: &Node) -> &Element {
        if let Node::Element(e) = node { e } else { panic!("Expected Element") }
    }

    #[test]
    fn test_simple_element() {
        let dom = parse("<h1>Hallo</h1>");
        let e = elem(&dom);
        assert_eq!(e.tag_name, "h1");
        assert_eq!(e.children, vec![Node::Text("Hallo".into())]);
    }

    #[test]
    fn test_nested_structure() {
        let html = "<html><body><p>Text</p></body></html>";
        let dom = parse(html);
        let html_el = elem(&dom);
        assert_eq!(html_el.tag_name, "html");

        let body = elem(&html_el.children[0]);
        assert_eq!(body.tag_name, "body");

        let p = elem(&body.children[0]);
        assert_eq!(p.tag_name, "p");
        assert_eq!(p.children[0], Node::Text("Text".into()));
    }

    #[test]
    fn test_attributes() {
        let dom = parse(r#"<p class="intro" id="main">Inhalt</p>"#);
        let e = elem(&dom);
        assert_eq!(e.attributes.get("class"), Some(&"intro".to_string()));
        assert_eq!(e.attributes.get("id"),    Some(&"main".to_string()));
    }

    #[test]
    fn test_self_closing() {
        let dom = parse("<div><br/><hr/></div>");
        let e = elem(&dom);
        assert_eq!(e.children.len(), 2);
    }

    #[test]
    fn test_comment_skipped() {
        let dom = parse("<div><!-- Kommentar -->Text</div>");
        let e = elem(&dom);
        assert_eq!(e.children, vec![Node::Text("Text".into())]);
    }

    #[test]
    fn test_full_page() {
        let html = r#"
            <!DOCTYPE html>
            <html lang="de">
              <head>
                <meta charset="UTF-8"/>
                <title>Testseite</title>
              </head>
              <body>
                <h1 class="title">Willkommen</h1>
                <p>Dies ist ein <em>einfacher</em> Test.</p>
              </body>
            </html>
        "#;
        let dom = parse(html);
        let html_el = elem(&dom);
        assert_eq!(html_el.tag_name, "html");
        assert_eq!(html_el.attributes.get("lang"), Some(&"de".to_string()));

        let body = elem(&html_el.children[1]);
        assert_eq!(body.tag_name, "body");
        assert_eq!(body.children.len(), 2); // h1 + p
    }

    #[test]
    fn test_script_with_less_than() {
        let html = "<script>if (a < b) { console.log('test'); }</script><p>Next</p>";
        let dom = parse(html);
        // Da parse() bei mehreren Top-Level-Elementen ein <document> drumherum baut:
        let doc = elem(&dom);
        assert_eq!(doc.children.len(), 2);
        let script = elem(&doc.children[0]);
        assert_eq!(script.tag_name, "script");
        assert_eq!(script.children[0], Node::Text("if (a < b) { console.log('test'); }".into()));
        let p = elem(&doc.children[1]);
        assert_eq!(p.tag_name, "p");
    }
}
