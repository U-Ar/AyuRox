# AyuRox

## 概要

書籍 Crafting Interpreters で導入されている動的型付け言語LoxのRustによるバイトコードインタプリタ実装

## ベンチマークタイム

craftinginterpreters.com/repo に公開されているテストスイートのうち、`test/benchmark` にある各種ベンチマークの実行時間をリリースビルドで測定

| execution time (s) | reference(clox with nan boxing) | v0.1.0 | v0.1.1 |
| - | - | - | - |
| binary_trees.lox | 0.86 | 3.20 | 3.04 |
| equality.lox(loop) | 0.70 | 2.67 | 2.49 |
| fib.lox | 0.49 | 1.98 | 1.94 |
| instantiation.lox | error | 0.92 | 0.92 |
| invocation.lox | 0.12 | 0.77 | 0.73 |
| method_call.lox | 0.09 | 0.54 | 0.50 |
| properties.lox | 0.17 | 1.42 | 1.42 |
| string_equality.lox | error | 1.24 | 1.07 |
| trees.lox | 0.95 | 4.81 | 4.50 |
| zoo.lox | 0.13 | 0.93 | 0.87 |

- zoo_batch.lox
  - v0.1.0: sum=102540000, batch=1709
  - v0.1.1: sum=109980000, batch=1833

## CHANGELOG

- 2026/01/04: v0.1.0 全テスト成功
- 2026/01/04: v0.1.1 コールスタックオーバヘッド削減(current_frameを別個に保持)
