use crate::value::{Value, ValueArray};

pub type OpCode = u8;
pub const OP_CONSTANT: OpCode = 0;
pub const OP_NIL: OpCode = 1;
pub const OP_TRUE: OpCode = 2;
pub const OP_FALSE: OpCode = 3;
pub const OP_POP: OpCode = 4;
pub const OP_GET_LOCAL: OpCode = 5;
pub const OP_SET_LOCAL: OpCode = 6;
pub const OP_GET_GLOBAL: OpCode = 7;
pub const OP_DEFINE_GLOBAL: OpCode = 8;
pub const OP_SET_GLOBAL: OpCode = 9;
pub const OP_GET_UPVALUE: OpCode = 10;
pub const OP_SET_UPVALUE: OpCode = 11;
pub const OP_EQUAL: OpCode = 12;
pub const OP_GREATER: OpCode = 13;
pub const OP_LESS: OpCode = 14;
pub const OP_ADD: OpCode = 15;
pub const OP_SUBTRACT: OpCode = 16;
pub const OP_MULTIPLY: OpCode = 17;
pub const OP_DIVIDE: OpCode = 18;
pub const OP_NOT: OpCode = 19;
pub const OP_NEGATE: OpCode = 20;
pub const OP_PRINT: OpCode = 21;
pub const OP_JUMP: OpCode = 22;
pub const OP_JUMP_IF_FALSE: OpCode = 23;
pub const OP_LOOP: OpCode = 24;
pub const OP_CALL: OpCode = 25;
pub const OP_CLOSURE: OpCode = 26;
pub const OP_CLOSE_UPVALUE: OpCode = 27;
pub const OP_RETURN: OpCode = 28;

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<u8>,
    pub lines: Vec<usize>,
    pub constants: ValueArray,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: Vec::new(),
            lines: Vec::new(),
            constants: ValueArray::new(),
        }
    }

    pub fn write(&mut self, byte: u8, line: usize) {
        self.code.push(byte);
        self.lines.push(line);
    }

    pub fn add_constant(&mut self, value: Value) -> usize {
        self.constants.write(value);
        self.constants.values.len() - 1
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}
