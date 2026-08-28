# Phase 4 completion report

Phase 4 ports the world/tile/time/lighting boundary of the verified Minicraft+
2.2.4 source at commit `47227a89fe05f1e3fe962f070f25c53111254a1f`.

## Delivered

- Six deterministic depths (`1, 0, -1, -2, -3, -4`) with Java-compatible RNG,
  original validation thresholds, linked stairs, and fixed-seed map hashes.
- 128, 256, and 512 world sizes; Normal, Forest, Desert, Plain, and Hell themes;
  Island, Box, Mountain, and Irregular terrain presets.
- All tile portions of the 2.2.4 structures: Air Wizard house, village variants
  and ruined overlays, cave spawner rooms, dungeon spawner/garden/chest rooms,
  gates, lock, boss room, and lava-pool variants.
- Tile IDs 0–58, all legacy ID aliases, tile data, door state, flower/crop/farm
  variants, environmental random ticks, fluid filling/solidification, sapling
  growth, crop growth/fertilization, and damage decay.
- Player collision and bare tile damage for the relevant tile classes. Tool and
  item-specific interactions, drops, furniture, and structure occupants remain
  intentionally coupled to the phase 5 item/entity implementation.
- Original 64,800-tick day cycle, connective 8×8 tile rendering, and combined
  player/lava/torch light masks with source-specific radii.
- Original difficulty/depth mob caps and natural-spawn eligibility/selection
  rules. Spawn requests are queued for materialization by phase 5 entities.

## Verification

- `cargo test`: 26 passed.
- `cargo clippy --all-targets -- -D warnings`: passed with zero warnings.
- `cargo build --release`: passed.
- Visual previews inspected for every depth plus the New World preset screen.
- `assets/` contains 403 copied source files totaling 1,764,651 bytes.
- Runtime source contains no path to the sibling Java repository; compiled
  resources resolve only through this repository's `assets/` tree, while custom
  packs/skins/saves resolve through the selected local game directory.

Diagnostic PNGs are generated under `target/phase4-previews/` and are not source
dependencies.
