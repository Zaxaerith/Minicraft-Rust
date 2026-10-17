# Fixed Java 2.3.0-dev gameplay oracle

`run.ps1` executes the actual upstream Java implementation at commit
`4dc00332e24a727b96be7b885d5cbbdd9c85263c` (`Game.VERSION = 2.3.0-dev2`).
It does not reference experimental Infdev/main code or change the 2.2.4 oracle.

Requirements: JDK 17 (`javac`/`java` on PATH), Git, PowerShell 7, the fixed source
at `.reference/release-2.3.0-dev`, its bare Git database at
`.reference/upstream.git`, and `.reference/official/minicraft-plus-2.2.4.jar`.
The jar supplies third-party dependencies only: **all** pinned client/common
Java files are recompiled, and fresh classes plus pinned resources precede the
jar on the runtime classpath. The driver checks all source/resource Git blob
hashes first and fails on any difference, including line endings; extract with
`core.autocrlf=false` or raw `git cat-file` bytes.

From the repository root:

```powershell
./scripts/parity230/run.ps1
```

Source/jar locations can be passed with `-Source` and `-DependencyJar`. Compile
products, source argument lists, skins, runtime logs, and output live only under
the root ignored `target/parity/java230` directory. The driver does not fetch,
modify reference sources, or update checked-in fixtures automatically.

The original seven compact JSON files in `output` are copied to
`crates/minicraft-client/tests/fixtures/java230` after inspecting actual output:

- `colors.json`: 16 actual registry instances, names, serialized item data,
  sprite keys and packed palette values.
- `wool.json`: actual item placement onto Hole, tile/data, shears harvesting,
  dropped item identity and durability consumption for every color. Java tile
  numbers are evidence of upstream values, **not** Rust save IDs.
- `beds.json`: actual furniture-item placement, color copy, sprite keys,
  collision radii, `Save.writeEntity` and `Load.loadEntity` for every color.
- `flowers.json`: actual item placement, variant data, punching harvest and
  drops for every variant.
- `recipes.json`: actual Dye Vat (35), Loom (57), and workbench Dye Vat (1)
  recipe objects, with product count and full aggregated cost maps.
- `furniture.json`: default red bed plus actual Dye Vat serialization,
  restoration, sprite keys and zero collision radii.
- `operations.json`: actual Rose -> Red Dye crafting and night-time bed use,
  plus an observed upstream bug: shoveling Violet drops Oxeye Daisy because
  `FlowerTile.interact` reads data after replacing the tile with Grass.

The Java oracle initializes tile and advancement registries, disables sound,
and builds isolated levels. Phase 2 additionally executes raw surface
generation and actual watering interactions as described below. No GUI, full
world import, random combat or rendered-frame equivalence is claimed. The Java shovel
bug is recorded rather than made into an expected Rust item-loss behavior.
The driver/harness and upstream-derived fixtures retain GPL-3.0 attribution.

Another source-level discrepancy is intentionally not adopted: in
`Player.tick` the `drop-one` / `drop-stack` branch condition is inverted relative
to its comment and the inventory menu: plain Q drops the whole held stack,
whereas Shift-Q decrements it by one. Rust uses Q for one and Shift-Q for the
whole stack consistently, deducting the referenced inventory count exactly once.
This is a documented item-conservation/input fix, not a claim of Java branch
parity. The small Java fixture does not invoke this keyboard branch.


## Phase 2: natural flora

The same harness now also writes:

- `generated-flora.json`: 19-bin species counts, canonical name/data FNV1a64
  hashes for eight actual `LevelGen.createAndValidateMap` scenarios: seeds 42
  and 256 at 128, seed42 all five themes at 128, seed42 normal at 256/512.
  The two normal 128 maps include every flower coordinate and species.
- `flora-{size}-{seed}-{theme}.tsv`: every raw tile name/data cell, including
  non-flower neighbors. These larger replay outputs remain ignored under target.
- `watering-flora.json`: all19 variants observed from **real** WateringCanItem
  interactions with grass, bounded at 50,000 uses; capacity is read from the
  actual item. The 1/60 chance is identified in pinned WateringCanItem.java,
  rather than estimated from this nondeterministic sample. Java constructs a
  new Random on each use, so this is reachability evidence, not a seeded trace
  or a statistical proof. No alteration/injection into upstream source is used.

```powershell
./scripts/parity230/run.ps1
cargo run --locked -p minicraft-parity -- java230-generation target/parity/java230/output
cargo run --release --locked -p minicraft-parity -- java230-flora-smoke target/parity/java230/phase2-smoke
```

The last command assembles genuine six-layer Rust worlds at 128/256/512,
checks full native disk save/reload, and reports time, serialized size and tile
buffer allocation (not process RSS). The preview camera centers on an actual
natural flower patch. It does not place flowers or alter terrain for a picture.
Java230Flora ports the pinned surface flower rules; other depths/structures
retain Rust's established 2.2.4 path. It is not a complete Java230 LevelGen port.


## Phase 3: saved-world migration and all raw depths

`Migration230.java`, compiled by the same `run.ps1`, calls the official
`new Save(worldname)` on controlled worlds containing six actual raw LevelGen
maps. It writes `migration-{base,colors,instances,score60}.zip`, actual registry,
inventory-load/global-unlocks JSON, `ornate-wood.json`, and
`generated-depths.json`. ZIP is only compact fixture packaging; the importer
accepts a real directory, never user archives. The controlled maps do not run
all `Level` structure construction. See `AUDIT_JAVA230_PHASE3.md` for strict
supported/refused data and exact gameplay validation boundaries.

```powershell
./scripts/parity230/run.ps1
cargo run --release --locked -p minicraft-parity -- java230-depths target/parity/java230/output
cargo test --release --locked -p minicraft-client --lib migration
# SOURCE is read-only; NEW_DESTINATION must not exist and its parent must exist.
./target/release/minicraft-parity.exe java230-import SOURCE NEW_DESTINATION
# Optional fourth argument: explicitly selected global Java Unlocks.json.
```

`java230-depths` compares every Tile/data cell of 24 maps, stopping at the first
coordinate mismatch: all six depths at128 seeds42/256,256 seed42,512 seed42.
The old surface-eight and all224 goldens remain independent. Raw map equivalence
does not establish full World.initWorld, per-layer seed derivation, structure,
loot, boss layout, or entity AI equivalence. No generation version was renamed.
Ornate Wood oracle invokes real placement/mining and reads real Recipe objects.

## Phase 3.6: actual World.initWorld stages and sheep

```powershell
./scripts/parity230/run-world36.ps1
cargo test --release --locked -p minicraft-client --lib assembly36_tests
cargo test --release --locked -p minicraft-client --lib behavior36_tests
```

JDK17 is required. The wrapper reuses `run.ps1` for complete pinned source and
resource verification, compilation, and all earlier oracles. It then launches
six separate JVMs:128 seeds42/256/0/-1,256 seed42,512 seed42. `WorldProbe36`
adds read-only constructor callbacks using JDK ASM; it never replaces an RNG,
changes a branch or advances game randomness. No reference source is edited.
`WorldStages36` reads the original Level Random48 state, maps, all entities
emitted by official Save.writeEntity, and source constructor chest counters.
There are30 actual observation points per world, in the original call order.

Each compressed `world36-SIZE-SEED.zip` contains the30 raw Tile/Data snapshots,
stage metadata and four official initial Save files. Complete official saves,
classes and unpacked maps remain under ignored `target/parity/java230/world36`.
All six committed fixtures reproduce byte-for-byte entry content. For seed42,
every official Save file also matches the uninstrumented WorldAssembly35 run.
These expected maps come from Java, never Rust. Tests compare every cell, the
constructor RNG states, every persistent entity/loot field, player state,
stairs, and native disk recovery. Old version goldens remain unchanged.

`Behavior36.java`, called by the existing Oracle230 driver, invokes original
Sheep dye/shear/death methods in48 color/difficulty scenarios, observes actual
Level.trySpawn color rolls, and records real KnightStatue/ObsidianKnight room
activation and removed-player behavior. Native tests additionally exercise real
World interaction, last shears durability, grass regrowth, saved color, pending
spawns and actual rendering. The dev2 sheep PNG is an attributed expansion;
the old403 resources are preserved. Java Save omits sheep color: importing it
initializes white exactly as Java Load does, rather than inventing past color.

The new native generation marker is `Java230World`. Normal new-world creation
uses it after this matrix passes; missing markers and historical Java230Flora
retain their old behavior. This does not establish whole-game AI, all themes,
all seeds, transient RNG, or GUI parity. See `AUDIT_JAVA230_PHASE36.md` for the
explicit arena player-edit protection and remaining advanced Spawner limit.

Both source runners now share batched, unfiltered Git blob verification in
`scripts/parity224/verify-source.ps1`: the same per-file byte check with two Git
calls, rather than hundreds of processes. Changed reference bytes still fail.
