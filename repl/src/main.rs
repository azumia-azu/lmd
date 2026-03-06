mod builtins;
mod eval;
mod repl;

use eval::new_env;

fn main() {
    let env = new_env();

    crate::repl::repl_loop(env.clone()).unwrap();
}
