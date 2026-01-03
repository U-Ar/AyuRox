use std::{ops::DerefMut, sync::atomic::Ordering, vec};

use crate::{
    chunk::{
        Chunk, OP_ADD, OP_CALL, OP_CLOSE_UPVALUE, OP_CLOSURE, OP_CONSTANT, OP_DEFINE_GLOBAL,
        OP_DIVIDE, OP_EQUAL, OP_FALSE, OP_GET_GLOBAL, OP_GET_LOCAL, OP_GET_UPVALUE, OP_GREATER,
        OP_JUMP, OP_JUMP_IF_FALSE, OP_LESS, OP_LOOP, OP_MULTIPLY, OP_NEGATE, OP_NIL, OP_NOT,
        OP_POP, OP_PRINT, OP_RETURN, OP_SET_GLOBAL, OP_SET_LOCAL, OP_SET_UPVALUE, OP_SUBTRACT,
        OP_TRUE,
    },
    compiler::Compiler,
    debug::print_value,
    memory::{
        ALLOCATED, GC_HEAP_GROW_FACTOR, GC_REQUESTED, Gc, NEXT_GC, mark_global_table, mark_object,
        mark_value, remove_white_strings, sweep, trace_reference,
    },
    table::{GlobalVariableTable, StringTable},
    value::{Obj, ObjClosure, ObjFunction, ObjType, ObjUpvalue, Value},
};

pub struct VM {
    pub current_chunk: Gc<Chunk>,
    pub frames: Vec<CallFrame>,
    pub stack: Vec<Value>,
    pub strings: StringTable,
    pub globals: GlobalVariableTable,
    pub open_upvalues: Option<Gc<Obj>>,
    pub objects: Option<Gc<Obj>>,
}

pub struct CallFrame {
    pub closure: Gc<Obj>,
    pub ip: usize,
    pub slot_start: usize,
}

pub struct RuntimeExpression {
    pub function: ObjFunction,
    pub strings: StringTable,
    pub objects: Option<Gc<Obj>>,
}

impl RuntimeExpression {
    pub fn new(function: ObjFunction, strings: StringTable, objects: Option<Gc<Obj>>) -> Self {
        RuntimeExpression {
            function,
            strings,
            objects,
        }
    }
}

pub enum InterpretResult {
    Ok,
    CompileError,
    RuntimeError,
}

pub fn interpret(source: &str) -> InterpretResult {
    let compiler = Compiler::new(source);

    let mut gc_gray_stack = Vec::new();

    if let Some(runtime_expression) = compiler.compile(&mut gc_gray_stack) {
        let mut vm = VM::new(runtime_expression);
        vm.define_native("clock", clock_native);
        vm.run(&mut gc_gray_stack)
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

fn clock_native(_arg_count: usize, _args: &Vec<Value>) -> Value {
    use std::time::{SystemTime, UNIX_EPOCH};
    let start = SystemTime::now();
    let since_the_epoch = start
        .duration_since(UNIX_EPOCH)
        .expect("Time went backwards");
    Value::new_number(since_the_epoch.as_secs_f64())
}

impl VM {
    pub fn new(runtime_expression: RuntimeExpression) -> Self {
        let chunk_ptr = runtime_expression.function.chunk.clone();

        let objects = runtime_expression.objects;

        let mut function_ptr = Gc::new(Obj::new_function(runtime_expression.function.clone()));
        function_ptr.next = objects;

        let mut closure_ptr = Gc::new(Obj::new_closure(ObjClosure {
            function: function_ptr.clone(),
            upvalues: Vec::new(),
        }));
        closure_ptr.next = Some(function_ptr);

        let frames = vec![CallFrame {
            closure: closure_ptr.clone(),
            ip: 0,
            slot_start: 0,
        }];

        let stack = vec![Value::new_obj(closure_ptr.clone())];

        VM {
            current_chunk: chunk_ptr,
            frames,
            stack,
            strings: runtime_expression.strings,
            globals: GlobalVariableTable::new(),
            open_upvalues: None,
            objects: Some(closure_ptr),
        }
    }

    pub fn new_managed_obj(&mut self, mut obj: Obj) -> Gc<Obj> {
        obj.next = self.objects.clone();
        let gc_obj = Gc::new(obj);
        self.objects = Some(gc_obj.clone());
        gc_obj
    }

    pub fn reset_stack(&mut self) {
        self.stack.clear();
    }

    pub fn runtime_error(&mut self, message: &str) {
        println!("{}", message);
        let frame = self.frames.last().unwrap();
        let inst_idx = frame.ip - 1;
        let line = self.current_chunk.lines[inst_idx];
        println!("[line {}] in script", line);

        for frame in self.frames.iter().rev() {
            let closure = &frame.closure;
            let function_name =
                if let Some(name) = &closure.as_closure().function.as_function().name {
                    name.as_string().clone()
                } else {
                    "<script>".to_string()
                };
            let inst_idx = frame.ip - 1;
            let line = closure.as_closure().function.as_function().chunk.lines[inst_idx];
            println!("[line {}] in {}", line, function_name);
        }

        self.reset_stack();
    }

    pub fn define_native(&mut self, name: &str, function: fn(usize, &Vec<Value>) -> Value) {
        let native = self.new_managed_obj(Obj::new_native(function));
        self.stack.push(Value::new_obj(native));
        self.globals.define(name, self.stack.pop().unwrap());
    }

    fn peek(&self, distance: usize) -> &Value {
        let len = self.stack.len();
        &self.stack[len - 1 - distance]
    }

    fn call_value(&mut self, callee: Value, arg_count: usize) -> bool {
        if let Value::Obj(obj) = callee {
            match &obj.obj_type {
                ObjType::Native(native) => {
                    let args_start = self.stack.len() - arg_count;
                    let result = native(arg_count, &self.stack);
                    self.stack.truncate(args_start - 1);
                    self.stack.push(result);
                    return true;
                }
                ObjType::Closure(closure) => {
                    return self.call(closure, arg_count);
                }
                _ => {}
            }
        }
        self.runtime_error("Can only call functions.");
        false
    }

    fn call(&mut self, closure: &ObjClosure, arg_count: usize) -> bool {
        let function = closure.function.as_function();

        if arg_count != function.arity {
            self.runtime_error(&format!(
                "Expected {} arguments but got {}.",
                function.arity, arg_count
            ));
            return false;
        }

        self.current_chunk = function.chunk.clone();
        let frame = CallFrame {
            closure: self.new_managed_obj(Obj::new_closure(closure.clone())),
            ip: 0,
            slot_start: self.stack.len() - arg_count - 1,
        };
        self.frames.push(frame);
        true
    }

    fn capture_upvalue(&mut self, local_index: u8) -> Gc<Obj> {
        let stack_index = self.frames.last().unwrap().slot_start + (local_index as usize);

        let mut prev_upvalue = None;
        let mut upvalue = self.open_upvalues.clone();
        while upvalue.is_some()
            && upvalue.as_ref().unwrap().as_upvalue().location.unwrap() > stack_index
        {
            let next_upvalue = upvalue.as_ref().unwrap().as_upvalue().next.clone();
            prev_upvalue = upvalue;
            upvalue = next_upvalue;
        }

        if let Some(existing_upvalue) = upvalue.as_ref()
            && existing_upvalue.as_upvalue().location.unwrap() == stack_index
        {
            return upvalue.unwrap();
        }

        let created_upvalue = self.new_managed_obj(Obj::new_upvalue(ObjUpvalue {
            location: Some(stack_index),
            closed: None,
            next: upvalue,
        }));

        if let Some(prev) = &mut prev_upvalue {
            prev.as_upvalue_mut().next = Some(created_upvalue.clone());
        } else {
            self.open_upvalues = Some(created_upvalue.clone());
        }

        created_upvalue
    }

    fn close_upvalues(&mut self, last_index: usize) {
        while let Some(upvalue) = &mut self.open_upvalues {
            if upvalue.as_upvalue().location.unwrap() >= last_index {
                let closed_value = self.stack[upvalue.as_upvalue().location.unwrap()].clone();
                upvalue.as_upvalue_mut().closed = Some(closed_value);
                upvalue.as_upvalue_mut().location = None;
                self.open_upvalues = upvalue.as_upvalue().next.clone();
            } else {
                break;
            }
        }
    }

    fn run(&mut self, gc_gray_stack: &mut Vec<Gc<Obj>>) -> InterpretResult {
        loop {
            self.debug_trace_execution();
            self.safe_point(gc_gray_stack);

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
                OP_GET_LOCAL => {
                    let slot = self.read_byte() as usize;
                    let value = self.stack[self.frames.last().unwrap().slot_start + slot].clone();
                    self.stack.push(value);
                }
                OP_SET_LOCAL => {
                    let slot = self.read_byte() as usize;
                    let value = self.peek(0).clone();
                    self.stack[self.frames.last().unwrap().slot_start + slot] = value;
                }
                OP_GET_GLOBAL => {
                    let constant = self.read_constant();
                    if let Value::Obj(obj) = constant {
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
                OP_SET_GLOBAL => {
                    let constant = self.read_constant();
                    if let Value::Obj(obj) = constant {
                        if let ObjType::String(name) = &obj.obj_type {
                            let value = self.stack.pop().unwrap();
                            if !self.globals.set(name, value) {
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
                OP_GET_UPVALUE => {
                    let slot = self.read_byte() as usize;
                    let value = if let Some(location) =
                        self.frames.last().unwrap().closure.as_closure().upvalues[slot]
                            .as_upvalue()
                            .location
                    {
                        self.stack[location].clone()
                    } else {
                        self.frames.last().unwrap().closure.as_closure().upvalues[slot]
                            .as_upvalue()
                            .closed
                            .as_ref()
                            .unwrap()
                            .clone()
                    };
                    self.stack.push(value);
                }
                OP_SET_UPVALUE => {
                    let slot = self.read_byte() as usize;
                    if let Obj {
                        obj_type: ObjType::Closure(closure),
                        ..
                    } = self.frames.last_mut().unwrap().closure.deref_mut()
                    {
                        let upvalue = &mut closure.upvalues[slot];
                        if let Obj {
                            obj_type: ObjType::Upvalue(upvalue),
                            ..
                        } = upvalue.deref_mut()
                        {
                            upvalue.location = Some(self.stack.len() - 1);
                        }
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

                        let string = if let Some(interned) = self.strings.get(&result) {
                            interned.clone()
                        } else {
                            let new_string = self.new_managed_obj(Obj::new_string(result.clone()));
                            self.strings.insert(result, new_string.clone());
                            new_string
                        };

                        let obj = Value::new_obj(string);
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
                OP_JUMP => {
                    let offset = self.read_short() as usize;
                    self.frames.last_mut().unwrap().ip += offset;
                }
                OP_JUMP_IF_FALSE => {
                    let offset = self.read_short() as usize;
                    if is_falsey(self.peek(0)) {
                        self.frames.last_mut().unwrap().ip += offset;
                    }
                }
                OP_LOOP => {
                    let offset = self.read_short() as usize;
                    self.frames.last_mut().unwrap().ip -= offset;
                }
                OP_CALL => {
                    let arg_count = self.read_byte() as usize;
                    if !self.call_value(self.peek(arg_count).clone(), arg_count) {
                        return InterpretResult::RuntimeError;
                    }
                }
                OP_CLOSURE => {
                    let constant = self.read_constant();

                    if let Value::Obj(obj) = constant
                        && let ObjType::Function(_) = &obj.obj_type
                    {
                        let mut upvalues = Vec::new();
                        for _i in 0..obj.as_function().upvalue_count {
                            let is_local = self.read_byte();
                            let index = self.read_byte();
                            if is_local == 1 {
                                upvalues.push(self.capture_upvalue(index));
                            } else {
                                upvalues.push(
                                    self.frames.last().unwrap().closure.as_closure().upvalues
                                        [index as usize]
                                        .clone(),
                                );
                            }
                        }
                        let closure = self.new_managed_obj(Obj::new_closure(ObjClosure {
                            function: obj,
                            upvalues,
                        }));

                        self.stack.push(Value::new_obj(closure));
                    } else {
                        self.runtime_error("Expected function for closure.");
                        return InterpretResult::RuntimeError;
                    }
                }
                OP_CLOSE_UPVALUE => {
                    self.close_upvalues(self.stack.len() - 1);
                    self.stack.pop();
                }
                OP_RETURN => {
                    let result = self.stack.pop().unwrap();
                    self.close_upvalues(self.frames.last().unwrap().slot_start);
                    let frame = self.frames.pop().unwrap();
                    if self.frames.is_empty() {
                        self.stack.pop();
                        return InterpretResult::Ok;
                    }
                    self.stack.truncate(frame.slot_start);
                    self.stack.push(result);

                    let frame = self.frames.last().unwrap();
                    self.current_chunk = frame
                        .closure
                        .as_closure()
                        .function
                        .as_function()
                        .chunk
                        .clone();
                }
                _ => {
                    println!("Unknown opcode {}", instruction);
                    return InterpretResult::RuntimeError;
                }
            }
        }
    }

    fn read_byte(&mut self) -> u8 {
        let frame = self.frames.last_mut().unwrap();
        let byte = self.current_chunk.code[frame.ip];
        frame.ip += 1;
        byte
    }

    fn read_short(&mut self) -> u16 {
        let frame = self.frames.last_mut().unwrap();
        frame.ip += 2;
        ((self.current_chunk.code[frame.ip - 2] as u16) << 8)
            | (self.current_chunk.code[frame.ip - 1] as u16)
    }

    fn read_constant(&mut self) -> Value {
        let constant_index = self.read_byte() as usize;
        self.current_chunk.constants.values[constant_index].clone()
    }

    // --- GC methods ---
    pub fn safe_point(&mut self, gc_gray_stack: &mut Vec<Gc<Obj>>) {
        if GC_REQUESTED.swap(false, Ordering::Relaxed) {
            self.collect_garbage(gc_gray_stack);
        }
    }
    fn collect_garbage(&mut self, gc_gray_stack: &mut Vec<Gc<Obj>>) {
        #[cfg(feature = "debug_log_gc")]
        let before = {
            println!("-- gc begin");
            ALLOCATED.load(Ordering::Relaxed)
        };

        self.mark_roots(gc_gray_stack);
        trace_reference(gc_gray_stack);
        remove_white_strings(&mut self.strings);
        sweep(&mut self.objects);

        let next_gc = ALLOCATED.load(Ordering::Relaxed) * GC_HEAP_GROW_FACTOR;
        NEXT_GC.store(next_gc, Ordering::Relaxed);

        #[cfg(feature = "debug_log_gc")]
        {
            println!("-- gc end");
            let after = ALLOCATED.load(Ordering::Relaxed);
            println!(
                "   collected {} bytes (from {} to {}) next at {}",
                before.saturating_sub(after),
                before,
                after,
                next_gc
            );
        }
    }
    fn mark_roots(&mut self, gc_gray_stack: &mut Vec<Gc<Obj>>) {
        for value in &mut self.stack {
            mark_value(value, gc_gray_stack);
        }

        for frame in &mut self.frames {
            mark_object(frame.closure.clone(), gc_gray_stack);
        }

        if let Some(upvalue) = &self.open_upvalues {
            mark_object(upvalue.clone(), gc_gray_stack);
        }

        mark_global_table(&mut self.globals, gc_gray_stack);
    }
}
