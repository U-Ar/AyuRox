use crate::{
    chunk::{Chunk, OP_CONSTANT, OP_NEGATE, OP_RETURN},
    value::Value,
};

pub struct VM {
    pub chunk: Chunk,
    pub ip: usize,
    pub stack: Vec<Value>,
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
            stack: Vec::new(),
        }
    }

    pub fn reset_stack(&mut self) {
        self.stack.clear();
    }

    pub fn interpret(&mut self, chunk: Chunk) {
        self.chunk = chunk;
        self.ip = 0;
        self.run();
    }

    fn run(&mut self) -> InterpretResult {
        loop {
            self.debug_trace_execution();

            let instruction = self.read_byte();
            match instruction {
                OP_CONSTANT => {
                    let constant = self.read_constant();
                    self.stack.push(constant);
                }
                OP_NEGATE => {
                    let value = self.stack.pop().unwrap();
                    self.stack.push(-value);
                }
                OP_RETURN => {
                    println!("{}", self.stack.pop().unwrap());
                    return InterpretResult::Ok;
                }
                _ => {
                    println!("Unknown opcode {}", instruction);
                    return InterpretResult::RuntimeError;
                }
            }
        }
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
