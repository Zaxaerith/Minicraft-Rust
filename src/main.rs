#![windows_subsystem = "windows"]

mod app;
mod assets;
mod audio;
mod config;
mod content;
mod gfx;
mod input;
mod item;
mod localization;
mod network;
mod resource_pack;
mod storage;
mod world;
mod worlds;

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = if arguments.iter().any(|value| value == "--self-check") {
        app::self_check(&arguments)
    } else if let Some(index) = arguments.iter().position(|value| value == "--server") {
        let default_address = network::default_server_address();
        let address = arguments
            .get(index + 1)
            .filter(|value| !value.starts_with("--"))
            .map(String::as_str)
            .unwrap_or(&default_address);
        network::run_server(address)
    } else if let Some(index) = arguments
        .iter()
        .position(|value| value == "--multiplayer-probe")
    {
        arguments
            .get(index + 1)
            .zip(arguments.get(index + 2))
            .ok_or_else(|| "--multiplayer-probe requires an address and username".to_owned())
            .and_then(|(address, username)| network::run_client_probe(address, username))
    } else if let Some(index) = arguments
        .iter()
        .position(|value| value == "--render-preview")
    {
        arguments
            .get(index + 1)
            .ok_or_else(|| "--render-preview requires an output PNG path".to_owned())
            .and_then(|path| app::render_preview(&arguments, std::path::Path::new(path)))
    } else if let Some(index) = arguments
        .iter()
        .position(|value| value == "--render-world-preview")
    {
        arguments
            .get(index + 1)
            .ok_or_else(|| "--render-world-preview requires an output PNG path".to_owned())
            .and_then(|path| app::render_world_preview(&arguments, std::path::Path::new(path)))
    } else if let Some(index) = arguments
        .iter()
        .position(|value| value == "--render-ui-preview")
    {
        arguments
            .get(index + 1)
            .zip(arguments.get(index + 2))
            .ok_or_else(|| {
                "--render-ui-preview requires a screen name and output PNG path".to_owned()
            })
            .and_then(|(name, path)| {
                app::render_ui_preview(&arguments, name, std::path::Path::new(path))
            })
    } else {
        app::run(&arguments)
    };
    if let Err(error) = result {
        eprintln!("Minicraft Rust failed: {error}");
        std::process::exit(1);
    }
}
