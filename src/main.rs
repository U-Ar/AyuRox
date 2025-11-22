use std::{env::args, sync::atomic::Ordering};

use ayurox::{
    memory::ALLOCATED,
    vm::{InterpretResult, interpret},
};

fn repl() {
    loop {
        print!("> ");
        std::io::Write::flush(&mut std::io::stdout()).unwrap();
        let mut line = String::new();
        std::io::stdin().read_line(&mut line).unwrap();
        if line.trim().is_empty() {
            println!("Exiting REPL.");
            break;
        }
        interpret(line.as_str());
    }
}

fn run_file(filename: &str) {
    let source = std::fs::read_to_string(filename).expect("Could not read file");
    let source = source.as_str();

    let result = interpret(source);
    match result {
        InterpretResult::Ok => {}
        InterpretResult::CompileError => {
            std::process::exit(65);
        }
        InterpretResult::RuntimeError => {
            std::process::exit(70);
        }
    }
}

fn main() {
    if args().len() == 1 {
        repl();
    } else if args().len() == 2 {
        let filename = &args().nth(1).unwrap();
        run_file(filename);
    } else {
        eprintln!("Usage: ayurox [script]\n");
        std::process::exit(64);
    }

    println!(
        "Allocated memory: {} bytes",
        ALLOCATED.load(Ordering::SeqCst)
    );
}
