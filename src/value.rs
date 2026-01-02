use crate::{chunk::Chunk, memory::Gc};

#[derive(Clone)]
pub enum Value {
    Bool(bool),
    Nil,
    Number(f64),
    Obj(Gc<Obj>),
}

#[derive(Clone)]
pub struct Obj {
    pub obj_type: ObjType,
    pub next: Option<Gc<Obj>>,
}

#[derive(Clone)]
pub enum ObjType {
    String(String),
    Function(ObjFunction),
    Native(NativeFunction),
    Closure(ObjClosure),
}

#[derive(Clone)]
pub struct ObjFunction {
    pub arity: usize,
    pub chunk: Gc<Chunk>,
    pub name: Option<Gc<Obj>>,
}

#[derive(Clone)]
pub struct ObjClosure {
    pub function: Gc<Obj>,
}

#[derive(Clone, PartialEq, Eq)]
pub enum FunctionType {
    Function,
    Script,
}

type NativeFunction = fn(usize, &Vec<Value>) -> Value;

impl Value {
    pub fn new_bool(b: bool) -> Self {
        Value::Bool(b)
    }
    pub fn new_nil() -> Self {
        Value::Nil
    }
    pub fn new_number(n: f64) -> Self {
        Value::Number(n)
    }
    pub fn new_obj(obj: Gc<Obj>) -> Self {
        Value::Obj(obj)
    }
    pub fn is_bool(&self) -> bool {
        matches!(self, Value::Bool(_))
    }
    pub fn is_nil(&self) -> bool {
        matches!(self, Value::Nil)
    }
    pub fn is_number(&self) -> bool {
        matches!(self, Value::Number(_))
    }
    pub fn is_obj(&self) -> bool {
        matches!(self, Value::Obj(_))
    }
    pub fn is_obj_string(&self) -> bool {
        match self {
            Value::Obj(obj) => matches!(obj.obj_type, ObjType::String(_)),
            _ => false,
        }
    }
    pub fn is_obj_function(&self) -> bool {
        match self {
            Value::Obj(obj) => matches!(obj.obj_type, ObjType::Function(_)),
            _ => false,
        }
    }
    pub fn is_obj_closure(&self) -> bool {
        match self {
            Value::Obj(obj) => matches!(obj.obj_type, ObjType::Closure(_)),
            _ => false,
        }
    }
    pub fn is_obj_native(&self) -> bool {
        match self {
            Value::Obj(obj) => matches!(obj.obj_type, ObjType::Native(_)),
            _ => false,
        }
    }
    pub fn is_obj_type(&self, obj_type: &ObjType) -> bool {
        match self {
            Value::Obj(obj) => {
                std::mem::discriminant(&obj.obj_type) == std::mem::discriminant(obj_type)
            }
            _ => false,
        }
    }
    pub fn obj_type(&self) -> Option<&ObjType> {
        match self {
            Value::Obj(obj) => Some(&obj.obj_type),
            _ => None,
        }
    }
    pub fn as_bool(&self) -> bool {
        if let Value::Bool(b) = self {
            *b
        } else {
            panic!("Value is not a bool");
        }
    }
    pub fn as_number(&self) -> f64 {
        if let Value::Number(n) = self {
            *n
        } else {
            panic!("Value is not a number");
        }
    }
    pub fn as_obj(&self) -> &Obj {
        if let Value::Obj(obj) = self {
            obj
        } else {
            panic!("Value is not an object");
        }
    }
    pub fn as_string(&self) -> &String {
        if let Value::Obj(obj) = self {
            if let ObjType::String(s) = &obj.obj_type {
                s
            } else {
                panic!("Object is not a string");
            }
        } else {
            panic!("Value is not an object");
        }
    }
    pub fn as_function(&self) -> &ObjFunction {
        if let Value::Obj(obj) = self {
            if let ObjType::Function(f) = &obj.obj_type {
                f
            } else {
                panic!("Object is not a function");
            }
        } else {
            panic!("Value is not an object");
        }
    }
    pub fn as_closure(&self) -> &ObjClosure {
        if let Value::Obj(obj) = self {
            if let ObjType::Closure(c) = &obj.obj_type {
                c
            } else {
                panic!("Object is not a closure");
            }
        } else {
            panic!("Value is not an object");
        }
    }
    pub fn is_equal(&self, other: &Value) -> bool {
        match (self, other) {
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::Number(a), Value::Number(b)) => a == b,
            (Value::Obj(a), Value::Obj(b)) => a.ptr_eq(b),
            _ => false,
        }
    }
}

impl Obj {
    pub fn new_string(s: String) -> Self {
        Obj {
            obj_type: ObjType::String(s),
            next: None,
        }
    }
    pub fn new_function(function: ObjFunction) -> Self {
        Obj {
            obj_type: ObjType::Function(function),
            next: None,
        }
    }
    pub fn new_native(native: NativeFunction) -> Self {
        Obj {
            obj_type: ObjType::Native(native),
            next: None,
        }
    }
    pub fn new_closure(closure: ObjClosure) -> Self {
        Obj {
            obj_type: ObjType::Closure(closure),
            next: None,
        }
    }
    pub fn as_string(&self) -> &String {
        if let ObjType::String(s) = &self.obj_type {
            s
        } else {
            panic!("Object is not a string");
        }
    }
    pub fn as_function(&self) -> &ObjFunction {
        if let ObjType::Function(f) = &self.obj_type {
            f
        } else {
            panic!("Object is not a function");
        }
    }
    pub fn as_closure(&self) -> &ObjClosure {
        if let ObjType::Closure(c) = &self.obj_type {
            c
        } else {
            panic!("Object is not a closure");
        }
    }
}

#[derive(Clone)]
pub struct ValueArray {
    pub values: Vec<Value>,
}

impl ValueArray {
    pub fn new() -> Self {
        ValueArray { values: Vec::new() }
    }

    pub fn write(&mut self, value: Value) {
        self.values.push(value);
    }
}
impl Default for ValueArray {
    fn default() -> Self {
        Self::new()
    }
}
