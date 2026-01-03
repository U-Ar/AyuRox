use std::{collections::HashMap, hash::BuildHasher};

use crate::{
    memory::Gc,
    value::{Obj, Value},
};

#[derive(Clone)]
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

pub type StringTable = Table<Gc<Obj>>;
pub type ValueTable = Table<Value>;

#[derive(Debug, Clone)]
pub struct Table<T> {
    pub table: HashMap<String, T, BuildHasherFNV>,
}

impl<T> Table<T> {
    pub fn new() -> Self {
        Table {
            table: HashMap::with_hasher(BuildHasherFNV::new()),
        }
    }

    pub fn get(&self, name: &str) -> Option<&T> {
        self.table.get(name)
    }

    pub fn insert(&mut self, name: String, value: T) -> Option<T> {
        self.table.insert(name, value)
    }

    pub fn set(&mut self, name: &str, value: T) -> bool {
        if self.table.contains_key(name) {
            self.table.insert(name.to_string(), value);
            true
        } else {
            false
        }
    }
}

impl<T> Default for Table<T> {
    fn default() -> Self {
        Self::new()
    }
}
