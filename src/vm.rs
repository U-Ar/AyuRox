use crate::{
    chunk::{
        Chunk, OP_ADD, OP_CONSTANT, OP_DEFINE_GLOBAL, OP_DIVIDE, OP_EQUAL, OP_FALSE, OP_GET_GLOBAL,
        OP_GREATER, OP_LESS, OP_MULTIPLY, OP_NEGATE, OP_NIL, OP_NOT, OP_POP, OP_PRINT, OP_RETURN,
        OP_SUBTRACT, OP_TRUE,
    },
    compiler::Compiler,
    debug::print_value,
    table::{GlobalVariableTable, StringTable},
    value::{ObjType, Value},
};

pub struct VM {
    pub chunk: Chunk,
    pub ip: usize,
    pub stack: Vec<Value>,
    pub strings: StringTable,
    pub globals: GlobalVariableTable,
}

pub struct RuntimeExpression {
    pub chunk: Chunk,
    pub strings: StringTable,
}

impl RuntimeExpression {
    pub fn new(chunk: Chunk, strings: StringTable) -> Self {
        RuntimeExpression { chunk, strings }
    }
}

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

pub fn interpret(source: &str) -> InterpretResult {
    let compiler = Compiler::new(source);
    if let Some(runtime_expression) = compiler.compile() {
        let mut vm = VM::new(runtime_expression);
        vm.run()
    } else {
        InterpretResult::CompileError
    }
}

fn is_falsey(value: &Value) -> bool {
    match value {
        Value::Nil => true,
        Value::Bool(b) => !*b,
        _ => false,
    }
}

impl VM {
    pub fn new(runtime_expression: RuntimeExpression) -> Self {
        VM {
            chunk: runtime_expression.chunk,
            ip: 0,
            stack: Vec::new(),
            strings: runtime_expression.strings,
            globals: GlobalVariableTable::new(),
        }
    }

    pub fn reset_stack(&mut self) {
        self.stack.clear();
    }

    pub fn runtime_error(&mut self, message: &str) {
        println!("{}", message);
        let inst_idx = self.ip - 1;
        let line = self.chunk.lines[inst_idx];
        println!("[line {}] in script", line);
        self.reset_stack();
    }

    fn peek(&self, distance: usize) -> &Value {
        let len = self.stack.len();
        &self.stack[len - 1 - distance]
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
                OP_FALSE => {
                    self.stack.push(Value::new_bool(false));
                }
                OP_NIL => {
                    self.stack.push(Value::new_nil());
                }
                OP_TRUE => {
                    self.stack.push(Value::new_bool(true));
                }
                OP_POP => {
                    self.stack.pop();
                }
                OP_GET_GLOBAL => {
                    let constant = self.read_constant();
                    if let Value::Obj(obj) = constant {
                        #[allow(irrefutable_let_patterns)]
                        if let ObjType::String(name) = &obj.obj_type {
                            if let Some(value) = self.globals.get(name) {
                                self.stack.push(value.clone());
                            } else {
                                self.runtime_error(&format!("Undefined variable '{}'.", name));
                                return InterpretResult::RuntimeError;
                            }
                        } else {
                            self.runtime_error("Invalid variable name.");
                            return InterpretResult::RuntimeError;
                        }
                    } else {
                        self.runtime_error("Invalid variable name.");
                        return InterpretResult::RuntimeError;
                    }
                }
                OP_DEFINE_GLOBAL => {
                    let constant = self.read_constant();
                    if let Value::Obj(obj) = constant {
                        #[allow(irrefutable_let_patterns)]
                        if let ObjType::String(name) = &obj.obj_type {
                            let value = self.stack.pop().unwrap();
                            self.globals.define(name, value);
                        } else {
                            self.runtime_error("Invalid variable name.");
                            return InterpretResult::RuntimeError;
                        }
                    } else {
                        self.runtime_error("Invalid variable name.");
                        return InterpretResult::RuntimeError;
                    }
                }
                OP_EQUAL => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Value::new_bool(a.is_equal(&b)));
                }
                OP_GREATER => {
                    if !self.peek(0).is_number() || !self.peek(1).is_number() {
                        self.runtime_error("Operands must be numbers.");
                        return InterpretResult::RuntimeError;
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::new_bool(a.as_number() > b.as_number()));
                }
                OP_LESS => {
                    if !self.peek(0).is_number() || !self.peek(1).is_number() {
                        self.runtime_error("Operands must be numbers.");
                        return InterpretResult::RuntimeError;
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::new_bool(a.as_number() < b.as_number()));
                }
                OP_ADD => {
                    if self.peek(0).is_obj_string() && self.peek(1).is_obj_string() {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        let result = format!("{}{}", a.as_string(), b.as_string());
                        let obj = Value::new_obj(self.strings.intern(&result));
                        self.stack.push(obj);
                        continue;
                    } else if self.peek(0).is_number() && self.peek(1).is_number() {
                        let b = self.stack.pop().unwrap();
                        let a = self.stack.pop().unwrap();
                        self.stack
                            .push(Value::new_number(a.as_number() + b.as_number()));
                    } else {
                        self.runtime_error("Operands must be two numbers or two strings.");
                        return InterpretResult::RuntimeError;
                    }
                }
                OP_SUBTRACT => {
                    if !self.peek(0).is_number() || !self.peek(1).is_number() {
                        self.runtime_error("Operands must be numbers.");
                        return InterpretResult::RuntimeError;
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::new_number(a.as_number() - b.as_number()));
                }
                OP_MULTIPLY => {
                    if !self.peek(0).is_number() || !self.peek(1).is_number() {
                        self.runtime_error("Operands must be numbers.");
                        return InterpretResult::RuntimeError;
                    }
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::new_number(a.as_number() * b.as_number()));
                }
                OP_DIVIDE => {
                    if !self.peek(0).is_number() || !self.peek(1).is_number() {
                        self.runtime_error("Operands must be numbers.");
                        return InterpretResult::RuntimeError;
                    }
                    let b = self.stack.pop().unwrap();
                    if b.as_number() == 0.0 {
                        self.runtime_error("Division by zero.");
                        return InterpretResult::RuntimeError;
                    }
                    let a = self.stack.pop().unwrap();
                    self.stack
                        .push(Value::new_number(a.as_number() / b.as_number()));
                }
                OP_NOT => {
                    let value = self.stack.pop().unwrap();
                    self.stack.push(Value::new_bool(is_falsey(&value)));
                }
                OP_NEGATE => {
                    let value = self.stack.pop().unwrap();
                    if !value.is_number() {
                        self.runtime_error("Operand must be a number.");
                        return InterpretResult::RuntimeError;
                    }
                    self.stack.push(Value::new_number(-value.as_number()));
                }
                OP_PRINT => {
                    print_value(&self.stack.pop().unwrap());
                    println!();
                }
                OP_RETURN => {
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
        self.chunk.constants.values[constant_index].clone()
    }
}
