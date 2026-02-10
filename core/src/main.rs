mod ast;
mod builtins;
mod eval;
mod lexer;
mod mcp_client;
mod parser;
mod tool_executor;

use crate::eval::Evaluator;
use crate::parser::Parser;
use std::env;
use std::fs;

use std::io::{self, Write};

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "--version" {
        println!("Aria-Lang v1.0.0 (Contest Edition)");
        return;
    }

    if args.len() == 1 {
        run_repl();
        return;
    }

    let source = fs::read_to_string(&args[1]).expect("Failed to read source file");
    run_source(&source);
}

fn run_source(source: &str) {
    let mut parser = Parser::new(source);
    match parser.parse_program() {
        Ok(program) => {
            let mut evaluator = Evaluator::new();
            evaluator.eval_program(program);
        }
        Err(e) => {
            eprintln!("[Syntax Error] {}", e);
            std::process::exit(1);
        }
    }
}

fn run_repl() {
    println!("Aria-Lang v1.0.0 REPL");
    println!("Type 'exit' to quit.");
    let mut evaluator = Evaluator::new();

    loop {
        print!("aria> ");
        io::stdout().flush().unwrap();

        let mut input = String::new();
        io::stdin().read_line(&mut input).unwrap();

        let input = input.trim();
        if input == "exit" {
            break;
        }
        if input.is_empty() {
            continue;
        }

        let mut parser = Parser::new(input);
        match parser.parse_program() {
            Ok(program) => evaluator.eval_program(program),
            Err(e) => eprintln!("[Syntax Error] {}", e),
        }
    }
}
