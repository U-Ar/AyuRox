use std::{collections::HashMap, hash::BuildHasher};

use crate::{
    memory::Gc,
    value::{Obj, Value},
};

pub struct BuildHasherFNV {}

pub struct FNVHasher {
    hash: u32,
}

impl BuildHasherFNV {
    pub fn new() -> Self {
        BuildHasherFNV {}
    }
}

impl BuildHasher for BuildHasherFNV {
    type Hasher = FNVHasher;

    fn build_hasher(&self) -> Self::Hasher {
        FNVHasher { hash: 2166136261 }
    }
}

impl Default for BuildHasherFNV {
    fn default() -> Self {
        Self::new()
    }
}

impl std::hash::Hasher for FNVHasher {
    fn finish(&self) -> u64 {
        self.hash as u64
    }

    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.hash ^= *byte as u32;
            self.hash = self.hash.wrapping_mul(16777619);
        }
    }
}

pub struct StringTable {
    pub table: HashMap<String, Gc<Obj>, BuildHasherFNV>,
}

impl StringTable {
    pub fn new() -> Self {
        StringTable {
            table: HashMap::with_hasher(BuildHasherFNV::new()),
        }
    }

    pub fn intern(&mut self, string: &str) -> Gc<Obj> {
        if let Some(obj) = self.table.get(string) {
            return obj.clone();
        }

        let obj = Gc::new(Obj::new_string(string.to_string()));
        self.table.insert(string.to_string(), obj.clone());
        obj
    }
}

impl Default for StringTable {
    fn default() -> Self {
        Self::new()
    }
}

pub struct GlobalVariableTable {
    pub table: HashMap<String, Value, BuildHasherFNV>,
}

impl GlobalVariableTable {
    pub fn new() -> Self {
        GlobalVariableTable {
            table: HashMap::with_hasher(BuildHasherFNV::new()),
        }
    }

    pub fn define(&mut self, name: &str, value: Value) {
        self.table.insert(name.to_string(), value);
    }

    pub fn get(&self, name: &str) -> Option<&Value> {
        self.table.get(name)
    }
}

impl Default for GlobalVariableTable {
    fn default() -> Self {
        Self::new()
    }
}
