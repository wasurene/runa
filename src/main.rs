use std::env;
use std::fs;
use std::process::ExitCode;

use runa::lexer::Lexer;
use runa::parser::Parser;
use runa::interpreter::Interpreter;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 {
        eprintln!("usage: runa <file.runa>");
        return ExitCode::FAILURE;
    }
    let src = match fs::read_to_string(&args[1]) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("runa: cannot read {}: {}", args[1], e);
            return ExitCode::FAILURE;
        }
    };

    let tokens = match Lexer::new(&src).tokenize() {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lexer error: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let mut parser = Parser::new(tokens);
    let program = match parser.parse() {
        Ok(p) => p,
        Err(e) => {
            eprintln!("Parser error: {}", e);
            return ExitCode::FAILURE;
        }
    };

    let mut interpreter = Interpreter::new();
    if let Err(e) = interpreter.run(&program) {
        eprintln!("Runtime error: {}", e);
        return ExitCode::FAILURE;
    }

    ExitCode::SUCCESS
}