use crate::value::{Value, ValueArray};

pub type OpCode = u8;
pub const OP_CONSTANT: OpCode = 0;
pub const OP_NEGATE: OpCode = 1;
pub const OP_RETURN: OpCode = 2;

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

    pub fn add_constant(&mut self, value: Value) -> u8 {
        self.constants.write(value);
        (self.constants.values.len() - 1) as u8
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}
