use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::error::{Error, Result, Span};
use crate::parser::{Expr, Program, Stmt};

#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    Str(String),
    Bool(bool),
    Nil,
    List(Rc<RefCell<Vec<Value>>>),
    Struct(Rc<RefCell<HashMap<String, Value>>>),
    Function(Rc<Function>),
    Builtin(fn(Vec<Value>) -> Result<Value>),
}

impl Value {
    pub fn to_string(&self) -> String {
        match self {
            Value::Int(n) => n.to_string(),
            Value::Float(f) => f.to_string(),
            Value::Str(s) => s.clone(),
            Value::Bool(b) => b.to_string(),
            Value::Nil => "nil".to_string(),
            Value::List(l) => {
                let items: Vec<String> = l.borrow().iter().map(|v| v.to_string()).collect();
                format!("[{}]", items.join(", "))
            }
            Value::Struct(s) => {
                let items: Vec<String> = s.borrow().iter().map(|(k, v)| format!("{}: {}", k, v.to_string())).collect();
                format!("{{ {} }}", items.join(", "))
            }
            Value::Function(f) => format!("<func {}>", f.name),
            Value::Builtin(_) => "<builtin>".to_string(),
        }
    }

    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::Str(_) => "string",
            Value::Bool(_) => "bool",
            Value::Nil => "nil",
            Value::List(_) => "list",
            Value::Struct(_) => "struct",
            Value::Function(_) | Value::Builtin(_) => "function",
        }
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<Stmt>,
    pub env: Rc<RefCell<Environment>>,
    pub receiver_name: Option<String>,
}

#[derive(Debug)]
pub struct Environment {
    pub values: HashMap<String, Value>,
    pub parent: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
    pub fn new() -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Environment {
            values: HashMap::new(),
            parent: None,
        }))
    }

    pub fn new_enclosed(parent: Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
        Rc::new(RefCell::new(Environment {
            values: HashMap::new(),
            parent: Some(parent),
        }))
    }

    pub fn define(&mut self, name: String, value: Value) {
        self.values.insert(name, value);
    }

    pub fn get(&self, name: &str) -> Option<Value> {
        if let Some(v) = self.values.get(name) {
            return Some(v.clone());
        }
        if let Some(p) = &self.parent {
            return p.borrow().get(name);
        }
        None
    }

    pub fn assign(&mut self, name: &str, value: Value) -> bool {
        if self.values.contains_key(name) {
            self.values.insert(name.to_string(), value);
            return true;
        }
        if let Some(p) = &self.parent {
            return p.borrow_mut().assign(name, value);
        }
        false
    }
}

enum Flow {
    Normal,
    Return(Value),
    Break,
    Continue,
}

pub struct Interpreter {
    global: Rc<RefCell<Environment>>,
    structs: HashMap<String, Vec<String>>,
    methods: HashMap<String, HashMap<String, Rc<Function>>>,
}

impl Interpreter {
    pub fn new() -> Self {
        let global = Environment::new();
        let mut interp = Interpreter {
            global,
            structs: HashMap::new(),
            methods: HashMap::new(),
        };
        interp.define_builtins();
        interp
    }

    fn define_builtins(&mut self) {
        let mut env = self.global.borrow_mut();

        env.define(
            "print".to_string(),
            Value::Builtin(|args| {
                let out: Vec<String> = args.iter().map(|v| v.to_string()).collect();
                println!("{}", out.join(" "));
                Ok(Value::Nil)
            }),
        );

        env.define(
            "len".to_string(),
            Value::Builtin(|args| {
                if args.len() != 1 {
                    return Err(Error::new("len() ожидает 1 аргумент", Span::new(0, 0)));
                }
                match &args[0] {
                    Value::Str(s) => Ok(Value::Int(s.len() as i64)),
                    Value::List(l) => Ok(Value::Int(l.borrow().len() as i64)),
                    _ => Err(Error::new("len() работает только со строками и списками", Span::new(0, 0))),
                }
            }),
        );

        env.define(
            "sqrt".to_string(),
            Value::Builtin(|args| {
                if args.len() != 1 { return Err(Error::new("sqrt() ожидает 1 аргумент", Span::new(0, 0))); }
                match &args[0] {
                    Value::Int(n) => Ok(Value::Float((*n as f64).sqrt())),
                    Value::Float(f) => Ok(Value::Float(f.sqrt())),
                    _ => Err(Error::new("sqrt() ожидает число", Span::new(0, 0))),
                }
            }),
        );

        env.define(
            "abs".to_string(),
            Value::Builtin(|args| {
                if args.len() != 1 { return Err(Error::new("abs() ожидает 1 аргумент", Span::new(0, 0))); }
                match &args[0] {
                    Value::Int(n) => Ok(Value::Int(n.abs())),
                    Value::Float(f) => Ok(Value::Float(f.abs())),
                    _ => Err(Error::new("abs() ожидает число", Span::new(0, 0))),
                }
            }),
        );

        env.define(
            "push".to_string(),
            Value::Builtin(|args| {
                if args.len() != 2 { return Err(Error::new("push() ожидает 2 аргумента", Span::new(0, 0))); }
                match &args[0] {
                    Value::List(l) => {
                        l.borrow_mut().push(args[1].clone());
                        Ok(args[0].clone())
                    }
                    _ => Err(Error::new("push() ожидает список", Span::new(0, 0))),
                }
            }),
        );

        env.define(
            "pop".to_string(),
            Value::Builtin(|args| {
                if args.len() != 1 { return Err(Error::new("pop() ожидает 1 аргумент", Span::new(0, 0))); }
                match &args[0] {
                    Value::List(l) => {
                        let mut list = l.borrow_mut();
                        if list.is_empty() {
                            return Err(Error::new("pop() из пустого списка", Span::new(0, 0)));
                        }
                        Ok(list.pop().unwrap())
                    }
                    _ => Err(Error::new("pop() ожидает список", Span::new(0, 0))),
                }
            }),
        );

        env.define(
            "type".to_string(),
            Value::Builtin(|args| {
                if args.len() != 1 { return Err(Error::new("type() ожидает 1 аргумент", Span::new(0, 0))); }
                Ok(Value::Str(args[0].type_name().to_string()))
            }),
        );

        env.define(
            "str".to_string(),
            Value::Builtin(|args| {
                if args.len() != 1 { return Err(Error::new("str() ожидает 1 аргумент", Span::new(0, 0))); }
                Ok(Value::Str(args[0].to_string()))
            }),
        );

        env.define(
            "int".to_string(),
            Value::Builtin(|args| {
                if args.len() != 1 { return Err(Error::new("int() ожидает 1 аргумент", Span::new(0, 0))); }
                match &args[0] {
                    Value::Int(n) => Ok(Value::Int(*n)),
                    Value::Float(f) => Ok(Value::Int(*f as i64)),
                    Value::Str(s) => s.parse::<i64>().map(Value::Int).map_err(|_| Error::new("неверный int", Span::new(0, 0))),
                    _ => Err(Error::new("int() не может преобразовать", Span::new(0, 0))),
                }
            }),
        );

        env.define(
            "float".to_string(),
            Value::Builtin(|args| {
                if args.len() != 1 { return Err(Error::new("float() ожидает 1 аргумент", Span::new(0, 0))); }
                match &args[0] {
                    Value::Int(n) => Ok(Value::Float(*n as f64)),
                    Value::Float(f) => Ok(Value::Float(*f)),
                    Value::Str(s) => s.parse::<f64>().map(Value::Float).map_err(|_| Error::new("неверный float", Span::new(0, 0))),
                    _ => Err(Error::new("float() не может преобразовать", Span::new(0, 0))),
                }
            }),
        );
    }

    pub fn run(&mut self, program: &Program) -> Result<Value> {
        let last = Value::Nil;
        for stmt in &program.statements {
            match self.exec(stmt, self.global.clone())? {
                Flow::Return(v) => return Ok(v),
                _ => {}
            }
        }
        Ok(last)
    }

    fn exec(&mut self, stmt: &Stmt, env: Rc<RefCell<Environment>>) -> Result<Flow> {
        match stmt {
            Stmt::Expression(expr) => {
                self.eval(expr, env)?;
            }
            Stmt::VarDecl { name, value } | Stmt::ShortDecl { name, value } => {
                let val = self.eval(value, env.clone())?;
                env.borrow_mut().define(name.clone(), val);
            }
            Stmt::If { condition, then_branch, else_branch } => {
                let cond = self.eval(condition, env.clone())?;
                if let Value::Bool(true) = cond {
                    let new_env = Environment::new_enclosed(env);
                    for s in then_branch {
                        match self.exec(s, new_env.clone())? {
                            Flow::Return(v) => return Ok(Flow::Return(v)),
                            Flow::Break => return Ok(Flow::Break),
                            Flow::Continue => return Ok(Flow::Continue),
                            _ => {}
                        }
                    }
                } else if let Some(else_b) = else_branch {
                    let new_env = Environment::new_enclosed(env);
                    for s in else_b {
                        match self.exec(s, new_env.clone())? {
                            Flow::Return(v) => return Ok(Flow::Return(v)),
                            Flow::Break => return Ok(Flow::Break),
                            Flow::Continue => return Ok(Flow::Continue),
                            _ => {}
                        }
                    }
                }
            }
            Stmt::For { condition, body, .. } => {
                loop {
                    if let Some(cond) = condition {
                        let val = self.eval(cond, env.clone())?;
                        match val {
                            Value::Bool(false) => break,
                            Value::Bool(true) => {}
                            _ => return Err(Error::new("Условие цикла должно быть bool", Span::new(0, 0))),
                        }
                    }
                    let loop_env = Environment::new_enclosed(env.clone());
                    let mut break_loop = false;
                    for s in body {
                        match self.exec(s, loop_env.clone())? {
                            Flow::Return(v) => return Ok(Flow::Return(v)),
                            Flow::Break => { break_loop = true; break; }
                            Flow::Continue => break,
                            _ => {}
                        }
                    }
                    if break_loop { break; }
                }
            }
            Stmt::FuncDecl { name, params, body } => {
                let func = Function {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    env: env.clone(),
                    receiver_name: None,
                };
                env.borrow_mut().define(name.clone(), Value::Function(Rc::new(func)));
            }
            Stmt::MethodDecl { receiver_name, receiver_type, name, params, body } => {
                let func = Function {
                    name: name.clone(),
                    params: params.clone(),
                    body: body.clone(),
                    env: env.clone(),
                    receiver_name: Some(receiver_name.clone()),
                };
                self.methods
                    .entry(receiver_type.clone())
                    .or_insert_with(HashMap::new)
                    .insert(name.clone(), Rc::new(func));
            }
            Stmt::StructDecl { name, fields } => {
                self.structs.insert(name.clone(), fields.clone());
            }
            Stmt::Return(expr) => {
                let val = if let Some(e) = expr {
                    self.eval(e, env)?
                } else {
                    Value::Nil
                };
                return Ok(Flow::Return(val));
            }
            Stmt::Break => return Ok(Flow::Break),
            Stmt::Continue => return Ok(Flow::Continue),
            Stmt::Block(stmts) => {
                let new_env = Environment::new_enclosed(env);
                for s in stmts {
                    match self.exec(s, new_env.clone())? {
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        Flow::Break => return Ok(Flow::Break),
                        Flow::Continue => return Ok(Flow::Continue),
                        _ => {}
                    }
                }
            }
        }
        Ok(Flow::Normal)
    }

    fn eval(&mut self, expr: &Expr, env: Rc<RefCell<Environment>>) -> Result<Value> {
        match expr {
            Expr::Int(n) => Ok(Value::Int(*n)),
            Expr::Float(f) => Ok(Value::Float(*f)),
            Expr::Str(s) => Ok(Value::Str(s.clone())),
            Expr::Bool(b) => Ok(Value::Bool(*b)),
            Expr::Nil => Ok(Value::Nil),
            Expr::Ident(name) => env
                .borrow()
                .get(name)
                .ok_or_else(|| Error::new(format!("Неизвестная переменная '{}'", name), Span::new(0, 0))),
            Expr::List(elements) => {
                let mut list = Vec::new();
                for e in elements {
                    list.push(self.eval(e, env.clone())?);
                }
                Ok(Value::List(Rc::new(RefCell::new(list))))
            }
            Expr::StructLiteral { name, fields } => {
                let expected_fields = self.structs.get(name).ok_or_else(|| {
                    Error::new(format!("Неизвестная структура '{}'", name), Span::new(0, 0))
                })?;
                if fields.len() != expected_fields.len() {
                    return Err(Error::new(format!("Неверное количество полей для '{}'", name), Span::new(0, 0)));
                }
                let mut map = HashMap::new();
                for (k, v) in fields {
                    let val = self.eval(v, env.clone())?;
                    map.insert(k.clone(), val);
                }
                Ok(Value::Struct(Rc::new(RefCell::new(map))))
            }
            Expr::Binary { left, op, right } => {
                let l = self.eval(left, env.clone())?;
                let r = self.eval(right, env.clone())?;
                self.eval_binary(l, op, r)
            }
            Expr::Unary { op, right } => {
                let r = self.eval(right, env)?;
                match op.as_str() {
                    "-" => match r {
                        Value::Int(n) => Ok(Value::Int(-n)),
                        Value::Float(f) => Ok(Value::Float(-f)),
                        _ => Err(Error::new("Унарный минус только для чисел", Span::new(0, 0))),
                    },
                    "!" => match r {
                        Value::Bool(b) => Ok(Value::Bool(!b)),
                        _ => Err(Error::new("'!' только для bool", Span::new(0, 0))),
                    },
                    _ => unreachable!(),
                }
            }
            Expr::Assign { target, value } => {
                let val = self.eval(value, env.clone())?;
                match &**target {
                    Expr::Ident(name) => {
                        if !env.borrow_mut().assign(name, val.clone()) {
                            return Err(Error::new(format!("Переменная '{}' не объявлена", name), Span::new(0, 0)));
                        }
                    }
                    Expr::FieldAccess { object, field } => {
                        let obj = self.eval(object, env)?;
                        if let Value::Struct(s) = obj {
                            s.borrow_mut().insert(field.clone(), val);
                        } else {
                            return Err(Error::new("Присваивание полю только для структур", Span::new(0, 0)));
                        }
                    }
                    _ => return Err(Error::new("Недопустимая цель присваивания", Span::new(0, 0))),
                }
                Ok(Value::Nil)
            }
            Expr::FieldAccess { object, field } => {
                let obj = self.eval(object, env)?;
                if let Value::Struct(s) = obj {
                    s.borrow().get(field).cloned().ok_or_else(|| {
                        Error::new(format!("Поле '{}' не найдено", field), Span::new(0, 0))
                    })
                } else {
                    Err(Error::new("Доступ к полю только у структур", Span::new(0, 0)))
                }
            }
            Expr::Index { object, index } => {
                let obj = self.eval(object, env.clone())?;
                let idx = self.eval(index, env)?;
                if let Value::List(l) = obj {
                    if let Value::Int(i) = idx {
                        let list = l.borrow();
                        if i < 0 || i as usize >= list.len() {
                            return Err(Error::new("Индекс вне диапазона", Span::new(0, 0)));
                        }
                        Ok(list[i as usize].clone())
                    } else {
                        Err(Error::new("Индекс должен быть int", Span::new(0, 0)))
                    }
                } else {
                    Err(Error::new("Индексация только для списков", Span::new(0, 0)))
                }
            }
            Expr::Call { callee, args } => {
                let mut evaluated_args = Vec::new();
                for a in args {
                    evaluated_args.push(self.eval(a, env.clone())?);
                }

                if let Expr::FieldAccess { object, field } = &**callee {
                    let obj = self.eval(object, env.clone())?;
                    if let Value::Struct(_) = &obj {
                        let mut method_found = None;
                        for (_, methods) in &self.methods {
                            if let Some(m) = methods.get(field) {
                                method_found = Some(m.clone());
                                break;
                            }
                        }
                        if let Some(m) = method_found {
                            return self.call_function(m, evaluated_args, Some(obj));
                        }
                    }
                }

                let func = self.eval(callee, env)?;
                match func {
                    Value::Function(f) => self.call_function(f, evaluated_args, None),
                    Value::Builtin(b) => b(evaluated_args),
                    _ => Err(Error::new("Не является функцией", Span::new(0, 0))),
                }
            }
        }
    }

    fn call_function(&mut self, func: Rc<Function>, args: Vec<Value>, receiver: Option<Value>) -> Result<Value> {
        if args.len() != func.params.len() {
            return Err(Error::new("Неверное количество аргументов", Span::new(0, 0)));
        }
        let env = Environment::new_enclosed(func.env.clone());
        if let (Some(name), Some(rec)) = (&func.receiver_name, receiver) {
            env.borrow_mut().define(name.clone(), rec);
        }
        for (param, arg) in func.params.iter().zip(args) {
            env.borrow_mut().define(param.clone(), arg);
        }
        for stmt in &func.body {
            match self.exec(stmt, env.clone())? {
                Flow::Return(v) => return Ok(v),
                _ => {}
            }
        }
        Ok(Value::Nil)
    }

    fn eval_binary(&self, l: Value, op: &str, r: Value) -> Result<Value> {
        match (l, r) {
            (Value::Int(a), Value::Int(b)) => match op {
                "+" => a.checked_add(b).map(Value::Int).ok_or_else(|| Error::new("Переполнение", Span::new(0, 0))),
                "-" => a.checked_sub(b).map(Value::Int).ok_or_else(|| Error::new("Переполнение", Span::new(0, 0))),
                "*" => a.checked_mul(b).map(Value::Int).ok_or_else(|| Error::new("Переполнение", Span::new(0, 0))),
                "/" => {
                    if b == 0 { return Err(Error::new("Деление на ноль", Span::new(0, 0))); }
                    Ok(Value::Float(a as f64 / b as f64))
                }
                "%" => {
                    if b == 0 { return Err(Error::new("Деление на ноль", Span::new(0, 0))); }
                    Ok(Value::Int(a % b))
                }
                "==" => Ok(Value::Bool(a == b)),
                "!=" => Ok(Value::Bool(a != b)),
                "<" => Ok(Value::Bool(a < b)),
                "<=" => Ok(Value::Bool(a <= b)),
                ">" => Ok(Value::Bool(a > b)),
                ">=" => Ok(Value::Bool(a >= b)),
                _ => Err(Error::new("Неизвестный оператор", Span::new(0, 0))),
            },
            (Value::Float(a), Value::Float(b)) => match op {
                "+" => Ok(Value::Float(a + b)),
                "-" => Ok(Value::Float(a - b)),
                "*" => Ok(Value::Float(a * b)),
                "/" => Ok(Value::Float(a / b)),
                "==" => Ok(Value::Bool(a == b)),
                "!=" => Ok(Value::Bool(a != b)),
                "<" => Ok(Value::Bool(a < b)),
                "<=" => Ok(Value::Bool(a <= b)),
                ">" => Ok(Value::Bool(a > b)),
                ">=" => Ok(Value::Bool(a >= b)),
                _ => Err(Error::new("Неизвестный оператор для float", Span::new(0, 0))),
            },
            (Value::Int(a), Value::Float(b)) => self.eval_binary(Value::Float(a as f64), op, Value::Float(b)),
            (Value::Float(a), Value::Int(b)) => self.eval_binary(Value::Float(a), op, Value::Float(b as f64)),
            (Value::Str(a), Value::Str(b)) => match op {
                "+" => Ok(Value::Str(format!("{}{}", a, b))),
                "==" => Ok(Value::Bool(a == b)),
                "!=" => Ok(Value::Bool(a != b)),
                "<" => Ok(Value::Bool(a < b)),
                "<=" => Ok(Value::Bool(a <= b)),
                ">" => Ok(Value::Bool(a > b)),
                ">=" => Ok(Value::Bool(a >= b)),
                _ => Err(Error::new("Неверный оператор для строк", Span::new(0, 0))),
            },
            (Value::Bool(a), Value::Bool(b)) => match op {
                "&&" => Ok(Value::Bool(a && b)),
                "||" => Ok(Value::Bool(a || b)),
                "==" => Ok(Value::Bool(a == b)),
                "!=" => Ok(Value::Bool(a != b)),
                _ => Err(Error::new("Неверный оператор для bool", Span::new(0, 0))),
            },
            (Value::Nil, Value::Nil) => match op {
                "==" => Ok(Value::Bool(true)),
                "!=" => Ok(Value::Bool(false)),
                _ => Err(Error::new("Неверный оператор для nil", Span::new(0, 0))),
            },
            _ => Err(Error::new("Несовместимые типы", Span::new(0, 0))),
        }
    }
}