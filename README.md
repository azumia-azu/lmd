# Lmd 语法设计

Lmd以学习编译原理为目的的语言, 同时因为作者比较好奇函数式, 所以想设计一门函数式语言
名字来源与代表函数式语言的lambda 演算, 程序目标为实现一个简单的解释器

## 语言基础

惰性求值

### Expression

* Variable: 变量/常量
* Lambda抽象: `\x -> expr`
* 应用(左结合): `f a b`
* Let绑定: `let [x = expr1]+ in expr2`
* Types
  * Number: 数字
  * Bool: 布尔值
  * List: 列表
  * Function: 函数类型