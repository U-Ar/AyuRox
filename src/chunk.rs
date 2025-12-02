use crate::value::{Value, ValueArray};

pub type OpCode = u8;
pub const OP_CONSTANT: OpCode = 0;
pub const OP_NIL: OpCode = 1;
pub const OP_TRUE: OpCode = 2;
pub const OP_FALSE: OpCode = 3;
pub const OP_EQUAL: OpCode = 4;
pub const OP_GREATER: OpCode = 5;
pub const OP_LESS: OpCode = 6;
pub const OP_ADD: OpCode = 7;
pub const OP_SUBTRACT: OpCode = 8;
pub const OP_MULTIPLY: OpCode = 9;
pub const OP_DIVIDE: OpCode = 10;
pub const OP_NOT: OpCode = 11;
pub const OP_NEGATE: OpCode = 12;
pub const OP_RETURN: OpCode = 13;

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
