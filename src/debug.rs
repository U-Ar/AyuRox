use crate::chunk::{Chunk, OP_RETURN};

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

        match self.code[offset] {
            OP_RETURN => simple_instruction("OP_RETURN", offset),
            _ => {
                println!("Unknown opcode {}", self.code[offset]);
                offset + 1
            }
        }
    }
}

fn simple_instruction(name: &str, offset: usize) -> usize {
    println!("{name}");
    offset + 1
}
