use crate::ast::Expr;
use crate::grammar;
use lalrpop_util::{ParseError as LrParseError, lexer};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ParseErrorKind {
    InvalidToken,
    UnexpectedEof,
    UnexpectedToken,
    ExtraToken,
    User,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ParseError {
    pub kind: ParseErrorKind,
    pub location: usize,
    pub found: Option<String>,
    pub expected: Vec<String>,
    pub message: Option<String>,
}

impl ParseError {
    pub fn is_unexpected_eof(&self) -> bool {
        matches!(self.kind, ParseErrorKind::UnexpectedEof)
    }
}

impl std::fmt::Display for ParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(message) = &self.message {
            return write!(f, "{message}");
        }

        let kind = match self.kind {
            ParseErrorKind::InvalidToken => "invalid token",
            ParseErrorKind::UnexpectedEof => "unexpected end of input",
            ParseErrorKind::UnexpectedToken => "unexpected token",
            ParseErrorKind::ExtraToken => "extra token",
            ParseErrorKind::User => "parser user error",
        };

        if self.expected.is_empty() {
            write!(f, "{kind} at {location}", location = self.location)
        } else {
            write!(
                f,
                "{kind} at {location}, expected one of: {}",
                self.expected.join(", "),
                location = self.location
            )
        }
    }
}

impl std::error::Error for ParseError {}

type InternalParseError<'a> = LrParseError<usize, lexer::Token<'a>, &'static str>;

pub fn parse(input: &str) -> Result<Expr, ParseError> {
    grammar::ExprParser::new()
        .parse(input)
        .map_err(ParseError::from_internal)
}

pub fn try_parse(input: &str) -> Result<(), ParseError> {
    grammar::ExprParser::new()
        .parse(input)
        .map(|_| ())
        .map_err(ParseError::from_internal)
}

impl ParseError {
    fn from_internal(err: InternalParseError<'_>) -> Self {
        match err {
            InternalParseError::InvalidToken { location } => Self {
                kind: ParseErrorKind::InvalidToken,
                location,
                found: None,
                expected: vec![],
                message: None,
            },
            InternalParseError::UnrecognizedEof { location, expected } => Self {
                kind: ParseErrorKind::UnexpectedEof,
                location,
                found: None,
                expected,
                message: None,
            },
            InternalParseError::UnrecognizedToken { token, expected } => {
                let (location, found, _) = token;
                Self {
                    kind: ParseErrorKind::UnexpectedToken,
                    location,
                    found: Some(format!("{found:?}")),
                    expected,
                    message: None,
                }
            }
            InternalParseError::ExtraToken { token } => {
                let (location, found, _) = token;
                Self {
                    kind: ParseErrorKind::ExtraToken,
                    location,
                    found: Some(format!("{found:?}")),
                    expected: vec![],
                    message: None,
                }
            }
            InternalParseError::User { error } => Self {
                kind: ParseErrorKind::User,
                location: 0,
                found: None,
                expected: vec![],
                message: Some(error.to_string()),
            },
        }
    }
}
