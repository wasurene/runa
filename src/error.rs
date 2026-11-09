#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Span { start, end }
    }
}

#[derive(Debug, Clone)]
pub struct Error {
    pub message: String,
    pub span: Span,
}

impl Error {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Error { message: message.into(), span }
    }
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} (at {}..{})", self.message, self.span.start, self.span.end)
    }
}

impl std::error::Error for Error {}

pub type Result<T> = std::result::Result<T, Error>;