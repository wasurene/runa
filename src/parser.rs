use crate::error::{Error, Result};
use crate::lexer::{Token, TokenKind};

#[derive(Debug, Clone)]
pub enum Expr {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Nil,
    Ident(String),
    Binary {
        left: Box<Expr>,
        op: String,
        right: Box<Expr>,
    },
    Unary {
        op: String,
        right: Box<Expr>,
    },
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
    FieldAccess {
        object: Box<Expr>,
        field: String,
    },
    Index {
        object: Box<Expr>,
        index: Box<Expr>,
    },
    List(Vec<Expr>),
    StructLiteral {
        name: String,
        fields: Vec<(String, Expr)>,
    },
}

#[derive(Debug, Clone)]
pub enum Stmt {
    Expression(Expr),
    VarDecl { name: String, value: Expr },
    ShortDecl { name: String, value: Expr },
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
    },
    For {
        init: Option<Box<Stmt>>,
        condition: Option<Expr>,
        post: Option<Box<Stmt>>,
        body: Vec<Stmt>,
    },
    FuncDecl {
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    MethodDecl {
        receiver_name: String,
        receiver_type: String,
        name: String,
        params: Vec<String>,
        body: Vec<Stmt>,
    },
    StructDecl {
        name: String,
        fields: Vec<String>,
    },
    Return(Option<Expr>),
    Break,
    Continue,
    Block(Vec<Stmt>),
}

#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Stmt>,
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
    no_struct_lit: bool,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, current: 0, no_struct_lit: false }
    }

    pub fn parse(&mut self) -> Result<Program> {
        let mut statements = Vec::new();
        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        Ok(Program { statements })
    }

    fn declaration(&mut self) -> Result<Stmt> {
        if self.match_token(&[TokenKind::Struct]) {
            return self.struct_decl();
        }
        if self.match_token(&[TokenKind::Func]) {
            return self.func_decl();
        }
        if self.match_token(&[TokenKind::Var]) {
            return self.var_decl();
        }
        self.statement()
    }

    fn statement(&mut self) -> Result<Stmt> {
        if self.match_token(&[TokenKind::If]) {
            return self.if_stmt();
        }
        if self.match_token(&[TokenKind::For]) {
            return self.for_stmt();
        }
        if self.match_token(&[TokenKind::Return]) {
            return self.return_stmt();
        }
        if self.match_token(&[TokenKind::Break]) {
            self.match_token(&[TokenKind::Semi]);
            return Ok(Stmt::Break);
        }
        if self.match_token(&[TokenKind::Continue]) {
            self.match_token(&[TokenKind::Semi]);
            return Ok(Stmt::Continue);
        }
        if self.match_token(&[TokenKind::LBrace]) {
            return Ok(Stmt::Block(self.block()?));
        }
        self.expr_stmt()
    }

    fn expr_stmt(&mut self) -> Result<Stmt> {
        if let TokenKind::Ident(name) = self.peek().kind.clone() {
            if self.peek_next().kind == TokenKind::Declare {
                self.advance();
                self.advance();
                let value = self.expression()?;
                self.match_token(&[TokenKind::Semi]);
                return Ok(Stmt::ShortDecl { name, value });
            }
        }

        let expr = self.expression()?;
        self.match_token(&[TokenKind::Semi]);
        Ok(Stmt::Expression(expr))
    }

    fn var_decl(&mut self) -> Result<Stmt> {
        let name = self.consume_ident("ожидалось имя переменной")?;
        self.consume(&TokenKind::Assign, "ожидался '=' после имени")?;
        let value = self.expression()?;
        self.match_token(&[TokenKind::Semi]);
        Ok(Stmt::VarDecl { name, value })
    }

    fn if_stmt(&mut self) -> Result<Stmt> {
        self.no_struct_lit = true;
        let condition = self.expression()?;
        self.no_struct_lit = false;
        
        self.consume(&TokenKind::LBrace, "ожидался '{' после условия")?;
        let then_branch = self.block()?;

        let mut else_branch = None;
        if self.match_token(&[TokenKind::Else]) {
            self.consume(&TokenKind::LBrace, "ожидался '{' после else")?;
            else_branch = Some(self.block()?);
        }

        Ok(Stmt::If { condition, then_branch, else_branch })
    }

    fn for_stmt(&mut self) -> Result<Stmt> {
        if self.check(&TokenKind::LBrace) {
            self.consume(&TokenKind::LBrace, "ожидался '{'")?;
            let body = self.block()?;
            return Ok(Stmt::For { init: None, condition: None, post: None, body });
        }

        self.no_struct_lit = true;
        let condition = self.expression()?;
        self.no_struct_lit = false;

        self.consume(&TokenKind::LBrace, "ожидался '{' после условия цикла")?;
        let body = self.block()?;
        Ok(Stmt::For { init: None, condition: Some(condition), post: None, body })
    }

    fn return_stmt(&mut self) -> Result<Stmt> {
        let value = if !self.check(&TokenKind::Semi) && !self.check(&TokenKind::RBrace) {
            Some(self.expression()?)
        } else {
            None
        };
        self.match_token(&[TokenKind::Semi]);
        Ok(Stmt::Return(value))
    }

    fn block(&mut self) -> Result<Vec<Stmt>> {
        let mut statements = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }
        self.consume(&TokenKind::RBrace, "ожидалась '}'")?;
        Ok(statements)
    }

    fn struct_decl(&mut self) -> Result<Stmt> {
        let name = self.consume_ident("ожидалось имя структуры")?;
        self.consume(&TokenKind::LBrace, "ожидался '{'")?;
        let mut fields = Vec::new();
        if !self.check(&TokenKind::RBrace) {
            loop {
                fields.push(self.consume_ident("ожидалось имя поля")?);
                if !self.match_token(&[TokenKind::Comma]) { break; }
                if self.check(&TokenKind::RBrace) { break; }
            }
        }
        self.consume(&TokenKind::RBrace, "ожидалась '}'")?;
        Ok(Stmt::StructDecl { name, fields })
    }

    fn func_decl(&mut self) -> Result<Stmt> {
        if self.match_token(&[TokenKind::LParen]) {
            let receiver_name = self.consume_ident("ожидалось имя получателя")?;
            let receiver_type = self.consume_ident("ожидался тип получателя")?;
            self.consume(&TokenKind::RParen, "ожидалась ')'")?;
            let name = self.consume_ident("ожидалось имя метода")?;
            
            self.consume(&TokenKind::LParen, "ожидалась '('")?;
            let params = self.parameters()?;
            self.consume(&TokenKind::LBrace, "ожидался '{'")?;
            let body = self.block()?;
            
            return Ok(Stmt::MethodDecl { receiver_name, receiver_type, name, params, body });
        }

        let name = self.consume_ident("ожидалось имя функции")?;
        self.consume(&TokenKind::LParen, "ожидалась '('")?;
        let params = self.parameters()?;
        self.consume(&TokenKind::LBrace, "ожидался '{'")?;
        let body = self.block()?;
        Ok(Stmt::FuncDecl { name, params, body })
    }

    fn parameters(&mut self) -> Result<Vec<String>> {
        let mut params = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                params.push(self.consume_ident("ожидалось имя параметра")?);
                if !self.match_token(&[TokenKind::Comma]) { break; }
                if self.check(&TokenKind::RParen) { break; }
            }
        }
        self.consume(&TokenKind::RParen, "ожидалась ')'")?;
        Ok(params)
    }

    fn expression(&mut self) -> Result<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> Result<Expr> {
        let expr = self.logic_or()?;

        if self.match_token(&[TokenKind::Assign]) {
            let value = self.assignment()?;
            return Ok(Expr::Assign { target: Box::new(expr), value: Box::new(value) });
        }
        Ok(expr)
    }

    fn logic_or(&mut self) -> Result<Expr> {
        let mut expr = self.logic_and()?;
        while self.match_token(&[TokenKind::Or]) {
            let right = self.logic_and()?;
            expr = Expr::Binary { left: Box::new(expr), op: "||".to_string(), right: Box::new(right) };
        }
        Ok(expr)
    }

    fn logic_and(&mut self) -> Result<Expr> {
        let mut expr = self.equality()?;
        while self.match_token(&[TokenKind::And]) {
            let right = self.equality()?;
            expr = Expr::Binary { left: Box::new(expr), op: "&&".to_string(), right: Box::new(right) };
        }
        Ok(expr)
    }

    fn equality(&mut self) -> Result<Expr> {
        let mut expr = self.comparison()?;
        while self.match_token(&[TokenKind::Eq, TokenKind::NotEq]) {
            let op = if self.previous().kind == TokenKind::Eq { "==" } else { "!=" };
            let right = self.comparison()?;
            expr = Expr::Binary { left: Box::new(expr), op: op.to_string(), right: Box::new(right) };
        }
        Ok(expr)
    }

    fn comparison(&mut self) -> Result<Expr> {
        let mut expr = self.term()?;
        while self.match_token(&[TokenKind::Gt, TokenKind::GtEq, TokenKind::Lt, TokenKind::LtEq]) {
            let op = match self.previous().kind {
                TokenKind::Gt => ">", TokenKind::GtEq => ">=",
                TokenKind::Lt => "<", TokenKind::LtEq => "<=",
                _ => unreachable!(),
            };
            let right = self.term()?;
            expr = Expr::Binary { left: Box::new(expr), op: op.to_string(), right: Box::new(right) };
        }
        Ok(expr)
    }

    fn term(&mut self) -> Result<Expr> {
        let mut expr = self.factor()?;
        while self.match_token(&[TokenKind::Plus, TokenKind::Minus]) {
            let op = if self.previous().kind == TokenKind::Plus { "+" } else { "-" };
            let right = self.factor()?;
            expr = Expr::Binary { left: Box::new(expr), op: op.to_string(), right: Box::new(right) };
        }
        Ok(expr)
    }

    fn factor(&mut self) -> Result<Expr> {
        let mut expr = self.unary()?;
        while self.match_token(&[TokenKind::Star, TokenKind::Slash, TokenKind::Percent]) {
            let op = match self.previous().kind {
                TokenKind::Star => "*", TokenKind::Slash => "/", TokenKind::Percent => "%",
                _ => unreachable!(),
            };
            let right = self.unary()?;
            expr = Expr::Binary { left: Box::new(expr), op: op.to_string(), right: Box::new(right) };
        }
        Ok(expr)
    }

    fn unary(&mut self) -> Result<Expr> {
        if self.match_token(&[TokenKind::Not, TokenKind::Minus]) {
            let op = if self.previous().kind == TokenKind::Not { "!" } else { "-" };
            let right = self.unary()?;
            return Ok(Expr::Unary { op: op.to_string(), right: Box::new(right) });
        }
        self.call()
    }

    fn call(&mut self) -> Result<Expr> {
        let mut expr = self.primary()?;

        loop {
            if self.match_token(&[TokenKind::LParen]) {
                let args = self.arguments()?;
                expr = Expr::Call { callee: Box::new(expr), args };
            } else if self.match_token(&[TokenKind::Dot]) {
                let field = self.consume_ident("ожидалось имя поля или метода")?;
                expr = Expr::FieldAccess { object: Box::new(expr), field };
            } else if self.match_token(&[TokenKind::LBracket]) {
                let index = self.expression()?;
                self.consume(&TokenKind::RBracket, "ожидалась ']'")?;
                expr = Expr::Index { object: Box::new(expr), index: Box::new(index) };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn arguments(&mut self) -> Result<Vec<Expr>> {
        let mut args = Vec::new();
        if !self.check(&TokenKind::RParen) {
            loop {
                args.push(self.expression()?);
                if !self.match_token(&[TokenKind::Comma]) { break; }
                if self.check(&TokenKind::RParen) { break; }
            }
        }
        self.consume(&TokenKind::RParen, "ожидалась ')'")?;
        Ok(args)
    }

    fn primary(&mut self) -> Result<Expr> {
        match self.peek().kind.clone() {
            TokenKind::Int(n) => { self.advance(); Ok(Expr::Int(n)) }
            TokenKind::Float(f) => { self.advance(); Ok(Expr::Float(f)) }
            TokenKind::Str(s) => { self.advance(); Ok(Expr::Str(s)) }
            TokenKind::True => { self.advance(); Ok(Expr::Bool(true)) }
            TokenKind::False => { self.advance(); Ok(Expr::Bool(false)) }
            TokenKind::Nil => { self.advance(); Ok(Expr::Nil) }
            TokenKind::Ident(name) => {
                self.advance();
                if self.check(&TokenKind::LBrace) && !self.no_struct_lit {
                    self.advance();
                    let mut fields = Vec::new();
                    if !self.check(&TokenKind::RBrace) {
                        loop {
                            let field_name = self.consume_ident("ожидалось имя поля")?;
                            self.consume(&TokenKind::Colon, "ожидался ':'")?;
                            let value = self.expression()?;
                            fields.push((field_name, value));
                            if !self.match_token(&[TokenKind::Comma]) { break; }
                            if self.check(&TokenKind::RBrace) { break; }
                        }
                    }
                    self.consume(&TokenKind::RBrace, "ожидалась '}'")?;
                    return Ok(Expr::StructLiteral { name, fields });
                }
                Ok(Expr::Ident(name))
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.expression()?;
                self.consume(&TokenKind::RParen, "ожидалась ')'")?;
                Ok(expr)
            }
            TokenKind::LBracket => {
                self.advance();
                let mut elements = Vec::new();
                if !self.check(&TokenKind::RBracket) {
                    loop {
                        elements.push(self.expression()?);
                        if !self.match_token(&[TokenKind::Comma]) { break; }
                        if self.check(&TokenKind::RBracket) { break; }
                    }
                }
                self.consume(&TokenKind::RBracket, "ожидалась ']'")?;
                Ok(Expr::List(elements))
            }
            _ => {
                let tok = self.peek();
                Err(Error::new("Ожидалось выражение", tok.span))
            }
        }
    }

    fn match_token(&mut self, types: &[TokenKind]) -> bool {
        for t in types {
            if self.check(t) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn check(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() { return false; }
        std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind)
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() { self.current += 1; }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenKind::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn peek_next(&self) -> &Token {
        if self.current + 1 >= self.tokens.len() {
            &self.tokens[self.tokens.len() - 1]
        } else {
            &self.tokens[self.current + 1]
        }
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn consume(&mut self, kind: &TokenKind, message: &str) -> Result<&Token> {
        if self.check(kind) {
            return Ok(self.advance());
        }
        Err(Error::new(message, self.peek().span))
    }

    fn consume_ident(&mut self, message: &str) -> Result<String> {
        if let TokenKind::Ident(name) = self.peek().kind.clone() {
            self.advance();
            return Ok(name);
        }
        Err(Error::new(message, self.peek().span))
    }
}