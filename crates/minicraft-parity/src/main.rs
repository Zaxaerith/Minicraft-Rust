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
    if backend == "java224-generation" {
        return compare_java224_generation(Path::new(&trace_path));
    }
    if backend == "java224-noise" {
        return compare_java224_noise(Path::new(&trace_path));
    }
    if backend != "rust" {
        return Err(format!("unsupported parity backend {backend:?}"));
    }
    let trace = load_trace(Path::new(&trace_path))?;
    let snapshots = match trace.system.as_str() {
        "java_random" => minicraft_client::parity::java_random_snapshots(&trace)?,
        "fixed_step" => minicraft_client::parity::fixed_step_snapshots(&trace)?,
        "focus_update" => minicraft_client::parity::focus_update_snapshots(&trace)?,
        "core_prelude" => minicraft_client::parity::core_prelude_snapshots(&trace)?,
        "notifications" => minicraft_client::parity::notification_snapshots(&trace)?,
        "renderer" => minicraft_client::parity::renderer_snapshots(&trace)?,
        "game" => minicraft_client::parity::game_snapshots(&trace)?,
        "display_stack" => minicraft_client::parity::display_stack_snapshots(&trace)?,
        "title_menu" => minicraft_client::parity::title_menu_snapshots(&trace)?,
        "pause_menu" => minicraft_client::parity::pause_menu_snapshots(&trace)?,
        "audio_event" => minicraft_client::parity::audio_event_snapshots(&trace)?,
        "audio_pcm" => minicraft_client::parity::audio_pcm_snapshots(&trace)?,
        "player_motion" => minicraft_client::parity::player_motion_snapshots(&trace)?,
        "combat" => minicraft_client::parity::combat_snapshots(&trace)?,
        "inventory_crafting" => minicraft_client::parity::inventory_crafting_snapshots(&trace)?,
        "render_frame" => minicraft_client::parity::render_frame_snapshots(&trace)?,
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

fn compare_java224_noise(directory: &Path) -> Result<(), String> {
    let mut different = false;
    for feature in [16, 32] {
        let path = directory.join(format!("noise-{feature}.txt"));
        let saved = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let bits = minicraft_client::parity::java224_noise_bits(feature)?;
        let rows: Vec<_> = saved.lines().collect();
        if rows.len() != bits.len() {
            return Err("invalid Java noise array length".to_owned());
        }
        let mut mismatches = 0;
        for (index, (row, rust)) in rows.iter().zip(bits).enumerate() {
            let java = u64::from_str_radix(row, 16).map_err(|e| e.to_string())?;
            if java != rust {
                if mismatches == 0 {
                    println!(
                        "DIFF noise feature={feature} first=({}, {}) Java={java:016x} Rust={rust:016x}",
                        index % 128,
                        index / 128
                    );
                }
                mismatches += 1;
            }
        }
        if mismatches == 0 {
            println!("PASS noise feature={feature}: 16384 exact double bit patterns");
        } else {
            different = true;
            println!("feature={feature}: {mismatches} unequal noise values");
        }
    }
    if different {
        Err("Java LevelGen noise differs".to_owned())
    } else {
        Ok(())
    }
}

/// Exit with a failure on differences, and report the first unequal cell for
/// every depth. This compares raw LevelGen output, not complete World assembly.
fn compare_java224_generation(directory: &Path) -> Result<(), String> {
    use minicraft_client::level::{WorldSpec, generate_level};
    let normalize = |name: &str| {
        name.chars()
            .filter(char::is_ascii_alphanumeric)
            .flat_map(char::to_lowercase)
            .collect::<String>()
    };
    let mut different = false;
    for depth in [1, 0, -1, -2, -3, -4] {
        let path = directory.join(format!("level-{depth}.tsv"));
        let saved = fs::read_to_string(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let rows: Vec<_> = saved.lines().collect();
        if rows.len() != 128 * 128 {
            return Err(format!("invalid Java level length for {depth}"));
        }
        let rust = generate_level(128, 128, depth, 42, WorldSpec::new(128, 0, 0));
        let mut mismatches = 0;
        for (index, row) in rows.iter().enumerate() {
            let (name, data) = row.split_once('\t').ok_or("invalid Java reference row")?;
            let data = data.parse::<i16>().map_err(|e| e.to_string())? as u16;
            let tile = rust.tiles[index];
            if (match normalize(name).as_str() {
                "lapis" => "lapisore".to_owned(),
                "obsidian" => "obsidianfloor".to_owned(),
                "cloudcactus" => "cloudore".to_owned(),
                "woodplanks" => "woodfloor".to_owned(),
                "stonebricks" => "stonefloor".to_owned(),
                _ => normalize(name),
            }) != normalize(&format!("{tile:?}"))
                || data != rust.data[index]
            {
                if mismatches == 0 {
                    println!(
                        "DIFF depth={depth} first=({}, {}) Java={name}/{data} Rust={tile:?}/{}",
                        index % 128,
                        index / 128,
                        rust.data[index]
                    );
                }
                mismatches += 1;
            }
        }
        if mismatches == 0 {
            println!("PASS depth={depth}: 16384 tiles and data");
        } else {
            different = true;
            println!("depth={depth}: {mismatches} unequal cells");
        }
    }
    if different {
        Err("Java 2.2.4 LevelGen differs; see first unequal cells above".to_owned())
    } else {
        Ok(())
    }
}

fn load_trace(path: &Path) -> Result<ParityTrace, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("cannot read parity trace {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|error| format!("cannot parse parity trace {}: {error}", path.display()))
}
