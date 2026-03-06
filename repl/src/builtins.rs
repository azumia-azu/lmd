use std::collections::HashMap;
use std::fmt::Display;

use eyre::{Result, bail, eyre};
use lmd_core::ast::{Literal, Number};
use crate::eval::show_value;
use crate::eval::Value;

pub fn builtin_functions() -> HashMap<String, Value> {
    let mut map = HashMap::new();

    // Add built-in functions here, e.g.:
    // map.insert("+".to_string(), Value::BuiltinFunction(...));
    map.insert("+".to_string(), Value::BuiltinFunction(BuiltinFunction {
        op: BuiltinOperation::Add,
        arg1: Box::new(None),
    }));
    map.insert("-".to_string(), Value::BuiltinFunction(BuiltinFunction {
        op: BuiltinOperation::Sub,
        arg1: Box::new(None),
    }));
    map.insert("*".to_string(), Value::BuiltinFunction(BuiltinFunction {
        op: BuiltinOperation::Mul,
        arg1: Box::new(None),
    }));
    map.insert("/".to_string(), Value::BuiltinFunction(BuiltinFunction {
        op: BuiltinOperation::Div,
        arg1: Box::new(None),
    }));

    map
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum BuiltinOperation {
    Add,
    Sub,
    Mul,
    Div,
}

impl std::fmt::Display for BuiltinOperation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op_str = match self {
            BuiltinOperation::Add => "+",
            BuiltinOperation::Sub => "-",
            BuiltinOperation::Mul => "*",
            BuiltinOperation::Div => "/",
        };
        write!(f, "{}", op_str)
    }
}

impl BuiltinOperation {
    fn apply(&self , lhs: &Value, rhs: &Value) -> Result<Value> {
        // upcasting int -> float -> str
        // if either operand is a string, convert both to string and concatenate, only permit + operator
        // if either operand is a float, convert both to float and perform the operation
        // otherwise, both operands are int, perform the operation as int
        match (lhs, rhs) {
             (Value::Literal(Literal::Str(_)), _) | (_, Value::Literal(Literal::Str(_))) => {
                if self != &BuiltinOperation::Add {
                    bail!("type error: only + operator is supported for string operands")
                }
                let s1 = match lhs {
                    Value::Literal(Literal::Str(s)) => s.clone(),
                    Value::Literal(Literal::Number(n)) => n.to_string(),
                    _ => bail!("type error: incompatible types: {}", show_value(lhs)),
                };

                let s2 = match rhs {
                    Value::Literal(Literal::Str(s)) => s.clone(),
                    Value::Literal(Literal::Number(n)) => n.to_string(),
                    _ => bail!("type error: incompatible types: {}", show_value(rhs)),
                };
                Ok(Value::Literal(Literal::Str(format!("{}{}", s1, s2))))
            }
            (Value::Literal(Literal::Number(Number::Float(_))), _) | (_, Value::Literal(Literal::Number(Number::Float(_)))) => {
                let f1 = match lhs {
                    Value::Literal(Literal::Number(Number::Int(i))) => *i as f64,
                    Value::Literal(Literal::Number(Number::Float(fl))) => *fl,
                    _ => bail!("type error: incompatible types: {}", show_value(lhs)),
                };

                let f2 = match rhs {
                    Value::Literal(Literal::Number(Number::Int(i))) => *i as f64,
                    Value::Literal(Literal::Number(Number::Float(fl))) => *fl,
                    _ => bail!("type error: incompatible types: {}", show_value(rhs)),
                };

                let result = match self {
                    BuiltinOperation::Add => f1 + f2,
                    BuiltinOperation::Sub => f1 - f2,
                    BuiltinOperation::Mul => f1 * f2,
                    BuiltinOperation::Div => f1 / f2,
                };
                Ok(Value::Literal(Literal::Number(Number::Float(result))))
            }

            (Value::Literal(Literal::Number(Number::Int(i1))), Value::Literal(Literal::Number(Number::Int(i2)))) => {
                let result = match self {
                    BuiltinOperation::Add => i1 + i2,
                    BuiltinOperation::Sub => i1 - i2,
                    BuiltinOperation::Mul => i1 * i2,
                    BuiltinOperation::Div => i1.checked_div(*i2).ok_or_else(|| eyre!("integer division overflow or division by zero"))?,
                };
                Ok(Value::Literal(Literal::Number(Number::Int(result))))
            }
            _ => bail!("type error: incompatible types for operator {}: {}, {}", self, show_value(lhs), show_value(rhs)),
        }

    }
}

#[derive(Debug, Clone)]
pub struct BuiltinFunction {
    op: BuiltinOperation,
    arg1: Box<Option<Value>>,
}

impl Display for BuiltinFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let op_str = match self.op {
            BuiltinOperation::Add => "+",
            BuiltinOperation::Sub => "-",
            BuiltinOperation::Mul => "*",
            BuiltinOperation::Div => "/",
        };
        write!(f, "<builtin function {}>", op_str)
    }
}

pub fn apply_builtin_function(mut builtin: BuiltinFunction, arg: Value) -> Result<Value> {
    if builtin.arg1.is_none() {
        // first argument
        Ok(Value::BuiltinFunction(BuiltinFunction {
            op: builtin.op.clone(),
            arg1: Box::new(Some(arg)),
        }))
    } else {
        // second argument, apply the operation
        let arg1 = builtin.arg1.take().unwrap();
        builtin.op.apply(&arg1, &arg)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn int(v: isize) -> Value {
        Value::Literal(Literal::Number(Number::Int(v)))
    }

    fn float(v: f64) -> Value {
        Value::Literal(Literal::Number(Number::Float(v)))
    }

    fn apply_curried(op: BuiltinOperation, lhs: Value, rhs: Value) -> Result<Value> {
        let first = apply_builtin_function(
            BuiltinFunction {
                op,
                arg1: Box::new(None),
            },
            lhs,
        )?;

        match first {
            Value::BuiltinFunction(partial) => apply_builtin_function(partial, rhs),
            other => bail!("expected builtin function after first apply, got {other:?}"),
        }
    }

    #[test]
    fn div_int_by_zero_returns_error() {
        let err = apply_curried(BuiltinOperation::Div, int(1), int(0)).unwrap_err();
        assert!(err.to_string().contains("division by zero"));
    }

    #[test]
    fn div_float_by_zero_returns_infinity() {
        let result = apply_curried(BuiltinOperation::Div, float(1.0), float(0.0)).unwrap();
        match result {
            Value::Literal(Literal::Number(Number::Float(v))) => assert!(v.is_infinite() && v.is_sign_positive()),
            other => panic!("expected float literal, got {other:?}"),
        }
    }

    #[test]
    fn div_negative_float_by_zero_returns_negative_infinity() {
        let result = apply_curried(BuiltinOperation::Div, float(-1.0), float(0.0)).unwrap();
        match result {
            Value::Literal(Literal::Number(Number::Float(v))) => assert!(v.is_infinite() && v.is_sign_negative()),
            other => panic!("expected float literal, got {other:?}"),
        }
    }

    #[test]
    fn div_zero_by_zero_returns_nan() {
        let result = apply_curried(BuiltinOperation::Div, float(0.0), float(0.0)).unwrap();
        match result {
            Value::Literal(Literal::Number(Number::Float(v))) => assert!(v.is_nan()),
            other => panic!("expected float literal, got {other:?}"),
        }
    }

    #[test]
    fn div_int_overflow_returns_error() {
        let err = apply_curried(BuiltinOperation::Div, int(isize::MIN), int(-1)).unwrap_err();
        assert!(err.to_string().contains("overflow") || err.to_string().contains("division by zero"));
    }

    #[test]
    fn add_ints_returns_int_literal() {
        let result = apply_curried(BuiltinOperation::Add, int(2), int(3)).unwrap();
        assert!(matches!(result, Value::Literal(Literal::Number(Number::Int(5)))));
    }
}
