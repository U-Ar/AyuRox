use crate::{
    chunk::{
        Chunk, OP_ADD, OP_CALL, OP_CLOSE_UPVALUE, OP_CLOSURE, OP_CONSTANT, OP_DEFINE_GLOBAL,
        OP_DIVIDE, OP_EQUAL, OP_FALSE, OP_GET_GLOBAL, OP_GET_LOCAL, OP_GET_UPVALUE, OP_GREATER,
        OP_JUMP, OP_JUMP_IF_FALSE, OP_LESS, OP_LOOP, OP_MULTIPLY, OP_NEGATE, OP_NIL, OP_NOT,
        OP_POP, OP_PRINT, OP_RETURN, OP_SET_GLOBAL, OP_SET_LOCAL, OP_SET_UPVALUE, OP_SUBTRACT,
        OP_TRUE,
    },
    memory::Gc,
    scanner::{Scanner, Token, TokenType},
    table::StringTable,
    value::{FunctionType, Obj, ObjFunction, Value},
    vm::RuntimeExpression,
};

pub struct Compiler<'a> {
    parser: Parser<'a>,

    function_arena: Vec<FunctionScope>,

    strings: StringTable,

    scope_depth: i32,
}

struct FunctionScope {
    pub function_type: FunctionType,
    pub function: ObjFunction,
    pub locals: Vec<Local>,
    pub upvalues: Vec<Upvalue>,
}

impl<'a> Compiler<'a> {
    pub fn new(source: &'a str) -> Self {
        let function_arena = vec![FunctionScope {
            function_type: FunctionType::Script,
            function: ObjFunction {
                arity: 0,
                upvalue_count: 0,
                chunk: Gc::new(Chunk::new()),
                name: None,
            },
            locals: vec![Local {
                name: Token {
                    token_type: TokenType::Identifier,
                    start: 0,
                    length: 0,
                    line: 0,
                    error_message: None,
                },
                depth: 0,
                is_captured: false,
            }],
            upvalues: Vec::new(),
        }];

        Compiler {
            parser: Parser::new(Scanner::new(source)),
            function_arena,
            strings: StringTable::new(),
            scope_depth: 0,
        }
    }

    #[allow(dead_code)]
    fn current_chunk(&self) -> &Chunk {
        &self.function_arena.last().unwrap().function.chunk
    }

    fn current_chunk_mut(&mut self) -> &mut Chunk {
        &mut self.function_arena.last_mut().unwrap().function.chunk
    }

    fn current_locals(&self) -> &Vec<Local> {
        &self.function_arena.last().unwrap().locals
    }

    fn current_locals_mut(&mut self) -> &mut Vec<Local> {
        &mut self.function_arena.last_mut().unwrap().locals
    }

    fn begin_function_scope(&mut self, function_type: FunctionType) {
        let name = if function_type == FunctionType::Function {
            Some(
                self.strings.intern(
                    self.parser
                        .scanner
                        .get_source(self.parser.previous.start, self.parser.previous.length),
                ),
            )
        } else {
            None
        };
        self.function_arena.push(FunctionScope {
            function_type,
            function: ObjFunction {
                arity: 0,
                chunk: Gc::new(Chunk::new()),
                name,
                upvalue_count: 0,
            },
            locals: vec![Local {
                name: Token {
                    token_type: TokenType::Identifier,
                    start: 0,
                    length: 0,
                    line: 0,
                    error_message: None,
                },
                depth: 0,
                is_captured: false,
            }],
            upvalues: Vec::new(),
        });
    }

    fn end_function_scope(&mut self) -> FunctionScope {
        self.function_arena.pop().unwrap()
    }

    pub fn compile(mut self) -> Option<RuntimeExpression> {
        self.parser.advance();

        while !self.parser.match_token(TokenType::Eof) {
            self.declaration();
        }

        self.emit_return();
        self.debug_print_code();

        if self.parser.had_error {
            None
        } else {
            Some(RuntimeExpression::new(
                self.function_arena[0].function.clone(),
                self.strings,
            ))
        }
    }

    fn emit_byte(&mut self, byte: u8) {
        let line = self.parser.previous.line;
        self.current_chunk_mut().write(byte, line);
    }

    fn emit_bytes(&mut self, byte1: u8, byte2: u8) {
        self.emit_byte(byte1);
        self.emit_byte(byte2);
    }

    fn emit_loop(&mut self, loop_start: usize) {
        self.emit_byte(OP_LOOP);

        let offset = self.current_chunk_mut().code.len() - loop_start + 2;
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
        self.current_chunk_mut().code.len() - 2
    }

    fn emit_return(&mut self) {
        self.emit_byte(OP_NIL);
        self.emit_byte(OP_RETURN);
    }

    fn emit_constant(&mut self, value: Value) {
        let constant = self.make_constant(value);
        self.emit_bytes(OP_CONSTANT, constant);
    }

    fn patch_jump(&mut self, offset: usize) {
        let jump = self.current_chunk_mut().code.len() - offset - 2;
        if jump > u16::MAX as usize {
            eprintln!("Too much code to jump over.");
        }

        self.current_chunk_mut().code[offset] = ((jump >> 8) & 0xff) as u8;
        self.current_chunk_mut().code[offset + 1] = (jump & 0xff) as u8;
    }

    fn make_constant(&mut self, value: Value) -> u8 {
        let constant = self.current_chunk_mut().add_constant(value);
        if constant > 255 {
            eprintln!("Too many constants in one chunk.");
            return 0;
        }
        constant as u8
    }

    fn begin_scope(&mut self) {
        self.scope_depth += 1;
    }

    fn end_scope(&mut self) {
        self.scope_depth -= 1;

        while !self.current_locals_mut().is_empty()
            && self.current_locals_mut().last().unwrap().depth > self.scope_depth
        {
            if self.current_locals().last().unwrap().is_captured {
                self.emit_byte(OP_CLOSE_UPVALUE);
            } else {
                self.emit_byte(OP_POP);
            }
            self.current_locals_mut().pop();
        }
    }

    fn declaration(&mut self) {
        if self.parser.match_token(TokenType::Fun) {
            self.fun_declaration();
        } else if self.parser.match_token(TokenType::Var) {
            self.var_declaration();
        } else {
            self.statement();
        }

        if self.parser.panic_mode {
            self.parser.synchronize();
        }
    }

    fn fun_declaration(&mut self) {
        let global = self.parse_variable("Expect function name.");
        self.mark_initialized();
        self.function(FunctionType::Function);
        self.define_variable(global);
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
        } else if self.parser.match_token(TokenType::Return) {
            self.return_statement();
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

        let mut loop_start = self.current_chunk_mut().code.len();
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
            let increment_start = self.current_chunk_mut().code.len();
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
        let loop_start = self.current_chunk_mut().code.len();
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

    fn return_statement(&mut self) {
        if self.function_arena.last().unwrap().function_type == FunctionType::Script {
            self.parser
                .error("Cannot return from top-level code.".to_string());
        }

        if self.parser.match_token(TokenType::Semicolon) {
            self.emit_return();
        } else {
            self.expression();
            self.parser
                .consume(TokenType::Semicolon, "Expect ';' after return value.");
            self.emit_byte(OP_RETURN);
        }
    }

    fn block(&mut self) {
        while !self.parser.check(TokenType::RightBrace) && !self.parser.check(TokenType::Eof) {
            self.declaration();
        }
        self.parser
            .consume(TokenType::RightBrace, "Expect '}' after block.");
    }

    fn function(&mut self, function_type: FunctionType) {
        self.begin_function_scope(function_type);
        self.begin_scope();

        self.parser
            .consume(TokenType::LeftParen, "Expect '(' after function name.");
        if !self.parser.check(TokenType::RightParen) {
            loop {
                self.function_arena.last_mut().unwrap().function.arity += 1;
                if self.function_arena.last().unwrap().function.arity > 255 {
                    self.parser
                        .error_at_current("Cannot have more than 255 parameters.".to_string());
                }
                let param_constant = self.parse_variable("Expect parameter name.");
                self.define_variable(param_constant);

                if !self.parser.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        self.parser
            .consume(TokenType::RightParen, "Expect ')' after parameters.");

        self.parser
            .consume(TokenType::LeftBrace, "Expect '{' before function body.");
        self.block();

        self.emit_return();
        self.debug_print_code();

        self.end_scope();
        let function_scope = self.end_function_scope();
        let function = function_scope.function;
        let upvalues = function_scope.upvalues;

        let constant = self.make_constant(Value::new_obj(Gc::new(Obj::new_function(function))));
        self.emit_bytes(OP_CLOSURE, constant);

        for upvalue in upvalues {
            self.emit_byte(if upvalue.is_local { 1 } else { 0 });
            self.emit_byte(upvalue.index);
        }
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

    fn resolve_current_local(&mut self, name: &Token) -> Option<usize> {
        self.resolve_local(name, self.function_arena.len() - 1)
    }

    fn resolve_local(&mut self, name: &Token, arena_index: usize) -> Option<usize> {
        for (i, local) in self.function_arena[arena_index]
            .locals
            .iter()
            .enumerate()
            .rev()
        {
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

    fn resolve_current_upvalue(&mut self, name: &Token) -> Option<usize> {
        self.resolve_upvalue(name, self.function_arena.len() - 1)
    }

    fn resolve_upvalue(&mut self, name: &Token, arena_index: usize) -> Option<usize> {
        if arena_index == 0 {
            return None;
        }

        let enclosing = arena_index - 1;

        if let Some(local_index) = self.resolve_local(name, enclosing) {
            self.function_arena[enclosing].locals[local_index].is_captured = true;
            return Some(self.add_upvalue(local_index, true, arena_index));
        }

        if let Some(upvalue_index) = self.resolve_upvalue(name, enclosing) {
            return Some(self.add_upvalue(upvalue_index, false, arena_index));
        }

        None
    }

    fn add_upvalue(&mut self, index: usize, is_local: bool, arena_index: usize) -> usize {
        for (i, upvalue) in self.function_arena[arena_index].upvalues.iter().enumerate() {
            if upvalue.index as usize == index && upvalue.is_local == is_local {
                return i;
            }
        }

        self.function_arena[arena_index].upvalues.push(Upvalue {
            index: index as u8,
            is_local,
        });
        self.function_arena[arena_index].function.upvalue_count += 1;

        self.function_arena[arena_index].upvalues.len() - 1
    }

    fn declare_variable(&mut self) {
        if self.scope_depth == 0 {
            return;
        }

        let name = self.parser.previous.clone();

        let mut is_error = false;
        for local in self.current_locals().iter().rev() {
            if local.depth != -1 && local.depth < self.scope_depth {
                break;
            }

            if self.identifiers_equal(&name, &local.name) {
                is_error = true;
                break;
            }
        }

        if is_error {
            self.parser
                .error("Already a variable with this name in this scope.".to_string());
        }

        self.add_local(name);
    }

    fn add_local(&mut self, name: Token) {
        if self.current_locals().len() >= 256 {
            self.parser
                .error("Too many local variables in function.".to_string());
            return;
        }

        self.current_locals_mut().push(Local {
            name,
            depth: -1,
            is_captured: false,
        });
    }

    fn define_variable(&mut self, global: u8) {
        if self.scope_depth > 0 {
            self.mark_initialized();
            return;
        }
        self.emit_bytes(OP_DEFINE_GLOBAL, global);
    }

    fn argument_list(&mut self) -> u8 {
        let mut arg_count = 0;
        if !self.parser.check(TokenType::RightParen) {
            loop {
                self.expression();
                if arg_count == 255 {
                    self.parser
                        .error("Cannot have more than 255 arguments.".to_string());
                }
                arg_count += 1;
                if !self.parser.match_token(TokenType::Comma) {
                    break;
                }
            }
        }
        self.parser
            .consume(TokenType::RightParen, "Expect ')' after arguments.");
        arg_count
    }

    fn mark_initialized(&mut self) {
        if self.scope_depth == 0 {
            return;
        }
        self.current_locals_mut().last_mut().unwrap().depth = self.scope_depth;
    }

    fn named_variable(&mut self, name: Token, can_assign: bool) {
        let (arg, get_op, set_op) = match self.resolve_current_local(&name) {
            Some(local) => (local as u8, OP_GET_LOCAL, OP_SET_LOCAL),
            None => match self.resolve_current_upvalue(&name) {
                Some(upvalue) => (upvalue as u8, OP_GET_UPVALUE, OP_SET_UPVALUE),
                None => {
                    let arg = self.identifier_constant(&name);
                    (arg, OP_GET_GLOBAL, OP_SET_GLOBAL)
                }
            },
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
            let name = match self.function_arena.last().unwrap().function_type {
                FunctionType::Function => self
                    .function_arena
                    .last()
                    .unwrap()
                    .function
                    .name
                    .as_ref()
                    .map_or("<script>", |name| name.as_string()),
                FunctionType::Script => "<script>",
            }
            .to_string();
            self.current_chunk_mut().disassemble(&name);
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
    is_captured: bool,
}

#[derive(Clone)]
struct Upvalue {
    index: u8,
    is_local: bool,
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
        infix: Some(call),
        precedence: Precedence::Call,
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

fn call(compiler: &mut Compiler, _can_assign: bool) {
    let arg_count = compiler.argument_list();
    compiler.emit_bytes(OP_CALL, arg_count);
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
