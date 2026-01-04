# AyuRox

## 概要

書籍 Crafting Interpreters で導入されている動的型付け言語LoxのRustによるバイトコードインタプリタ実装

## ベンチマークタイム

craftinginterpreters.com/repo に公開されているテストスイートのうち、`test/benchmark` にある各種ベンチマークの実行時間をリリースビルドで測定

| execution time (s) | reference(clox with nan boxing) | v0.1.0 | v0.1.1 | v0.1.2 |
| - | - | - | - | - |
| binary_trees.lox | 0.86 | 3.20 | 3.04 | 2.86 |
| equality.lox(loop) | 0.70 | 2.67 | 2.49 | 2.48 |
| fib.lox | 0.49 | 1.98 | 1.94 | 1.82 |
| instantiation.lox | error | 0.92 | 0.92 | 0.86 |
| invocation.lox | 0.12 | 0.77 | 0.73 | 0.69 |
| method_call.lox | 0.09 | 0.54 | 0.50 | 0.48 |
| properties.lox | 0.17 | 1.42 | 1.42 | 1.30 |
| string_equality.lox | error | 1.24 | 1.07 | 1.02 |
| trees.lox | 0.95 | 4.81 | 4.50 | 4.24 |
| zoo.lox | 0.13 | 0.93 | 0.87 | 0.86 |

- zoo_batch.lox
  - v0.1.0: sum=102540000, batch=1709
  - v0.1.1: sum=109980000, batch=1833
  - v0.1.2: sum=111360000, batch=1856

## CHANGELOG

- 2026/01/04: v0.1.0 全テスト成功
- 2026/01/04: v0.1.1 コールスタックオーバヘッド削減(current_frameを別個に保持)
- 2026/01/04: v0.1.2 バイトコード読み込み時の境界チェック排除
