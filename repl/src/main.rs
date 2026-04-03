use lmd_repl::eval::new_env;

fn main() {
    let env = new_env();

    lmd_repl::repl::repl_loop(env.clone()).unwrap();
}
