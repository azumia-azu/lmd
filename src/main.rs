use core::panic;
use itertools::Itertools;
use std::{cell::RefCell, collections::HashMap, fmt::Display, rc::Rc};

/// AST
#[derive(Clone, Debug)]
enum Expr {
    Literal(Literal),
    Var(String),
    Func(String, Box<Expr>),
    App(Box<Expr>, Box<Expr>),
    Let(Vec<(String, Expr)>, Box<Expr>),
}

#[derive(Clone, Copy, Debug)]
enum Literal {
    Int(isize),
}

impl Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(i) => write!(f, "{}", i),
        }
    }
}

fn show(expr: &Expr) -> String {
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
struct Env {
    parent: Option<Rc<Env>>,
    value: RefCell<HashMap<String, Value>>,
}

impl Env {
    fn new(parent: Option<Rc<Env>>, value: HashMap<String, Value>) -> Self {
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
enum Value {
    Closure {
        param: String,
        body: Expr,
        env: Rc<Env>,
    },
    Thunk(Rc<RefCell<Thunk>>),
    Literal(Literal),
}

#[derive(Debug)]
enum ThunkState {
    Unevaluated,
    Evaluating,
    Evaluated(Value),
}

#[derive(Debug)]
struct Thunk {
    expr: Expr,
    env: Rc<Env>,
    state: ThunkState,
}

impl Thunk {
    fn update_state(&mut self, state: ThunkState) {
        self.state = state;
    }
}

fn force(v: Value) -> Value {
    match v {
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
            computed
        }
        other => other,
    }
}

fn eval(e: Expr, env: Rc<Env>) -> Value {
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
            let f = force(eval(*lhs, env.clone()));
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
            todo!("recursive let binding");
            let let_env = Rc::new(Env::new(Some(env.clone()), HashMap::new()));
            for (name, expr) in vars {
                let thunk = Value::Thunk(Rc::new(RefCell::new(Thunk {
                    expr: expr.clone(),
                    env: let_env.clone(),
                    state: ThunkState::Unevaluated,
                })));

                let_env.insert(name, thunk);
            }

            eval(*body, let_env.clone())
        }
    }
}

fn new_int(i: isize) -> Expr {
    Expr::Literal(Literal::Int(i))
}

// Usage
fn new_var(name: &str) -> Expr {
    Expr::Var(name.to_owned())
}

fn new_func(arg: &str, body: &Expr) -> Expr {
    Expr::Func(arg.to_owned(), Box::new(body.clone()))
}

fn new_apply(lhs: &Expr, rhs: &Expr) -> Expr {
    Expr::App(Box::new(lhs.clone()), Box::new(rhs.clone()))
}

fn new_bind(name: &str, value: Expr, body: &Expr) -> Expr {
    Expr::Let(vec![(name.to_owned(), value)], Box::new(body.clone()))
}

fn main() {
    let simple_x = new_var("x");
    let id = new_func("x", &simple_x);
    let apply_id = new_apply(&id, &new_var("a"));
    let let_bind = new_bind("a", new_int(4), &apply_id);
    println!("{}", show(&simple_x));
    println!("{}", show(&id));
    println!("{}", show(&apply_id));
    println!(
        "{:#?}",
        force(eval(let_bind, Rc::new(Env::new(None, HashMap::new()))))
    )
}
