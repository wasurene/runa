use runa::lexer::{Lexer, TokenKind};

fn kinds(src: &str) -> Vec<TokenKind> {
    Lexer::new(src).tokenize().unwrap().into_iter().map(|t| t.kind).collect()
}

fn err(src: &str) -> String {
    Lexer::new(src).tokenize().unwrap_err().message
}

#[test]
fn keywords() {
    assert_eq!(
        kinds("func var if else for return break continue struct true false nil"),
        vec![
            TokenKind::Func, TokenKind::Var, TokenKind::If, TokenKind::Else,
            TokenKind::For, TokenKind::Return, TokenKind::Break, TokenKind::Continue,
            TokenKind::Struct, TokenKind::True, TokenKind::False, TokenKind::Nil,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn logic_symbol_and_word_are_same_token() {
    assert_eq!(kinds("a && b"), kinds("a and b"));
    assert_eq!(kinds("a || b"), kinds("a or b"));
    assert_eq!(kinds("!x"), kinds("not x"));
}

#[test]
fn logic_tokens() {
    assert_eq!(
        kinds("a && b || !c and d or not e"),
        vec![
            TokenKind::Ident("a".into()), TokenKind::And,
            TokenKind::Ident("b".into()), TokenKind::Or,
            TokenKind::Not, TokenKind::Ident("c".into()),
            TokenKind::And,
            TokenKind::Ident("d".into()), TokenKind::Or,
            TokenKind::Not, TokenKind::Ident("e".into()),
            TokenKind::Eof,
        ]
    );
}

#[test]
fn numbers() {
    assert_eq!(kinds("0 123 3.14 42.0"), vec![
        TokenKind::Int(0), TokenKind::Int(123),
        TokenKind::Float(3.14), TokenKind::Float(42.0),
        TokenKind::Eof,
    ]);
}

#[test]
fn trailing_dot_is_error() {
    assert!(err("3.").contains("expected digit"));
    assert!(err("3..5").contains("expected digit"));
}

#[test]
fn integers_out_of_i64_range_are_error() {
    assert!(err("999999999999999999999999999").contains("out of range"));
}

#[test]
fn strings() {
    assert_eq!(kinds(r#""hi""#), vec![TokenKind::Str("hi".into()), TokenKind::Eof]);
    assert_eq!(kinds(r#""a\nb""#), vec![TokenKind::Str("a\nb".into()), TokenKind::Eof]);
    assert_eq!(kinds(r#""\"quoted\"""#), vec![TokenKind::Str("\"quoted\"".into()), TokenKind::Eof]);
}

#[test]
fn unterminated_string() {
    assert!(err(r#""abc"#).contains("unterminated string"));
}

#[test]
fn invalid_escape() {
    assert!(err(r#""a\qb""#).contains("invalid escape"));
}

#[test]
fn comments_ignored() {
    assert_eq!(kinds("1 // two\n3"), vec![
        TokenKind::Int(1), TokenKind::Int(3), TokenKind::Eof,
    ]);
    assert_eq!(kinds("1 // comment without newline"), vec![
        TokenKind::Int(1), TokenKind::Eof,
    ]);
}

#[test]
fn operators_and_punctuation() {
    assert_eq!(
        kinds("+ - * / % . , ; : = := == != < <= > >= ( ) [ ] { }"),
        vec![
            TokenKind::Plus, TokenKind::Minus, TokenKind::Star, TokenKind::Slash,
            TokenKind::Percent, TokenKind::Dot, TokenKind::Comma, TokenKind::Semi,
            TokenKind::Colon, TokenKind::Assign, TokenKind::Declare, TokenKind::Eq,
            TokenKind::NotEq, TokenKind::Lt, TokenKind::LtEq, TokenKind::Gt, TokenKind::GtEq,
            TokenKind::LParen, TokenKind::RParen, TokenKind::LBracket, TokenKind::RBracket,
            TokenKind::LBrace, TokenKind::RBrace,
            TokenKind::Eof,
        ]
    );
}

#[test]
fn single_amp_or_pipe_is_error() {
    assert!(err("a & b").contains("expected '&&'"));
    assert!(err("a | b").contains("expected '||'"));
}

#[test]
fn spans_are_correct() {
    let toks = Lexer::new("var x := 42").tokenize().unwrap();
    assert_eq!(toks[0].span, runa::error::Span::new(0, 3));  // var
    assert_eq!(toks[1].span, runa::error::Span::new(4, 5));  // x
    assert_eq!(toks[2].span, runa::error::Span::new(6, 8));  // :=
    assert_eq!(toks[3].span, runa::error::Span::new(9, 11)); // 42
    assert_eq!(toks[4].span, runa::error::Span::new(11, 11)); // eof
}

#[test]
fn hello_file() {
    let src = r#"
func main() {
    x := 42
    y := 3.14
    name := "Runa"
    flag := x > 0 && !(name == "")
    print("hello from", name, "x =", x, "y =", y, "flag =", flag)
}

main()
"#;
    let toks = Lexer::new(src).tokenize().unwrap();
    assert_eq!(toks.last().unwrap().kind, TokenKind::Eof);
}