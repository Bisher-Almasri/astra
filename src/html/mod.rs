use std::collections::HashMap;

use crate::dom::Node;

#[cfg(test)]
mod html_test;


#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedEof,
    InvalidCharacter(char),
    MalformedTag(String),
    MissingClosingTag(String),
    InvalidAttribute(String),
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ParseError::UnexpectedEof => write!(f, "Unexpected end of input"),
            ParseError::InvalidCharacter(c) => write!(f, "Invalid character: '{}'", c),
            ParseError::MalformedTag(tag) => write!(f, "Missing closing tag: '{}'", tag),
            ParseError::MissingClosingTag(tag) => write!(f, "Missingclosing tag: '{}'", tag),
            ParseError::InvalidAttribute(attr) => write!(f, "Invalid attribute: '{}'", attr),
        }
    }
}

impl std::error::Error for ParseError {}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    StartTag {
        name: String,
        attributes: HashMap<String, String>,
        self_closing: bool,
    },
    EndTag {
        name: String,
    },
    Text(String),
    Doctype(String),
    Comment(String),
    Eof,
}

#[derive(Debug)]
pub struct HtmlTokenizer {
    input: String,
    position: usize,
}

impl HtmlTokenizer {
    pub fn new(input: String) -> Self {
        Self { input, position: 0 }
    }

    pub fn next_token(&mut self) -> Result<Token, ParseError> {
        self.skip_whitespace();

        if self.position >= self.input.len() {
            return Ok(Token::Eof);
        }

        let current_char = self.current_char()?;

        if current_char == '<' {
            self.parse_tag()
        } else {
            self.parse_text()
        }
    }

    pub fn has_more_tokens(&self) -> bool {
        self.position < self.input.len()
    }

    fn current_char(&self) -> Result<char, ParseError> {
        self.input[self.position..]
            .chars()
            .next()
            .ok_or(ParseError::UnexpectedEof)
    }

    fn advance(&mut self) {
        if let Some(ch) = self.input[self.position..].chars().next() {
            self.position += ch.len_utf8();
        }
    }

    fn skip_whitespace(&mut self) {
        while self.position < self.input.len() {
            if let Some(ch) = self.input[self.position..].chars().next() {
                if ch.is_whitespace() {
                    self.position += ch.len_utf8();
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn parse_tag(&mut self) -> Result<Token, ParseError> {
        self.advance();

        if let Ok(ch) = self.current_char() {
            if ch == '!' {
                return self.parse_special_construct();
            } else if ch == '/' {
                self.advance();
                let tag_name = self.parse_tag_name()?;
                self.skip_to_char('>')?;
                self.advance();
                return Ok(Token::EndTag { name: tag_name });
            }
        } else {
            return Err(ParseError::UnexpectedEof);
        }

        let tag_name = self.parse_tag_name()?;

        let attributes = self.parse_attributes()?;

        let self_closing = if let Ok(ch) = self.current_char() {
            if ch == '/' {
                self.advance();
                true
            } else {
                false
            }
        } else {
            false
        };

        self.skip_to_char('>')?;
        self.advance();

        Ok(Token::StartTag {
            name: tag_name,
            attributes,
            self_closing,
        })
    }

    fn parse_special_construct(&mut self) -> Result<Token, ParseError> {
        self.advance();

        if self.input[self.position..].starts_with("DOCTYPE") {
            self.position += 7;

            let mut doctype_content = String::new();
            while self.position < self.input.len() {
                let ch = self.current_char()?;
                if ch == '>' {
                    self.advance();
                    break;
                } else {
                    doctype_content.push(ch);
                    self.advance();
                }
            }

            return Ok(Token::Doctype(doctype_content.trim().to_string()));
        }

        if self.input[self.position..].starts_with("--") {
            self.position += 2;

            let mut comment_content = String::new();
            while self.position < self.input.len() - 2 {
                if self.input[self.position..].starts_with("-->") {
                    self.position += 3;
                    break;
                } else {
                    comment_content.push(self.current_char()?);
                    self.advance();
                }
            }

            return Ok(Token::Comment(comment_content));
        }

        Err(ParseError::MalformedTag(
            "Unknown special construct".to_string(),
        ))
    }

    fn parse_tag_name(&mut self) -> Result<String, ParseError> {
        let mut name = String::new();

        while self.position < self.input.len() {
            let ch = self.current_char()?;
            if ch.is_alphanumeric() || ch == '-' || ch == '_' {
                name.push(ch);
                self.advance();
            } else {
                break;
            }
        }

        if name.is_empty() {
            Err(ParseError::MalformedTag("Empty tag name".to_string()))
        } else {
            Ok(name)
        }
    }

    fn parse_attributes(&mut self) -> Result<HashMap<String, String>, ParseError> {
        let mut attributes = HashMap::new();

        loop {
            self.skip_whitespace();

            if self.position >= self.input.len() {
                break;
            }

            let ch = self.current_char()?;
            if ch == '>' || ch == '/' {
                break;
            }

            let attr_name = self.parse_tag_name()?;

            self.skip_whitespace();

            if let Ok(ch) = self.current_char() {
                if ch == '=' {
                    self.advance();
                    self.skip_whitespace();
                    let attr_value = self.parse_attribute_value()?;
                    attributes.insert(attr_name, attr_value);
                } else {
                    attributes.insert(attr_name, String::new());
                }
            } else {
                attributes.insert(attr_name, String::new());
                break;
            }
        }

        Ok(attributes)
    }

    fn parse_attribute_value(&mut self) -> Result<String, ParseError> {
        let ch = self.current_char()?;

        if ch == '"' || ch == '\'' {
            let quote_char = ch;
            self.advance();

            let mut value = String::new();
            while self.position < self.input.len() {
                let ch = self.current_char()?;
                if ch == quote_char {
                    self.advance();
                    break;
                } else {
                    value.push(ch);
                    self.advance();
                }
            }
            Ok(value)
        } else {
            let mut value = String::new();
            while self.position < self.input.len() {
                let ch = self.current_char()?;
                if ch.is_whitespace() || ch == '>' || ch == '/' {
                    break;
                } else {
                    value.push(ch);
                    self.advance();
                }
            }
            Ok(value)
        }
    }

    fn parse_text(&mut self) -> Result<Token, ParseError> {
        let mut text = String::new();

        while self.position < self.input.len() {
            let ch = self.current_char()?;
            if ch == '<' {
                break;
            } else {
                text.push(ch);
                self.advance();
            }
        }

        Ok(Token::Text(text.trim().to_string()))
    }

    fn skip_to_char(&mut self, target: char) -> Result<(), ParseError> {
        while self.position < self.input.len() {
            let ch = self.current_char()?;
            if ch == target {
                return Ok(());
            }
            self.advance();
        }
        Err(ParseError::UnexpectedEof)
    }
}

#[derive(Debug)]
pub struct HtmlParser {
    tokenizer: HtmlTokenizer,
}

impl HtmlParser {
    pub fn new(html: String) -> Self {
        Self {
            tokenizer: HtmlTokenizer::new(html),
        }
    }

    pub fn parse(&mut self) -> Result<Node, ParseError> {
        let mut nodes = Vec::new();

        while self.tokenizer.has_more_tokens() {
            let token = self.tokenizer.next_token()?;
            match token {
                Token::Eof => break,
                Token::Text(text) => {
                    if !text.trim().is_empty() {
                        nodes.push(Node::text(text));
                    }
                }
                Token::StartTag {
                    name,
                    attributes,
                    self_closing,
                } => {
                    if self_closing || is_void_element(&name) {
                        nodes.push(Node::elem(name, attributes, vec![]));
                    } else {
                        let children = self.parse_children(&name)?;
                        nodes.push(Node::elem(name, attributes, children));
                    }
                }
                Token::EndTag { .. } => {
                    return Err(ParseError::MalformedTag("Unexpected end tag".to_string()));
                }
                Token::Doctype(_) | Token::Comment(_) => {}
            }
        }

        if nodes.len() == 1 {
            Ok(nodes.into_iter().next().unwrap())
        } else {
            Ok(Node::elem("document".to_string(), HashMap::new(), nodes))
        }
    }

    fn parse_children(&mut self, parent_tag: &str) -> Result<Vec<Node>, ParseError> {
        let mut children = Vec::new();

        while self.tokenizer.has_more_tokens() {
            let token = self.tokenizer.next_token()?;
            match token {
                Token::Eof => {
                    return Err(ParseError::MissingClosingTag(parent_tag.to_string()));
                }
                Token::Text(text) => {
                    if !text.trim().is_empty() {
                        children.push(Node::text(text));
                    }
                }
                Token::StartTag {
                    name,
                    attributes,
                    self_closing,
                } => {
                    if self_closing || is_void_element(&name) {
                        children.push(Node::elem(name, attributes, vec![]));
                    } else {
                        let grandchildren = self.parse_children(&name)?;
                        children.push(Node::elem(name, attributes, grandchildren));
                    }
                }
                Token::EndTag { name } => {
                    if name == parent_tag {
                        return Ok(children);
                    } else {
                        return Err(ParseError::MalformedTag(format!(
                            "Expected closing tag for '{}', found '{}'",
                            parent_tag, name
                        )));
                    }
                }
                Token::Doctype(_) | Token::Comment(_) => {}
            }
        }

        Err(ParseError::MissingClosingTag(parent_tag.to_string()))
    }
}

fn is_void_element(tag_name: &str) -> bool {
    matches!(
        tag_name.to_ascii_lowercase().as_str(),
        "area"
            | "base"
            | "br"
            | "col"
            | "embed"
            | "hr"
            | "img"
            | "input"
            | "link"
            | "meta"
            | "param"
            | "source"
            | "track"
            | "wbr"
    )
}
