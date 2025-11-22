use crate::{chunk::Chunk, scanner::Scanner};

pub struct Compiler<'a> {
    scanner: Scanner<'a>,
}

impl<'a> Compiler<'a> {
    pub fn new(source: &'a str) -> Self {
        Compiler {
            scanner: Scanner::new(source),
        }
    }

    pub fn compile(&mut self) -> Option<Chunk> {
        self.scanner.advance();
        None
    }
}
