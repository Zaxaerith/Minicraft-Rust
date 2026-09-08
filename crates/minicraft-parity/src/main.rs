use std::{env, fs, path::Path};

use minicraft_common::parity::ParityTrace;

fn main() {
    if let Err(error) = run() {
        eprintln!("minicraft-parity failed: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut arguments = env::args().skip(1);
    let backend = arguments
        .next()
        .ok_or_else(|| "usage: minicraft-parity rust <trace.json>".to_owned())?;
    let trace_path = arguments
        .next()
        .ok_or_else(|| "usage: minicraft-parity rust <trace.json>".to_owned())?;
    if arguments.next().is_some() {
        return Err("unexpected extra parity arguments".to_owned());
    }
    if backend != "rust" {
        return Err(format!("unsupported parity backend {backend:?}"));
    }
    let trace = load_trace(Path::new(&trace_path))?;
    let snapshots = match trace.system.as_str() {
        "java_random" => minicraft_rust::parity::java_random_snapshots(&trace)?,
        "fixed_step" => minicraft_rust::parity::fixed_step_snapshots(&trace)?,
        "focus_update" => minicraft_rust::parity::focus_update_snapshots(&trace)?,
        "core_prelude" => minicraft_rust::parity::core_prelude_snapshots(&trace)?,
        "notifications" => minicraft_rust::parity::notification_snapshots(&trace)?,
        "renderer" => minicraft_rust::parity::renderer_snapshots(&trace)?,
        "game" => minicraft_rust::parity::game_snapshots(&trace)?,
        "display_stack" => minicraft_rust::parity::display_stack_snapshots(&trace)?,
        "title_menu" => minicraft_rust::parity::title_menu_snapshots(&trace)?,
        "pause_menu" => minicraft_rust::parity::pause_menu_snapshots(&trace)?,
        "audio_event" => minicraft_rust::parity::audio_event_snapshots(&trace)?,
        "audio_pcm" => minicraft_rust::parity::audio_pcm_snapshots(&trace)?,
        "player_motion" => minicraft_rust::parity::player_motion_snapshots(&trace)?,
        "combat" => minicraft_rust::parity::combat_snapshots(&trace)?,
        "inventory_crafting" => minicraft_rust::parity::inventory_crafting_snapshots(&trace)?,
        "render_frame" => minicraft_rust::parity::render_frame_snapshots(&trace)?,
        system => return Err(format!("unsupported parity system {system:?}")),
    };
    for snapshot in snapshots {
        println!(
            "{}",
            serde_json::to_string(&snapshot).map_err(|error| error.to_string())?
        );
    }
    Ok(())
}

fn load_trace(path: &Path) -> Result<ParityTrace, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read parity trace {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("cannot parse parity trace {}: {error}", path.display()))
}
