use quickcheck::{Arbitrary, Gen};
use std::collections::HashMap;

use crate::html::HtmlParser;

#[derive(Debug, Clone)]
struct TagName(String);

impl Arbitrary for TagName {
    fn arbitrary(g: &mut Gen) -> Self {
        let tags = ["div", "span", "p", "section", "article", "main", "header", "footer", "ul", "li"];
        TagName(tags[usize::arbitrary(g) % tags.len()].to_string())
    }
}

#[derive(Debug, Clone)]
struct SafeAttrs(HashMap<String, String>);

impl Arbitrary for SafeAttrs {
    fn arbitrary(g: &mut Gen) -> Self {
        let keys = ["id", "class", "data-x", "role"];
        let mut map = HashMap::new();
        let n = usize::arbitrary(g) % 3;
        for _ in 0..n {
            let key = keys[usize::arbitrary(g) % keys.len()].to_string();
            let val: String = String::arbitrary(g)
                .chars()
                .filter(|c| c.is_alphanumeric() || *c == '-' || *c == '_')
                .take(8)
                .collect();
            map.insert(key, val);
        }
        SafeAttrs(map)
    }
}

#[derive(Debug, Clone)]
struct SafeText(String);

impl Arbitrary for SafeText {
    fn arbitrary(g: &mut Gen) -> Self {
        let text: String = String::arbitrary(g)
            .chars()
            .filter(|c| c.is_alphanumeric() || c.is_whitespace())
            .take(20)
            .collect();
        SafeText(text)
    }
}

fn build_simple_html(tag: &str, attrs: &HashMap<String, String>, text: &str) -> String {
    let attr_str: String = attrs
        .iter()
        .map(|(k, v)| format!(" {}=\"{}\"", k, v))
        .collect();
    format!("<{}{}>{}</{}>", tag, attr_str, text, tag)
}

fn build_nested_html(outer: &str, inner: &str, text: &str) -> String {
    format!("<{}><{}>{}</{}></{}>", outer, inner, text, inner, outer)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dom::NodeType;
    use quickcheck_macros::quickcheck;

    #[quickcheck]
    fn nested_elements_preserve_parent_child_relationships(
        outer: TagName,
        inner: TagName,
        text: SafeText,
    ) -> bool {
        let html = build_nested_html(&outer.0, &inner.0, &text.0);
        let mut parser = HtmlParser::new(html);
        match parser.parse() {
            Ok(root) => {
                match &root.node_type {
                    NodeType::Element(data) => {
                        if data.tag_name != outer.0 {
                            return false;
                        }
                        if root.children.len() != 1 {
                            return false;
                        }
                        match &root.children[0].node_type {
                            NodeType::Element(inner_data) => inner_data.tag_name == inner.0,
                            _ => false,
                        }
                    }
                    _ => false,
                }
            }
            Err(_) => false,
        }
    }

    #[quickcheck]
    fn attributes_preserved_in_dom(tag: TagName, attrs: SafeAttrs, text: SafeText) -> bool {
        let html = build_simple_html(&tag.0, &attrs.0, &text.0);
        let mut parser = HtmlParser::new(html);
        match parser.parse() {
            Ok(root) => match &root.node_type {
                NodeType::Element(data) => {
                    attrs.0.iter().all(|(k, v)| data.attributes.get(k) == Some(v))
                }
                _ => false,
            },
            Err(_) => false,
        }
    }

    #[quickcheck]
    fn text_content_preserved_in_dom(tag: TagName, text: SafeText) -> bool {
        let trimmed = text.0.trim().to_string();
        if trimmed.is_empty() {
            return true; 
        }
        let html = format!("<{}>{}</{}>", tag.0, trimmed, tag.0);
        let mut parser = HtmlParser::new(html);
        match parser.parse() {
            Ok(root) => {
                root.children.iter().any(|child| match &child.node_type {
                    NodeType::Text(t) => t.trim() == trimmed,
                    _ => false,
                })
            }
            Err(_) => false,
        }
    }

    #[quickcheck]
    fn malformed_html_does_not_panic(input: String) -> bool {
        let mut parser = HtmlParser::new(input);
        let _ = parser.parse(); 
        true
    }

    #[quickcheck]
    fn unclosed_tag_returns_error(tag: TagName) -> bool {
        let html = format!("<{}>some text", tag.0);
        let mut parser = HtmlParser::new(html);
        parser.parse().is_err()
    }

    #[quickcheck]
    fn valid_html_parses_successfully(tag: TagName, text: SafeText) -> bool {
        let html = format!("<{}>{}</{}>", tag.0, text.0, tag.0);
        let mut parser = HtmlParser::new(html);
        parser.parse().is_ok()
    }
}
