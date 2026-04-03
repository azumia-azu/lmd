use eyre::{Result, bail, eyre};
use itertools::Itertools;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use lmd_core::ast::{Expr, Literal, Number};

use crate::builtins::{BuiltinFunction, apply_builtin_function, builtin_functions};

pub fn new_env() -> Rc<Env> {
    Rc::new(Env::new(None, builtin_functions()))
}

#[allow(dead_code)]
pub fn show(expr: &Expr) -> String {
    show_prec(expr, 0)
}

fn show_prec(expr: &Expr, prec: usize) -> String {
    match expr {
        Expr::Literal(l) => l.to_string(),
        Expr::Var(v) => v.clone(),
        Expr::Func(arg, body) => {
            let s = format!("\\{} -> {}", arg, show_prec(body, 0));
            if prec > 0 { format!("({})", s) } else { s }
        }
        Expr::App(f, x) => {
            // application is left-associative; atoms bind tight
            let s = format!("{} {}", show_prec(f, 1), show_prec(x, 2));
            if prec > 1 { format!("({})", s) } else { s }
        }
        Expr::Let(vars, body) => {
            let lets = vars
                .iter()
                .map(|let_item| format!("{} = {};", let_item.0, show_prec(&let_item.1, 0)))
                .join("");
            format!("let {} in {{{}}}", lets, show_prec(body, 0))
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let s = format!(
                "if {} then {{{}}} else {{{}}}",
                show_prec(cond, 0),
                show_prec(then_branch, 0),
                show_prec(else_branch, 0)
            );
            if prec > 0 { format!("({})", s) } else { s }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Env {
    parent: Option<Rc<Env>>,
    value: RefCell<HashMap<String, Value>>,
}

impl Env {
    pub fn new(parent: Option<Rc<Env>>, value: HashMap<String, Value>) -> Self {
        Env {
            parent,
            value: RefCell::new(value),
        }
    }

    fn get(&self, k: &str) -> Option<Value> {
        if let Some(v) = self.value.borrow().get(k).cloned() {
            Some(v)
        } else {
            self.parent.as_ref().and_then(|p| p.get(k))
        }
    }

    fn insert(&self, k: String, v: Value) {
        self.value.borrow_mut().insert(k, v);
    }
}

// Eval
#[derive(Clone, Debug)]
pub enum Value {
    Closure {
        param: String,
        body: Expr,
        env: Rc<Env>,
    },
    Thunk(Rc<RefCell<Thunk>>),
    Literal(Literal),
    BuiltinFunction(BuiltinFunction),
}

pub fn show_value(v: &Value) -> String {
    match v {
        Value::Literal(l) => l.to_string(),
        Value::Closure { .. } => "<closure>".to_owned(),
        Value::Thunk(_) => "<thunk>".to_owned(),
        Value::BuiltinFunction(b) => format!("{}", b),
    }
}

impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", show_value(self))
    }
}

#[derive(Debug)]
enum ThunkState {
    Unevaluated,
    Evaluating,
    Evaluated(Value),
}

#[derive(Debug)]
pub struct Thunk {
    expr: Expr,
    env: Rc<Env>,
    state: ThunkState,
}

impl Thunk {
    fn update_state(&mut self, state: ThunkState) {
        self.state = state;
    }
}

pub fn mk_thunk(expr: Expr, env: Rc<Env>) -> Value {
    Value::Thunk(Rc::new(RefCell::new(Thunk {
        expr,
        env,
        state: ThunkState::Unevaluated,
    })))
}

pub fn force_whnf(v: Value) -> Result<Value> {
    let mut cur = v;
    loop {
        match cur {
            Value::Thunk(cell) => {
                match &cell.borrow().state {
                    ThunkState::Unevaluated => {}
                    ThunkState::Evaluated(v) => return Ok(v.clone()),
                    ThunkState::Evaluating => {
                        bail!("blackhole: recursive thunk is forced while evaluationg")
                    }
                }

                cell.borrow_mut().update_state(ThunkState::Evaluating);

                let (expr, env) = {
                    let t = cell.borrow();
                    (t.expr.clone(), t.env.clone())
                };

                let computed = eval(expr, env)?;

                cell.borrow_mut()
                    .update_state(ThunkState::Evaluated(computed.clone()));

                cur = computed;
            }
            other => return Ok(other),
        }
    }
}

/// Evaluates an expression to a lazy `Value`.
///
/// # Examples
///
/// ```
/// use eyre::Result;
/// use lmd_core::ast::{Literal, Number};
/// use lmd_core::parser::parse;
/// use lmd_repl::eval::{Value, eval, force_whnf, new_env};
///
/// fn main() -> Result<()> {
///     let src = r#"
///         let fib = \n ->
///             if n == 0 then 0
///             else if n == 1 then 1
///             else fib (n - 1) + fib (n - 2)
///         in fib 10
///     "#;
///
///     let expr = parse(src)?;
///     let value = force_whnf(eval(expr, new_env())?)?;
///
///     assert!(matches!(
///         value,
///         Value::Literal(Literal::Number(Number::Int(55)))
///     ));
///
///     Ok(())
/// }
/// ```
pub fn eval(e: Expr, env: Rc<Env>) -> Result<Value> {
    match e {
        Expr::Literal(l) => Ok(Value::Literal(l)),
        Expr::Var(name) => env
            .get(&name)
            .ok_or_else(|| eyre!("unbound variable: {}", name)),
        Expr::Func(arg, body) => Ok(Value::Closure {
            param: arg,
            body: *body,
            env,
        }),
        Expr::App(lhs, rhs) => {
            let f = force_whnf(eval(*lhs, env.clone())?)?;
            match f {
                Value::Closure {
                    param,
                    body,
                    env: closure_env,
                } => {
                    let thunk = mk_thunk(*rhs, env.clone());

                    let mut new_map = HashMap::new();
                    new_map.insert(param, thunk);

                    eval(body, Rc::new(Env::new(Some(closure_env.clone()), new_map)))
                }
                Value::BuiltinFunction(builtin) => {
                    let arg = mk_thunk(*rhs, env.clone());
                    apply_builtin_function(builtin, arg)
                }
                _ => bail!("attempted to apply a non-function expression."),
            }
        }
        Expr::Let(vars, body) => {
            let let_env = Rc::new(Env::new(Some(env.clone()), HashMap::new()));

            let mut cells: HashMap<String, Rc<RefCell<Thunk>>> = HashMap::new();
            for (name, _) in &vars {
                let cell = Rc::new(RefCell::new(Thunk {
                    expr: Expr::Literal(Literal::Number(Number::Int(0))), // dummy
                    env: let_env.clone(),
                    state: ThunkState::Unevaluated,
                }));
                let_env.insert(name.clone(), Value::Thunk(cell.clone()));
                cells.insert(name.clone(), cell);
            }

            for (name, expr) in vars {
                let cell = cells.remove(&name).unwrap();
                let mut t = cell.borrow_mut();
                t.expr = expr;
                t.env = let_env.clone();
                t.state = ThunkState::Unevaluated;
            }

            eval(*body, let_env)
        }
        Expr::If {
            cond,
            then_branch,
            else_branch,
        } => {
            let cond_val = force_whnf(eval(*cond, env.clone())?)?;
            match cond_val {
                Value::Literal(Literal::Bool(true)) => eval(*then_branch, env),
                Value::Literal(Literal::Bool(false)) => eval(*else_branch, env),
                _ => bail!("condition of if expression must be a boolean literal"),
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use lmd_core::parser::parse;

    fn int_lit(v: isize) -> Expr {
        Expr::Literal(Literal::Number(Number::Int(v)))
    }

    fn eval_src_to_whnf(src: &str) -> Result<Value> {
        let env = new_env();
        let expr = parse(src).map_err(|e| eyre!("parse error: {e}"))?;
        let value = eval(expr, env)?;
        force_whnf(value)
    }

    #[test]
    fn show_app_nested_right_argument_adds_parentheses() {
        let expr = Expr::App(
            Box::new(Expr::Var("f".to_string())),
            Box::new(Expr::App(
                Box::new(Expr::Var("g".to_string())),
                Box::new(Expr::Var("x".to_string())),
            )),
        );

        assert_eq!(show(&expr), "f (g x)");
    }

    #[test]
    fn eval_application_identity_function_returns_argument() {
        let env = new_env();
        let expr = Expr::App(
            Box::new(Expr::Func(
                "x".to_string(),
                Box::new(Expr::Var("x".to_string())),
            )),
            Box::new(int_lit(42)),
        );

        let value = eval(expr, env).unwrap();
        let whnf = force_whnf(value).unwrap();
        assert!(matches!(
            whnf,
            Value::Literal(Literal::Number(Number::Int(42)))
        ));
    }

    #[test]
    fn eval_let_empty_bindings_returns_body_value() {
        let env = new_env();
        let expr = Expr::Let(vec![], Box::new(int_lit(7)));

        let value = eval(expr, env).unwrap();
        let whnf = force_whnf(value).unwrap();
        assert!(matches!(
            whnf,
            Value::Literal(Literal::Number(Number::Int(7)))
        ));
    }

    #[test]
    fn eval_unbound_variable_returns_error() {
        let env = new_env();
        let err = eval(Expr::Var("missing".to_string()), env).unwrap_err();
        assert!(err.to_string().contains("unbound variable"));
    }

    #[test]
    fn eval_applying_non_function_returns_error() {
        let env = new_env();
        let expr = Expr::App(Box::new(int_lit(1)), Box::new(int_lit(2)));

        let err = eval(expr, env).unwrap_err();
        assert!(err.to_string().contains("non-function"));
    }

    #[test]
    fn force_whnf_thunk_in_evaluating_state_returns_blackhole_error() {
        let env = new_env();
        let thunk = Thunk {
            expr: int_lit(1),
            env,
            state: ThunkState::Evaluating,
        };

        let err = force_whnf(Value::Thunk(Rc::new(RefCell::new(thunk)))).unwrap_err();
        assert!(err.to_string().contains("blackhole"));
    }

    #[test]
    fn eval_if_true_returns_then_branch_value() {
        let env = new_env();
        let expr = Expr::If {
            cond: Box::new(Expr::Literal(Literal::Bool(true))),
            then_branch: Box::new(int_lit(11)),
            else_branch: Box::new(int_lit(22)),
        };

        let value = eval(expr, env).unwrap();
        let whnf = force_whnf(value).unwrap();
        assert!(matches!(
            whnf,
            Value::Literal(Literal::Number(Number::Int(11)))
        ));
    }

    #[test]
    fn eval_if_false_returns_else_branch_value() {
        let env = new_env();
        let expr = Expr::If {
            cond: Box::new(Expr::Literal(Literal::Bool(false))),
            then_branch: Box::new(int_lit(11)),
            else_branch: Box::new(int_lit(22)),
        };

        let value = eval(expr, env).unwrap();
        let whnf = force_whnf(value).unwrap();
        assert!(matches!(
            whnf,
            Value::Literal(Literal::Number(Number::Int(22)))
        ));
    }

    #[test]
    fn eval_if_non_boolean_condition_returns_error() {
        let env = new_env();
        let expr = Expr::If {
            cond: Box::new(int_lit(1)),
            then_branch: Box::new(int_lit(11)),
            else_branch: Box::new(int_lit(22)),
        };

        let err = eval(expr, env).unwrap_err();
        assert!(
            err.to_string()
                .contains("condition of if expression must be a boolean literal")
        );
    }

    #[test]
    fn eval_if_true_does_not_evaluate_else_branch() {
        let env = new_env();
        let expr = Expr::If {
            cond: Box::new(Expr::Literal(Literal::Bool(true))),
            then_branch: Box::new(int_lit(7)),
            else_branch: Box::new(Expr::App(Box::new(int_lit(1)), Box::new(int_lit(2)))),
        };

        let value = eval(expr, env).unwrap();
        let whnf = force_whnf(value).unwrap();
        assert!(matches!(
            whnf,
            Value::Literal(Literal::Number(Number::Int(7)))
        ));
    }

    #[test]
    fn eval_if_false_does_not_evaluate_then_branch() {
        let env = new_env();
        let expr = Expr::If {
            cond: Box::new(Expr::Literal(Literal::Bool(false))),
            then_branch: Box::new(Expr::App(Box::new(int_lit(1)), Box::new(int_lit(2)))),
            else_branch: Box::new(int_lit(9)),
        };

        let value = eval(expr, env).unwrap();
        let whnf = force_whnf(value).unwrap();
        assert!(matches!(
            whnf,
            Value::Literal(Literal::Number(Number::Int(9)))
        ));
    }

    #[test]
    fn eval_parsed_if_expression_with_non_grouped_expression_branches() {
        let whnf = eval_src_to_whnf("if true then 1+2 else 3+4").unwrap();
        assert!(matches!(
            whnf,
            Value::Literal(Literal::Number(Number::Int(3)))
        ));
    }

    #[test]
    fn eval_parsed_if_expression_with_expression_condition_returns_error() {
        let err = eval_src_to_whnf("if 1+2 then 3 else 4").unwrap_err();
        assert!(
            err.to_string()
                .contains("condition of if expression must be a boolean literal")
        );
    }

    #[test]
    fn eval_parsed_nested_if_expression_in_then_branch() {
        let whnf = eval_src_to_whnf("if true then if false then 1 else 2 else 3").unwrap();
        assert!(matches!(
            whnf,
            Value::Literal(Literal::Number(Number::Int(2)))
        ));
    }

    #[test]
    fn eval_and_short_circuits_rhs() {
        let whnf = eval_src_to_whnf("false && missing").unwrap();
        assert!(matches!(whnf, Value::Literal(Literal::Bool(false))));
    }

    #[test]
    fn eval_or_short_circuits_rhs() {
        let whnf = eval_src_to_whnf("true || missing").unwrap();
        assert!(matches!(whnf, Value::Literal(Literal::Bool(true))));
    }

    #[test]
    fn eval_and_forces_rhs_when_lhs_is_true() {
        let err = eval_src_to_whnf("true && missing").unwrap_err();
        assert!(err.to_string().contains("unbound variable"));
    }

    #[test]
    fn eval_or_forces_rhs_when_lhs_is_false() {
        let err = eval_src_to_whnf("false || missing").unwrap_err();
        assert!(err.to_string().contains("unbound variable"));
    }

    #[test]
    fn eval_add_forces_rhs() {
        let err = eval_src_to_whnf("1 + missing").unwrap_err();
        assert!(err.to_string().contains("unbound variable"));
    }
}
