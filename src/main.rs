mod app;
mod assets;
mod config;
mod content;
mod gfx;
mod input;
mod item;
mod localization;
mod resource_pack;
mod world;
mod worlds;

fn main() {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let result = if let Some(index) = arguments
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
        eprintln!("Minicraft+ Rust failed: {error}");
        std::process::exit(1);
    }
}
