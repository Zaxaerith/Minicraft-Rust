#![windows_subsystem = "windows"]

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    if let Err(error) = minicraft_rust::run_cli(&arguments) {
        eprintln!("Minicraft Rust failed: {error}");
        std::process::exit(1);
    }
}
