use std::collections::HashMap;
use std::fmt::Display;

use eyre::{Result, bail, eyre};
use lmd_core::ast::{Literal, Number};

use crate::eval::Value;
use crate::eval::force_whnf;
use crate::eval::show_value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum BuiltinKind {
    Add,
    Sub,
    Mul,
    Div,
    Ge,
    Gt,
    Le,
    Lt,
    Eq,
    Ne,
    Not,
    And,
    Or,
}

#[derive(Debug, Clone, Copy)]
struct BuiltinSpec {
    name: &'static str,
    arity: usize,
    kind: BuiltinKind,
}

const BUILTIN_SPECS: &[BuiltinSpec] = &[
    BuiltinSpec {
        name: "+",
        arity: 2,
        kind: BuiltinKind::Add,
    },
    BuiltinSpec {
        name: "-",
        arity: 2,
        kind: BuiltinKind::Sub,
    },
    BuiltinSpec {
        name: "*",
        arity: 2,
        kind: BuiltinKind::Mul,
    },
    BuiltinSpec {
        name: "/",
        arity: 2,
        kind: BuiltinKind::Div,
    },
    BuiltinSpec {
        name: ">=",
        arity: 2,
        kind: BuiltinKind::Ge,
    },
    BuiltinSpec {
        name: ">",
        arity: 2,
        kind: BuiltinKind::Gt,
    },
    BuiltinSpec {
        name: "<=",
        arity: 2,
        kind: BuiltinKind::Le,
    },
    BuiltinSpec {
        name: "<",
        arity: 2,
        kind: BuiltinKind::Lt,
    },
    BuiltinSpec {
        name: "==",
        arity: 2,
        kind: BuiltinKind::Eq,
    },
    BuiltinSpec {
        name: "!=",
        arity: 2,
        kind: BuiltinKind::Ne,
    },
    BuiltinSpec {
        name: "!",
        arity: 1,
        kind: BuiltinKind::Not,
    },
    BuiltinSpec {
        name: "&&",
        arity: 2,
        kind: BuiltinKind::And,
    },
    BuiltinSpec {
        name: "||",
        arity: 2,
        kind: BuiltinKind::Or,
    },
];

pub fn builtin_functions() -> HashMap<String, Value> {
    let mut map = HashMap::new();

    for spec in BUILTIN_SPECS {
        map.insert(
            spec.name.to_string(),
            Value::BuiltinFunction(BuiltinFunction::from_spec(*spec)),
        );
    }

    map
}

#[derive(Debug, Clone)]
pub struct BuiltinFunction {
    name: &'static str,
    kind: BuiltinKind,
    arity: usize,
    args: Vec<Value>,
}

impl BuiltinFunction {
    fn from_spec(spec: BuiltinSpec) -> Self {
        Self {
            name: spec.name,
            kind: spec.kind,
            arity: spec.arity,
            args: Vec::with_capacity(spec.arity),
        }
    }
}

impl Display for BuiltinFunction {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<builtin function {}>", self.name)
    }
}

pub fn apply_builtin_function(mut builtin: BuiltinFunction, arg: Value) -> Result<Value> {
    builtin.args.push(arg);

    if builtin.args.len() < builtin.arity {
        return Ok(Value::BuiltinFunction(builtin));
    }

    if builtin.args.len() > builtin.arity {
        bail!(
            "builtin {} expected {} args, got {}",
            builtin.name,
            builtin.arity,
            builtin.args.len()
        );
    }

    execute_builtin(builtin.kind, &builtin.args)
}

fn execute_builtin(kind: BuiltinKind, args: &[Value]) -> Result<Value> {
    match (kind, args) {
        (BuiltinKind::Add, [lhs, rhs]) => {
            eval_add(&force_whnf(lhs.clone())?, &force_whnf(rhs.clone())?)
        }
        (BuiltinKind::Sub, [lhs, rhs]) => {
            eval_sub(&force_whnf(lhs.clone())?, &force_whnf(rhs.clone())?)
        }
        (BuiltinKind::Mul, [lhs, rhs]) => {
            eval_mul(&force_whnf(lhs.clone())?, &force_whnf(rhs.clone())?)
        }
        (BuiltinKind::Div, [lhs, rhs]) => {
            eval_div(&force_whnf(lhs.clone())?, &force_whnf(rhs.clone())?)
        }
        (BuiltinKind::Ge, [lhs, rhs]) => {
            eval_num_cmp(&force_whnf(lhs.clone())?, &force_whnf(rhs.clone())?, ">=")
        }
        (BuiltinKind::Gt, [lhs, rhs]) => {
            eval_num_cmp(&force_whnf(lhs.clone())?, &force_whnf(rhs.clone())?, ">")
        }
        (BuiltinKind::Le, [lhs, rhs]) => {
            eval_num_cmp(&force_whnf(lhs.clone())?, &force_whnf(rhs.clone())?, "<=")
        }
        (BuiltinKind::Lt, [lhs, rhs]) => {
            eval_num_cmp(&force_whnf(lhs.clone())?, &force_whnf(rhs.clone())?, "<")
        }
        (BuiltinKind::Eq, [lhs, rhs]) => Ok(bool_lit(eval_eq(
            &force_whnf(lhs.clone())?,
            &force_whnf(rhs.clone())?,
        )?)),
        (BuiltinKind::Ne, [lhs, rhs]) => Ok(bool_lit(!eval_eq(
            &force_whnf(lhs.clone())?,
            &force_whnf(rhs.clone())?,
        )?)),
        (BuiltinKind::Not, [v]) => Ok(bool_lit(!expect_bool(&force_whnf(v.clone())?, "!")?)),
        (BuiltinKind::And, [lhs, rhs]) => eval_and(lhs, rhs),
        (BuiltinKind::Or, [lhs, rhs]) => eval_or(lhs, rhs),
        _ => bail!("builtin arity/type mismatch"),
    }
}

fn bool_lit(v: bool) -> Value {
    Value::Literal(Literal::Bool(v))
}

fn expect_bool(v: &Value, op: &str) -> Result<bool> {
    match v {
        Value::Literal(Literal::Bool(b)) => Ok(*b),
        _ => bail!(
            "type error: operator {} expects boolean literal, got {}",
            op,
            show_value(v)
        ),
    }
}

fn as_f64_number(v: &Value, op: &str) -> Result<f64> {
    match v {
        Value::Literal(Literal::Number(Number::Int(i))) => Ok(*i as f64),
        Value::Literal(Literal::Number(Number::Float(f))) => Ok(*f),
        _ => bail!(
            "type error: operator {} expects numeric operands, got {}",
            op,
            show_value(v)
        ),
    }
}

fn eval_num_cmp(lhs: &Value, rhs: &Value, op: &str) -> Result<Value> {
    let l = as_f64_number(lhs, op)?;
    let r = as_f64_number(rhs, op)?;

    let out = match op {
        ">" => l > r,
        ">=" => l >= r,
        "<" => l < r,
        "<=" => l <= r,
        _ => unreachable!("unsupported compare op"),
    };

    Ok(bool_lit(out))
}

fn eval_eq(lhs: &Value, rhs: &Value) -> Result<bool> {
    match (lhs, rhs) {
        (Value::Literal(Literal::Bool(a)), Value::Literal(Literal::Bool(b))) => Ok(a == b),
        (Value::Literal(Literal::Str(a)), Value::Literal(Literal::Str(b))) => Ok(a == b),
        (Value::Literal(Literal::Number(_)), Value::Literal(Literal::Number(_))) => {
            Ok(as_f64_number(lhs, "==")? == as_f64_number(rhs, "==")?)
        }
        _ => bail!(
            "type error: incompatible types for operator ==: {}, {}",
            show_value(lhs),
            show_value(rhs)
        ),
    }
}

fn eval_add(lhs: &Value, rhs: &Value) -> Result<Value> {
    match (lhs, rhs) {
        (Value::Literal(Literal::Str(_)), _) | (_, Value::Literal(Literal::Str(_))) => {
            let s1 = match lhs {
                Value::Literal(Literal::Str(s)) => s.clone(),
                Value::Literal(Literal::Number(n)) => n.to_string(),
                Value::Literal(Literal::Bool(b)) => b.to_string(),
                _ => bail!("type error: incompatible types: {}", show_value(lhs)),
            };

            let s2 = match rhs {
                Value::Literal(Literal::Str(s)) => s.clone(),
                Value::Literal(Literal::Number(n)) => n.to_string(),
                Value::Literal(Literal::Bool(b)) => b.to_string(),
                _ => bail!("type error: incompatible types: {}", show_value(rhs)),
            };
            Ok(Value::Literal(Literal::Str(format!("{}{}", s1, s2))))
        }
        (Value::Literal(Literal::Number(Number::Float(_))), _)
        | (_, Value::Literal(Literal::Number(Number::Float(_)))) => {
            Ok(Value::Literal(Literal::Number(Number::Float(
                as_f64_number(lhs, "+")? + as_f64_number(rhs, "+")?,
            ))))
        }
        (
            Value::Literal(Literal::Number(Number::Int(i1))),
            Value::Literal(Literal::Number(Number::Int(i2))),
        ) => Ok(Value::Literal(Literal::Number(Number::Int(i1 + i2)))),
        _ => bail!(
            "type error: incompatible types for operator +: {}, {}",
            show_value(lhs),
            show_value(rhs)
        ),
    }
}

fn eval_sub(lhs: &Value, rhs: &Value) -> Result<Value> {
    match (lhs, rhs) {
        (Value::Literal(Literal::Number(Number::Float(_))), _)
        | (_, Value::Literal(Literal::Number(Number::Float(_)))) => {
            Ok(Value::Literal(Literal::Number(Number::Float(
                as_f64_number(lhs, "-")? - as_f64_number(rhs, "-")?,
            ))))
        }
        (
            Value::Literal(Literal::Number(Number::Int(i1))),
            Value::Literal(Literal::Number(Number::Int(i2))),
        ) => Ok(Value::Literal(Literal::Number(Number::Int(i1 - i2)))),
        _ => bail!(
            "type error: incompatible types for operator -: {}, {}",
            show_value(lhs),
            show_value(rhs)
        ),
    }
}

fn eval_mul(lhs: &Value, rhs: &Value) -> Result<Value> {
    match (lhs, rhs) {
        (Value::Literal(Literal::Number(Number::Float(_))), _)
        | (_, Value::Literal(Literal::Number(Number::Float(_)))) => {
            Ok(Value::Literal(Literal::Number(Number::Float(
                as_f64_number(lhs, "*")? * as_f64_number(rhs, "*")?,
            ))))
        }
        (
            Value::Literal(Literal::Number(Number::Int(i1))),
            Value::Literal(Literal::Number(Number::Int(i2))),
        ) => Ok(Value::Literal(Literal::Number(Number::Int(i1 * i2)))),
        _ => bail!(
            "type error: incompatible types for operator *: {}, {}",
            show_value(lhs),
            show_value(rhs)
        ),
    }
}

fn eval_div(lhs: &Value, rhs: &Value) -> Result<Value> {
    match (lhs, rhs) {
        (Value::Literal(Literal::Number(Number::Float(_))), _)
        | (_, Value::Literal(Literal::Number(Number::Float(_)))) => {
            Ok(Value::Literal(Literal::Number(Number::Float(
                as_f64_number(lhs, "/")? / as_f64_number(rhs, "/")?,
            ))))
        }
        (
            Value::Literal(Literal::Number(Number::Int(i1))),
            Value::Literal(Literal::Number(Number::Int(i2))),
        ) => Ok(Value::Literal(Literal::Number(Number::Int(
            i1.checked_div(*i2)
                .ok_or_else(|| eyre!("integer division overflow or division by zero"))?,
        )))),
        _ => bail!(
            "type error: incompatible types for operator /: {}, {}",
            show_value(lhs),
            show_value(rhs)
        ),
    }
}

fn eval_and(lhs: &Value, rhs: &Value) -> Result<Value> {
    if !expect_bool(&force_whnf(lhs.clone())?, "&&")? {
        return Ok(bool_lit(false));
    }

    Ok(bool_lit(expect_bool(&force_whnf(rhs.clone())?, "&&")?))
}

fn eval_or(lhs: &Value, rhs: &Value) -> Result<Value> {
    if expect_bool(&force_whnf(lhs.clone())?, "||")? {
        return Ok(bool_lit(true));
    }

    Ok(bool_lit(expect_bool(&force_whnf(rhs.clone())?, "||")?))
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

    fn bool_v(v: bool) -> Value {
        Value::Literal(Literal::Bool(v))
    }

    fn apply_symbol(op: &str, args: Vec<Value>) -> Result<Value> {
        let mut env = builtin_functions();
        let mut cur = env
            .remove(op)
            .ok_or_else(|| eyre!("missing builtin op: {op}"))?;

        for arg in args {
            match cur {
                Value::BuiltinFunction(partial) => {
                    cur = apply_builtin_function(partial, arg)?;
                }
                other => bail!("expected builtin function before apply, got {other:?}"),
            }
        }

        Ok(cur)
    }

    #[test]
    fn div_int_by_zero_returns_error() {
        let err = apply_symbol("/", vec![int(1), int(0)]).unwrap_err();
        assert!(err.to_string().contains("division by zero"));
    }

    #[test]
    fn div_float_by_zero_returns_infinity() {
        let result = apply_symbol("/", vec![float(1.0), float(0.0)]).unwrap();
        match result {
            Value::Literal(Literal::Number(Number::Float(v))) => {
                assert!(v.is_infinite() && v.is_sign_positive())
            }
            other => panic!("expected float literal, got {other:?}"),
        }
    }

    #[test]
    fn div_negative_float_by_zero_returns_negative_infinity() {
        let result = apply_symbol("/", vec![float(-1.0), float(0.0)]).unwrap();
        match result {
            Value::Literal(Literal::Number(Number::Float(v))) => {
                assert!(v.is_infinite() && v.is_sign_negative())
            }
            other => panic!("expected float literal, got {other:?}"),
        }
    }

    #[test]
    fn div_zero_by_zero_returns_nan() {
        let result = apply_symbol("/", vec![float(0.0), float(0.0)]).unwrap();
        match result {
            Value::Literal(Literal::Number(Number::Float(v))) => assert!(v.is_nan()),
            other => panic!("expected float literal, got {other:?}"),
        }
    }

    #[test]
    fn div_int_overflow_returns_error() {
        let err = apply_symbol("/", vec![int(isize::MIN), int(-1)]).unwrap_err();
        assert!(
            err.to_string().contains("overflow") || err.to_string().contains("division by zero")
        );
    }

    #[test]
    fn add_ints_returns_int_literal() {
        let result = apply_symbol("+", vec![int(2), int(3)]).unwrap();
        assert!(matches!(
            result,
            Value::Literal(Literal::Number(Number::Int(5)))
        ));
    }

    #[test]
    fn not_bool_returns_bool_literal() {
        let result = apply_symbol("!", vec![bool_v(true)]).unwrap();
        assert!(matches!(result, Value::Literal(Literal::Bool(false))));
    }

    #[test]
    fn and_bools_returns_bool_literal() {
        let result = apply_symbol("&&", vec![bool_v(true), bool_v(false)]).unwrap();
        assert!(matches!(result, Value::Literal(Literal::Bool(false))));
    }

    #[test]
    fn compare_numbers_returns_bool_literal() {
        let result = apply_symbol(">=", vec![int(2), int(2)]).unwrap();
        assert!(matches!(result, Value::Literal(Literal::Bool(true))));
    }

    #[test]
    fn builtin_registry_contains_logic_and_compare_ops() {
        let env = builtin_functions();
        for op in ["==", "!=", "<", "<=", ">", ">=", "!", "&&", "||"] {
            assert!(env.contains_key(op), "missing builtin op: {op}");
        }
    }
}
