use std::{io::{self, Write}, rc::Rc};

pub fn repl_loop(env: Rc<crate::eval::Env>) -> eyre::Result<()> {
    loop {
        let Some(src) = read_form("lmd> ", "....> ")? else {
            break;
        };

        let src_trimmed = src.trim();
        if src_trimmed.is_empty() {
            continue;
        }

        match lmd_core::parser::parse(src_trimmed) {
            Ok(expr) => {
                match crate::eval::eval(expr, env.clone()) {
                    Ok(v) => {
                        match crate::eval::force_whnf(v) {
                            Ok(v) => println!("{}", v),
                            Err(e) => eprintln!("Evaluation error: {:?}", e),
                        }
                    },
                    Err(e) => eprintln!("Evaluation error: {:?}", e),
                }
            }
            Err(e) => eprintln!("Parse error: {:?}", e),
        }
    }

    Ok(())
}

fn read_form(prompt: &str, cont_prompt: &str) -> eyre::Result<Option<String>> {
    let stdin = io::stdin();
    let mut buf = String::new();
    let mut line = String::new();

    loop {
        line.clear();

        if buf.is_empty() {
            print!("{}", prompt);
        } else {
            print!("{}", cont_prompt);
        }
        io::stdout().flush()?;

        let n = stdin.read_line(&mut line)?;
        if n == 0 {
            return if buf.trim().is_ascii() {
                Ok(None)
            } else {
                Ok(Some(buf))
            };
        }

        if buf.trim().is_empty() {
            let t = line.trim();
            if t == ":quit" || t == ":q" {
                return Ok(None);
            }
        }

        buf.push_str(&line);
        if buf.trim().is_empty() {
            buf.clear();
            continue;
        }

        if should_continue(&buf) {
            continue;
        }


                // 看起来完整：尝试 parse
        match lmd_core::parser::try_parse(buf.trim_end()) {
            Ok(()) => return Ok(Some(buf)),
            Err(e) if e.is_unexpected_eof() => continue, // 缺后续，继续读
            Err(_) => return Ok(Some(buf)), // 非 EOF 错：交给外层报错并清 buffer
        }

    }
    unimplemented!()
}

fn should_continue(src: &str) -> bool {
    // 1) 括号/花括号/方括号配平（忽略字符串）
    let (par, bra, brk, in_str) = balance_delims(src);
    if in_str || par != 0 || bra != 0 || brk != 0 {
        return true;
    }

    // 2) 末尾如果是明显“还没写完”的符号，继续读
    let t = src.trim_end();
    if t.ends_with('=') || t.ends_with("->") || t.ends_with('\\') {
        return true;
    }
    // 如果你 let 语法常跨行，`let ...` 没看到 `in` 也可以继续读
    if t.contains("let") && !t.contains(" in ") && t.ends_with('\n') {
        // 只做启发式，不要太激进；你也可以删掉这条
        // return true;
    }

    false
}

fn balance_delims(src: &str) -> (i32, i32, i32, bool) {
    let mut par = 0i32; // ()
    let mut bra = 0i32; // {}
    let mut brk = 0i32; // []
    let mut in_str = false;
    let mut escape = false;

    for ch in src.chars() {
        if in_str {
            if escape {
                escape = false;
                continue;
            }
            match ch {
                '\\' => escape = true,
                '"' => in_str = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_str = true,
            '(' => par += 1,
            ')' => par -= 1,
            '{' => bra += 1,
            '}' => bra -= 1,
            '[' => brk += 1,
            ']' => brk -= 1,
            _ => {}
        }
    }

    (par, bra, brk, in_str)
}


