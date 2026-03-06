# Lmd

Lmd 是一门用于学习编译原理和函数式语言实现的实验语言。
当前仓库包含：
- `lmd-core`：AST + parser（lalrpop）
- `lmd-repl`：惰性求值解释器（call-by-need）

## 当前能力

- Lambda 表达式：`\\x -> expr`
- 递归 `let`：`let x = e1; y = e2; in e3`
- 函数应用（左结合）：`f a b`
- 数字与字符串字面量
- 基础算术内建：`+ - * /`

## 快速开始

```bash
cargo run -p lmd-repl
```

REPL 中可输入：
- `:q` 或 `:quit` 退出

## 语法（当前）

```ebnf
Expr        ::= LetExpr | LambdaExpr | AddExpr

LetExpr     ::= "let" Binding (";" Binding)* ";"? "in" Expr
Binding     ::= Ident "=" Expr

LambdaExpr  ::= "\\" Ident "->" Expr

AddExpr     ::= FactorExpr (("+" | "-") FactorExpr)*
FactorExpr  ::= AppExpr (("*" | "/") AppExpr)*
AppExpr     ::= Atom Atom*

Atom        ::= Literal
             | Ident
             | "(" Expr ")"
             | "{" Expr "}"
             | "(" ("+" | "-" | "*" | "/") ")"
             | "(" ("+" | "-" | "*" | "/") Expr ")"

Literal     ::= Float | Int | String
Ident       ::= /[a-zA-Z_][a-zA-Z0-9_]*/
```

## 运算符优先级与结合性

从高到低：
1. 原子（字面量/变量/括号）
2. 函数应用（左结合）
3. `* /`（左结合）
4. `+ -`（左结合）
5. `lambda` / `let`

示例：
- `1 + 2 * 3` 解析为 `1 + (2 * 3)`
- `(+) 1 2` 合法（将运算符当函数）
- `(+ 1) 2` 合法（部分应用后再应用）

## 求值语义

- 默认惰性求值（thunk + 共享，call-by-need）。
- 普通函数应用参数按需求值。
- 内建算术在执行时是严格的：会把参与运算的参数强制到可计算值。

## 数值语义

- `Int` 除法：使用安全整除，`division by zero` 或整数除法溢出会返回错误。
- `Float` 除法：遵循 IEEE 754。
  - `1.0 / 0.0 -> inf`
  - `-1.0 / 0.0 -> -inf`
  - `0.0 / 0.0 -> NaN`

## 示例

```lmd
\\x -> x
```

```lmd
let id = \\x -> x; in id 42
```

```lmd
1 + 2 * 3
```

```lmd
(+) 1 2
```

```lmd
(+ 1) 2
```

## 当前未覆盖

- 类型系统 / 类型推导
- 模式匹配
- 代数数据类型
- 模块系统
