use std::{env, fs, process};

fn print_help() {
    println!("QnEform {}", qneform::version());
    println!();
    println!("Usage:");
    println!("  qneform --version       Print version");
    println!("  qneform --help          Show this help");
    println!("  qneform check <file>    Lex and parse a QnEform source file");
    println!();
    println!("The current frontend is a syntax prototype; it does not execute quantum programs.");
}

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("--version") | Some("-V") => {
            println!("qneform {}", qneform::version());
        }
        Some("--help") | Some("-h") | None => print_help(),
        Some("check") => {
            let Some(path) = args.next() else {
                eprintln!("error: 'check' requires a source file path");
                process::exit(2);
            };
            if args.next().is_some() {
                eprintln!("error: unexpected extra arguments");
                process::exit(2);
            }
            let source = match fs::read_to_string(&path) {
                Ok(source) => source,
                Err(error) => {
                    eprintln!("error: could not read {path}: {error}");
                    process::exit(1);
                }
            };
            match qneform::parse_source(&source) {
                Ok(program) => {
                    println!("Syntax OK: {} top-level statement(s)", program.statements.len());
                }
                Err(error) => {
                    eprintln!("Syntax error: {error}");
                    process::exit(1);
                }
            }
        }
        Some(other) => {
            eprintln!("error: unsupported command: {other}");
            eprintln!("Run 'qneform --help' for usage.");
            process::exit(2);
        }
    }
}
