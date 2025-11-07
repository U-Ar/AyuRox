use crate::value::{Value, ValueArray};

pub type OpCode = u8;
pub const OP_CONSTANT: OpCode = 0;
pub const OP_RETURN: OpCode = 1;

pub struct Chunk {
    pub code: Vec<u8>,
    pub constants: ValueArray,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: Vec::new(),
            constants: ValueArray::new(),
        }
    }

    pub fn write(&mut self, byte: u8) {
        self.code.push(byte);
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
