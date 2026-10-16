# Minicraft Rust — Rust reconstruction

This repository is a from-scratch Rust reconstruction of
[Minicraft+](https://github.com/MinicraftPlus/minicraft-plus-revived) 2.2.4.
It is licensed under GPL-3.0, like the upstream project.

## Acceptance status

**Stage 15 structure migration, engineering gates, and user play acceptance
are complete (2026-09-12).** The remaining optional seven-dimension items that
still require human / oracle play are listed below.

Historical acceptance checkpoint (2026-09-12):

| Gate | Result |
| --- | --- |
| Root package unit tests | n/a (thin entry) |
| `minicraft-client` lib tests | 240 passed |
| `minicraft-common` lib tests | 6 passed |
| `minicraft-server` unit + e2e | 3 + 2 passed |
| `--self-check` (embedded assets + PCM path) | passed |
| Windows x64 package (`scripts/package.py`) | produced when run |
| Fixed-seed generation / save round-trip / backup recovery | covered by unit tests |
| Two-day logical soak (bounded) | covered by unit test |
| Java 2.2.4 fixture import | covered by unit test |
| Item/tile/potion/tool/achievement/locale registry checks | covered by unit tests |
| Protocol registry order / port 4225 / login / heartbeat e2e | covered |
| Embedded sound WAVE validation | covered in client |

Still required before claiming full stage-15 / 1:1 completion:

- Expanded Java↔Rust trajectory and golden-image matrix beyond current fixtures;
- Human listening on real Windows audio devices for the full effect set.

2026-10-09 hardening audit: atomic save recovery, resource path/ZIP/PNG budgets,
bounded relay connections, optimized-build item consumption, and full-bag death
transfers have regression coverage. See [audit and upstream feature matrix](AUDIT_PARITY_224.md)
for evidence, remaining Java import/parity limitations, and Windows play checks.
The 403 original 2.2.4 resource files still match byte for byte.

2026-10-16 feature branch: Java 2.3.0-dev colored dyes, wool, beds, Dye Vat,
and placeable flowers are integrated on top of the verified 2.2.4 fixes.
See [phase-one implementation and compatibility record](AUDIT_JAVA230_PHASE1.md)
for actual Java comparisons, native-save guarantees, remaining natural flower
sources, and Windows play checks. This does not port Infdev or full Java 2.3 worlds.

## What currently runs

2026-09-18 local fixes: the controls guide now uses Java's line wrapping and
white entry text, the language list is centered, the resource-pack page again
shows its default pack/check mark/logo, and exhausted tools are removed from
the inventory when an attack uses their last durability. The title splash now
changes on each new title screen using Java 2.2.4 text. The title uses the
bundled Minicraft pixel lettering with a new `RUST` suffix. Windows builds embed
`assets/branding/minicraft-rust.ico` into the EXE and uses it for the game
window (Windows SDK `rc.exe` is required). The ICO contains independently
rendered pixel-sharp sizes from 16 to 256 pixels. To regenerate the branding
assets, install Pillow and run
`python branding/generate.py`; normal builds do not require Pillow.

2026-09-19 container fix: keyboard pane navigation now follows Java's
`CURSOR-*` controls, the focused player/chest panel is visibly distinguished,
and controllers can switch panels with the D-pad/stick or shoulder triggers.

The root `minicraft-rust` package is a thin binary entry that launches the
client in `minicraft-client`. It opens a native window with the original
288×192 pixel canvas and local 2.2.4 artwork.

Covered systems (implemented and under test, not a completion claim):

- Title / play / worlds / new-world / options / help / book / achievements /
  skins / resource packs / controls / languages / control-guide menus on the
  client `Menu` stack;
- Deterministic six-depth generation, tile registry 0–58, lighting overlay,
  player/mob/combat/items/recipes/furniture/progression/save formats;
- Java save import, atomic Rust snapshots, resource packs, remappable keys;
- PCM mixing + WASAPI on Windows;
- JSON-lines multiplayer relay on port 4225 (`--server`, `--multiplayer-probe`,
  or `cargo run -p minicraft-server`). No multiplayer world-sync UI yet.

The crate tree under `crates/` holds the staged Java-parity modules
(`minicraft-common` / `minicraft-client` / `minicraft-server` /
`minicraft-parity`). See [workspace layout](crates/README.md).

## Repository and distribution scope

GitHub Actions run format, build, workspace tests, Clippy, optimized item-consumption
regressions, and a headless self-check on Windows, Linux, and macOS. The new matrix
has not yet run remotely; the local audit was verified on Windows x64 only.
Release archives remain locally assembled; CI does not create packages, tags, or releases.

This repository includes game source, Cargo manifests and lockfile, copied runtime
assets, license, and the local packaging script. Local `tests/`,
`tools/`, Java reference copies, internal audit/plan notes, save data, logs,
generated images, and build/package output are excluded.

## Build and run

```powershell
cargo run --release
```

Validate embedded runtime data and the shared PCM path without a window:

```powershell
cargo run --release -- --self-check --savedir .\test-data
```

Windows device refill timing (silent):

```powershell
cargo run --release -- --audio-probe
```

Standalone multiplayer relay:

```powershell
cargo run --release -p minicraft-server -- 0.0.0.0:4225
cargo run --release -- --multiplayer-probe 127.0.0.1:4225 PLAYER_NAME
```

Tests and lint:

```powershell
cargo test --workspace --locked
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
```

Deterministic Windows archive:

```powershell
cargo build --release --locked
python scripts\package.py --platform windows-x86_64 --binary target\release\minicraft-rust.exe
```

Renderer diagnostics:

```powershell
cargo run --release -- --render-preview title.png --savedir .\test-data
cargo run --release -- --render-world-preview world.png --savedir .\test-data
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

Primary keys can be changed from **Options → Key Bindings**. Arrow keys remain
available for menu navigation and movement.

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
