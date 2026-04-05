#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedEof,
    InvalidCharacter(char),
    MissingClosingBracket,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    Simple(SimpleSelector),
}

pub type Specificity = (usize, usize, usize);

#[derive(Debug, Clone, PartialEq)]
pub struct SimpleSelector {
    pub tag_name: Option<String>,
    pub id: Option<String>,
    pub class: Vec<String>,
}

impl SimpleSelector {
    pub fn specificity(&self) -> Specificity {
        let a = self.id.iter().count();
        let b = self.class.len();
        let c = self.tag_name.iter().count();
        (a, b, c)
    }
}

impl Selector {
    pub fn specificity(&self) -> Specificity {
        match self {
            Selector::Simple(s) => s.specificity(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub name: String,
    pub value: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Keyword(String),
    Length(f32, Unit),
    ColorValue(Color),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Unit {
    Px,
    Em,
    Percent,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
    pub a: u8,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

impl Rule {
    pub fn max_specificity(&self) -> Specificity {
        self.selectors
            .iter()
            .map(|s| s.specificity())
            .max()
            .unwrap_or((0, 0, 0))
    }
}

#[derive(Debug)]
pub struct Stylesheet {
    pub rules: Vec<Rule>,
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedEof => write!(f, "Unexpected end of input"),
            ParseError::InvalidCharacter(c) => write!(f, "Invalid character: '{}'", c),
            ParseError::MissingClosingBracket => write!(f, "Missing closing bracket"),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum StyleOrigin {
    External,
    Inline,
}

#[derive(Debug, Clone)]
pub struct MatchedDeclaration {
    pub declaration: Declaration,
    pub specificity: Specificity,
    pub origin: StyleOrigin,
}

impl MatchedDeclaration {
    pub fn overrides(&self, other: &MatchedDeclaration) -> bool {
        if self.origin != other.origin {
            return self.origin > other.origin;
        }
        self.specificity >= other.specificity
    }
}

pub fn resolve_cascade(declarations: &[MatchedDeclaration]) -> Option<&MatchedDeclaration> {
    declarations.iter().reduce(|winner, candidate| {
        if candidate.overrides(winner) {
            candidate
        } else {
            winner
        }
    })
}

#[derive(Debug)]
pub struct CssTokenizer {
    input: String,
    position: usize,
}

impl CssTokenizer {
    pub fn new(input: String) -> Self {
        Self { input, position: 0 }
    }

    pub fn parse(&mut self) -> Result<Stylesheet, ParseError> {
        let mut rules = Vec::new();

        while self.skip_whitespace() {
            rules.push(self.parse_rule()?);
        }

        Ok(Stylesheet { rules })
    }

    fn parse_rule(&mut self) -> Result<Rule, ParseError> {
        let selectors = self.parse_selectors()?;
        self.expect_char('{')?;

        let declarations = self.parse_declarations()?;
        self.expect_char('}')?;

        Ok(Rule {
            selectors,
            declarations,
        })
    }

    fn parse_selectors(&mut self) -> Result<Vec<Selector>, ParseError> {
        let mut selectors = Vec::new();

        loop {
            self.skip_whitespace();
            let selector = self.parse_simple_selector()?;
            selectors.push(Selector::Simple(selector));

            self.skip_whitespace();

            match self.current_char()? {
                ',' => {
                    self.position += 1;
                    self.skip_whitespace();
                }
                '{' => break,
                c => return Err(ParseError::InvalidCharacter(c)),
            }
        }

        Ok(selectors)
    }

    fn parse_simple_selector(&mut self) -> Result<SimpleSelector, ParseError> {
        let mut tag_name = None;
        let mut id = None;
        let mut class = Vec::new();

        loop {
            match self.current_char()? {
                '#' => {
                    self.position += 1;
                    let id_name = self.consume_while(|c| c.is_alphanumeric() || c == '-' || c == '_');
                    if id_name.is_empty() {
                        return Err(ParseError::InvalidCharacter('#'));
                    }
                    id = Some(id_name);
                }
                '.' => {
                    self.position += 1;
                    let class_name = self.consume_while(|c| c.is_alphanumeric() || c == '-' || c == '_');
                    if class_name.is_empty() {
                        return Err(ParseError::InvalidCharacter('.'));
                    }
                    class.push(class_name);
                }
                c if c.is_alphabetic() => {
                    let tag = self.consume_while(|c| c.is_alphanumeric() || c == '-');
                    if tag.is_empty() {
                        return Err(ParseError::InvalidCharacter(c));
                    }
                    tag_name = Some(tag);
                }
                _ => break,
            }
        }

        if tag_name.is_none() && id.is_none() && class.is_empty() {
            return Err(ParseError::InvalidCharacter(self.current_char()?));
        }

        Ok(SimpleSelector {
            tag_name,
            id,
            class,
        })
    }

    fn parse_declarations(&mut self) -> Result<Vec<Declaration>, ParseError> {
        let mut declarations = Vec::new();

        loop {
            self.skip_whitespace();
            if self.current_char()? == '}' {
                break;
            }

            let name = self.consume_while(|c| c.is_alphanumeric() || c == '-');
            if name.is_empty() {
                return Err(ParseError::InvalidCharacter(self.current_char()?));
            }

            self.skip_whitespace();
            self.expect_char(':')?;
            self.skip_whitespace();

            let value = self.parse_value()?;

            self.skip_whitespace();
            self.expect_char(';')?;

            declarations.push(Declaration { name, value });
        }

        Ok(declarations)
    }

    fn parse_value(&mut self) -> Result<Value, ParseError> {
        self.skip_whitespace();

        let value_str = self.consume_while(|c| c != ';' && c != '}').trim().to_string();

        if value_str.is_empty() {
            return Err(ParseError::InvalidCharacter(self.current_char()?));
        }

        if let Some(length_value) = Self::try_parse_length(&value_str) {
            return Ok(length_value);
        }

        if let Some(color_value) = Self::try_parse_color(&value_str) {
            return Ok(color_value);
        }

        Ok(Value::Keyword(value_str))
    }

    fn try_parse_length(value: &str) -> Option<Value> {
        if value.ends_with("px") {
            if let Ok(num) = value[..value.len() - 2].parse::<f32>() {
                return Some(Value::Length(num, Unit::Px));
            }
        } else if value.ends_with("em") {
            if let Ok(num) = value[..value.len() - 2].parse::<f32>() {
                return Some(Value::Length(num, Unit::Em));
            }
        } else if value.ends_with('%') {
            if let Ok(num) = value[..value.len() - 1].parse::<f32>() {
                return Some(Value::Length(num, Unit::Percent));
            }
        }
        None
    }

    fn try_parse_color(value: &str) -> Option<Value> {
        match value.to_lowercase().as_str() {
            "red"   => Some(Value::ColorValue(Color { r: 255, g: 0,   b: 0,   a: 255 })),
            "green" => Some(Value::ColorValue(Color { r: 0,   g: 255, b: 0,   a: 255 })),
            "blue"  => Some(Value::ColorValue(Color { r: 0,   g: 0,   b: 255, a: 255 })),
            "black" => Some(Value::ColorValue(Color { r: 0,   g: 0,   b: 0,   a: 255 })),
            "white" => Some(Value::ColorValue(Color { r: 255, g: 255, b: 255, a: 255 })),
            _ => None,
        }
    }

    fn skip_whitespace(&mut self) -> bool {
        self.consume_while(|c| c.is_whitespace());
        self.position < self.input.len()
    }

    fn consume_while<F>(&mut self, test: F) -> String
    where
        F: Fn(char) -> bool,
    {
        let mut result = String::new();
        while let Ok(c) = self.current_char() {
            if !test(c) {
                break;
            }
            result.push(c);
            self.position += c.len_utf8();
        }
        result
    }

    fn expect_char(&mut self, expected: char) -> Result<(), ParseError> {
        match self.current_char() {
            Ok(c) if c == expected => {
                self.position += c.len_utf8();
                Ok(())
            }
            Ok(c) => Err(ParseError::InvalidCharacter(c)),
            Err(_) => Err(ParseError::UnexpectedEof),
        }
    }

    fn current_char(&self) -> Result<char, ParseError> {
        self.input[self.position..]
            .chars()
            .next()
            .ok_or(ParseError::UnexpectedEof)
    }
}

#[derive(Debug)]
pub struct CssParser {
    tokenizer: CssTokenizer,
}

impl CssParser {
    pub fn new(css: String) -> Self {
        Self {
            tokenizer: CssTokenizer::new(css),
        }
    }

    pub fn parse(&mut self) -> Result<Stylesheet, ParseError> {
        self.tokenizer.parse()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_tag_selector() {
        let mut parser = CssParser::new("h1 { color: red; }".to_string());
        let stylesheet = parser.parse().unwrap();

        assert_eq!(stylesheet.rules.len(), 1);
        let rule = &stylesheet.rules[0];

        assert_eq!(rule.selectors.len(), 1);
        let Selector::Simple(selector) = &rule.selectors[0];
        assert_eq!(selector.tag_name, Some("h1".to_string()));
        assert_eq!(selector.id, None);
        assert!(selector.class.is_empty());

        assert_eq!(rule.declarations.len(), 1);
        let decl = &rule.declarations[0];
        assert_eq!(decl.name, "color");
        let Value::ColorValue(color) = &decl.value else { panic!("Expected ColorValue") };
        assert_eq!((color.r, color.g, color.b), (255, 0, 0));
    }

    #[test]
    fn test_parse_class_selector() {
        let mut parser = CssParser::new(".highlight { font-size: 16px; }".to_string());
        let stylesheet = parser.parse().unwrap();

        let rule = &stylesheet.rules[0];
        let Selector::Simple(selector) = &rule.selectors[0];
        assert_eq!(selector.tag_name, None);
        assert_eq!(selector.id, None);
        assert_eq!(selector.class, vec!["highlight".to_string()]);

        let decl = &rule.declarations[0];
        assert_eq!(decl.name, "font-size");
        let Value::Length(size, Unit::Px) = &decl.value else { panic!("Expected Length/Px") };
        assert_eq!(*size, 16.0);
    }

    #[test]
    fn test_parse_id_selector() {
        let mut parser = CssParser::new("#main { width: 100%; }".to_string());
        let stylesheet = parser.parse().unwrap();

        let rule = &stylesheet.rules[0];
        let Selector::Simple(selector) = &rule.selectors[0];
        assert_eq!(selector.tag_name, None);
        assert_eq!(selector.id, Some("main".to_string()));
        assert!(selector.class.is_empty());

        let decl = &rule.declarations[0];
        assert_eq!(decl.name, "width");
        let Value::Length(size, Unit::Percent) = &decl.value else { panic!("Expected Length/Percent") };
        assert_eq!(*size, 100.0);
    }

    #[test]
    fn test_parse_combined_selector() {
        let mut parser = CssParser::new("div.container#main { margin: 10px; }".to_string());
        let stylesheet = parser.parse().unwrap();

        let rule = &stylesheet.rules[0];
        let Selector::Simple(selector) = &rule.selectors[0];
        assert_eq!(selector.tag_name, Some("div".to_string()));
        assert_eq!(selector.id, Some("main".to_string()));
        assert_eq!(selector.class, vec!["container".to_string()]);
    }

    #[test]
    fn test_parse_multiple_selectors() {
        let mut parser = CssParser::new("h1, h2, h3 { font-weight: bold; }".to_string());
        let stylesheet = parser.parse().unwrap();

        let rule = &stylesheet.rules[0];
        assert_eq!(rule.selectors.len(), 3);

        for (i, expected_tag) in ["h1", "h2", "h3"].iter().enumerate() {
            let Selector::Simple(selector) = &rule.selectors[i];
            assert_eq!(selector.tag_name, Some(expected_tag.to_string()));
        }
    }

    #[test]
    fn test_parse_multiple_declarations() {
        let mut parser = CssParser::new(
            "p { color: blue; font-size: 14px; margin: 5px; }".to_string(),
        );
        let stylesheet = parser.parse().unwrap();

        let rule = &stylesheet.rules[0];
        assert_eq!(rule.declarations.len(), 3);

        let Value::ColorValue(color) = &rule.declarations[0].value else { panic!() };
        assert_eq!(color.b, 255);

        let Value::Length(size, Unit::Px) = &rule.declarations[1].value else { panic!() };
        assert_eq!(*size, 14.0);

        let Value::Length(margin, Unit::Px) = &rule.declarations[2].value else { panic!() };
        assert_eq!(*margin, 5.0);
    }

    #[test]
    fn test_parse_keyword_value() {
        let mut parser = CssParser::new("div { display: block; }".to_string());
        let stylesheet = parser.parse().unwrap();

        let decl = &stylesheet.rules[0].declarations[0];
        assert_eq!(decl.name, "display");
        let Value::Keyword(kw) = &decl.value else { panic!("Expected Keyword") };
        assert_eq!(kw, "block");
    }

    #[test]
    fn test_parse_multiple_rules() {
        let css = "h1 { color: red; } p { font-size: 12px; }";
        let mut parser = CssParser::new(css.to_string());
        let stylesheet = parser.parse().unwrap();
        assert_eq!(stylesheet.rules.len(), 2);
    }

    #[test]
    fn test_specificity_tag_only() {
        let sel = SimpleSelector { tag_name: Some("div".into()), id: None, class: vec![] };
        assert_eq!(sel.specificity(), (0, 0, 1));
    }

    #[test]
    fn test_specificity_class_only() {
        let sel = SimpleSelector { tag_name: None, id: None, class: vec!["foo".into()] };
        assert_eq!(sel.specificity(), (0, 1, 0));
    }

    #[test]
    fn test_specificity_id_only() {
        let sel = SimpleSelector { tag_name: None, id: Some("bar".into()), class: vec![] };
        assert_eq!(sel.specificity(), (1, 0, 0));
    }

    #[test]
    fn test_specificity_combined() {
        let sel = SimpleSelector {
            tag_name: Some("div".into()),
            id: Some("main".into()),
            class: vec!["container".into()],
        };
        assert_eq!(sel.specificity(), (1, 1, 1));
    }

    #[test]
    fn test_specificity_multiple_classes() {
        let sel = SimpleSelector {
            tag_name: None,
            id: None,
            class: vec!["a".into(), "b".into(), "c".into()],
        };
        assert_eq!(sel.specificity(), (0, 3, 0));
    }

    #[test]
    fn test_id_beats_class() {
        let id_sel = SimpleSelector { tag_name: None, id: Some("x".into()), class: vec![] };
        let class_sel = SimpleSelector {
            tag_name: None,
            id: None,
            class: vec!["a".into(), "b".into(), "c".into(), "d".into()],
        };
        assert!(id_sel.specificity() > class_sel.specificity());
    }

    #[test]
    fn test_rule_max_specificity() {
        let mut parser = CssParser::new("h1, #hero { color: red; }".to_string());
        let stylesheet = parser.parse().unwrap();
        assert_eq!(stylesheet.rules[0].max_specificity(), (1, 0, 0));
    }

    #[test]
    fn test_inline_overrides_external_same_specificity() {
        let external = MatchedDeclaration {
            declaration: Declaration { name: "color".into(), value: Value::Keyword("blue".into()) },
            specificity: (0, 1, 0),
            origin: StyleOrigin::External,
        };
        let inline = MatchedDeclaration {
            declaration: Declaration { name: "color".into(), value: Value::Keyword("red".into()) },
            specificity: (0, 1, 0),
            origin: StyleOrigin::Inline,
        };

        let decls = [external, inline];
        let winner = resolve_cascade(&decls).unwrap();
        let Value::Keyword(kw) = &winner.declaration.value else { panic!() };
        assert_eq!(kw, "red");
    }

    #[test]
    fn test_higher_specificity_wins_same_origin() {
        let low = MatchedDeclaration {
            declaration: Declaration { name: "color".into(), value: Value::Keyword("blue".into()) },
            specificity: (0, 0, 1),
            origin: StyleOrigin::External,
        };
        let high = MatchedDeclaration {
            declaration: Declaration { name: "color".into(), value: Value::Keyword("red".into()) },
            specificity: (0, 1, 0),
            origin: StyleOrigin::External,
        };

        let decls = [low, high];
        let winner = resolve_cascade(&decls).unwrap();
        let Value::Keyword(kw) = &winner.declaration.value else { panic!() };
        assert_eq!(kw, "red");
    }

    #[test]
    fn test_inline_beats_higher_specificity_external() {
        let external_high = MatchedDeclaration {
            declaration: Declaration { name: "color".into(), value: Value::Keyword("blue".into()) },
            specificity: (1, 2, 3),
            origin: StyleOrigin::External,
        };
        let inline_low = MatchedDeclaration {
            declaration: Declaration { name: "color".into(), value: Value::Keyword("red".into()) },
            specificity: (0, 0, 0),
            origin: StyleOrigin::Inline,
        };

        let decls = [external_high, inline_low];
        let winner = resolve_cascade(&decls).unwrap();
        let Value::Keyword(kw) = &winner.declaration.value else { panic!() };
        assert_eq!(kw, "red");
    }

    #[test]
    fn test_resolve_cascade_empty() {
        let result = resolve_cascade(&[]);
        assert!(result.is_none());
    }
}
