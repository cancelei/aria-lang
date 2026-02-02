mod lexer;

use logos::Logos;
use crate::lexer::Token;

fn main() {
    let input = r#"
        let $user = "Glauber"
        think { "Initializing Aria-Lang..." }
        gate "Ready to proceed?" {
            print "Hello, world!"
            print $user
        }
    "#;

    println!("--- Aria-Lang PoC Lexer ---");
    println!("Input:\n{}", input);
    println!("\nTokens:");

    let lex = Token::lexer(input);
    for token in lex {
        match token {
            Ok(t) => println!("  {:?}", t),
            Err(_) => eprintln!("  [Error] Invalid token"),
        }
    }
}
