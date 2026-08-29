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
| `screen/*` | state-specific UI modules | Phase 6 complete: front end, gameplay, content, mode, and progression overlays |
| `level/LevelGen`, `Structure` | `world/generation`, `world/structure` | Phase 4 terrain, validation, structures, and stair linking complete |
| `level/tile/*` | `world`, `world/tile_behavior` | Phase 4 registry, data, ticks, collision, rendering, and legacy IDs complete |
| `entity/*` | `world/entity` arena and typed entities | Phase 5 complete: natural mobs, bosses, combat, projectiles, drops, furniture, and occupants |
| `item/*` | `item` registry, inventory, recipes | Phase 5 complete: full catalog, tools, stations, armor, potions, fishing, containers, and placement |
| `saveload/*` | versioned serde save layer | Phase 7 complete |
| `network/*`, server | protocol/client/server modules | Phase 7 complete |

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

### 4. Complete world, tiles, time, and lighting — complete

- [x] Port base terrain for all six depths (`-4..1`), Java-compatible seed
  behavior, exact continuous validation, stairs linking, natural-spawn intents,
  and level transitions for 128/256/512 worlds and all themes/terrain types.
- [x] Port every 2.2.4 tile structure, including villages, cave spawner rooms,
  dungeon rooms/gates/lock/boss room/lava pools, and the Air Wizard house.
  Their furniture/entity occupants are created by the phase 5 entity layer.
- [x] Register all tile IDs 0–58 and load their local copied textures.
- [x] Port every 2.2.4 legacy compatibility ID mapping and tile-side collision,
  bare damage, doors, fluid, sapling, farm, crop, and damage-decay behavior.
  Tool/item costs, drops, and recipes remain together with items in phase 5.
- [x] Port the 64,800-tick four-quarter day clock and depth-aware player light
  mask.
- [x] Port random tile ticking/data, data-driven variants, multi-source player/
  lava/torch light maps, connective 8×8 texture composition, and the original
  difficulty/depth/time/light-aware natural-spawn rules and mob caps.

Acceptance met: fixed seeds satisfy six per-depth hash fixtures; all 59 current
IDs and every legacy ID round-trip through registry tests; tile data and core
interactions are exercised; six depth previews pass visual inspection. Detailed
evidence is recorded in [PHASE4_REPORT.md](PHASE4_REPORT.md).

### 5. Entity, combat, item, and crafting loop — complete

- [x] Add stable entity IDs, per-level storage, deferred natural-spawn
  materialization, lifecycle/despawn, tile collision, Y sorting, and item drops.
- [x] Add the nine naturally spawned 2.2.4 species with local copied sprites,
  passive wandering, hostile pursuit/contact damage, health, melee damage, and
  Java drop categories.
- [x] Add stack inventory semantics, transactional costs, pickup entities, all
  Java hand recipes, and an interactive crafting pane.
- [x] Port all Java tool kinds/tiers, sprites, durability/damage formulas, and
  recipes; make the workbench a placeable, colliding, usable furniture entity.
- [x] Generalize crafting furniture to workbench, oven, furnace, anvil,
  enchanter, and loom; add their local sprites and placement/use path; and
  connect furnace ingots to iron/gold/gem sword, axe, and pickaxe recipes.
- [x] Add player hurt cooldown, death, inventory drops, and surface respawn.
- [x] Add the 2.2.4 hunger/food loop: difficulty-specific time, movement,
  stamina, healing, and starvation behavior; all ten food values; oven food
  recipes; golden apples; consumption costs; and a dedicated HUD meter.
- [x] Add armor, all potion effects, particles, projectiles, and
  species-specific AI/attacks.
- [x] Add every passive/hostile/boss mob and furniture entity, including
  structure occupants.
- [x] Complete the item registry and tool recipes/sprites, expand the
  six stations to every Java recipe, then
  add fishing, containers, and placement for other furniture/tile items.

Acceptance met: a survival world supports the original gather → craft → mine →
boss progression without debug intervention. The 46-test suite, strict Clippy
gate, release build, and visual diagnostics are recorded in
[PHASE5_PROGRESS.md](PHASE5_PROGRESS.md).

Phase 5 completion evidence is recorded in
[PHASE5_PROGRESS.md](PHASE5_PROGRESS.md).

### 6. Modes and content systems — complete

- [x] Add survival, creative, hardcore, and score world modes, including the
  original score time choices, timer, multiplier, scoring events, death
  penalty, creative inventory/invulnerability/non-consumption, and hardcore
  terminal death.
- [x] Complete farming interactions and add usable night-only surface beds,
  editable per-level signs, furniture pickup, and world-readable regular and
  Antidious books.
- [x] Parse and execute the copied 2.2.4 tutorial and quest resources: five
  tutorial steps and four groups containing fourteen quests, with parent/
  unlocking criteria, event history, rewards, settings toggles, and HUD status.
- [x] Connect all seventeen bundled achievements to runtime events and retain
  the phase 3 skin/resource-pack interfaces.
- [x] Present Air Wizard and Obsidian Knight victory state, gated obsidian/boss
  construction, score rewards, achievements, and final story completion.

Acceptance met: bundled counts are asserted as 5 tutorials, 4 quest groups,
14 quests, and 17 achievements; progression events and all four mode branches
are covered by the 50-test suite. Strict Clippy, release build, and local-asset
visual evidence are recorded in [PHASE6_REPORT.md](PHASE6_REPORT.md). Runtime
state persistence remains deliberately assigned to phase 7.

### 7. Save/audio/network parity — complete

- [x] Persist preferences and complete world/unlock state through versioned
  snapshots, atomic replacement, previous-file backups, autosave, exit save,
  validation, and primary-to-backup recovery.
- [x] Detect and import 2.2.4 Java saves on first load, including the Java
  x/y serialization transpose, named/legacy/torch tiles, player statistics,
  modes, inventory/tools, furniture/container contents, mobs, signs, potion
  effects, and global achievement unlocks.
- [x] Embed and connect all ten copied sound effects behind the persistent
  sound toggle; add native Windows XInput D-pad/left-stick and action mappings
  alongside the remappable keyboard path without an SDK-link dependency.
- [x] Retain the complete Java 2.2.4 protocol registry and port 4225, then add a
  bounded versioned JSON-lines transport, login validation, presence, latest
  player-state replay/broadcast, heartbeat, disconnect cleanup, standalone
  Rust server, and client probe.

Acceptance met: native round trips and backup recovery, a complete Java 2.2.4
import fixture, all ten WAV headers, controller edge semantics, two real TCP
clients, and a 512-heartbeat server soak pass in the 59-test suite. Strict
Clippy and release-build evidence is recorded in
[PHASE7_REPORT.md](PHASE7_REPORT.md).

### 8. Release parity and hardening — complete

- [x] Pin and audit the official 2.2.4 JAR, verify all 403 copied resources,
  and publish the behavior/intentional-difference checklist.
- [x] Add deterministic save/resume, 512 malformed-snapshot mutations, bounded
  malformed/oversized save/resource/protocol cases, and a 129,600-tick soak.
- [x] Add SDL2 audio/controller backends for Linux and macOS while retaining
  WinMM/XInput on Windows; embed all runtime assets on every target.
- [x] Add Windows, Linux, and macOS locked CI/release matrices, headless
  embedded-resource smoke checks, and deterministic self-contained ZIPs.
- [x] Complete license/credits, compatibility boundaries, baseline identity,
  and reproducible package metadata.

Acceptance met locally: the official artifact and all 403 resources pass
custody audit; 65 tests pass; formatting, strict Clippy, and release build are
clean; a Windows archive reproduces byte-for-byte across successive builds;
and its extracted executable passes `--self-check` from outside the source
tree. The committed CI/release matrices apply the same gates to Windows, Linux,
and macOS. Detailed evidence and compatibility boundaries are recorded in
[PHASE8_REPORT.md](PHASE8_REPORT.md) and
[COMPATIBILITY.md](COMPATIBILITY.md).
