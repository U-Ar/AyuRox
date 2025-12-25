use crate::{
    chunk::{
        Chunk, OP_ADD, OP_CONSTANT, OP_DEFINE_GLOBAL, OP_DIVIDE, OP_EQUAL, OP_FALSE, OP_GET_GLOBAL,
        OP_GET_LOCAL, OP_GREATER, OP_JUMP, OP_JUMP_IF_FALSE, OP_LESS, OP_LOOP, OP_MULTIPLY,
        OP_NEGATE, OP_NIL, OP_NOT, OP_POP, OP_PRINT, OP_RETURN, OP_SET_GLOBAL, OP_SET_LOCAL,
        OP_SUBTRACT, OP_TRUE,
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

    locals: Vec<Local>,
    scope_depth: i32,
}

impl<'a> Compiler<'a> {
    pub fn new(source: &'a str) -> Self {
        Compiler {
            parser: Parser::new(Scanner::new(source)),
            chunk: Box::new(Chunk::new()),
            strings: StringTable::new(),
            locals: Vec::new(),
            scope_depth: 0,
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

    fn emit_loop(&mut self, loop_start: usize) {
        self.emit_byte(OP_LOOP);

        let offset = self.current_chunk().code.len() - loop_start + 2;
        if offset > u16::MAX as usize {
            eprintln!("Loop body too large.");
        }
        self.emit_byte(((offset >> 8) & 0xff) as u8);
        self.emit_byte((offset & 0xff) as u8);
    }

    fn emit_jump(&mut self, instruction: u8) -> usize {
        self.emit_byte(instruction);
        self.emit_byte(0xff);
        self.emit_byte(0xff);
        self.current_chunk().code.len() - 2
    }

    fn emit_return(&mut self) {
        self.emit_byte(OP_RETURN);
    }

    fn emit_constant(&mut self, value: Value) {
        let constant = self.make_constant(value);
        self.emit_bytes(OP_CONSTANT, constant);
    }

    fn patch_jump(&mut self, offset: usize) {
        let jump = self.current_chunk().code.len() - offset - 2;
        if jump > u16::MAX as usize {
            eprintln!("Too much code to jump over.");
        }

        self.current_chunk().code[offset] = ((jump >> 8) & 0xff) as u8;
        self.current_chunk().code[offset + 1] = (jump & 0xff) as u8;
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

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;

        while !self.locals.is_empty() && self.locals[self.locals.len() - 1].depth > self.scope_depth
        {
            self.emit_byte(OP_POP);
            self.locals.pop();
        }
    }

    fn declaration(&mut self) {
        if self.parser.match_token(TokenType::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if self.parser.panic_mode {
            self.parser.synchronize();
        }
    }

    fn var_declaration(&mut self) {
        let global = self.parse_variable("Expect variable name.");

        if self.parser.match_token(TokenType::Equal) {
            self.expression();
        } else {
            self.emit_byte(OP_NIL);
        }
        self.parser.consume(
            TokenType::Semicolon,
            "Expect ';' after variable declaration.",
        );

        self.define_variable(global);
    }

    fn statement(&mut self) {
        if self.parser.match_token(TokenType::Print) {
            self.print_statement();
        } else if self.parser.match_token(TokenType::For) {
            self.for_statement();
        } else if self.parser.match_token(TokenType::If) {
            self.if_statement();
        } else if self.parser.match_token(TokenType::While) {
            self.while_statement();
        } else if self.parser.match_token(TokenType::LeftBrace) {
            self.begin_scope();
            self.block();
            self.end_scope();
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

    fn for_statement(&mut self) {
        self.begin_scope();
        self.parser
            .consume(TokenType::LeftParen, "Expect '(' after 'for'.");
        if self.parser.match_token(TokenType::Semicolon) {
            // No initializer.
        } else if self.parser.match_token(TokenType::Var) {
            self.var_declaration();
        } else {
            self.expression_statement();
        }

        let mut loop_start = self.current_chunk().code.len();
        let mut exit_jump = None;
        if !self.parser.match_token(TokenType::Semicolon) {
            self.expression();
            self.parser
                .consume(TokenType::Semicolon, "Expect ';' after loop condition.");

            exit_jump = Some(self.emit_jump(OP_JUMP_IF_FALSE));
            self.emit_byte(OP_POP);
        }

        self.parser
            .consume(TokenType::RightParen, "Expect ')' after for clauses.");

        if !self.parser.match_token(TokenType::RightParen) {
            let body_jump = self.emit_jump(OP_JUMP);
            let increment_start = self.current_chunk().code.len();
            self.expression();
            self.emit_byte(OP_POP);
            self.parser
                .consume(TokenType::RightParen, "Expect ')' after for clauses.");

            self.emit_loop(loop_start);
            loop_start = increment_start;
            self.patch_jump(body_jump);
        }

        self.statement();
        self.emit_loop(loop_start);

        if let Some(exit_jump) = exit_jump {
            self.patch_jump(exit_jump);
            self.emit_byte(OP_POP);
        }
        self.end_scope();
    }

    fn while_statement(&mut self) {
        let loop_start = self.current_chunk().code.len();
        self.parser
            .consume(TokenType::LeftParen, "Expect '(' after 'while'.");
        self.expression();
        self.parser
            .consume(TokenType::RightParen, "Expect ')' after condition.");

        let exit_jump = self.emit_jump(OP_JUMP_IF_FALSE);

        self.emit_byte(OP_POP);
        self.statement();
        self.emit_loop(loop_start);

        self.patch_jump(exit_jump);
        self.emit_byte(OP_POP);
    }

    fn if_statement(&mut self) {
        self.parser
            .consume(TokenType::LeftParen, "Expect '(' after 'if'.");
        self.expression();
        self.parser
            .consume(TokenType::RightParen, "Expect ')' after condition.");

        let then_jump = self.emit_jump(OP_JUMP_IF_FALSE);
        self.emit_byte(OP_POP);

        self.statement();

        let else_jump = self.emit_jump(OP_JUMP);

        self.patch_jump(then_jump);
        self.emit_byte(OP_POP);

        if self.parser.match_token(TokenType::Else) {
            self.statement();
        }

        self.patch_jump(else_jump);
    }

    fn block(&mut self) {
        while !self.parser.check(TokenType::RightBrace) && !self.parser.check(TokenType::Eof) {
            self.declaration();
        }
        self.parser
            .consume(TokenType::RightBrace, "Expect '}' after block.");
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
        let can_assign = precedence <= Precedence::Assignment;

        let prefix_rule = PARSE_RULES[self.parser.previous.token_type as usize].prefix;
        if let Some(prefix) = prefix_rule {
            prefix(self, can_assign);
        } else {
            self.parser.error("Expect expression.".to_string());
            return;
        }

        while precedence <= PARSE_RULES[self.parser.current.token_type as usize].precedence {
            self.parser.advance();
            let infix_rule = PARSE_RULES[self.parser.previous.token_type as usize].infix;
            if let Some(infix) = infix_rule {
                infix(self, can_assign);
            }
        }

        if can_assign && self.parser.match_token(TokenType::Equal) {
            self.parser.error("Invalid assignment target.".to_string());
        }
    }

    fn parse_variable(&mut self, message: &str) -> u8 {
        self.parser.consume(TokenType::Identifier, message);

        self.declare_variable();
        if self.scope_depth > 0 {
            return 0;
        }

        self.identifier_constant(&self.parser.previous.clone())
    }

    fn identifier_constant(&mut self, name: &Token) -> u8 {
        let value = Value::new_obj(
            self.strings
                .intern(self.parser.scanner.get_source(name.start, name.length)),
        );
        self.make_constant(value)
    }

    fn identifiers_equal(&self, a: &Token, b: &Token) -> bool {
        if a.length != b.length {
            return false;
        }
        self.parser.scanner.get_source(a.start, a.length)
            == self.parser.scanner.get_source(b.start, b.length)
    }

    fn resolve_local(&mut self, name: &Token) -> Option<usize> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if self.identifiers_equal(name, &local.name) {
                if local.depth == -1 {
                    self.parser
                        .error("Cannot read local variable in its own initializer.".to_string());
                }
                return Some(i);
            }
        }
        None
    }

    fn declare_variable(&mut self) {
        if self.scope_depth == 0 {
            return;
        }

        let name = self.parser.previous.clone();

        for local in self.locals.iter().rev() {
            if local.depth != -1 && local.depth < self.scope_depth {
                break;
            }

            if self.identifiers_equal(&name, &local.name) {
                self.parser
                    .error("Already a variable with this name in this scope.".to_string());
            }
        }

        self.add_local(name);
    }

    fn add_local(&mut self, name: Token) {
        if self.locals.len() >= 256 {
            self.parser
                .error("Too many local variables in function.".to_string());
            return;
        }

        self.locals.push(Local { name, depth: -1 });
    }

    fn define_variable(&mut self, global: u8) {
        if self.scope_depth > 0 {
            self.mark_initialized();
            return;
        }
        self.emit_bytes(OP_DEFINE_GLOBAL, global);
    }

    fn mark_initialized(&mut self) {
        let last_index = self.locals.len() - 1;
        self.locals[last_index].depth = self.scope_depth;
    }

    fn named_variable(&mut self, name: Token, can_assign: bool) {
        let (arg, get_op, set_op) = match self.resolve_local(&name) {
            Some(local) => (local as u8, OP_GET_LOCAL, OP_SET_LOCAL),
            None => {
                let arg = self.identifier_constant(&name);
                (arg, OP_GET_GLOBAL, OP_SET_GLOBAL)
            }
        };

        if can_assign && self.parser.match_token(TokenType::Equal) {
            self.expression();
            self.emit_bytes(set_op, arg);
        } else {
            self.emit_bytes(get_op, arg);
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

type ParseFn = fn(&mut Compiler, bool) -> ();

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

#[derive(Clone)]
struct Local {
    name: Token,
    depth: i32,
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
        prefix: Some(variable),
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
        infix: Some(and),
        precedence: Precedence::And,
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
        infix: Some(or),
        precedence: Precedence::Or,
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

fn number(compiler: &mut Compiler, _can_assign: bool) {
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

fn grouping(compiler: &mut Compiler, _can_assign: bool) {
    compiler.expression();
    compiler
        .parser
        .consume(TokenType::RightParen, "Expect ')' after expression.");
}

fn unary(compiler: &mut Compiler, _can_assign: bool) {
    let operator_type = compiler.parser.previous.token_type;

    compiler.parse_precedence(Precedence::Unary);

    match operator_type {
        TokenType::Bang => compiler.emit_byte(OP_NOT),
        TokenType::Minus => compiler.emit_byte(OP_NEGATE),
        _ => unreachable!(), // Unreachable.
    }
}

fn binary(compiler: &mut Compiler, _can_assign: bool) {
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

fn literal(compiler: &mut Compiler, _can_assign: bool) {
    match compiler.parser.previous.token_type {
        TokenType::False => compiler.emit_byte(OP_FALSE),
        TokenType::Nil => compiler.emit_byte(OP_NIL),
        TokenType::True => compiler.emit_byte(OP_TRUE),
        _ => unreachable!(), // Unreachable.
    }
}

fn string(compiler: &mut Compiler, _can_assign: bool) {
    let ptr = compiler.strings.intern(compiler.parser.scanner.get_source(
        compiler.parser.previous.start + 1,
        compiler.parser.previous.length - 2,
    ));
    compiler.emit_constant(Value::new_obj(ptr));
}

fn variable(compiler: &mut Compiler, can_assign: bool) {
    compiler.named_variable(compiler.parser.previous.clone(), can_assign)
}

fn and(compiler: &mut Compiler, _can_assign: bool) {
    let end_jump = compiler.emit_jump(OP_JUMP_IF_FALSE);

    compiler.emit_byte(OP_POP);
    compiler.parse_precedence(Precedence::And);

    compiler.patch_jump(end_jump);
}

fn or(compiler: &mut Compiler, _can_assign: bool) {
    let else_jump = compiler.emit_jump(OP_JUMP_IF_FALSE);
    let end_jump = compiler.emit_jump(OP_JUMP);

    compiler.patch_jump(else_jump);
    compiler.emit_byte(OP_POP);

    compiler.parse_precedence(Precedence::Or);
    compiler.patch_jump(end_jump);
}
