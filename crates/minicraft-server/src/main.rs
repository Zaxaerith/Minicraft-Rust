//! Standalone multiplayer server entry point.
//!
//! Unlike the windowed Windows client, this binary always attaches a console
//! so listen addresses and fatal bind errors remain visible.

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let default_address = minicraft_server::default_server_address();
    let address = arguments
        .first()
        .filter(|value| !value.starts_with("--"))
        .map(String::as_str)
        .unwrap_or(&default_address);
    if let Err(error) = minicraft_server::run_server(address) {
        eprintln!("Minicraft server failed: {error}");
        std::process::exit(1);
    }
}
