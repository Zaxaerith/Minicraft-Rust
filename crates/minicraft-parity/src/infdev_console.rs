//! Opt-in experimental chunk explorer. No production world or GUI dispatch.
use minicraft_client::experimental::infdev::{Cell, Ground, InfdevWorld, paged, terrain};
use std::{
    io::{self, BufRead, Write},
    path::Path,
};

pub fn run(command: &str, args: Vec<String>) -> Result<(), String> {
    let mut world = match (command, args.as_slice()) {
        ("infdev-surface-create", [root, name, seed]) => Host::Paged(paged::World::create(
            Path::new(root),
            name,
            seed.parse::<i64>().map_err(|e| e.to_string())?,
            4,
        )?),
        ("infdev-upgrade", [source, root, name]) => Host::Paged(paged::World::upgrade_legacy(
            Path::new(source),
            Path::new(root),
            name,
            4,
        )?),
        ("infdev-create", [root, name, seed]) => Host::Legacy(InfdevWorld::create(
            Path::new(root),
            name,
            seed.parse::<i64>().map_err(|e| e.to_string())?,
            4,
        )?),
        ("infdev-open", [directory]) => {
            let dir = Path::new(directory);
            if dir
                .join("checkpoint.json")
                .try_exists()
                .map_err(|e| e.to_string())?
                || dir
                    .join("checkpoint.json.bak")
                    .try_exists()
                    .map_err(|e| e.to_string())?
            {
                Host::Paged(paged::World::open(dir, 4)?)
            } else {
                Host::Legacy(InfdevWorld::open(dir, 4)?)
            }
        }
        _ => {
            return Err(
                "usage: infdev-surface-create ROOT NAME SEED | infdev-open DIRECTORY | infdev-upgrade SOURCE ROOT NAME | infdev-create ROOT NAME SEED (legacy)".into(),
            );
        }
    };
    println!(
        "Experimental Infdev cursor (no Survival/GUI), generator={}. {}",
        world.generator(),
        world.directory().display()
    );
    println!(
        "Commands: status | view | move DX DY (each -256..256) | set grass|water|sand|rock|tree|flower|cactus|stairs DATA | visit X Y | gc | unload | save | quit"
    );
    if world.recovered_backup() {
        println!("RECOVERED complete validated backup checkpoint");
    }
    status(&mut world)?;
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
            ["status"] => status(&mut world)?,
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
                status(&mut world)?;
            }
            ["set", ground, data] => {
                let ground = match *ground {
                    "grass" => Ground::Grass,
                    "water" => Ground::Water,
                    "sand" => Ground::Sand,
                    "rock" => Ground::Rock,
                    "tree" => Ground::Tree,
                    "flower" => Ground::Flower,
                    "cactus" => Ground::Cactus,
                    "stairs" => Ground::StairsDown,
                    _ => return Err("unknown experimental ground".into()),
                };
                let data = data.parse::<u16>().map_err(|e| e.to_string())?;
                let (x, y) = world.position();
                world.set_cell(x, y, Cell { ground, data })?;
                status(&mut world)?;
            }
            ["visit", x, y] => {
                world.visit(
                    x.parse::<i64>().map_err(|e| e.to_string())?,
                    y.parse::<i64>().map_err(|e| e.to_string())?,
                )?;
                status(&mut world)?;
            }
            ["gc"] => match &mut world {
                Host::Paged(w) => println!("COLLECTED {:?}", w.collect_garbage()?),
                Host::Legacy(_) => {
                    return Err("legacy collection refused: use explicit copy upgrade".into());
                }
            },
            ["save"] => {
                world.checkpoint()?;
                println!("SAVED {} complete chunks", world.saved_chunk_count()?);
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

enum Host {
    Legacy(InfdevWorld),
    Paged(paged::World),
}
impl Host {
    fn directory(&self) -> &Path {
        match self {
            Self::Legacy(w) => w.directory(),
            Self::Paged(w) => w.directory(),
        }
    }
    fn generator(&self) -> String {
        match self {
            Self::Legacy(_) => "LegacyHeightV1/4A".into(),
            Self::Paged(w) => format!("{:?}/paged-v2", w.generator()),
        }
    }
    fn seed(&self) -> i64 {
        match self {
            Self::Legacy(w) => w.seed(),
            Self::Paged(w) => w.seed(),
        }
    }
    fn position(&self) -> (i64, i64) {
        match self {
            Self::Legacy(w) => {
                let (x, y) = w.position();
                (i64::from(x), i64::from(y))
            }
            Self::Paged(w) => w.position(),
        }
    }
    fn recovered_backup(&self) -> bool {
        match self {
            Self::Legacy(w) => w.recovered_backup(),
            Self::Paged(w) => w.recovered_backup(),
        }
    }
    fn resident_count(&self) -> usize {
        match self {
            Self::Legacy(w) => w.resident_count(),
            Self::Paged(w) => w.resident_count(),
        }
    }
    fn saved_chunk_count(&mut self) -> Result<u64, String> {
        match self {
            Self::Legacy(w) => Ok(w.saved_chunk_count() as u64),
            Self::Paged(w) => w.saved_chunk_count(),
        }
    }
    fn step(&mut self, x: i32, y: i32) -> Result<(), String> {
        match self {
            Self::Legacy(w) => w.step(x, y),
            Self::Paged(w) => w.step(x, y),
        }
    }
    fn visit(&mut self, x: i64, y: i64) -> Result<(), String> {
        match self {
            Self::Legacy(_) => Err("64-bit visit requires paged world; use copy upgrade".into()),
            Self::Paged(w) => w.visit(x, y),
        }
    }
    fn checkpoint(&mut self) -> Result<(), String> {
        match self {
            Self::Legacy(w) => w.checkpoint(),
            Self::Paged(w) => w.checkpoint(),
        }
    }
    fn unload_all(&mut self) -> Result<(), String> {
        match self {
            Self::Legacy(w) => w.unload_all(),
            Self::Paged(w) => w.unload_all(),
        }
    }
    fn cell(&mut self, x: i64, y: i64) -> Result<Cell, String> {
        match self {
            Self::Legacy(w) => w.cell(
                x.try_into().map_err(|_| "legacy x overflow")?,
                y.try_into().map_err(|_| "legacy y overflow")?,
            ),
            Self::Paged(w) => w.cell(x, y),
        }
    }
    fn set_cell(&mut self, x: i64, y: i64, c: Cell) -> Result<(), String> {
        match self {
            Self::Legacy(w) => w.set_cell(
                x.try_into().map_err(|_| "legacy x overflow")?,
                y.try_into().map_err(|_| "legacy y overflow")?,
                c,
            ),
            Self::Paged(w) => w.set_cell(x, y, c),
        }
    }
}

fn status(world: &mut Host) -> Result<(), String> {
    let (x, y) = world.position();
    let chunk = paged::Key::at(x, y);
    println!(
        "seed={} position=({x},{y}) chunk=({},{}) resident={} saved={}",
        world.seed(),
        chunk.x,
        chunk.y,
        world.resident_count(),
        world.saved_chunk_count()?
    );
    if let Host::Paged(w) = world
        && let Some(issue) = w.collection_warning()
    {
        println!("GC WARNING (checkpoint saved): {issue}");
    }
    Ok(())
}

fn view(world: &mut Host) -> Result<(), String> {
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
                    Ground::Tree => 'T',
                    Ground::Flower => '*',
                    Ground::Cactus => 'C',
                    Ground::StairsDown => '>',
                }
            });
        }
        println!("{row}");
    }
    let cell = world.cell(px, py)?;
    println!("cursor cell={:?} data={}", cell.ground, cell.data);
    Ok(())
}

pub fn compare(args: Vec<String>) -> Result<(), String> {
    let [directory] = args.as_slice() else {
        return Err("usage: infdev-terrain-check OUTPUT_DIRECTORY".into());
    };
    let root = Path::new(directory);
    let order = std::fs::read_to_string(root.join("biome-order.txt")).map_err(|e| e.to_string())?;
    if order.replace("\r\n", "\n") != "Rock\nForest\nOcean\nRiver\nDesert\nRiverBank\nSurface\n" {
        return Err("Java biome traversal order differs".into());
    }
    let cases =
        std::fs::read_to_string(root.join("terrain-cases.tsv")).map_err(|e| e.to_string())?;
    let mut total = 0;
    let mut eligible = 0;
    let mut differences = 0;
    for line in cases.lines() {
        let v: Vec<_> = line.split('\t').collect();
        if v.len() != 6 {
            return Err("bad terrain case".into());
        }
        let expected = format!("terrain-{}-{}-{}.bin", v[0], v[1], v[2]);
        if v[3] != expected || expected.contains(['/', '\\']) {
            return Err("invalid observation filename".into());
        }
        let data = std::fs::read(root.join(expected)).map_err(|e| e.to_string())?;
        let r = terrain::compare_observation(
            &data,
            v[0].parse::<i64>().map_err(|e| e.to_string())?,
            v[1].parse::<i64>().map_err(|e| e.to_string())?,
            v[2].parse::<i64>().map_err(|e| e.to_string())?,
        )?;
        println!(
            "PASS deterministic fields {}/{}/{}: cells={} time-eligible={} actual-time-choice-differences={}",
            v[0], v[1], v[2], r.cells, r.time_eligible, r.time_choice_differences
        );
        total += r.cells;
        eligible += r.time_eligible;
        differences += r.time_choice_differences;
    }
    if total == 0 {
        return Err("no Java cases".into());
    }
    println!(
        "TOTAL cells={total}, time-eligible={eligible}, actual differing time choices={differences}; NOT a full deterministic-tile equality claim"
    );
    Ok(())
}
