use itertools::Itertools;
use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::ast::{Expr, Literal};

pub fn new_int(i: isize) -> Expr {
    Expr::Literal(Literal::Int(i))
}

// Usage
pub fn new_var(name: &str) -> Expr {
    Expr::Var(name.to_owned())
}

pub fn new_func(arg: &str, body: &Expr) -> Expr {
    Expr::Func(arg.to_owned(), Box::new(body.clone()))
}

pub fn new_apply(lhs: &Expr, rhs: &Expr) -> Expr {
    Expr::App(Box::new(lhs.clone()), Box::new(rhs.clone()))
}

pub fn new_bind(name: &str, value: Expr, body: &Expr) -> Expr {
    Expr::Let(vec![(name.to_owned(), value)], Box::new(body.clone()))
}

pub fn show(expr: &Expr) -> String {
    show_prec(expr, 0)
}

fn show_prec(expr: &Expr, prec: usize) -> String {
    match expr {
        Expr::Literal(l) => l.to_string(),
        Expr::Var(v) => v.clone(),
        Expr::Func(arg, body) => {
            let s = format!("\\{}. {}", arg, show_prec(body, 0));
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
}

pub fn show_value(v: &Value) -> String {
    match v {
        Value::Literal(l) => l.to_string(),
        Value::Closure { .. } => "<closure>".to_owned(),
        Value::Thunk(_) => "<thunk>".to_owned(),
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

pub fn force_whnf(v: Value) -> Value {
    let mut cur = v;
    loop {
        match cur {
            Value::Thunk(cell) => {
                match &cell.borrow().state {
                    ThunkState::Unevaluated => {}
                    ThunkState::Evaluated(v) => return v.clone(),
                    ThunkState::Evaluating => {
                        panic!("blackhole: recursive thunk is forced while evaluationg")
                    }
                }

                cell.borrow_mut().update_state(ThunkState::Evaluating);

                let (expr, env) = {
                    let t = cell.borrow();
                    (t.expr.clone(), t.env.clone())
                };

                let computed = eval(expr, env);

                cell.borrow_mut()
                    .update_state(ThunkState::Evaluated(computed.clone()));

                cur = computed;
            }
            other => return other,
        }
    }
}

pub fn eval(e: Expr, env: Rc<Env>) -> Value {
    match e {
        Expr::Literal(l) => Value::Literal(l),
        Expr::Var(name) => env
            .get(&name)
            .unwrap_or_else(|| panic!("unbound variable: {}", name)),
        Expr::Func(arg, body) => Value::Closure {
            param: arg,
            body: *body,
            env,
        },
        Expr::App(lhs, rhs) => {
            let f = force_whnf(eval(*lhs, env.clone()));
            match f {
                Value::Closure {
                    param,
                    body,
                    env: closure_env,
                } => {
                    let thunk = Value::Thunk(Rc::new(RefCell::new(Thunk {
                        expr: *rhs,
                        env: env.clone(),
                        state: ThunkState::Unevaluated,
                    })));

                    let mut new_map = HashMap::new();
                    new_map.insert(param, thunk);

                    eval(body, Rc::new(Env::new(Some(closure_env.clone()), new_map)))
                }
                _ => panic!("attempted to apply a non-function expression."),
            }
        }
        Expr::Let(vars, body) => {
            let let_env = Rc::new(Env::new(Some(env.clone()), HashMap::new()));

            let mut cells: HashMap<String, Rc<RefCell<Thunk>>> = HashMap::new();
            for (name, _) in &vars {
                let cell = Rc::new(RefCell::new(Thunk {
                    expr: Expr::Literal(Literal::Int(0)),
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
    }
}
