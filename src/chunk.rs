pub type OpCode = u8;
pub const OP_RETURN: OpCode = 0;

pub struct Chunk {
    pub code: Vec<u8>,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk { code: Vec::new() }
    }

    pub fn write(&mut self, byte: u8) {
        self.code.push(byte);
    }
}

impl Default for Chunk {
    fn default() -> Self {
        Self::new()
    }
}
