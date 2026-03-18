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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::{Expr, Literal, Number};

    fn assert_var(expr: &Expr, expected: &str) {
        match expr {
            Expr::Var(name) => assert_eq!(name, expected),
            other => panic!("expected variable '{expected}', got {other:?}"),
        }
    }

    fn assert_int(expr: &Expr, expected: isize) {
        match expr {
            Expr::Literal(Literal::Number(Number::Int(v))) => assert_eq!(*v, expected),
            other => panic!("expected int literal {expected}, got {other:?}"),
        }
    }

    fn assert_left_assoc_var_app(expr: &Expr, names: &[&str]) {
        assert!(!names.is_empty(), "names must not be empty");
        if names.len() == 1 {
            assert_var(expr, names[0]);
            return;
        }

        match expr {
            Expr::App(lhs, rhs) => {
                assert_var(rhs, names[names.len() - 1]);
                assert_left_assoc_var_app(lhs, &names[..names.len() - 1]);
            }
            other => panic!("expected left-associated app chain, got {other:?}"),
        }
    }

    #[test]
    fn parse_variable_identifier_success() {
        let expr = parse("abc_1").unwrap();
        assert_var(&expr, "abc_1");
    }

    #[test]
    fn parse_application_left_associative() {
        let expr = parse("f g x").unwrap();

        match &expr {
            Expr::App(fg, x) => {
                assert_var(x, "x");
                match fg.as_ref() {
                    Expr::App(f, g) => {
                        assert_var(f, "f");
                        assert_var(g, "g");
                    }
                    other => panic!("expected nested application, got {other:?}"),
                }
            }
            other => panic!("expected application expression, got {other:?}"),
        }
    }

    #[test]
    fn parse_application_left_associative_table_driven() {
        let cases = vec![
            ("f g", vec!["f", "g"]),
            ("f g x", vec!["f", "g", "x"]),
            ("f g x y", vec!["f", "g", "x", "y"]),
            ("a b c d e", vec!["a", "b", "c", "d", "e"]),
        ];

        for (input, chain) in cases {
            let expr = parse(input).unwrap();
            assert_left_assoc_var_app(&expr, &chain);
        }
    }

    #[test]
    fn parse_application_parentheses_override_left_associativity() {
        let expr = parse("f (g x)").unwrap();

        match &expr {
            Expr::App(f, gx) => {
                assert_var(f, "f");
                match gx.as_ref() {
                    Expr::App(g, x) => {
                        assert_var(g, "g");
                        assert_var(x, "x");
                    }
                    other => panic!("expected grouped right argument, got {other:?}"),
                }
            }
            other => panic!("expected application expression, got {other:?}"),
        }
    }

    #[test]
    fn parse_lambda_body_as_application() {
        let expr = parse("\\x -> f x").unwrap();

        match &expr {
            Expr::Func(param, body) => {
                assert_eq!(param, "x");
                match body.as_ref() {
                    Expr::App(f, x) => {
                        assert_var(f, "f");
                        assert_var(x, "x");
                    }
                    other => panic!("expected application in lambda body, got {other:?}"),
                }
            }
            other => panic!("expected lambda expression, got {other:?}"),
        }
    }

    #[test]
    fn parse_let_with_trailing_semicolon() {
        let expr = parse("let x = 1; y = x; in y").unwrap();

        match &expr {
            Expr::Let(bindings, body) => {
                assert_eq!(bindings.len(), 2);
                assert_eq!(bindings[0].0, "x");
                assert_int(&bindings[0].1, 1);
                assert_eq!(bindings[1].0, "y");
                assert_var(&bindings[1].1, "x");
                assert_var(body, "y");
            }
            other => panic!("expected let expression, got {other:?}"),
        }
    }

    #[test]
    fn try_parse_reports_unexpected_eof_for_incomplete_lambda() {
        let err = try_parse("\\x ->").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::UnexpectedEof);
        assert!(err.is_unexpected_eof());
    }

    #[test]
    fn parse_reports_unexpected_token_for_invalid_start_token() {
        let err = parse(")").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::UnexpectedToken);
        assert_eq!(err.location, 0);
        assert!(err.found.is_some());
    }

    #[test]
    fn try_parse_reports_unexpected_eof_for_empty_input() {
        let err = try_parse("").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::UnexpectedEof);
        assert!(!err.expected.is_empty());
    }

    #[test]
    fn parse_negative_integer_literal() {
        let expr = parse("-42").unwrap();
        assert_int(&expr, -42);
    }

    #[test]
    fn parse_infix_addition_desugars_to_operator_application() {
        let expr = parse("1+2").unwrap();
        match &expr {
            Expr::App(f1, rhs) => {
                assert_int(rhs, 2);
                match f1.as_ref() {
                    Expr::App(op, lhs) => {
                        assert_var(op, "+");
                        assert_int(lhs, 1);
                    }
                    other => panic!("expected operator application head, got {other:?}"),
                }
            }
            other => panic!("expected application for infix addition, got {other:?}"),
        }
    }

    #[test]
    fn parse_infix_precedence_mul_over_add() {
        let expr = parse("1+2*3").unwrap();
        match &expr {
            Expr::App(f1, rhs_add) => {
                match f1.as_ref() {
                    Expr::App(op_add, lhs_add) => {
                        assert_var(op_add, "+");
                        assert_int(lhs_add, 1);
                    }
                    other => panic!("expected addition head, got {other:?}"),
                }

                match rhs_add.as_ref() {
                    Expr::App(f2, rhs_mul) => {
                        assert_int(rhs_mul, 3);
                        match f2.as_ref() {
                            Expr::App(op_mul, lhs_mul) => {
                                assert_var(op_mul, "*");
                                assert_int(lhs_mul, 2);
                            }
                            other => panic!("expected multiplication head, got {other:?}"),
                        }
                    }
                    other => panic!("expected multiplication expression on add rhs, got {other:?}"),
                }
            }
            other => panic!("expected addition expression, got {other:?}"),
        }
    }

    #[test]
    fn parse_prefix_operator_partial_then_apply() {
        let expr = parse("(+ 1) 2").unwrap();
        match &expr {
            Expr::App(f, rhs) => {
                assert_int(rhs, 2);
                match f.as_ref() {
                    Expr::App(op, lhs) => {
                        assert_var(op, "+");
                        assert_int(lhs, 1);
                    }
                    other => panic!("expected partial prefix op as function, got {other:?}"),
                }
            }
            other => panic!("expected final application, got {other:?}"),
        }
    }

    #[test]
    fn parse_parenthesized_operator_full_application() {
        let expr = parse("(+) 1 2").unwrap();
        match &expr {
            Expr::App(f, rhs) => {
                assert_int(rhs, 2);
                match f.as_ref() {
                    Expr::App(op, lhs) => {
                        assert_var(op, "+");
                        assert_int(lhs, 1);
                    }
                    other => panic!("expected left-associated operator application, got {other:?}"),
                }
            }
            other => panic!("expected application chain, got {other:?}"),
        }
    }

    #[test]
    fn parse_boolean_literals() {
        let expr_true = parse("true").unwrap();
        assert!(matches!(expr_true, Expr::Literal(Literal::Bool(true))));

        let expr_false = parse("false").unwrap();
        assert!(matches!(expr_false, Expr::Literal(Literal::Bool(false))));
    }

    #[test]
    fn parse_if_expression_with_boolean_condition() {
        let expr = parse("if true then 1 else 2").unwrap();
        match expr {
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                assert!(matches!(*cond, Expr::Literal(Literal::Bool(true))));
                assert_int(&then_branch, 1);
                assert_int(&else_branch, 2);
            }
            other => panic!("expected if expression, got {other:?}"),
        }
    }

    #[test]
    fn parse_if_expression_with_grouped_branch_expression() {
        let expr = parse("if true then {1+2} else {3+4}").unwrap();
        match expr {
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                assert!(matches!(*cond, Expr::Literal(Literal::Bool(true))));
                assert!(matches!(*then_branch, Expr::App(_, _)));
                assert!(matches!(*else_branch, Expr::App(_, _)));
            }
            other => panic!("expected if expression, got {other:?}"),
        }
    }

    #[test]
    fn try_parse_reports_unexpected_eof_for_incomplete_if_expression() {
        let err = try_parse("if true then 1 else").unwrap_err();
        assert_eq!(err.kind, ParseErrorKind::UnexpectedEof);
        assert!(err.is_unexpected_eof());
    }

    #[test]
    fn parse_if_expression_with_non_grouped_expression_branches() {
        let expr = parse("if true then 1+2 else 3+4").unwrap();
        match expr {
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                assert!(matches!(*cond, Expr::Literal(Literal::Bool(true))));
                assert!(matches!(*then_branch, Expr::App(_, _)));
                assert!(matches!(*else_branch, Expr::App(_, _)));
            }
            other => panic!("expected if expression, got {other:?}"),
        }
    }

    #[test]
    fn parse_if_expression_with_expression_condition() {
        let expr = parse("if 1+2 then 3 else 4").unwrap();
        match expr {
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                assert!(matches!(*cond, Expr::App(_, _)));
                assert_int(&then_branch, 3);
                assert_int(&else_branch, 4);
            }
            other => panic!("expected if expression, got {other:?}"),
        }
    }

    #[test]
    fn parse_application_with_if_as_argument() {
        let expr = parse("f (if true then 1 else 2)").unwrap();
        match expr {
            Expr::App(f, arg) => {
                assert_var(&f, "f");
                assert!(matches!(*arg, Expr::If { .. }));
            }
            other => panic!("expected application expression, got {other:?}"),
        }
    }

    #[test]
    fn parse_nested_if_expression_in_then_branch() {
        let expr = parse("if true then if false then 1 else 2 else 3").unwrap();
        match expr {
            Expr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                assert!(matches!(*cond, Expr::Literal(Literal::Bool(true))));
                assert!(matches!(*then_branch, Expr::If { .. }));
                assert_int(&else_branch, 3);
            }
            other => panic!("expected outer if expression, got {other:?}"),
        }
    }

    #[test]
    fn parse_rejects_reserved_keyword_as_let_binding_name() {
        let keywords = ["if", "then", "else", "let", "in", "true", "false"];

        for kw in keywords {
            let src = format!("let {kw} = 1 in 1");
            let err = parse(&src).unwrap_err();
            assert_eq!(err.kind, ParseErrorKind::UnexpectedToken);
        }
    }

    #[test]
    fn parse_rejects_reserved_keyword_as_lambda_parameter() {
        let keywords = ["if", "then", "else", "let", "in", "true", "false"];

        for kw in keywords {
            let src = format!("\\{kw} -> {kw}");
            let err = parse(&src).unwrap_err();
            assert_eq!(err.kind, ParseErrorKind::UnexpectedToken);
        }
    }

    #[test]
    fn parse_rejects_reserved_keyword_as_variable_identifier() {
        let keywords = ["then", "else", "let", "in"];

        for kw in keywords {
            let err = parse(kw).unwrap_err();
            assert!(matches!(
                err.kind,
                ParseErrorKind::UnexpectedToken | ParseErrorKind::UnexpectedEof
            ));
        }
    }

    #[test]
    fn parse_accepts_identifier_that_contains_keyword_prefix() {
        let expr = parse("let ifx = 1; true_value = ifx; in true_value").unwrap();

        match expr {
            Expr::Let(bindings, body) => {
                assert_eq!(bindings.len(), 2);
                assert_eq!(bindings[0].0, "ifx");
                assert_eq!(bindings[1].0, "true_value");
                assert_var(&bindings[1].1, "ifx");
                assert_var(&body, "true_value");
            }
            other => panic!("expected let expression, got {other:?}"),
        }
    }
}
