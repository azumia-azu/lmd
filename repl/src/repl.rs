use std::borrow::Cow;
use std::rc::Rc;

use reedline::{Prompt, PromptEditMode, PromptHistorySearch, Reedline, Signal};

pub fn repl_loop(env: Rc<crate::eval::Env>) -> eyre::Result<()> {
    let mut line_editor = Reedline::create();

    loop {
        let Some(src) = read_form(&mut line_editor, "lmd> ", "....> ")? else {
            break;
        };

        let src_trimmed = src.trim();
        if src_trimmed.is_empty() {
            continue;
        }

        match lmd_core::parser::parse(src_trimmed) {
            Ok(expr) => match crate::eval::eval(expr, env.clone()) {
                Ok(v) => match crate::eval::force_whnf(v) {
                    Ok(v) => println!("{}", v),
                    Err(e) => eprintln!("Evaluation error: {:?}", e),
                },
                Err(e) => eprintln!("Evaluation error: {:?}", e),
            },
            Err(e) => eprintln!("Parse error: {:?}", e),
        }
    }

    Ok(())
}

fn read_form(
    line_editor: &mut Reedline,
    prompt: &str,
    cont_prompt: &str,
) -> eyre::Result<Option<String>> {
    let mut buf = String::new();

    loop {
        let prompt = LmdPrompt::new(if buf.is_empty() { prompt } else { cont_prompt });
        match line_editor.read_line(&prompt)? {
            Signal::Success(line) => {
                if buf.trim().is_empty() {
                    let t = line.trim();
                    if t == ":quit" || t == ":q" {
                        return Ok(None);
                    }
                }

                if buf.is_empty() && line.trim().is_empty() {
                    continue;
                }

                buf.push_str(&line);
                buf.push('\n');

                if buf.trim().is_empty() {
                    buf.clear();
                    continue;
                }

                if should_continue(&buf) {
                    continue;
                }

                match lmd_core::parser::try_parse(buf.trim_end()) {
                    Ok(()) => return Ok(Some(buf)),
                    Err(e) if e.is_unexpected_eof() => continue,
                    Err(_) => return Ok(Some(buf)),
                }
            }
            Signal::CtrlD => {
                return if buf.trim().is_empty() {
                    Ok(None)
                } else {
                    Ok(Some(buf))
                };
            }
            Signal::CtrlC => return Ok(Some(String::new())),
        }
    }
}

fn should_continue(src: &str) -> bool {
    let (par, bra, brk, in_str) = balance_delims(src);
    if in_str || par != 0 || bra != 0 || brk != 0 {
        return true;
    }

    let t = src.trim_end();
    if t.ends_with('=') || t.ends_with("->") || t.ends_with('\\') {
        return true;
    }

    false
}

fn balance_delims(src: &str) -> (i32, i32, i32, bool) {
    let mut par = 0i32;
    let mut bra = 0i32;
    let mut brk = 0i32;
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

struct LmdPrompt<'a> {
    left: &'a str,
}

impl<'a> LmdPrompt<'a> {
    fn new(left: &'a str) -> Self {
        Self { left }
    }
}

impl Prompt for LmdPrompt<'_> {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        Cow::Borrowed(self.left)
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _edit_mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        let status = match history_search.status {
            reedline::PromptHistorySearchStatus::Passing => "",
            reedline::PromptHistorySearchStatus::Failing => "failing ",
        };

        Cow::Owned(format!(
            "({status}reverse-search: {}) ",
            history_search.term
        ))
    }
}
