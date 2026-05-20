/// Repräsentiert einen einzelnen Knoten im Document Object Model.
#[derive(Debug, Clone)]
pub struct Node {
    pub node_type: NodeType,
    pub children: Vec<Node>,
}

#[derive(Debug, Clone)]
pub enum NodeType {
    Element(ElementData),
    Text(String),
}

/// Metadaten eines Element-Knotens.
#[derive(Debug, Clone)]
pub struct ElementData {
    pub tag_name: String,
    pub attributes: Vec<(String, String)>,
}

impl ElementData {
    /// Gibt den Wert eines Attributs zurück, falls vorhanden.
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attributes
            .iter()
            .find(|(k, _)| k == name)
            .map(|(_, v)| v.as_str())
    }

    pub fn id(&self) -> Option<&str> {
        self.attr("id")
    }

    pub fn class(&self) -> Option<&str> {
        self.attr("class")
    }
}

impl Node {
    /// Erstellt einen Element-Knoten.
    pub fn element(
        tag: &str,
        attrs: Vec<(&str, &str)>,
        children: Vec<Node>,
    ) -> Self {
        Node {
            node_type: NodeType::Element(ElementData {
                tag_name: tag.to_string(),
                attributes: attrs
                    .into_iter()
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect(),
            }),
            children,
        }
    }

    /// Erstellt einen Text-Knoten.
    pub fn text(content: &str) -> Self {
        Node {
            node_type: NodeType::Text(content.to_string()),
            children: vec![],
        }
    }

    /// Gibt den Tag-Namen zurück (oder "#text" für Text-Knoten).
    pub fn tag_name(&self) -> &str {
        match &self.node_type {
            NodeType::Element(e) => &e.tag_name,
            NodeType::Text(_) => "#text",
        }
    }
}