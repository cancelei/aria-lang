//! Example of loading and using the Aria standard library

use aria_stdlib::{Stdlib, StdModule};

fn main() {
    println!("Aria Standard Library Demo\n");

    // Create a new stdlib instance
    let mut stdlib = Stdlib::new();

    // Load specific modules
    println!("Loading math module...");
    match stdlib.load_module(StdModule::Math) {
        Ok(program) => {
            println!("Math module loaded successfully!");
            println!("  Items: {}", program.items.len());
        }
        Err(e) => {
            eprintln!("Failed to load math module: {}", e);
        }
    }

    println!("\nLoading option module...");
    match stdlib.load_module(StdModule::Option) {
        Ok(program) => {
            println!("Option module loaded successfully!");
            println!("  Items: {}", program.items.len());
        }
        Err(e) => {
            eprintln!("Failed to load option module: {}", e);
        }
    }

    println!("\nLoading result module...");
    match stdlib.load_module(StdModule::Result) {
        Ok(program) => {
            println!("Result module loaded successfully!");
            println!("  Items: {}", program.items.len());
        }
        Err(e) => {
            eprintln!("Failed to load result module: {}", e);
        }
    }

    // Check loaded modules
    println!("\nLoaded modules:");
    for module in stdlib.loaded_modules() {
        println!("  - {}", module.path());
    }

    // Get module source
    println!("\nModule sources:");
    for module in StdModule::all() {
        let source = module.source();
        let lines = source.lines().count();
        println!("  - {}: {} lines", module.path(), lines);
    }
}
