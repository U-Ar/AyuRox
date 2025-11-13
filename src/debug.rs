use crate::{
    chunk::{Chunk, OP_CONSTANT, OP_RETURN},
    value::Value,
};

impl Chunk {
    pub fn disassemble(&self, name: &str) {
        println!("== {name} ==");

        let mut offset = 0;
        while offset < self.code.len() {
            offset = self.disassemble_instruction(offset);
        }
    }

    pub fn disassemble_instruction(&self, offset: usize) -> usize {
        print!("{:04} ", offset);
        if offset > 0 && self.lines[offset] == self.lines[offset - 1] {
            print!("   | ");
        } else {
            print!("{:4} ", self.lines[offset]);
        }

        match self.code[offset] {
            OP_CONSTANT => self.constant_instruction("OP_CONSTANT", offset),
            OP_RETURN => Self::simple_instruction("OP_RETURN", offset),
            _ => {
                println!("Unknown opcode {}", self.code[offset]);
                offset + 1
            }
        }
    }

    #[cfg(feature = "debug_trace_execution")]
    #[inline(always)]
    pub fn debug_trace_execution(&self, offset: usize) {
        self.disassemble_instruction(offset);
    }

    #[cfg(not(feature = "debug_trace_execution"))]
    #[inline(always)]
    pub fn debug_trace_execution(&self, _offset: usize) {
        // No-op when debug tracing is disabled
    }

    fn simple_instruction(name: &str, offset: usize) -> usize {
        println!("{name}");
        offset + 1
    }

    fn constant_instruction(&self, name: &str, offset: usize) -> usize {
        let constant_index = self.code[offset + 1] as usize;
        print!("{:-16} {:4} '", name, constant_index);
        print_value(self.constants.values[constant_index]);
        println!("'");
        offset + 2
    }
}

fn print_value(value: Value) {
    print!("{value}");
}
