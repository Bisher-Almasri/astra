#[derive(Debug, Clone, PartialEq)]
pub enum ParseError {
    UnexpectedEof,
    InvalidCharacter(char),
    MissingClosingBracket,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Selector {
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Declaration {
    pub property: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Rule {
    pub selectors: Vec<Selector>,
    pub declarations: Vec<Declaration>,
}

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

#[derive(Debug)]
pub struct CssTokenizer {
    input: String,
    position: usize,
}

impl CssTokenizer {
    pub fn new(input: String) -> Self {
        Self { input, position: 0 }
    }
}

#[derive(Debug)]
struct CssParser {
    tokenizer: CssTokenizer,
}

impl CssParser {
    pub fn new(html: String) -> Self {
        Self {
            tokenizer: CssTokenizer::new(html),
        }
    }

    pub fn parse(&mut self) -> Result<Stylesheet, ParseError> {
        Err(ParseError::UnexpectedEof) // place holder
    }
}
