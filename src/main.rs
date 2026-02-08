mod ast;
mod eval;

use std::collections::HashMap;
use std::rc::Rc;

use crate::eval::{
    Env, eval, force_whnf, new_apply, new_bind, new_func, new_int, new_var, show, show_value,
};

fn main() {
    let simple_x = new_var("x");
    let id = new_func("x", &simple_x);
    let apply_id = new_apply(&id, &new_var("a"));
    let let_bind = new_bind("a", new_int(4), &apply_id);
    println!("{}", show(&simple_x));
    println!("{}", show(&id));
    println!("{}", show(&apply_id));
    let result = force_whnf(eval(let_bind, Rc::new(Env::new(None, HashMap::new()))));
    println!("{}", show_value(&result))
}
