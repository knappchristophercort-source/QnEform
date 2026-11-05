//! Source lexer for the initial QnEform language subset.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    /// Zero-based UTF-8 byte offset into the source.
    pub start: usize,
    pub end: usize,
    /// One-based source coordinates.
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Keyword {
    Qbit,
    Measure,
    Let,
    Quantum,
    Classical,
    True,
    False,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenKind {
    Identifier(String),
    Number(String),
    String(String),
    Keyword(Keyword),
    Semicolon,
    Comma,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Colon,
    Assign, // :=
    Arrow,  // ->
    Plus,
    Minus,
    Star,
    Slash,
    Dot,
    Eof,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LexError {
    pub message: String,
    pub span: Span,
}

impl std::fmt::Display for LexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} at {}:{}", self.message, self.span.line, self.span.column)
    }
}

impl std::error::Error for LexError {}

pub struct Lexer<'a> {
    source: &'a str,
    offset: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        Self { source, offset: 0, line: 1, column: 1 }
    }

    fn peek(&self) -> Option<char> {
        self.source.get(self.offset..)?.chars().next()
    }

    fn peek_second(&self) -> Option<char> {
        let mut chars = self.source.get(self.offset..)?.chars();
        chars.next()?;
        chars.next()
    }

    fn bump(&mut self) -> Option<char> {
        let ch = self.peek()?;
        self.offset += ch.len_utf8();
        if ch == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }
        Some(ch)
    }

    fn span_from(&self, start: usize, line: usize, column: usize) -> Span {
        Span { start, end: self.offset, line, column }
    }

    fn error(&self, message: impl Into<String>, start: usize, line: usize, column: usize) -> LexError {
        LexError { message: message.into(), span: self.span_from(start, line, column) }
    }

    fn skip_trivia(&mut self) -> Result<(), LexError> {
        loop {
            while self.peek().is_some_and(char::is_whitespace) {
                self.bump();
            }
            if self.peek() == Some('/') && self.peek_second() == Some('/') {
                while let Some(ch) = self.bump() {
                    if ch == '\n' { break; }
                }
                continue;
            }
            if self.peek() == Some('/') && self.peek_second() == Some('*') {
                let start = self.offset;
                let line = self.line;
                let column = self.column;
                self.bump();
                self.bump();
                let mut closed = false;
                while let Some(ch) = self.bump() {
                    if ch == '*' && self.peek() == Some('/') {
                        self.bump();
                        closed = true;
                        break;
                    }
                }
                if !closed {
                    return Err(self.error("unterminated block comment", start, line, column));
                }
                continue;
            }
            return Ok(());
        }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>, LexError> {
        let mut tokens = Vec::new();
        loop {
            self.skip_trivia()?;
            let start = self.offset;
            let line = self.line;
            let column = self.column;
            let Some(ch) = self.peek() else {
                tokens.push(Token {
                    kind: TokenKind::Eof,
                    span: Span { start, end: start, line, column },
                });
                return Ok(tokens);
            };

            let kind = if ch.is_ascii_alphabetic() || ch == '_' {
                let mut word = String::new();
                while self.peek().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_') {
                    word.push(self.bump().expect("peeked character exists"));
                }
                match word.as_str() {
                    "qbit" => TokenKind::Keyword(Keyword::Qbit),
                    "measure" => TokenKind::Keyword(Keyword::Measure),
                    "let" => TokenKind::Keyword(Keyword::Let),
                    "quantum" => TokenKind::Keyword(Keyword::Quantum),
                    "classical" => TokenKind::Keyword(Keyword::Classical),
                    "true" => TokenKind::Keyword(Keyword::True),
                    "false" => TokenKind::Keyword(Keyword::False),
                    _ => TokenKind::Identifier(word),
                }
            } else if ch.is_ascii_digit() {
                let mut number = String::new();
                while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '_') {
                    number.push(self.bump().expect("peeked character exists"));
                }
                if self.peek() == Some('.') && self.peek_second().is_some_and(|c| c.is_ascii_digit()) {
                    number.push(self.bump().expect("decimal point exists"));
                    while self.peek().is_some_and(|c| c.is_ascii_digit() || c == '_') {
                        number.push(self.bump().expect("peeked character exists"));
                    }
                }
                TokenKind::Number(number)
            } else {
                match ch {
                    '"' => {
                        self.bump();
                        let mut value = String::new();
                        loop {
                            match self.bump() {
                                Some('"') => break,
                                Some('\\') => match self.bump() {
                                    Some('n') => value.push('\n'),
                                    Some('r') => value.push('\r'),
                                    Some('t') => value.push('\t'),
                                    Some('"') => value.push('"'),
                                    Some('\\') => value.push('\\'),
                                    Some(other) => return Err(self.error(
                                        format!("unsupported escape sequence: \\{other}"),
                                        start, line, column,
                                    )),
                                    None => return Err(self.error("unterminated string escape", start, line, column)),
                                },
                                Some(c) => value.push(c),
                                None => return Err(self.error("unterminated string literal", start, line, column)),
                            }
                        }
                        TokenKind::String(value)
                    }
                    ';' => { self.bump(); TokenKind::Semicolon }
                    ',' => { self.bump(); TokenKind::Comma }
                    '(' => { self.bump(); TokenKind::LParen }
                    ')' => { self.bump(); TokenKind::RParen }
                    '{' => { self.bump(); TokenKind::LBrace }
                    '}' => { self.bump(); TokenKind::RBrace }
                    ':' => {
                        self.bump();
                        if self.peek() == Some('=') {
                            self.bump();
                            TokenKind::Assign
                        } else {
                            TokenKind::Colon
                        }
                    }
                    '-' => {
                        self.bump();
                        if self.peek() == Some('>') {
                            self.bump();
                            TokenKind::Arrow
                        } else {
                            TokenKind::Minus
                        }
                    }
                    '+' => { self.bump(); TokenKind::Plus }
                    '*' => { self.bump(); TokenKind::Star }
                    '/' => { self.bump(); TokenKind::Slash }
                    '.' => { self.bump(); TokenKind::Dot }
                    other => return Err(self.error(format!("unexpected character '{other}'"), start, line, column)),
                }
            };

            tokens.push(Token { kind, span: self.span_from(start, line, column) });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_declarations_and_blocks() {
        let tokens = Lexer::new("quantum { qbit q0; measure q0; }").tokenize().unwrap();
        assert!(matches!(&tokens[0].kind, TokenKind::Keyword(Keyword::Quantum)));
        assert!(matches!(&tokens[2].kind, TokenKind::Keyword(Keyword::Qbit)));
        assert!(matches!(&tokens.last().unwrap().kind, TokenKind::Eof));
    }

    #[test]
    fn handles_comments_and_escaped_strings() {
        let tokens = Lexer::new("// heading\nlet message := \"hello\\nworld\"; /* done */")
            .tokenize().unwrap();
        assert!(tokens.iter().any(|token| {
            token.kind == TokenKind::String("hello\nworld".to_owned())
        }));
    }

    #[test]
    fn rejects_unterminated_string_and_comment() {
        assert!(Lexer::new("\"unfinished").tokenize().is_err());
        assert!(Lexer::new("/* unfinished").tokenize().is_err());
    }

    #[test]
    fn reports_utf8_byte_offsets_and_source_coordinates() {
        let tokens = Lexer::new("qbit α;").tokenize();
        assert!(tokens.is_err()); // identifiers are deliberately ASCII in v0.1
        let tokens = Lexer::new("qbit q0;\nmeasure q0;").tokenize().unwrap();
        assert_eq!(tokens[3].span.line, 2);
        assert_eq!(tokens[3].span.column, 1);
    }
}
