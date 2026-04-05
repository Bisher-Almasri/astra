use std::collections::HashMap;

use crate::css::{
    Declaration, MatchedDeclaration, Rule, Selector, SimpleSelector, StyleOrigin, Stylesheet,
    Value, resolve_cascade,
};
use crate::dom::{ElementData, Node, NodeType};

pub type PropertyMap = HashMap<String, Value>;

#[derive(Debug)]
pub struct StyledNode {
    pub node: Node,
    pub styles: PropertyMap,
    pub children: Vec<StyledNode>,
}

const INHERITED_PROPERTIES: &[&str] = &[
    "color",
    "font-size",
    "font-family",
    "font-weight",
    "font-style",
    "line-height",
    "text-align",
    "visibility",
];

pub struct StyleComputer {
    stylesheet: Stylesheet,
}

impl StyleComputer {
    pub fn new(stylesheet: Stylesheet) -> Self {
        Self { stylesheet }
    }

    pub fn compute_styles(&self, dom: &Node) -> StyledNode {
        self.compute_node(dom, &PropertyMap::new())
    }

    fn compute_node(&self, node: &Node, inherited: &PropertyMap) -> StyledNode {
        match &node.node_type {
            NodeType::Element(elem) => {
                let styles = self.compute_element_styles(elem, inherited);
                let child_inherited = self.build_inherited_map(&styles, inherited);
                let children = node
                    .children
                    .iter()
                    .map(|child| self.compute_node(child, &child_inherited))
                    .collect();
                StyledNode {
                    node: node.clone(),
                    styles,
                    children,
                }
            }
            NodeType::Text(_) => {
                let mut styles = PropertyMap::new();
                for prop in INHERITED_PROPERTIES {
                    if let Some(val) = inherited.get(*prop) {
                        styles.insert(prop.to_string(), val.clone());
                    }
                }
                StyledNode {
                    node: node.clone(),
                    styles,
                    children: vec![],
                }
            }
        }
    }

    fn compute_element_styles(
        &self,
        elem: &ElementData,
        inherited: &PropertyMap,
    ) -> PropertyMap {
        let mut matched: Vec<MatchedDeclaration> = self
            .stylesheet
            .rules
            .iter()
            .filter(|rule| self.rule_matches(rule, elem))
            .flat_map(|rule| {
                let spec = rule.max_specificity();
                rule.declarations.iter().map(move |decl| MatchedDeclaration {
                    declaration: decl.clone(),
                    specificity: spec,
                    origin: StyleOrigin::External,
                })
            })
            .collect();

        if let Some(inline_css) = elem.attributes.get("style") {
            let inline_decls = parse_inline_style(inline_css);
            for decl in inline_decls {
                matched.push(MatchedDeclaration {
                    declaration: decl,
                    specificity: (0, 0, 0),
                    origin: StyleOrigin::Inline,
                });
            }
        }

        let mut by_property: HashMap<String, Vec<MatchedDeclaration>> = HashMap::new();
        for m in matched {
            by_property
                .entry(m.declaration.name.clone())
                .or_default()
                .push(m);
        }

        let mut styles = PropertyMap::new();

        for prop in INHERITED_PROPERTIES {
            if let Some(val) = inherited.get(*prop) {
                styles.insert(prop.to_string(), val.clone());
            }
        }

        for (prop, decls) in &by_property {
            if let Some(winner) = resolve_cascade(decls) {
                styles.insert(prop.clone(), winner.declaration.value.clone());
            }
        }

        styles
    }

    fn rule_matches(&self, rule: &Rule, elem: &ElementData) -> bool {
        rule.selectors
            .iter()
            .any(|sel| self.selector_matches(sel, elem))
    }

    fn selector_matches(&self, selector: &Selector, elem: &ElementData) -> bool {
        match selector {
            Selector::Simple(simple) => self.simple_selector_matches(simple, elem),
        }
    }

    fn simple_selector_matches(&self, sel: &SimpleSelector, elem: &ElementData) -> bool {
        if let Some(ref tag) = sel.tag_name {
            if *tag != elem.tag_name {
                return false;
            }
        }

        if let Some(ref id) = sel.id {
            let elem_id = elem.attributes.get("id").map(String::as_str).unwrap_or("");
            if id != elem_id {
                return false;
            }
        }

        if !sel.class.is_empty() {
            let elem_classes: Vec<&str> = elem
                .attributes
                .get("class")
                .map(|s| s.split_whitespace().collect())
                .unwrap_or_default();
            for required_class in &sel.class {
                if !elem_classes.contains(&required_class.as_str()) {
                    return false;
                }
            }
        }

        true
    }

    fn build_inherited_map(
        &self,
        current_styles: &PropertyMap,
        parent_inherited: &PropertyMap,
    ) -> PropertyMap {
        let mut map = parent_inherited.clone();
        for prop in INHERITED_PROPERTIES {
            if let Some(val) = current_styles.get(*prop) {
                map.insert(prop.to_string(), val.clone());
            }
        }
        map
    }
}

fn parse_inline_style(style: &str) -> Vec<Declaration> {
    let mut decls = Vec::new();
    for part in style.split(';') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        if let Some(colon) = part.find(':') {
            let name = part[..colon].trim().to_string();
            let value_str = part[colon + 1..].trim().to_string();
            if name.is_empty() || value_str.is_empty() {
                continue;
            }
            let value = parse_inline_value(&value_str);
            decls.push(Declaration { name, value });
        }
    }
    decls
}

fn parse_inline_value(value_str: &str) -> Value {
    use crate::css::{Color, Unit};

    if value_str.ends_with("px") {
        if let Ok(n) = value_str[..value_str.len() - 2].parse::<f32>() {
            return Value::Length(n, Unit::Px);
        }
    } else if value_str.ends_with("em") {
        if let Ok(n) = value_str[..value_str.len() - 2].parse::<f32>() {
            return Value::Length(n, Unit::Em);
        }
    } else if value_str.ends_with('%') {
        if let Ok(n) = value_str[..value_str.len() - 1].parse::<f32>() {
            return Value::Length(n, Unit::Percent);
        }
    }

    match value_str.to_lowercase().as_str() {
        "red"   => return Value::ColorValue(Color { r: 255, g: 0,   b: 0,   a: 255 }),
        "green" => return Value::ColorValue(Color { r: 0,   g: 255, b: 0,   a: 255 }),
        "blue"  => return Value::ColorValue(Color { r: 0,   g: 0,   b: 255, a: 255 }),
        "black" => return Value::ColorValue(Color { r: 0,   g: 0,   b: 0,   a: 255 }),
        "white" => return Value::ColorValue(Color { r: 255, g: 255, b: 255, a: 255 }),
        _ => {}
    }

    Value::Keyword(value_str.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::css::{CssParser, Unit};
    use crate::dom::Node;
    use std::collections::HashMap;

    fn elem(tag: &str, attrs: &[(&str, &str)], children: Vec<Node>) -> Node {
        let mut map = HashMap::new();
        for (k, v) in attrs {
            map.insert(k.to_string(), v.to_string());
        }
        Node::elem(tag.to_string(), map, children)
    }

    fn parse_css(css: &str) -> Stylesheet {
        CssParser::new(css.to_string()).parse().unwrap()
    }

    #[test]
    fn test_tag_selector_matches() {
        let ss = parse_css("p { color: red; }");
        let computer = StyleComputer::new(ss);
        let dom = elem("p", &[], vec![]);
        let styled = computer.compute_styles(&dom);
        assert!(styled.styles.contains_key("color"));
    }

    #[test]
    fn test_tag_selector_no_match() {
        let ss = parse_css("p { color: red; }");
        let computer = StyleComputer::new(ss);
        let dom = elem("div", &[], vec![]);
        let styled = computer.compute_styles(&dom);
        assert!(!styled.styles.contains_key("color"));
    }

    #[test]
    fn test_class_selector_matches() {
        let ss = parse_css(".highlight { color: blue; }");
        let computer = StyleComputer::new(ss);
        let dom = elem("span", &[("class", "highlight")], vec![]);
        let styled = computer.compute_styles(&dom);
        assert!(styled.styles.contains_key("color"));
    }

    #[test]
    fn test_class_selector_multiple_classes() {
        let ss = parse_css(".bold { font-weight: bold; }");
        let computer = StyleComputer::new(ss);
        let dom = elem("p", &[("class", "text bold highlight")], vec![]);
        let styled = computer.compute_styles(&dom);
        assert!(styled.styles.contains_key("font-weight"));
    }

    #[test]
    fn test_id_selector_matches() {
        let ss = parse_css("#main { color: green; }");
        let computer = StyleComputer::new(ss);
        let dom = elem("div", &[("id", "main")], vec![]);
        let styled = computer.compute_styles(&dom);
        assert!(styled.styles.contains_key("color"));
    }

    #[test]
    fn test_combined_selector_all_parts_must_match() {
        let ss = parse_css("div.container { color: red; }");
        let computer = StyleComputer::new(ss);

        let dom_match = elem("div", &[("class", "container")], vec![]);
        let styled = computer.compute_styles(&dom_match);
        assert!(styled.styles.contains_key("color"));

        let dom_no = elem("span", &[("class", "container")], vec![]);
        let styled_no = computer.compute_styles(&dom_no);
        assert!(!styled_no.styles.contains_key("color"));
    }

    #[test]
    fn test_higher_specificity_wins() {
        let ss = parse_css("p { display: block; } #hero { display: inline; }");
        let computer = StyleComputer::new(ss);
        let dom = elem("p", &[("id", "hero")], vec![]);
        let styled = computer.compute_styles(&dom);
        let Value::Keyword(display) = styled.styles.get("display").unwrap() else { panic!() };
        assert_eq!(display, "inline");
    }

    #[test]
    fn test_inline_style_overrides_external() {
        let ss = parse_css("p { display: block; }");
        let computer = StyleComputer::new(ss);
        let dom = elem("p", &[("style", "display: inline;")], vec![]);
        let styled = computer.compute_styles(&dom);
        let Value::Keyword(display) = styled.styles.get("display").unwrap() else { panic!() };
        assert_eq!(display, "inline");
    }

    #[test]
    fn test_inline_style_length_value() {
        let ss = parse_css("");
        let computer = StyleComputer::new(ss);
        let dom = elem("div", &[("style", "font-size: 18px;")], vec![]);
        let styled = computer.compute_styles(&dom);
        let Value::Length(size, Unit::Px) = styled.styles.get("font-size").unwrap() else { panic!() };
        assert_eq!(*size, 18.0);
    }


    #[test]
    fn test_color_inherited_by_child() {
        let ss = parse_css("div { color: red; }");
        let computer = StyleComputer::new(ss);
        let child = elem("span", &[], vec![]);
        let dom = elem("div", &[], vec![child]);
        let styled = computer.compute_styles(&dom);
        let child_styled = &styled.children[0];
        assert!(child_styled.styles.contains_key("color"));
    }

    #[test]
    fn test_child_overrides_inherited_color() {
        let ss = parse_css("div { display: block; } span { display: inline; }");
        let computer = StyleComputer::new(ss);
        let child = elem("span", &[], vec![]);
        let dom = elem("div", &[], vec![child]);
        let styled = computer.compute_styles(&dom);
        let child_styled = &styled.children[0];
        let Value::Keyword(display) = child_styled.styles.get("display").unwrap() else { panic!() };
        assert_eq!(display, "inline");
    }

    #[test]
    fn test_non_inherited_property_not_propagated() {
        let ss = parse_css("div { margin: 10px; }");
        let computer = StyleComputer::new(ss);
        let child = elem("span", &[], vec![]);
        let dom = elem("div", &[], vec![child]);
        let styled = computer.compute_styles(&dom);
        let child_styled = &styled.children[0];
        assert!(!child_styled.styles.contains_key("margin"));
    }

    #[test]
    fn test_text_node_inherits_color() {
        let ss = parse_css("p { color: green; }");
        let computer = StyleComputer::new(ss);
        let text = Node::text("hello".to_string());
        let dom = elem("p", &[], vec![text]);
        let styled = computer.compute_styles(&dom);
        let text_styled = &styled.children[0];
        assert!(text_styled.styles.contains_key("color"));
    }
}
