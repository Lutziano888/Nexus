// ╔══════════════════════════════════════════════════════════════════════════╗
// ║   CSS Selectors – Präzise Element-Auswahl mit selectors Crate            ║
// ╚══════════════════════════════════════════════════════════════════════════╝

use std::collections::HashMap;

/// CSS-Selector Parser & Matcher
pub struct CssSelector {
    selector_text: String,
}

impl CssSelector {
    pub fn new() -> Self {
        CssSelector {
            selector_text: String::new(),
        }
    }

    /// Validiert einen CSS-Selector
    pub fn parse(&self, selector_str: &str) -> Result<String, String> {
        if selector_str.is_empty() {
            return Err("Empty selector".to_string());
        }

        // Einfaches Validieren der Selector-Syntax
        match selector_str {
            // ID Selectors: #id
            s if s.starts_with('#') && s.len() > 1 => {
                if s.chars().skip(1).all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                    Ok(format!("ID selector: {}", &s[1..]))
                } else {
                    Err("Invalid ID selector syntax".to_string())
                }
            }
            // Class Selectors: .class
            s if s.starts_with('.') && s.len() > 1 => {
                if s.chars().skip(1).all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
                    Ok(format!("Class selector: {}", &s[1..]))
                } else {
                    Err("Invalid class selector syntax".to_string())
                }
            }
            // Attribute Selectors: [attr], [attr=value], [attr~=value]
            s if s.starts_with('[') && s.ends_with(']') => {
                Ok(format!("Attribute selector: {}", s))
            }
            // Type Selectors: div, span, p, etc.
            s if s.chars().all(|c| c.is_alphabetic() || c == '_') => {
                Ok(format!("Type selector: {}", s))
            }
            _ => Err("Unsupported or invalid selector syntax".to_string()),
        }
    }

    /// Kombiniert mehrere Selektoren (comma-separated)
    pub fn parse_multiple(&self, selectors_str: &str) -> Result<Vec<String>, String> {
        selectors_str
            .split(',')
            .map(|s| self.parse(s.trim()))
            .collect()
    }

    /// Pseudo-Klassen erkennen (z.B. :hover, :focus, :visited)
    pub fn extract_pseudo_classes(selector_str: &str) -> Vec<String> {
        let pseudo_classes = vec![
            "hover", "focus", "active", "visited", "link",
            "first-child", "last-child", "nth-child", "nth-of-type",
            "empty", "enabled", "disabled", "checked",
        ];

        let mut found = Vec::new();
        for pseudo in pseudo_classes {
            if selector_str.contains(&format!(":{}", pseudo)) {
                found.push(pseudo.to_string());
            }
        }
        found
    }

    /// Extrahiert Attribut-Selektoren
    pub fn extract_attributes(selector_str: &str) -> Vec<String> {
        let mut attrs = Vec::new();
        let mut in_bracket = false;
        let mut current = String::new();

        for ch in selector_str.chars() {
            match ch {
                '[' => {
                    in_bracket = true;
                    current.clear();
                }
                ']' => {
                    if in_bracket {
                        attrs.push(current.clone());
                        in_bracket = false;
                    }
                }
                _ if in_bracket => current.push(ch),
                _ => {}
            }
        }

        attrs
    }

    /// Kombiniert Selektoren mit UND-Logik (z.B. "div.active" = div AND .active)
    pub fn parse_compound(selector_str: &str) -> Vec<String> {
        let mut parts = Vec::new();

        if selector_str.starts_with('#') {
            if let Some(space_idx) = selector_str.find(|c: char| c.is_whitespace()) {
                parts.push(selector_str[..space_idx].to_string());
                parts.push(selector_str[space_idx + 1..].to_string());
            } else {
                parts.push(selector_str.to_string());
            }
        } else {
            // Teile Class-Selektoren auf: "div.active" → ["div", ".active"]
            let mut current = String::new();
            for ch in selector_str.chars() {
                if ch == '.' {
                    if !current.is_empty() {
                        parts.push(current.clone());
                    }
                    current = String::from(".");
                } else {
                    current.push(ch);
                }
            }
            if !current.is_empty() {
                parts.push(current);
            }
        }

        parts
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_selector() {
        let css = CssSelector::new();
        assert!(css.parse("#myid").is_ok());
        assert!(css.parse("#my-id").is_ok());
        assert!(css.parse("#123invalid").is_err());
    }

    #[test]
    fn test_class_selector() {
        let css = CssSelector::new();
        assert!(css.parse(".myclass").is_ok());
        assert!(css.parse(".my-class").is_ok());
    }

    #[test]
    fn test_type_selector() {
        let css = CssSelector::new();
        assert!(css.parse("div").is_ok());
        assert!(css.parse("span").is_ok());
    }

    #[test]
    fn test_pseudo_classes() {
        let pseudo = CssSelector::extract_pseudo_classes("a:hover");
        assert!(pseudo.contains(&"hover".to_string()));

        let pseudo2 = CssSelector::extract_pseudo_classes("input:focus:checked");
        assert!(pseudo2.contains(&"focus".to_string()));
        assert!(pseudo2.contains(&"checked".to_string()));
    }

    #[test]
    fn test_compound_selector() {
        let parts = CssSelector::parse_compound("div.active");
        assert_eq!(parts.len(), 2);
        assert!(parts.contains(&"div".to_string()));
        assert!(parts.contains(&".active".to_string()));
    }
}
