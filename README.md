# Minicraft+ 2.2.4 — Rust reconstruction

This repository is a from-scratch Rust reconstruction of
[Minicraft+](https://github.com/MinicraftPlus/minicraft-plus-revived) 2.2.4.
It is licensed under GPL-3.0, like the upstream project.

## Current playable build

The current vertical slice opens a native window with the original 288×192
pixel canvas and copied 2.2.4 artwork. It includes:

- the original title and bitmap font assets;
- keyboard-driven title, play/world selection, options, help books,
  achievements, pause, and inventory screens;
- deterministic six-depth (`-4..1`) 128×128 generation based on the 2.2.4
  `LevelGen` noise implementation and a compatible `java.util.Random`;
- stair-linked level transitions and the complete tile ID 0–58 texture
  registry using copied local assets;
- the original 64,800-tick morning/day/evening/night clock with surface and
  underground player-centered darkness;
- player movement, collision, water slowdown, stamina, tree harvesting, and a
  minimal inventory/HUD.
- all 16 bundled localization files with English fallback;
- persistent FPS, difficulty, sound, autosave, locale, skin, and future world
  settings, including `--savedir` support;
- folder and ZIP resource-pack discovery, validation, enable/disable, priority
  ordering, texture/localization overrides, and malformed-pack isolation;
- the four bundled skins plus validated custom PNG skins from the local game
  directory;
- persistent remappable keyboard controls and reset-to-default support;
- local seed-backed world records, ready for full state serialization in the
  save/load phase.

This is not yet a feature-complete replacement for the Java game. The exact
remaining work and acceptance gates live in [docs/PORTING_PLAN.md](docs/PORTING_PLAN.md).

## Build and run

```powershell
cargo run --release
```

The dependency set is intentionally small. Tests and strict linting can be run
with:

```powershell
cargo test
cargo clippy --all-targets -- -D warnings
```

For renderer diagnostics without opening a window:

```powershell
cargo run --release -- --render-preview title.png --savedir .\test-data
cargo run --release -- --render-world-preview world.png --savedir .\test-data
cargo run --release -- --render-world-preview cave.png --depth -2 --savedir .\test-data
cargo run --release -- --render-ui-preview achievements achievements.png --savedir .\test-data
cargo run --release -- --render-ui-preview controls controls.png --savedir .\test-data
```

## Controls

| Action | Keys |
| --- | --- |
| Move/menu | Arrow keys or W/A/S/D |
| Select | Enter |
| Attack/harvest | C |
| Inventory | X |
| Pause/back | Escape |

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
