use std::{env, fs, process};

fn print_help() {
    println!("QnEform {}", qneform::version());
    println!();
    println!("Usage:");
    println!("  qneform --version       Print version");
    println!("  qneform --help          Show this help");
    println!("  qneform check <file>    Parse and semantically validate a source file");
    println!();
    println!("Validation does not execute or simulate a quantum program.");
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
            match qneform::validate_source(&source) {
                Ok(module) => {
                    println!(
                        "Semantic validation OK: {} instruction(s), {} symbol(s), IR {}",
                        module.instructions.len(),
                        module.symbols.len(),
                        module.ir_version
                    );
                }
                Err(error) => {
                    eprintln!("Validation failed:\n{error}");
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
