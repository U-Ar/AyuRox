use crate::{
    chunk::{Chunk, OP_CONSTANT, OP_RETURN},
    value::Value,
};

pub struct VM {
    chunk: Chunk,
    ip: usize,
}

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

impl VM {
    pub fn new() -> Self {
        VM {
            chunk: Chunk::new(),
            ip: 0,
        }
    }

    pub fn interpret(&mut self, chunk: Chunk) {
        self.chunk = chunk;
        self.ip = 0;
        self.run();
    }

    fn run(&mut self) -> InterpretResult {
        loop {
            let instruction = self.read_byte();
            match instruction {
                OP_CONSTANT => {
                    let constant = self.read_constant();
                    println!("{}", constant);
                    break;
                }
                OP_RETURN => {
                    return InterpretResult::Ok;
                }
                _ => {
                    println!("Unknown opcode {}", instruction);
                    // return InterpretResult::RuntimeError;
                }
            }
        }
        InterpretResult::Ok
    }

    fn read_byte(&mut self) -> u8 {
        let byte = self.chunk.code[self.ip];
        self.ip += 1;
        byte
    }

    fn read_constant(&mut self) -> Value {
        let constant_index = self.read_byte() as usize;
        self.chunk.constants.values[constant_index]
    }
}

impl Default for VM {
    fn default() -> Self {
        Self::new()
    }
}
