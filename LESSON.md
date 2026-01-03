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
- 書籍ではCompiler構造体をネストして関数定義時に現在見ているchunkを入れ替えるという方式をとっていたが、所有権管理の簡略化とオーバヘッドを減らす観点から配列スタックでFunctionScope構造体を管理する方針へ変更
- 後にClassCompilerが登場した際にも同様にClassScopeスタックで管理するつもり

## ハッシュマップ

- RustのHashMapはDoS攻撃対策のためデフォで乱数をハッシュ関数に用いる
  - 変えたい場合はwith_hasherで初期化する必要がある
  - BuildHasherトレイトを実装したやつを入れる
- `HashMap<String, Gc<Obj>>`にすればとりあえず文字列インターン化の用は足せるが、キーのStringが余計にメモリを食っているので要改善

## GCを実現するためのポインタ実装

- 内部でNonNullポインタを使う
  - 参照先の所有権を持たず、また内部可変性を実現している(借用チェックしない)ためメモリ管理をこちらの責任で持つことができる
- Dropトレイトではなにもしないことで、デストラクタが呼ばれても参照先の構造体は壊れないようにする
  - 参照先の構造体を壊すタイミングはこちらで決める

## コンパイル時と実行時をまたがるGC管理

- コンパイル時にも関数や定数オブジェクトを作成してヒープをランタイムに引き継ぐので、コンパイル時にもGCを行う
- グローバルアロケータの側ではAtomicBoolでGCが必要かどうかのフラグを伝達、CompilerやVMでcollect_garbageを起動する
