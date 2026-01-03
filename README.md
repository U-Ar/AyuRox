# AyuRox

## 概要

書籍 Crafting Interpreters で導入されている動的型付け言語LoxのRustによるバイトコードインタプリタ実装

## ベンチマークタイム

craftinginterpreters.com/repo に公開されているテストスイートのうち、`test/benchmark` にある各種ベンチマークの実行時間をリリースビルドで測定

- binary_trees.lox: 3.20 s
- equality.lox: 2.67 s (loop)
- fib.lox: 1.98 s
- instantiation.lox: 0.92 s
- invocation.lox: 0.77 s
- method_call.lox: 0.54 s
- properties.lox: 1.42 s
- string_equality.lox: ?
- trees.lox: 4.81 s
- zoo_batch.lox: sum=102540000, batch=1709
- zoo.lox: 0.93 s
