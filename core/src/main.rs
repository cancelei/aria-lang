mod lexer;
mod ast;
mod parser;
mod eval;

use std::env;
use std::fs;
use crate::parser::Parser;
use crate::eval::Evaluator;

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() > 1 && args[1] == "--version" {
        println!("Aria-Lang v0.1.0 (Contest Edition)");
        return;
    }

    let source = if args.len() > 1 {
        fs::read_to_string(&args[1]).expect("Failed to read source file")
    } else {
        r#"
            let $version = 0.1
            let $user = "Agentic Pioneer"
            
            print "--- Welcome to Aria-Lang ---"
            
            think { "Analyzing current environment..." }
            
            gate "Allow execution of agent tasks?" {
                agent welcome_bot {
                    print "Hello, world!"
                    print $user
                    print "Running Aria v0.1"
                }
            }
        "#.to_string()
    };

    let mut parser = Parser::new(&source);
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
