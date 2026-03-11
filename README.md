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

## Roadmap

### P0: 让递归可实用（最高优先级）
- 条件表达式：`if cond then e1 else e2`
- 布尔字面量：`true` / `false`
- 比较与相等运算：`== != < <= > >=`
- 逻辑运算：`&& || not`

原因：
- 当前已有递归 `let` 和算术，但没有分支与比较，递归无法写出终止条件，导致很多基础程序（如阶乘/斐波那契）不可表达。

### P1: 语法一致性与易用性
- 一元负号（表达式级）与字面量负数语义统一
- 运算符前缀/中缀写法规则进一步收敛（减少特殊 case）
- 错误信息增强（更清晰的 expected token 和位置上下文）

### P2: 语言能力扩展
- ADT 与模式匹配（`match`）
- 列表/元组等基础数据结构
- 更丰富的标准内建函数

### P3: 工程化能力
- 类型系统（先 HM 推导，再考虑类型注解）
- 模块/导入系统
- 更完整的 REPL 体验（历史、补全、`:type` 等）

### P4: 工具链与生态扩展
- Compiler：从解释执行扩展到编译管线（可先做 bytecode 或 IR，再到本地代码生成）
- Language Server：提供诊断、跳转、补全、悬浮信息、重命名等 IDE 能力
- Tree-sitter grammar：提供稳定增量解析，支撑高亮、结构化编辑与代码导航

## Milestones

- M1（2026-04-30）：P0 最小可用落地
- M2（2026-06-30）：P1 语法一致性与错误体验
- M3（2026-09-30）：P2 数据结构与匹配
- M4（2026-12-31）：P3 类型系统与模块系统
- M5（2027-03-31）：P4 工具链生态第一版
