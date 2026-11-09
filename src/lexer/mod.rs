pub mod token;

pub use token::{Token, TokenKind};

use crate::error::{Error, Result, Span};

pub struct Lexer<'a> {
    src: &'a str,
    bytes: &'a [u8],
    pos: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(src: &'a str) -> Self {
        Lexer { src, bytes: src.as_bytes(), pos: 0 }
    }

    pub fn tokenize(mut self) -> Result<Vec<Token>> {
        let mut tokens = Vec::new();
        loop {
            let tok = self.next_token()?;
            let is_eof = tok.kind == TokenKind::Eof;
            tokens.push(tok);
            if is_eof {
                break;
            }
        }
        Ok(tokens)
    }

    fn next_token(&mut self) -> Result<Token> {
        self.skip_whitespace_and_comments();
        let start = self.pos;
        if self.pos >= self.bytes.len() {
            return Ok(Token { kind: TokenKind::Eof, span: Span::new(start, start) });
        }
        let b = self.bytes[self.pos];
        match b {
            b'0'..=b'9' => self.lex_number(),
            b'"' => self.lex_string(),
            b'a'..=b'z' | b'A'..=b'Z' | b'_' => self.lex_ident_or_keyword(),
            b'+' => { self.advance(); Ok(self.tok(TokenKind::Plus, start)) }
            b'-' => { self.advance(); Ok(self.tok(TokenKind::Minus, start)) }
            b'*' => { self.advance(); Ok(self.tok(TokenKind::Star, start)) }
            b'/' => { self.advance(); Ok(self.tok(TokenKind::Slash, start)) }
            b'%' => { self.advance(); Ok(self.tok(TokenKind::Percent, start)) }
            b'.' => { self.advance(); Ok(self.tok(TokenKind::Dot, start)) }
            b',' => { self.advance(); Ok(self.tok(TokenKind::Comma, start)) }
            b';' => { self.advance(); Ok(self.tok(TokenKind::Semi, start)) }
            b'(' => { self.advance(); Ok(self.tok(TokenKind::LParen, start)) }
            b')' => { self.advance(); Ok(self.tok(TokenKind::RParen, start)) }
            b'[' => { self.advance(); Ok(self.tok(TokenKind::LBracket, start)) }
            b']' => { self.advance(); Ok(self.tok(TokenKind::RBracket, start)) }
            b'{' => { self.advance(); Ok(self.tok(TokenKind::LBrace, start)) }
            b'}' => { self.advance(); Ok(self.tok(TokenKind::RBrace, start)) }
            b'&' => self.lex_amp(start),
            b'|' => self.lex_pipe(start),
            b'!' => self.lex_bang(start),
            b'=' => self.lex_assign(start),
            b':' => self.lex_colon(start),
            b'<' => self.lex_lt(start),
            b'>' => self.lex_gt(start),
            _ => {
                let ch = self.src[self.pos..].chars().next().unwrap();
                Err(Error::new(
                    format!("unexpected character '{}'", ch),
                    Span::new(start, start + ch.len_utf8()),
                ))
            }
        }
    }

    fn tok(&self, kind: TokenKind, start: usize) -> Token {
        Token { kind, span: Span::new(start, self.pos) }
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn peek(&self) -> Option<u8> {
        self.bytes.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<u8> {
        self.bytes.get(self.pos + 1).copied()
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            while matches!(self.peek(), Some(b' ' | b'\t' | b'\r' | b'\n')) {
                self.advance();
            }
            if self.peek() == Some(b'/') && self.peek_next() == Some(b'/') {
                while let Some(b) = self.peek() {
                    if b == b'\n' {
                        break;
                    }
                    self.advance();
                }
            } else {
                break;
            }
        }
    }

    fn lex_number(&mut self) -> Result<Token> {
        let start = self.pos;
        while matches!(self.peek(), Some(b'0'..=b'9')) {
            self.advance();
        }
        if self.peek() == Some(b'.') {
            match self.peek_next() {
                Some(b'0'..=b'9') => {
                    self.advance(); // '.'
                    while matches!(self.peek(), Some(b'0'..=b'9')) {
                        self.advance();
                    }
                    let s = &self.src[start..self.pos];
                    let f: f64 = s.parse().map_err(|_| {
                        Error::new(format!("invalid float literal '{}'", s), Span::new(start, self.pos))
                    })?;
                    return Ok(self.tok(TokenKind::Float(f), start));
                }
                _ => {
                    return Err(Error::new(
                        "expected digit after '.' in number literal",
                        Span::new(start, self.pos + 1),
                    ));
                }
            }
        }
        let s = &self.src[start..self.pos];
        let n: i64 = s.parse().map_err(|_| {
            Error::new(format!("integer literal '{}' out of range for i64", s), Span::new(start, self.pos))
        })?;
        Ok(self.tok(TokenKind::Int(n), start))
    }

    fn lex_string(&mut self) -> Result<Token> {
        let start = self.pos;
        self.advance(); // opening quote
        let mut result = String::new();
        loop {
            match self.peek() {
                None => {
                    return Err(Error::new("unterminated string literal", Span::new(start, self.pos)));
                }
                Some(b'"') => {
                    self.advance();
                    return Ok(self.tok(TokenKind::Str(result), start));
                }
                Some(b'\\') => {
                    self.advance();
                    match self.peek() {
                        Some(b'n') => { result.push('\n'); self.advance(); }
                        Some(b't') => { result.push('\t'); self.advance(); }
                        Some(b'r') => { result.push('\r'); self.advance(); }
                        Some(b'\\') => { result.push('\\'); self.advance(); }
                        Some(b'"') => { result.push('"'); self.advance(); }
                        Some(_) => {
                            let ch = self.src[self.pos..].chars().next().unwrap();
                            return Err(Error::new(
                                format!("invalid escape sequence '\\{}'", ch),
                                Span::new(self.pos - 1, self.pos + ch.len_utf8()),
                            ));
                        }
                        None => {
                            return Err(Error::new("unterminated string literal", Span::new(start, self.pos)));
                        }
                    }
                }
                Some(_) => {
                    let ch = self.src[self.pos..].chars().next().unwrap();
                    result.push(ch);
                    self.pos += ch.len_utf8();
                }
            }
        }
    }

    fn lex_ident_or_keyword(&mut self) -> Result<Token> {
        let start = self.pos;
        while matches!(self.peek(), Some(b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'_')) {
            self.advance();
        }
        let s = &self.src[start..self.pos];
        let kind = match s {
            "func" => TokenKind::Func,
            "var" => TokenKind::Var,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "for" => TokenKind::For,
            "return" => TokenKind::Return,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "struct" => TokenKind::Struct,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "nil" => TokenKind::Nil,
            "and" => TokenKind::And,
            "or" => TokenKind::Or,
            "not" => TokenKind::Not,
            _ => TokenKind::Ident(s.to_string()),
        };
        Ok(self.tok(kind, start))
    }

    fn lex_amp(&mut self, start: usize) -> Result<Token> {
        if self.peek_next() == Some(b'&') {
            self.advance();
            self.advance();
            Ok(self.tok(TokenKind::And, start))
        } else {
            Err(Error::new("expected '&&', got '&'", Span::new(start, start + 1)))
        }
    }

    fn lex_pipe(&mut self, start: usize) -> Result<Token> {
        if self.peek_next() == Some(b'|') {
            self.advance();
            self.advance();
            Ok(self.tok(TokenKind::Or, start))
        } else {
            Err(Error::new("expected '||', got '|'", Span::new(start, start + 1)))
        }
    }

    fn lex_bang(&mut self, start: usize) -> Result<Token> {
        if self.peek_next() == Some(b'=') {
            self.advance();
            self.advance();
            Ok(self.tok(TokenKind::NotEq, start))
        } else {
            self.advance();
            Ok(self.tok(TokenKind::Not, start))
        }
    }

    fn lex_assign(&mut self, start: usize) -> Result<Token> {
        if self.peek_next() == Some(b'=') {
            self.advance();
            self.advance();
            Ok(self.tok(TokenKind::Eq, start))
        } else {
            self.advance();
            Ok(self.tok(TokenKind::Assign, start))
        }
    }

    fn lex_colon(&mut self, start: usize) -> Result<Token> {
        if self.peek_next() == Some(b'=') {
            self.advance();
            self.advance();
            Ok(self.tok(TokenKind::Declare, start))
        } else {
            self.advance();
            Ok(self.tok(TokenKind::Colon, start))
        }
    }

    fn lex_lt(&mut self, start: usize) -> Result<Token> {
        if self.peek_next() == Some(b'=') {
            self.advance();
            self.advance();
            Ok(self.tok(TokenKind::LtEq, start))
        } else {
            self.advance();
            Ok(self.tok(TokenKind::Lt, start))
        }
    }

    fn lex_gt(&mut self, start: usize) -> Result<Token> {
        if self.peek_next() == Some(b'=') {
            self.advance();
            self.advance();
            Ok(self.tok(TokenKind::GtEq, start))
        } else {
            self.advance();
            Ok(self.tok(TokenKind::Gt, start))
        }
    }
}