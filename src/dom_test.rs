use quickcheck::{Arbitrary, Gen};
use std::collections::HashMap;

use crate::dom::{AttrMap, ElementData, Node, NodeType};

impl Arbitrary for ElementData {
    fn arbitrary(g: &mut Gen) -> Self {
        let tags = ["div", "span", "p", "a", "ul", "li", "h1", "section"];
        let tag_name = tags[usize::arbitrary(g) % tags.len()].to_string();

        let mut attributes: AttrMap = HashMap::new();
        let num_attrs = usize::arbitrary(g) % 3;
        for _ in 0..num_attrs {
            let keys = ["id", "class", "href", "style"];
            let key = keys[usize::arbitrary(g) % keys.len()].to_string();
            let value = String::arbitrary(g)
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .take(10)
                .collect();
            attributes.insert(key, value);
        }

        ElementData { tag_name, attributes }
    }
}

impl Arbitrary for NodeType {
    fn arbitrary(g: &mut Gen) -> Self {
        if bool::arbitrary(g) {
            NodeType::Element(ElementData::arbitrary(g))
        } else {
            let text: String = String::arbitrary(g)
                .chars()
                .filter(|c| c.is_alphanumeric() || c.is_whitespace())
                .take(20)
                .collect();
            NodeType::Text(text)
        }
    }
}

impl Arbitrary for Node {
    fn arbitrary(g: &mut Gen) -> Self {
        let node_type = NodeType::arbitrary(g);

        let children = match &node_type {
            NodeType::Text(_) => vec![],
            NodeType::Element(_) => {
                let num_children = if g.size() > 2 { usize::arbitrary(g) % 3 } else { 0 };
                let mut kids = Vec::new();
                if num_children > 0 {
                    let mut child_gen = Gen::new(g.size() / 2);
                    for _ in 0..num_children {
                        kids.push(Node::arbitrary(&mut child_gen));
                    }
                }
                kids
            }
        };

        Node { children, node_type }
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn text_nodes_have_no_children(text: String) -> bool {
        let node = Node::text(text);
        node.children.is_empty()
    }

    #[quickcheck]
    fn elem_nodes_preserve_tag_and_attrs(tag: String, num_children: u8) -> bool {
        let tag = if tag.is_empty() { "div".to_string() } else { tag };
        let attrs: AttrMap = HashMap::new();
        let children: Vec<Node> = (0..num_children % 4)
            .map(|i| Node::text(format!("child {i}")))
            .collect();
        let expected_children = children.len();
        let node = Node::elem(tag.clone(), attrs.clone(), children);

        match &node.node_type {
            NodeType::Element(data) => data.tag_name == tag && node.children.len() == expected_children,
            _ => false,
        }
    }

    #[quickcheck]
    fn arbitrary_nodes_are_valid(node: Node) -> bool {
        match &node.node_type {
            NodeType::Text(_) => node.children.is_empty(),
            NodeType::Element(_) => true,
        }
    }
}
