pub mod ast;
use ast::{Env, Expr, Literal, eval, force, show};
use std::collections::HashMap;
use std::rc::Rc;

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
