# 学び


## 所有権管理

- 構造体のフィールドを外部に明け渡したい場合、引数のselfを参照ではなく実体にすることでselfそのものをムーブする
  - 元の構造体は使えなくなるので覚悟

```rust 
pub fn compile(mut self) -> Option<Box<Chunk>> {
    self.parser.advance();
    self.expression();
    self.parser
        .consume(TokenType::Eof, "Expect end of expression.");
    self.end_compiler();
    Some(self.chunk)
}
```


## ハッシュマップ

- RustのHashMapはDoS攻撃対策のためデフォで乱数をハッシュ関数に用いる
  - 変えたい場合はwith_hasherで初期化する必要がある
  - BuildHasherトレイトを実装したやつを入れる

## GCを実現するためのポインタ実装

- 内部でNonNullポインタを使う
  - 参照先の所有権を持たず、また内部可変性を実現している(借用チェックしない)ためメモリ管理をこちらの責任で持つことができる
- Dropトレイトを実装して好きなタイミングでdropできるようにする