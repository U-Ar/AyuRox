use crate::{
    chunk::{
        Chunk, OP_ADD, OP_CONSTANT, OP_DIVIDE, OP_EQUAL, OP_FALSE, OP_GREATER, OP_LESS,
        OP_MULTIPLY, OP_NEGATE, OP_NIL, OP_NOT, OP_POP, OP_PRINT, OP_RETURN, OP_SUBTRACT, OP_TRUE,
    },
    scanner::{Scanner, Token, TokenType},
    table::StringTable,
    value::Value,
    vm::RuntimeExpression,
};

pub struct Compiler<'a> {
    parser: Parser<'a>,
    chunk: Box<Chunk>,
    strings: StringTable,
}

impl<'a> Compiler<'a> {
    pub fn new(source: &'a str) -> Self {
        Compiler {
            parser: Parser::new(Scanner::new(source)),
            chunk: Box::new(Chunk::new()),
            strings: StringTable::new(),
        }
    }

    fn current_chunk(&mut self) -> &mut Chunk {
        &mut self.chunk
    }

    pub fn compile(mut self) -> Option<RuntimeExpression> {
        self.parser.advance();

        while !self.parser.match_token(TokenType::Eof) {
            self.declaration();
        }

        self.end_compiler();
        Some(RuntimeExpression::new(*self.chunk, self.strings))
    }

    fn emit_byte(&mut self, byte: u8) {
        let line = self.parser.previous.line;
        self.current_chunk().write(byte, line);
    }

    fn emit_bytes(&mut self, byte1: u8, byte2: u8) {
        self.emit_byte(byte1);
        self.emit_byte(byte2);
    }

    fn emit_return(&mut self) {
        self.emit_byte(OP_RETURN);
    }

    fn emit_constant(&mut self, value: Value) {
        let constant = self.make_constant(value);
        self.emit_bytes(OP_CONSTANT, constant);
    }

    fn make_constant(&mut self, value: Value) -> u8 {
        let constant = self.current_chunk().add_constant(value);
        if constant > 255 {
            eprintln!("Too many constants in one chunk.");
            return 0;
        }
        constant as u8
    }

    fn end_compiler(&mut self) {
        self.emit_return();
        self.debug_print_code();
    }

    fn declaration(&mut self) {
        self.statement();

        if self.parser.panic_mode {
            self.parser.synchronize();
        }
    }

    fn statement(&mut self) {
        if self.parser.match_token(TokenType::Print) {
            self.print_statement();
        } else {
            self.expression_statement();
        }
    }

    fn print_statement(&mut self) {
        self.expression();
        self.parser
            .consume(TokenType::Semicolon, "Expect ';' after value.");
        self.emit_byte(OP_PRINT);
    }

    fn expression_statement(&mut self) {
        self.expression();
        self.parser
            .consume(TokenType::Semicolon, "Expect ';' after expression.");
        self.emit_byte(OP_POP);
    }

    fn expression(&mut self) {
        self.parse_precedence(Precedence::Assignment);
    }

    fn parse_precedence(&mut self, precedence: Precedence) {
        self.parser.advance();
        let prefix_rule = PARSE_RULES[self.parser.previous.token_type as usize].prefix;
        if let Some(prefix) = prefix_rule {
            prefix(self);
        } else {
            self.parser.error("Expect expression.".to_string());
            return;
        }

        while precedence <= PARSE_RULES[self.parser.current.token_type as usize].precedence {
            self.parser.advance();
            let infix_rule = PARSE_RULES[self.parser.previous.token_type as usize].infix;
            if let Some(infix) = infix_rule {
                infix(self);
            }
        }
    }

    #[cfg(feature = "debug_print_code")]
    #[inline(always)]
    pub fn debug_print_code(&mut self) {
        if !self.parser.had_error {
            self.current_chunk().disassemble("code");
        }
    }

    #[cfg(not(feature = "debug_print_code"))]
    #[inline(always)]
    pub fn debug_print_code(&mut self) {
        // No-op when debug printing is disabled
    }
}

struct Parser<'a> {
    scanner: Scanner<'a>,
    current: Token,
    previous: Token,
    had_error: bool,
    panic_mode: bool,
}

impl<'a> Parser<'a> {
    fn new(scanner: Scanner<'a>) -> Self {
        Parser {
            scanner,
            current: Token::default(),
            previous: Token::default(),
            had_error: false,
            panic_mode: false,
        }
    }

    fn advance(&mut self) {
        self.previous = self.current.clone();

        loop {
            self.current = self.scanner.scan_token();
            if self.current.token_type != TokenType::Error {
                break;
            }
            self.error_at_current(self.current.error_message.clone().unwrap());
        }
    }

    fn error_at_current(&mut self, message: String) {
        self.error_at(&self.current.clone(), message);
    }

    fn error(&mut self, message: String) {
        self.error_at(&self.previous.clone(), message);
    }

    fn error_at(&mut self, token: &Token, message: String) {
        if self.panic_mode {
            return;
        }
        self.panic_mode = true;

        eprint!("[line {}] Error", token.line);

        match token.token_type {
            TokenType::Eof => eprint!(" at end"),
            TokenType::Error => {}
            _ => eprint!(
                " at '{}'",
                self.scanner.get_source(token.start, token.length)
            ),
        }

        eprintln!(": {}", message);
        self.had_error = true;
    }

    fn consume(&mut self, token_type: TokenType, message: &str) {
        if self.current.token_type == token_type {
            self.advance();
            return;
        }
        self.error_at_current(message.to_string());
    }

    fn check(&self, token_type: TokenType) -> bool {
        self.current.token_type == token_type
    }

    pub fn match_token(&mut self, token_type: TokenType) -> bool {
        if !self.check(token_type) {
            return false;
        }
        self.advance();
        true
    }

    pub fn synchronize(&mut self) {
        self.panic_mode = false;

        while self.current.token_type != TokenType::Eof {
            if self.previous.token_type == TokenType::Semicolon {
                return;
            }

            match self.current.token_type {
                TokenType::Class
                | TokenType::Fun
                | TokenType::Var
                | TokenType::For
                | TokenType::If
                | TokenType::While
                | TokenType::Print
                | TokenType::Return => {
                    return;
                }
                _ => {}
            }
            self.advance();
        }
    }
}

type ParseFn = fn(&mut Compiler) -> ();

#[repr(u8)]
#[derive(PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
enum Precedence {
    None,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

impl From<u8> for Precedence {
    fn from(value: u8) -> Self {
        match value {
            0 => Precedence::None,
            1 => Precedence::Assignment,
            2 => Precedence::Or,
            3 => Precedence::And,
            4 => Precedence::Equality,
            5 => Precedence::Comparison,
            6 => Precedence::Term,
            7 => Precedence::Factor,
            8 => Precedence::Unary,
            9 => Precedence::Call,
            10 => Precedence::Primary,
            _ => Precedence::None,
        }
    }
}

#[derive(Clone, Copy)]
struct ParseRule {
    prefix: Option<ParseFn>,
    infix: Option<ParseFn>,
    precedence: Precedence,
}

const PARSE_RULES: [ParseRule; 256] = init_parse_rules();

const fn init_parse_rules() -> [ParseRule; 256] {
    let mut rules: [ParseRule; 256] = [ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    }; 256];

    rules[TokenType::LeftParen as usize] = ParseRule {
        prefix: Some(grouping),
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::RightParen as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::LeftBrace as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::RightBrace as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Comma as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Dot as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Minus as usize] = ParseRule {
        prefix: Some(unary),
        infix: Some(binary),
        precedence: Precedence::Term,
    };
    rules[TokenType::Plus as usize] = ParseRule {
        prefix: None,
        infix: Some(binary),
        precedence: Precedence::Term,
    };
    rules[TokenType::Semicolon as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Slash as usize] = ParseRule {
        prefix: None,
        infix: Some(binary),
        precedence: Precedence::Factor,
    };
    rules[TokenType::Star as usize] = ParseRule {
        prefix: None,
        infix: Some(binary),
        precedence: Precedence::Factor,
    };
    rules[TokenType::Bang as usize] = ParseRule {
        prefix: Some(unary),
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::BangEqual as usize] = ParseRule {
        prefix: None,
        infix: Some(binary),
        precedence: Precedence::Equality,
    };
    rules[TokenType::Equal as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::EqualEqual as usize] = ParseRule {
        prefix: None,
        infix: Some(binary),
        precedence: Precedence::Equality,
    };
    rules[TokenType::Greater as usize] = ParseRule {
        prefix: None,
        infix: Some(binary),
        precedence: Precedence::Comparison,
    };
    rules[TokenType::GreaterEqual as usize] = ParseRule {
        prefix: None,
        infix: Some(binary),
        precedence: Precedence::Comparison,
    };
    rules[TokenType::Less as usize] = ParseRule {
        prefix: None,
        infix: Some(binary),
        precedence: Precedence::Comparison,
    };
    rules[TokenType::LessEqual as usize] = ParseRule {
        prefix: None,
        infix: Some(binary),
        precedence: Precedence::Comparison,
    };
    rules[TokenType::Identifier as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::String as usize] = ParseRule {
        prefix: Some(string),
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Number as usize] = ParseRule {
        prefix: Some(number),
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::And as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Class as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Else as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::False as usize] = ParseRule {
        prefix: Some(literal),
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::For as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Fun as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::If as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Nil as usize] = ParseRule {
        prefix: Some(literal),
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Or as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Print as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Return as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Super as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::This as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::True as usize] = ParseRule {
        prefix: Some(literal),
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Var as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::While as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Error as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules[TokenType::Eof as usize] = ParseRule {
        prefix: None,
        infix: None,
        precedence: Precedence::None,
    };
    rules
}

fn number(compiler: &mut Compiler) {
    let value: f64 = compiler
        .parser
        .scanner
        .get_source(
            compiler.parser.previous.start,
            compiler.parser.previous.length,
        )
        .parse()
        .unwrap();
    compiler.emit_constant(Value::new_number(value));
}

fn grouping(compiler: &mut Compiler) {
    compiler.expression();
    compiler
        .parser
        .consume(TokenType::RightParen, "Expect ')' after expression.");
}

fn unary(compiler: &mut Compiler) {
    let operator_type = compiler.parser.previous.token_type;

    compiler.parse_precedence(Precedence::Unary);

    match operator_type {
        TokenType::Bang => compiler.emit_byte(OP_NOT),
        TokenType::Minus => compiler.emit_byte(OP_NEGATE),
        _ => unreachable!(), // Unreachable.
    }
}

fn binary(compiler: &mut Compiler) {
    let operator_type = compiler.parser.previous.token_type;
    let rule = &PARSE_RULES[operator_type as usize];
    compiler.parse_precedence(Precedence::from((rule.precedence as u8) + 1));

    match operator_type {
        TokenType::BangEqual => compiler.emit_bytes(OP_EQUAL, OP_NOT),
        TokenType::EqualEqual => compiler.emit_byte(OP_EQUAL),
        TokenType::Greater => compiler.emit_byte(OP_GREATER),
        TokenType::GreaterEqual => compiler.emit_bytes(OP_LESS, OP_NOT),
        TokenType::Less => compiler.emit_byte(OP_LESS),
        TokenType::LessEqual => compiler.emit_bytes(OP_GREATER, OP_NOT),
        TokenType::Plus => compiler.emit_byte(OP_ADD),
        TokenType::Minus => compiler.emit_byte(OP_SUBTRACT),
        TokenType::Star => compiler.emit_byte(OP_MULTIPLY),
        TokenType::Slash => compiler.emit_byte(OP_DIVIDE),
        _ => unreachable!(), // Unreachable.
    }
}

fn literal(compiler: &mut Compiler) {
    match compiler.parser.previous.token_type {
        TokenType::False => compiler.emit_byte(OP_FALSE),
        TokenType::Nil => compiler.emit_byte(OP_NIL),
        TokenType::True => compiler.emit_byte(OP_TRUE),
        _ => unreachable!(), // Unreachable.
    }
}

fn string(compiler: &mut Compiler) {
    let ptr = compiler.strings.intern(compiler.parser.scanner.get_source(
        compiler.parser.previous.start + 1,
        compiler.parser.previous.length - 2,
    ));
    compiler.emit_constant(Value::new_obj(ptr));
}
