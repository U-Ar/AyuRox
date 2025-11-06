use std::sync::atomic::Ordering;

use ayurox::{
    chunk::{Chunk, OP_RETURN},
    memory::ALLOCATED,
};

fn main() {
    println!("Hello, world!");

    let mut chunk = Chunk::new();
    chunk.write(OP_RETURN);

    chunk.disassemble("test chunk");

    println!(
        "Allocated memory: {} bytes",
        ALLOCATED.load(Ordering::SeqCst)
    );
}
