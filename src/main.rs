use std::sync::atomic::Ordering;

use ayurox::{
    chunk::{Chunk, OP_ADD, OP_CONSTANT, OP_DIVIDE, OP_NEGATE, OP_RETURN},
    memory::ALLOCATED,
    vm::VM,
};

fn main() {
    let mut vm = VM::new();

    println!("Hello, world!");

    let mut chunk = Chunk::new();

    let constant = chunk.add_constant(1.2);
    chunk.write(OP_CONSTANT, 123);
    chunk.write(constant, 123);

    let constant = chunk.add_constant(3.4);
    chunk.write(OP_CONSTANT, 123);
    chunk.write(constant, 123);

    chunk.write(OP_ADD, 123);

    let constant = chunk.add_constant(5.6);
    chunk.write(OP_CONSTANT, 123);
    chunk.write(constant, 123);

    chunk.write(OP_DIVIDE, 123);

    chunk.write(OP_NEGATE, 123);

    chunk.write(OP_RETURN, 123);

    chunk.disassemble("test chunk");

    vm.interpret(chunk);

    println!(
        "Allocated memory: {} bytes",
        ALLOCATED.load(Ordering::SeqCst)
    );
}
