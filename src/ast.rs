use std::fmt::Display;

/// Abstract Syntax Tree (AST) for Lmd
/// Lmd 的 AST 定义了 Lmd 语言的语法结构。它包括以下几种表达式类型：
/// - Literal: 表示一个字面量值，例如数字、字符串等.
/// - Variable: 表示一个变量，通常是一个标识符.
/// - Lambda abstraction: 表示一个匿名函数.
/// - Application: 函数应用，表示将一个函数应用于一个参数.
/// - Let: 表示一个 let 绑定.
///
#[derive(Clone, Debug)]
pub enum Expr {
    /// Literal: 表示一个字面量值，例如数字、字符串等。
    Literal(Literal),

    /// Variable: 表示一个变量，通常是一个标识符。
    Var(String),

    /// Lambda abstraction: 表示一个匿名函数，通常使用反斜杠（\）或 lambda 关键字来表示。
    /// 例如，表达式 \x. x + 1 表示一个匿名函数，接受一个参数 x，并返回 x + 1 的结果。
    /// 在 Lmd 中，lambda 表达式的优先级最低，这意味着在没有括号的情况下，lambda 表达式会绑定最松。例如，表达式 \x. f x 会被解析为 \x. (f x)，而不是 (\x. f
    Func(String, Box<Expr>),

    /// Application: 函数应用，表示将一个函数应用于一个参数。
    /// 例如，表达式 f x 表示将函数 f 应用于参数 x。
    /// 在 Lmd 中，函数应用是左结合的，这意味着在没有括号的情况下，函数应用会从左到右进行解析。
    /// 例如，表达式 f g x 会被解析为 (f g) x，而不是 f (g x)。因此，函数应用的优先级较高，原子
    /// 表达式（如变量和字面量）绑定最紧，函数应用次之，lambda 表达式绑定最松。
    App(Box<Expr>, Box<Expr>),

    /// Let: Lmd 中 的 let 为recursive let，允许在 let 定义中使用自己
    /// 例如：let fact = \n. if n == 0 then 1 else n * fact (n - 1) in fact 5
    /// 在这个例子中，fact 在自己的定义中被使用了，这就是 recursive let 的特点。
    /// 如果不允许 recursive let，那么在 fact 的定义中就不能使用 fact，这样就无法定义递归函数了。
    /// 因此，Lmd 中的 let 是 recursive let，这使得我们能够定义递归函数。
    Let(Vec<(String, Expr)>, Box<Expr>),
}

/// Literal: 表示一个字面量值，例如数字、字符串等。
#[derive(Clone, Debug)]
pub enum Literal {
    Int(isize),
    Float(f64),
    Str(String),
}

impl Display for Literal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(i) => write!(f, "{}", i),
            Self::Float(fl) => write!(f, "{}", fl),
            Self::Str(s) => write!(f, "\"{}\"", s),
        }
    }
}
