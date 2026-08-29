# Minicraft Rust — Rust reconstruction

This repository is a from-scratch Rust reconstruction of
[Minicraft+](https://github.com/MinicraftPlus/minicraft-plus-revived) 2.2.4.
It is licensed under GPL-3.0, like the upstream project.

## Current playable build

The current vertical slice opens a native window with the original 288×192
pixel canvas and copied 2.2.4 artwork. It includes:

- the original title and bitmap font assets;
- keyboard-driven title, play/world selection, options, help books,
  achievements, pause, and inventory screens;
- deterministic six-depth (`-4..1`) 128/256/512 generation based on the 2.2.4
  `LevelGen` noise implementation and a compatible `java.util.Random`, with all
  five themes, four terrain types, continuous validation, and tile structures;
- stair-linked level transitions and the complete tile ID 0–58 registry using
  copied local assets, legacy ID mappings, tile data/ticks, data-driven variants,
  and connective textures;
- the original 64,800-tick morning/day/evening/night clock with multi-source
  player/lava/torch lighting above and below ground;
- player movement, tile-aware collision, water slowdown, stamina, progressive
  tile damage, doors, stairs, health/contact damage, death, and surface respawn;
- Java-compatible hunger decay from time, movement, low stamina, and healing;
  food consumption, passive high-hunger healing, difficulty-specific starvation
  floors, and separate health/stamina/hunger HUD meters;
- per-level entity arenas with stable IDs, lifecycle/despawn, collision, Y-sorted
  rendering, difficulty/depth/time/light-aware natural spawning, and original
  mob caps;
- nine naturally spawning species plus the Air Wizard and Obsidian Knight, with
  copied local sprites, drops, species state machines, arrows, boss projectiles,
  creeper explosions, sheep shearing, and phase-aware boss combat;
- the complete 141 Java non-tool identities plus its recipe-only Arcane
  Fertilizer output, eight tool kinds and
  all applicable tiers, with copied item/tool sprites, Java durability/damage
  formulas, armor, twelve potions, four fishing rods, buckets, clothing,
  fertilizing, watering, planting, and harvesting;
- every Java 2.2.4 recipe group through scrollable hand/workbench/oven/furnace/
  anvil/enchanter/loom menus, including transactional tool and claymore costs;
- placeable crafting furniture, lanterns, beds, TNT, composters, ordinary and
  locked dungeon chests, nine spawner kinds, and the dungeon boss statue;
- populated sky/village/cave/dungeon structures and a connected survival
  gather → craft → mine → boss progression, including both boss drops and
  resummoning items;
- survival, creative, hardcore, and timed score world modes, with creative
  invulnerability/non-consumption, hardcore terminal death, and score timer,
  multiplier, mob/crop/pickup scoring, and death penalty;
- five data-driven tutorial steps, four quest groups containing fourteen
  quests and rewards, all seventeen achievement flags, configurable progress
  HUD visibility, and boss-linked story completion;
- night-only surface sleeping, editable per-level signs, world-readable book
  items (including the Antidious volume), and safe furniture pickup;
- pickaxe-gated iron/gold/gem/lapis/cloud ore damage and collectible ore drops,
  plus axe/shovel/hoe ground work, crop loops, sand/cloud gathering, and
  gem-pickaxe hard rock;
- all 16 bundled localization files with English fallback;
- persistent FPS, difficulty, sound, autosave, locale, skin, tutorial/quest
  flags, and active world size/theme/terrain/mode/score-time settings, including
  `--savedir` support;
- versioned full-world snapshots covering all six levels, entities, player,
  inventory, mode, signs, progression, and deterministic RNG state, with
  periodic autosave, atomic replacement, and automatic backup recovery;
- first-load import of local Java 2.2.4 save directories, including named and
  legacy tile mappings, items/tools, furniture/container contents, mobs,
  potion effects, signs, and unlocked achievements; imported worlds are then
  stored wholly inside the Rust save directory;
- folder and ZIP resource-pack discovery, validation, enable/disable, priority
  ordering, texture/localization overrides, and malformed-pack isolation;
- the four bundled skins plus validated custom PNG skins from the local game
  directory;
- persistent remappable keyboard controls and reset-to-default support, plus
  controller direction/left-stick and action controls through XInput on Windows
  and the embedded SDL mapping database on Linux/macOS;
- all ten copied 2.2.4 WAV effects, connected to menus and gameplay and governed
  by the persistent sound toggle;
- a versioned JSON-lines TCP protocol on the original port 4225, a standalone
  Rust relay server, login/presence/state/heartbeat handling, and a client
  connectivity probe.

The eight-phase reconstruction plan is complete.

## Build and run

```powershell
cargo run --release
```

The release executable can validate all embedded runtime data without opening a
window:

```powershell
cargo run --release -- --self-check --savedir .\test-data
```

To run the standalone multiplayer service and verify it from another process:

```powershell
cargo run --release -- --server 0.0.0.0:4225
cargo run --release -- --multiplayer-probe 127.0.0.1:4225 PLAYER_NAME
```

The dependency set is intentionally small. Tests and strict linting can be run
with:

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
```

To build and create a deterministic Windows archive:

```powershell
cargo build --release --locked
python scripts\package.py --platform windows-x86_64 --binary target\release\minicraft-rust.exe
```

The GitHub workflows apply the locked test, lint, build, self-check, and package
gates to Windows, Linux, and macOS.

For renderer diagnostics without opening a window:

```powershell
cargo run --release -- --render-preview title.png --savedir .\test-data
cargo run --release -- --render-world-preview world.png --savedir .\test-data
cargo run --release -- --render-world-preview cave.png --depth -2 --savedir .\test-data
cargo run --release -- --render-world-preview entities.png --entities --savedir .\test-data
cargo run --release -- --render-world-preview workbench.png --workbench-ui --savedir .\test-data
cargo run --release -- --render-world-preview food.png --food-ui --savedir .\test-data
cargo run --release -- --render-world-preview stations.png --stations --savedir .\test-data
cargo run --release -- --render-ui-preview achievements achievements.png --savedir .\test-data
cargo run --release -- --render-ui-preview controls controls.png --savedir .\test-data
cargo run --release -- --render-ui-preview options options.png --savedir .\test-data
cargo run --release -- --render-world-preview score.png --score-ui --progress-ui --savedir .\test-data
cargo run --release -- --render-world-preview book.png --book-ui --savedir .\test-data
cargo run --release -- --render-world-preview sign.png --sign-ui --savedir .\test-data
```

## Controls

| Action | Keys |
| --- | --- |
| Move/menu | Arrow keys or W/A/S/D |
| Select / craft | Enter |
| Attack / harvest / eat | C |
| Inventory | X |
| Equip inventory item/tool | C while inventory is open |
| Place equipped crafting station | C |
| Use placed crafting station | Enter while facing it |
| Pick up furniture | V while facing it |
| Read equipped book | C |
| Edit a sign | Enter while facing it; type, Backspace, then Enter to save |
| Pause/back | Escape |

Controllers use the D-pad or left stick to move, A to select, B to pause/back,
X to attack, Y or Start for inventory, and the left shoulder button to pick up
furniture.

The primary keys can be changed from **Options → Key Bindings**. Arrow keys
remain available for menu navigation and movement.

## Version baseline

The upstream `v2.2.4` Git tag currently points at commit `42ba919`, whose source
identifies itself as `2.3.0-infdev3`. The verified 2.2.4 source baseline is the
upstream `2.2.X` branch at commit
`47227a89fe05f1e3fe962f070f25c53111254a1f`; both `build.gradle` and
`Game.VERSION` identify that tree as 2.2.4. A read-only copy is kept under the
ignored `.reference/` directory while porting.

All 403 upstream client resource files have been copied into this repository's
`assets/` directory. The Rust executable never loads source or resources from
the sibling Java repository.
