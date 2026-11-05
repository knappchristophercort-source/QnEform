use std::env;

fn main() {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("--version") | Some("-V") => {
            println!("qneform {}", qneform::version());
        }
        Some("--help") | Some("-h") | None => {
            println!("QnEform {}", qneform::version());
            println!();
            println!("Usage:");
            println!("  qneform --version   Print version");
            println!("  qneform --help      Show this help");
            println!();
            println!("Compiler and quantum execution features are not implemented yet.");
        }
        Some(other) => {
            eprintln!("error: unsupported argument: {other}");
            eprintln!("Run 'qneform --help' for usage.");
            std::process::exit(2);
        }
    }
}
