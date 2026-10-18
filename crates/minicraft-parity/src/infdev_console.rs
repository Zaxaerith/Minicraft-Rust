//! Opt-in executable Phase4A explorer. No production world or GUI dispatch.
use minicraft_client::experimental::infdev::{Cell, ChunkCoord, Ground, InfdevWorld};
use std::{
    io::{self, BufRead, Write},
    path::Path,
};

pub fn run(command: &str, args: Vec<String>) -> Result<(), String> {
    let mut world = match (command, args.as_slice()) {
        ("infdev-create", [root, name, seed]) => InfdevWorld::create(
            Path::new(root),
            name,
            seed.parse::<i64>().map_err(|e| e.to_string())?,
            4,
        )?,
        ("infdev-open", [directory]) => InfdevWorld::open(Path::new(directory), 4)?,
        _ => {
            return Err(
                "usage: infdev-create EXISTING_ROOT NAME SEED | infdev-open WORLD_DIRECTORY".into(),
            );
        }
    };
    println!(
        "Experimental Infdev4A cursor (no Survival physics/biomes). {}",
        world.directory().display()
    );
    println!(
        "Commands: status | view | move DX DY (each -256..256) | set grass|water|sand|rock DATA | unload | save | quit"
    );
    if world.recovered_backup() {
        println!("RECOVERED complete validated backup checkpoint");
    }
    status(&world);
    let stdin = io::stdin();
    let mut input = stdin.lock();
    loop {
        let mut line = String::new();
        // Read at most 256 bytes per command; never accept an unlimited line.
        let count = std::io::Read::take(&mut input, 256)
            .read_line(&mut line)
            .map_err(|e| e.to_string())?;
        if count == 0 {
            world.checkpoint()?;
            return Ok(());
        }
        if count == 256 || !line.ends_with('\n') {
            return Err(
                "unterminated or oversized console command; last checkpoint preserved".into(),
            );
        }
        let words: Vec<_> = line.split_whitespace().collect();
        match words.as_slice() {
            [] => (),
            ["status"] => status(&world),
            ["view"] => view(&mut world)?,
            ["move", dx, dy] => {
                let dx = dx.parse::<i32>().map_err(|e| e.to_string())?;
                let dy = dy.parse::<i32>().map_err(|e| e.to_string())?;
                if dx.unsigned_abs() > 256 || dy.unsigned_abs() > 256 {
                    return Err("move distance exceeds console limit".into());
                }
                for _ in 0..dx.unsigned_abs() {
                    world.step(dx.signum(), 0)?;
                }
                for _ in 0..dy.unsigned_abs() {
                    world.step(0, dy.signum())?;
                }
                status(&world);
            }
            ["set", ground, data] => {
                let ground = match *ground {
                    "grass" => Ground::Grass,
                    "water" => Ground::Water,
                    "sand" => Ground::Sand,
                    "rock" => Ground::Rock,
                    _ => return Err("unknown experimental ground".into()),
                };
                let data = data.parse::<u16>().map_err(|e| e.to_string())?;
                let (x, y) = world.position();
                world.set_cell(x, y, Cell { ground, data })?;
                status(&world);
            }
            ["save"] => {
                world.checkpoint()?;
                println!("SAVED {} complete chunks", world.saved_chunk_count());
            }
            ["unload"] => {
                world.unload_all()?;
                println!("UNLOADED; all dirty chunks committed");
            }
            ["quit"] => {
                world.checkpoint()?;
                println!("SAVED and closed");
                return Ok(());
            }
            _ => return Err("invalid console command; previous checkpoint preserved".into()),
        }
        io::stdout().flush().map_err(|e| e.to_string())?;
    }
}

fn status(world: &InfdevWorld) {
    let (x, y) = world.position();
    let chunk = ChunkCoord::at(x, y);
    println!(
        "seed={} position=({x},{y}) chunk=({},{}) resident={} saved={}",
        world.seed(),
        chunk.x,
        chunk.y,
        world.resident_count(),
        world.saved_chunk_count()
    );
}

fn view(world: &mut InfdevWorld) -> Result<(), String> {
    let (px, py) = world.position();
    for dy in -3..=3 {
        let mut row = String::new();
        for dx in -5..=5 {
            let x = px.checked_add(dx).ok_or("viewport x overflow")?;
            let y = py.checked_add(dy).ok_or("viewport y overflow")?;
            let cell = world.cell(x, y)?;
            row.push(if dx == 0 && dy == 0 {
                '@'
            } else {
                match cell.ground {
                    Ground::Grass => '.',
                    Ground::Water => '~',
                    Ground::Sand => ':',
                    Ground::Rock => '#',
                }
            });
        }
        println!("{row}");
    }
    let cell = world.cell(px, py)?;
    println!("cursor cell={:?} data={}", cell.ground, cell.data);
    Ok(())
}
