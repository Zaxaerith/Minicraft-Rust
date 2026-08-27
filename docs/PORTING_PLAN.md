# Minicraft+ 2.2.4 Rust porting plan

## Ground rules

- Behavioral baseline: upstream `2.2.X` commit
  `47227a89fe05f1e3fe962f070f25c53111254a1f`.
- Release cross-check: official `minicraft-plus-2.2.4.jar` and the 2.2.4 release
  note (resource-pack startup fix).
- No Java runtime, JNI, embedded JAR, or source translation at runtime.
- No resource path may escape this repository. Upstream resources are copied
  into `assets/` before use.
- Every phase ends with tests, `cargo clippy -- -D warnings`, and a release
  build. Compatibility claims are added only after a behavior is exercised.

## Source-to-Rust map

| Java 2.2.4 area | Rust destination | Status |
| --- | --- | --- |
| `core`, `Updater`, `Renderer` | `app`, `gfx`, fixed-step loop | Foundation complete |
| `core/io/InputHandler` | `input` | Remappable keyboard input complete |
| `gfx/*` | `gfx`, `assets` | Pixel/image/font foundation complete |
| `screen/*` | state-specific UI modules | Front-end hierarchy and gameplay overlays complete |
| `level/LevelGen` | `world/generation` | Six-depth base terrain and stair linking complete |
| `level/tile/*` | `world/tile` and behavior modules | IDs 0–58 and textures complete; behaviors in progress |
| `entity/*` | entity arena and typed components | Player slice started |
| `item/*` | item registry, inventory, recipes | Pending |
| `saveload/*` | versioned serde save layer | Pending |
| `network/*`, server | protocol/client/server crates | Pending |

## Ordered phases

### 1. Baseline and asset custody — complete

- Read upstream README, build configuration, changelog, entry point, rendering,
  world, level, tile, player, localization, and resource-pack code.
- Resolve the broken 2.2.4 tag and pin the verified commit.
- Inventory the baseline: 210 client Java files and 403 resource files.
- Copy all 403 resources (1,764,651 bytes) and GPL-3.0 license locally.

### 2. Rust platform and rendering foundation — complete

- Native `minifb` window at the original 288×192 logical resolution.
- PNG decoding, alpha blending, sprite-region blitting, bitmap font, primitives.
- Keyboard input, menus, game-state transitions, pause/inventory overlays.
- Offline-compilable dependency lockfile and strict lint gate.

### 3. Front end, settings, localization, resource packs — complete

- [x] Persist display, audio, difficulty, locale, skin, resource-pack order,
  world size/theme/type, tutorial, and quest settings.
- [x] Load every built-in locale with English fallback and format substitution.
- [x] Load folder and ZIP resource packs in priority order, validate metadata,
  and hot-reload migrated sprites/localization without a startup crash.
- [x] Implement the four built-in skins and validated custom skin discovery.
- [x] Complete the title/help/achievement/world-selection hierarchy, including
  original paginated books, the achievement registry, and seed-backed world
  records. Full world-state persistence remains in phase 7.
- [x] Add interactive input remapping; controller support remains grouped with
  the complete input/audio work in phase 7.

Acceptance: every front-end screen is reachable and a malformed pack is
reported and skipped without terminating the game.

### 4. Complete world, tiles, time, and lighting

- [x] Port base terrain for all six depths (`-4..1`), Java-compatible seed
  behavior, validation guards, stairs linking, spawning, and level transitions.
- [ ] Port every 2.2.4 structure and finish exact per-depth validation parity.
- [x] Register all tile IDs 0–58 and load their local copied textures.
- [ ] Port legacy compatibility ID mappings and every tile interaction.
- [x] Port the 64,800-tick four-quarter day clock and depth-aware player light
  mask.
- [ ] Port tile ticking/data, full multi-source light maps, connective texture
  composition, and difficulty-aware spawning.

Acceptance: fixed seeds satisfy per-depth snapshot/hash fixtures and all tiles
pass interaction/serialization tests.

### 5. Entity, combat, item, and crafting loop

- Entity storage/lifecycle, collision, sorting, particles, projectiles, drops.
- Player health/stamina/hunger/armor/effects/death/respawn.
- Every passive/hostile/boss mob and furniture entity.
- Full item registry, stack limits, tools, durability, potions, fishing,
  inventory containers, recipes, and crafting stations.

Acceptance: a survival world supports the original gather → craft → mine →
boss progression without debug intervention.

### 6. Modes and content systems

- Survival, creative, hardcore, and score modes.
- Farming, beds, signs, quests, tutorials, achievements, books, skins, and
  resource-pack UI.
- Story progression and both bosses.

Acceptance: data-driven content counts and progression flags match 2.2.4.

### 7. Save/audio/network parity

- Preferences/unlocks/world save and load with atomic writes and backups.
- Read 2.2.4 Java saves; preserve documented legacy migrations.
- All ten sound effects, volume toggle, and controller support.
- Multiplayer protocol plus standalone Rust server corresponding to the Java
  common/server modules.

Acceptance: round-trip fixtures, Java-save imports, two-client multiplayer, and
server soak tests pass.

### 8. Release parity and hardening

- Behavior checklist against the official 2.2.4 JAR.
- Determinism, fuzz, malformed-resource/save, long-run, and performance tests.
- Windows, Linux, and macOS release packaging with bundled local assets.
- Final license/credits and reproducible build documentation.

Acceptance: no known phase checklist gaps, clean strict lint/test builds, and a
documented compatibility report.
