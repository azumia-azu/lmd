# Lmd 语法设计

Lmd 是一门用于学习编译原理和函数式语言实现的实验语言，目标是实现一个小型解释器。

当前版本聚焦：
- 惰性求值（call-by-need）
- Lambda 演算核心表达式
- 递归 `let`

## Parser v1 范围

Parser 第一版只保证以下表达式可解析：
- 变量
- 字面量：`Int` / `Float` / `String`
- Lambda 抽象：`\x -> expr`
- 函数应用（左结合）：`f a b`
- Let 绑定（递归，多绑定）：`let x = e1; y = e2; in e3`

## 词法约定（Lexer）

### 关键字
- `let`
- `in`

### 符号
- `\` lambda 起始
- `->` lambda 箭头
- `=` let 绑定赋值
- `;` let 绑定分隔
- `(` `)` 分组
- `{` `}` 分组（可选，便于兼容当前 pretty print）

### 标识符
- 正则建议：`[a-zA-Z_][a-zA-Z0-9_]*`
- 不能与关键字重名（`let`, `in`）

### 字面量
- `Int`: `-?[0-9]+`
- `Float`: `-?[0-9]+\.[0-9]+`
- `String`: 双引号包裹，如 `"hello"`（v1 可先支持最小转义集或不支持转义）

### 空白
- 空格、制表符、换行均可作为分隔符

## 语法约定（EBNF）

```ebnf
Expr        ::= LetExpr | LambdaExpr | AppExpr

LetExpr     ::= "let" Binding (";" Binding)* ";"? "in" Expr
Binding     ::= Ident "=" Expr

LambdaExpr  ::= "\" Ident "->" Expr

AppExpr     ::= Atom (Atom)*

Atom        ::= Literal
             | Ident
             | "(" Expr ")"
             | "{" Expr "}"

Literal     ::= Float | Int | String
Ident       ::= /[a-zA-Z_][a-zA-Z0-9_]*/
```

说明：
- `LetExpr` 为递归 let：同一 `let` 块内绑定可相互引用，也可引用自身。
- `AppExpr ::= Atom (Atom)*` 表示函数应用左结合。
- `LetExpr` / `LambdaExpr` 优先级低于应用表达式。

## 优先级与结合性

从高到低：
1. 原子表达式：字面量、变量、括号/花括号分组
2. 应用：左结合（`f a b` 解析为 `(f a) b`）
3. Lambda：右侧尽可能长（`\x -> f x y` 解析为 `\x -> ((f x) y)`）
4. Let：`in` 右侧尽可能长

## 语义约定

- `let` 是递归 let（对应当前 evaluator 行为）。
- 变量查找按词法作用域，从当前环境向外层环境查找。
- 对未绑定变量在运行时报错。

## 与 AST 的映射

- `Literal` -> `Expr::Literal`
- `Ident` -> `Expr::Var`
- `\x -> body` -> `Expr::Func("x", body)`
- `f a` -> `Expr::App(f, a)`
- `let x = e1; y = e2; in body` -> `Expr::Let(vec![("x", e1), ("y", e2)], body)`

## 示例

```lmd
\x -> x
```

```lmd
(\x -> x) 42
```

```lmd
let id = \x -> x; in id 1
```

```lmd
let f = \x -> x; g = \y -> y; in f (g 3)
```

## 非目标（v1 暂不做）

- 类型系统与类型推导
- 布尔、列表及内建运算符优先级体系
- 模式匹配
