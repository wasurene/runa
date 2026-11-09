use crate::error::Span;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Int(i64),
    Float(f64),
    Str(String),
    Ident(String),

    // Ключевые слова
    Func,
    Var,
    If,
    Else,
    For,
    Return,
    Break,
    Continue,
    Struct,
    True,
    False,
    Nil,

    // Логика: &&/and, ||/or, !/not — единый токен
    And,
    Or,
    Not,

    // Арифметика
    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    // Сравнения
    Eq,
    NotEq,
    Lt,
    LtEq,
    Gt,
    GtEq,

    // Присваивание
    Assign,  // =
    Declare, // :=

    // Пунктуация
    Dot,
    Comma,
    Semi,
    Colon,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}