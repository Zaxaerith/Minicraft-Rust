# Phase 7 save, audio, controller, and network report

## Save ownership and durability

The native save is a serde snapshot tagged with format `2` and game version
`2.2.4-rust`. It contains the complete runtime `World`, including every level
tile/data vector, pending spawns and entity arenas, the player and inventory,
active item/tool durability, time/mode/score state, signs, content progression,
boss/story flags, and the Java-compatible RNG state.

Settings, new-world metadata, and world state all use the same local atomic
write path: write and sync a `.new` file, move the previous primary to `.bak`,
then rename the new primary into place. Reads try the primary followed by the
backup. Worlds save on creation, every 1,800 active ticks when autosave is
enabled, on return to title, and when the window closes.

No save or runtime asset path crosses into the sibling Java project.

## Java 2.2.4 migration

Directories containing `Game.miniplussave` and the six `LevelN` files are
listed beside native worlds. Their first load imports:

- seed, difficulty, mode, score time, clocks, and both boss flags;
- all six tile/data planes, including Java's x-outer serialization transpose,
  old numeric IDs, old torch-underlay names, and renamed tile aliases;
- player health/extra health, hunger, armor, score, level, and potion effects;
- active and stored stack items, tools/durability, watering-can content,
  furniture, chest contents, mobs, sheep state, and spawners;
- per-level sign JSON and global `Unlocks.json` achievements.

The original Java files are not modified. A successful import is atomically
written as the Rust world's `state.json`, so subsequent runtime loading is
entirely local to the Rust repository/save directory.

## Audio and controller input

The ten upstream WAV files are copied under `assets/assets/sound` and embedded
at compile time. Menu select/confirm and the craft, pickup, player-hurt,
monster-hurt, fuse, explosion, death, and boss-death gameplay paths emit typed
events. The existing sound setting suppresses playback immediately when off.

On Windows the backend uses the system `winmm` RIFF-memory player. XInput is
loaded dynamically from `xinput1_4.dll` with an `xinput9_1_0.dll` fallback, so
building does not require the optional XInput SDK import library. D-pad and
left-stick directions are continuous with edge-triggered menu movement; A, B,
X, Y/Start, and left shoulder map to select, back, attack, inventory, and
pickup. Keyboard controls remain active simultaneously. Other platform audio
and controller backends are part of the phase 8 packaging matrix.

## Multiplayer transport

The Rust protocol keeps the upstream port `4225` and all 33 Java 2.2.4
`InputType` values. The implemented transport is bounded newline-delimited JSON
with an explicit protocol version. It supports validated login, initialization,
username/presence lists, join/leave broadcasts, latest player-state replay and
movement broadcast, notifications, ping/pong, and orderly disconnect cleanup.

Run a server and a connectivity probe with:

```powershell
cargo run --release -- --server 0.0.0.0:4225
cargo run --release -- --multiplayer-probe 127.0.0.1:4225 PLAYER_NAME
```

The upstream 2.2.4 `Server.java` is empty and its client tree retains only the
protocol registry, so this is a Rust-native completion of that dormant module,
not a claim that the JSON wire encoding interoperates with an existing Java
server.

## Acceptance evidence

- Native complete-state serialize/deserialize round trip and malformed-vector
  rejection.
- Atomic settings/world backup tests and corrupt-primary recovery.
- Java 2.2.4 fixture import covering levels, player, active inventory/tool,
  mobs/furniture, signs, potions, and achievements.
- All ten embedded resources assert valid RIFF/WAVE images.
- Controller direction and action edge behavior is deterministic under test.
- Two loopback TCP clients exchange presence and player state, then complete
  512 ordered ping/pong iterations and disconnect cleanup.
- Full suite: 59 passed, 0 failed.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo build --release`: clean.
