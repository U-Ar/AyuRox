use crate::{
    chunk::{
        Chunk, OP_ADD, OP_CONSTANT, OP_DEFINE_GLOBAL, OP_DIVIDE, OP_EQUAL, OP_FALSE, OP_GET_GLOBAL,
        OP_GET_LOCAL, OP_GREATER, OP_JUMP, OP_JUMP_IF_FALSE, OP_LESS, OP_LOOP, OP_MULTIPLY,
        OP_NEGATE, OP_NIL, OP_NOT, OP_POP, OP_PRINT, OP_RETURN, OP_SET_GLOBAL, OP_SET_LOCAL,
        OP_SUBTRACT, OP_TRUE,
    },
    value::{ObjType, Value},
    vm::VM,
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
            OP_FALSE => Self::simple_instruction("OP_FALSE", offset),
            OP_NIL => Self::simple_instruction("OP_NIL", offset),
            OP_TRUE => Self::simple_instruction("OP_TRUE", offset),
            OP_POP => Self::simple_instruction("OP_POP", offset),
            OP_GET_LOCAL => self.byte_instruction("OP_GET_LOCAL", offset),
            OP_SET_LOCAL => self.byte_instruction("OP_SET_LOCAL", offset),
            OP_GET_GLOBAL => self.constant_instruction("OP_GET_GLOBAL", offset),
            OP_DEFINE_GLOBAL => self.constant_instruction("OP_DEFINE_GLOBAL", offset),
            OP_SET_GLOBAL => self.constant_instruction("OP_SET_GLOBAL", offset),
            OP_EQUAL => Self::simple_instruction("OP_EQUAL", offset),
            OP_GREATER => Self::simple_instruction("OP_GREATER", offset),
            OP_LESS => Self::simple_instruction("OP_LESS", offset),
            OP_ADD => Self::simple_instruction("OP_ADD", offset),
            OP_SUBTRACT => Self::simple_instruction("OP_SUBTRACT", offset),
            OP_MULTIPLY => Self::simple_instruction("OP_MULTIPLY", offset),
            OP_DIVIDE => Self::simple_instruction("OP_DIVIDE", offset),
            OP_NOT => Self::simple_instruction("OP_NOT", offset),
            OP_NEGATE => Self::simple_instruction("OP_NEGATE", offset),
            OP_PRINT => Self::simple_instruction("OP_PRINT", offset),
            OP_JUMP => self.jump_instruction("OP_JUMP", 1, offset),
            OP_JUMP_IF_FALSE => self.jump_instruction("OP_JUMP_IF_FALSE", 1, offset),
            OP_LOOP => self.jump_instruction("OP_LOOP", -1, offset),
            OP_RETURN => Self::simple_instruction("OP_RETURN", offset),
            _ => {
                println!("Unknown opcode {}", self.code[offset]);
                offset + 1
            }
        }
    }

    fn simple_instruction(name: &str, offset: usize) -> usize {
        println!("{name}");
        offset + 1
    }

    fn constant_instruction(&self, name: &str, offset: usize) -> usize {
        let constant_index = self.code[offset + 1] as usize;
        print!("{:-16} {:4} '", name, constant_index);
        print_value(&self.constants.values[constant_index]);
        println!("'");
        offset + 2
    }

    fn byte_instruction(&self, name: &str, offset: usize) -> usize {
        let slot = self.code[offset + 1];
        println!("{:-16} {:4}", name, slot);
        offset + 2
    }

    fn jump_instruction(&self, name: &str, sign: i32, offset: usize) -> usize {
        let jump = ((self.code[offset + 1] as usize) << 8) | (self.code[offset + 2] as usize);
        println!(
            "{:-16} {:4} -> {}",
            name,
            offset,
            (offset as i32) + 3 + (sign * (jump as i32))
        );
        offset + 3
    }
}

pub fn print_value(value: &Value) {
    match value {
        Value::Bool(b) => print!("{b}"),
        Value::Nil => print!("nil"),
        Value::Number(n) => print!("{n}"),
        Value::Obj(obj) => match &obj.obj_type {
            ObjType::String(s) => print!("{s}"),
        },
    }
}

impl VM {
    #[cfg(feature = "debug_trace_execution")]
    #[inline(always)]
    pub fn debug_trace_execution(&self) {
        print!("          ");
        for slot in &self.stack {
            print!("[ ");
            print_value(slot);
            print!(" ]");
        }
        println!();
        self.chunk.disassemble_instruction(self.ip);
    }

    #[cfg(not(feature = "debug_trace_execution"))]
    #[inline(always)]
    pub fn debug_trace_execution(&self) {
        // No-op when debug tracing is disabled
    }
}
