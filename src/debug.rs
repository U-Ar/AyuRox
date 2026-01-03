use crate::{
    chunk::{
        Chunk, OP_ADD, OP_CALL, OP_CLASS, OP_CLOSE_UPVALUE, OP_CLOSURE, OP_CONSTANT,
        OP_DEFINE_GLOBAL, OP_DIVIDE, OP_EQUAL, OP_FALSE, OP_GET_GLOBAL, OP_GET_LOCAL,
        OP_GET_PROPERTY, OP_GET_SUPER, OP_GET_UPVALUE, OP_GREATER, OP_INHERIT, OP_INVOKE, OP_JUMP,
        OP_JUMP_IF_FALSE, OP_LESS, OP_LOOP, OP_METHOD, OP_MULTIPLY, OP_NEGATE, OP_NIL, OP_NOT,
        OP_POP, OP_PRINT, OP_RETURN, OP_SET_GLOBAL, OP_SET_LOCAL, OP_SET_PROPERTY, OP_SET_UPVALUE,
        OP_SUBTRACT, OP_SUPER_INVOKE, OP_TRUE,
    },
    value::{ObjFunction, ObjType, Value},
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
            OP_GET_UPVALUE => self.byte_instruction("OP_GET_UPVALUE", offset),
            OP_SET_UPVALUE => self.byte_instruction("OP_SET_UPVALUE", offset),
            OP_GET_PROPERTY => self.constant_instruction("OP_GET_PROPERTY", offset),
            OP_SET_PROPERTY => self.constant_instruction("OP_SET_PROPERTY", offset),
            OP_GET_SUPER => self.constant_instruction("OP_GET_SUPER", offset),
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
            OP_CALL => self.byte_instruction("OP_CALL", offset),
            OP_INVOKE => self.invoke_instruction("OP_INVOKE", offset),
            OP_SUPER_INVOKE => self.invoke_instruction("OP_SUPER_INVOKE", offset),
            OP_CLOSURE => {
                let mut offset = offset + 1;
                let constant_index = self.code[offset] as usize;
                offset += 1;
                print!("{:-16} {:4} '", "OP_CLOSURE", constant_index);
                print_value(&self.constants.values[constant_index]);
                println!();

                let function = self.constants.values[constant_index].as_obj().as_function();
                for _ in 0..function.upvalue_count {
                    let is_local = self.code[offset];
                    offset += 1;
                    let index = self.code[offset];
                    offset += 1;
                    println!(
                        "{:04}      |                     {} {:4}",
                        offset - 2,
                        if is_local == 1 { "local" } else { "upvalue" },
                        index
                    );
                }
                offset
            }
            OP_CLOSE_UPVALUE => Self::simple_instruction("OP_CLOSE_UPVALUE", offset),
            OP_RETURN => Self::simple_instruction("OP_RETURN", offset),
            OP_CLASS => self.constant_instruction("OP_CLASS", offset),
            OP_INHERIT => Self::simple_instruction("OP_INHERIT", offset),
            OP_METHOD => self.constant_instruction("OP_METHOD", offset),
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

    fn invoke_instruction(&self, name: &str, offset: usize) -> usize {
        let constant_index = self.code[offset + 1] as usize;
        let arg_count = self.code[offset + 2];
        print!(
            "{:-16} ({} args) {:4} '{:?}'",
            name, arg_count, constant_index, self.constants.values[constant_index],
        );
        println!();
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
            ObjType::Function(f) => print_function(f),
            ObjType::Native(_) => print!("<native fn>"),
            ObjType::Closure(c) => {
                let function = c.function.as_function();
                print_function(function);
            }
            ObjType::Upvalue(_) => {
                print!("upvalue")
            }
            ObjType::Class(class) => {
                print!("class {}", class.name.as_string())
            }
            ObjType::Instance(instance) => {
                print!("instance of {}", instance.class.as_class().name.as_string())
            }
            ObjType::BoundMethod(bound_method) => {
                let function = bound_method.method.as_closure().function.as_function();
                print_function(function);
            }
        },
    }
}

pub fn print_function(function: &ObjFunction) {
    if let Some(name) = &function.name {
        print!("<fn {}>", name.as_string());
    } else {
        print!("<script>");
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
        self.current_chunk
            .disassemble_instruction(self.frames.last().unwrap().ip);
    }

    #[cfg(not(feature = "debug_trace_execution"))]
    #[inline(always)]
    pub fn debug_trace_execution(&self) {
        // No-op when debug tracing is disabled
    }
}
