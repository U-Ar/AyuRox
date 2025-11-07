use std::sync::atomic::Ordering;

use ayurox::{
    chunk::{Chunk, OP_CONSTANT, OP_RETURN},
    memory::ALLOCATED,
};

fn main() {
    println!("Hello, world!");

    let mut chunk = Chunk::new();

    let constant = chunk.add_constant(1.2);
    chunk.write(OP_CONSTANT);
    chunk.write(constant);

    chunk.write(OP_RETURN);

    chunk.disassemble("test chunk");

    println!(
        "Allocated memory: {} bytes",
        ALLOCATED.load(Ordering::SeqCst)
    );
}
