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