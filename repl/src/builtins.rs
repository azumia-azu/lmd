use std::collections::HashMap;
use std::fmt::Display;

use eyre::{Result, bail, eyre};
use lmd_core::ast::{Literal, Number, Op};

use crate::eval::Value;
use crate::eval::force_whnf;
use crate::eval::show_value;

#[derive(Debug, Clone, Copy)]
struct BuiltinSpec {
    name: &'static str,
    arity: usize,
    kind: Op,
}

const BUILTIN_SPECS: &[BuiltinSpec] = &[
    BuiltinSpec {
        name: "+",
        arity: 2,
        kind: Op::Add,
    },
    BuiltinSpec {
        name: "-",
        arity: 2,
        kind: Op::Sub,
    },
    BuiltinSpec {
        name: "*",
        arity: 2,
        kind: Op::Mul,
    },
    BuiltinSpec {
        name: "/",
        arity: 2,
        kind: Op::Div,
    },
    BuiltinSpec {
        name: ">=",
        arity: 2,
        kind: Op::Ge,
    },
    BuiltinSpec {
        name: ">",
        arity: 2,
        kind: Op::Gt,
    },
    BuiltinSpec {
        name: "<=",
        arity: 2,
        kind: Op::Le,
    },
    BuiltinSpec {
        name: "<",
        arity: 2,
        kind: Op::Lt,
    },
    BuiltinSpec {
        name: "==",
        arity: 2,
        kind: Op::Eq,
    },
    BuiltinSpec {
        name: "!=",
        arity: 2,
        kind: Op::Ne,
    },
    BuiltinSpec {
        name: "!",
        arity: 1,
        kind: Op::Not,
    },
    BuiltinSpec {
        name: "neg",
        arity: 1,
        kind: Op::Neg,
    },
    BuiltinSpec {
        name: "&&",
        arity: 2,
        kind: Op::And,
    },
    BuiltinSpec {
        name: "||",
        arity: 2,
        kind: Op::Or,
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
    kind: Op,
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

pub fn builtin_value_from_op(op: Op) -> Value {
    let spec = BUILTIN_SPECS
        .iter()
        .find(|spec| spec.kind == op)
        .copied()
        .expect("missing builtin spec for operator");
    Value::BuiltinFunction(BuiltinFunction::from_spec(spec))
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

fn execute_builtin(kind: Op, args: &[Value]) -> Result<Value> {
    match (kind, args) {
        (Op::Add, [lhs, rhs]) => eval_add2(args2_whnf(lhs, rhs)?),
        (Op::Sub, [lhs, rhs]) => eval_sub2(args2_whnf(lhs, rhs)?),
        (Op::Mul, [lhs, rhs]) => eval_mul2(args2_whnf(lhs, rhs)?),
        (Op::Div, [lhs, rhs]) => eval_div2(args2_whnf(lhs, rhs)?),
        (Op::Ge, [lhs, rhs]) => eval_num_cmp2(args2_whnf(lhs, rhs)?, ">="),
        (Op::Gt, [lhs, rhs]) => eval_num_cmp2(args2_whnf(lhs, rhs)?, ">"),
        (Op::Le, [lhs, rhs]) => eval_num_cmp2(args2_whnf(lhs, rhs)?, "<="),
        (Op::Lt, [lhs, rhs]) => eval_num_cmp2(args2_whnf(lhs, rhs)?, "<"),
        (Op::Eq, [lhs, rhs]) => Ok(bool_lit(eval_eq2(args2_whnf(lhs, rhs)?)?)),
        (Op::Ne, [lhs, rhs]) => Ok(bool_lit(!eval_eq2(args2_whnf(lhs, rhs)?)?)),
        (Op::Not, [v]) => Ok(bool_lit(!expect_bool(&force_arg(v)?, "!")?)),
        (Op::Neg, [v]) => Ok(eval_neg(&force_arg(v)?)?),
        (Op::And, [lhs, rhs]) => eval_and(lhs, rhs),
        (Op::Or, [lhs, rhs]) => eval_or(lhs, rhs),
        _ => bail!("builtin arity/type mismatch"),
    }
}

fn force_arg(v: &Value) -> Result<Value> {
    force_whnf(v.clone())
}

fn args2_whnf(lhs: &Value, rhs: &Value) -> Result<(Value, Value)> {
    Ok((force_arg(lhs)?, force_arg(rhs)?))
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

fn as_number<'a>(v: &'a Value, op: &str) -> Result<&'a Number> {
    match v {
        Value::Literal(Literal::Number(n)) => Ok(n),
        _ => bail!(
            "type error: operator {} expects numeric operands, got {}",
            op,
            show_value(v)
        ),
    }
}

fn bounded_int(v: i128) -> Result<i64> {
    i64::try_from(v).map_err(|_| eyre!("integer overflow"))
}

fn int_lit(v: i64) -> Value {
    Value::Literal(Literal::Number(Number::Int(i128::from(v))))
}

fn eval_num_cmp(lhs: &Value, rhs: &Value, op: &str) -> Result<Value> {
    let out = match (as_number(lhs, op)?, as_number(rhs, op)?) {
        (Number::Int(l), Number::Int(r)) => match op {
            ">" => l > r,
            ">=" => l >= r,
            "<" => l < r,
            "<=" => l <= r,
            _ => unreachable!("unsupported compare op"),
        },
        (Number::Int(l), Number::Float(r)) => match op {
            ">" => (*l as f64) > *r,
            ">=" => (*l as f64) >= *r,
            "<" => (*l as f64) < *r,
            "<=" => (*l as f64) <= *r,
            _ => unreachable!("unsupported compare op"),
        },
        (Number::Float(l), Number::Int(r)) => match op {
            ">" => *l > (*r as f64),
            ">=" => *l >= (*r as f64),
            "<" => *l < (*r as f64),
            "<=" => *l <= (*r as f64),
            _ => unreachable!("unsupported compare op"),
        },
        (Number::Float(l), Number::Float(r)) => match op {
            ">" => l > r,
            ">=" => l >= r,
            "<" => l < r,
            "<=" => l <= r,
            _ => unreachable!("unsupported compare op"),
        },
    };

    Ok(bool_lit(out))
}

fn eval_num_cmp2((lhs, rhs): (Value, Value), op: &str) -> Result<Value> {
    eval_num_cmp(&lhs, &rhs, op)
}

fn eval_eq(lhs: &Value, rhs: &Value) -> Result<bool> {
    match (lhs, rhs) {
        (Value::Literal(Literal::Bool(a)), Value::Literal(Literal::Bool(b))) => Ok(a == b),
        (Value::Literal(Literal::Str(a)), Value::Literal(Literal::Str(b))) => Ok(a == b),
        (
            Value::Literal(Literal::Number(Number::Int(a))),
            Value::Literal(Literal::Number(Number::Int(b))),
        ) => Ok(a == b),
        (
            Value::Literal(Literal::Number(Number::Int(a))),
            Value::Literal(Literal::Number(Number::Float(b))),
        ) => Ok((*a as f64) == *b),
        (
            Value::Literal(Literal::Number(Number::Float(a))),
            Value::Literal(Literal::Number(Number::Int(b))),
        ) => Ok(*a == (*b as f64)),
        (
            Value::Literal(Literal::Number(Number::Float(a))),
            Value::Literal(Literal::Number(Number::Float(b))),
        ) => Ok(a == b),
        _ => bail!(
            "type error: incompatible types for operator ==: {}, {}",
            show_value(lhs),
            show_value(rhs)
        ),
    }
}

fn eval_eq2((lhs, rhs): (Value, Value)) -> Result<bool> {
    eval_eq(&lhs, &rhs)
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
        ) => Ok(int_lit(
            bounded_int(*i1)?
                .checked_add(bounded_int(*i2)?)
                .ok_or_else(|| eyre!("integer overflow"))?,
        )),
        _ => bail!(
            "type error: incompatible types for operator +: {}, {}",
            show_value(lhs),
            show_value(rhs)
        ),
    }
}

fn eval_add2((lhs, rhs): (Value, Value)) -> Result<Value> {
    eval_add(&lhs, &rhs)
}

fn eval_neg(hs: &Value) -> Result<Value> {
    match hs {
        Value::Literal(Literal::Number(Number::Float(_))) => Ok(Value::Literal(
            Literal::Number(Number::Float(-as_f64_number(hs, "-")?)),
        )),
        Value::Literal(Literal::Number(Number::Int(i))) => Ok(int_lit(
            bounded_int(*i)?
                .checked_neg()
                .ok_or_else(|| eyre!("integer overflow"))?,
        )),
        _ => bail!("type error: incompatible types for operator -: {}", show_value(hs)),
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
        ) => Ok(int_lit(
            bounded_int(*i1)?
                .checked_sub(bounded_int(*i2)?)
                .ok_or_else(|| eyre!("integer overflow"))?,
        )),
        _ => bail!(
            "type error: incompatible types for operator -: {}, {}",
            show_value(lhs),
            show_value(rhs)
        ),
    }
}

fn eval_sub2((lhs, rhs): (Value, Value)) -> Result<Value> {
    eval_sub(&lhs, &rhs)
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
        ) => Ok(int_lit(
            bounded_int(*i1)?
                .checked_mul(bounded_int(*i2)?)
                .ok_or_else(|| eyre!("integer overflow"))?,
        )),
        _ => bail!(
            "type error: incompatible types for operator *: {}, {}",
            show_value(lhs),
            show_value(rhs)
        ),
    }
}

fn eval_mul2((lhs, rhs): (Value, Value)) -> Result<Value> {
    eval_mul(&lhs, &rhs)
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
        ) => Ok(int_lit(
            bounded_int(*i1)?
                .checked_div(bounded_int(*i2)?)
                .ok_or_else(|| eyre!("integer division overflow or division by zero"))?,
        )),
        _ => bail!(
            "type error: incompatible types for operator /: {}, {}",
            show_value(lhs),
            show_value(rhs)
        ),
    }
}

fn eval_div2((lhs, rhs): (Value, Value)) -> Result<Value> {
    eval_div(&lhs, &rhs)
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

    fn int(v: i128) -> Value {
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
        let err = apply_symbol("/", vec![int(i128::from(i64::MIN)), int(-1)]).unwrap_err();
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
    fn neg_int_returns_int_literal() {
        let result = apply_symbol("neg", vec![int(2)]).unwrap();
        assert!(matches!(
            result,
            Value::Literal(Literal::Number(Number::Int(-2)))
        ));
    }

    #[test]
    fn neg_float_returns_float_literal() {
        let result = apply_symbol("neg", vec![float(2.5)]).unwrap();
        match result {
            Value::Literal(Literal::Number(Number::Float(v))) => assert_eq!(v, -2.5),
            other => panic!("expected float literal, got {other:?}"),
        }
    }

    #[test]
    fn neg_non_numeric_returns_type_error() {
        let err = apply_symbol("neg", vec![bool_v(true)]).unwrap_err();
        assert!(err.to_string().contains("operator -"));
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
    fn eq_large_ints_preserves_integer_precision() {
        let lhs = int((1i128 << 54) + 1);
        let rhs = int((1i128 << 54) + 2);
        let result = apply_symbol("==", vec![lhs, rhs]).unwrap();
        assert!(matches!(result, Value::Literal(Literal::Bool(false))));
    }

    #[test]
    fn cmp_large_ints_preserves_integer_precision() {
        let lhs = int((1i128 << 54) + 2);
        let rhs = int((1i128 << 54) + 1);
        let result = apply_symbol(">", vec![lhs, rhs]).unwrap();
        assert!(matches!(result, Value::Literal(Literal::Bool(true))));
    }

    #[test]
    fn builtin_registry_contains_logic_and_compare_ops() {
        let env = builtin_functions();
        for op in ["==", "!=", "<", "<=", ">", ">=", "!", "neg", "&&", "||"] {
            assert!(env.contains_key(op), "missing builtin op: {op}");
        }
    }
}
