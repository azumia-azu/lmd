use std::{cell::RefCell, rc::Rc};

enum Expression {
    Variable(String),
    Function(String, Expr),
    Application(Expr, Expr),
}

type Expr = Box<Expression>;

fn new_expr(expr: Expression) -> Expr {
    Box::new(expr)
}

fn get_expr(expr: &Expr) -> String {
    match **expr {
        Expression::Variable(ref var) => var.to_owned(),
        Expression::Function(ref arg, ref body) => format!("\\{}. {}", arg, get_expr(&body)),
        Expression::Application(ref lhs, ref rhs) => {
            format!("{} {}", get_expr(&lhs), get_expr(&rhs))
        }
    }
}

fn new_var(name: &str) -> Expr {
    new_expr(Expression::Variable(name.to_owned()))
}

fn new_func(arg: &str, body: &Expr) -> Expr {
    new_expr(Expression::Function(arg.to_owned(), body))
}

fn new_apply(lhs: &Expr, rhs: &Expr) -> Expr {
    new_expr(Expression::Application(*lhs.clone(), rhs.clone()))
}

fn main() {
    let simple_x = new_var("x");
    let id = new_func("x", &simple_x);
    let apply_id = new_apply(&id, &new_var("a"));

    println!("{}", get_expr(&simple_x));
    println!("{}", get_expr(&id));
    println!("{}", get_expr(&apply_id));
}
