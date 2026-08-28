# Phase 5 completion report: entities, combat, items, and crafting

Phase 5 is complete against the Rust porting boundary. All gameplay resources
used by this phase are compiled from this repository's copied `assets/` tree;
the executable never reads the sibling Java checkout.

## Entity and combat layer

- Every level owns a stable-ID entity arena with deferred spawns, collision,
  despawn, Y-sorted rendering, item pickup, and local furniture storage.
- All nine natural species plus the Air Wizard and Obsidian Knight are typed
  mobs with health, pursuit/wandering, contact damage, drops, hurt feedback,
  and copied local sprites.
- Species state covers slime jump windows, skeleton arrows, creeper fuses and
  explosions, sheep shearing/regrowth, spawner timers, both boss phases, boss
  radial projectiles, and Obsidian Knight projectile blocking.
- Arrow, spark, fire-spark, smash, and fire entities have finite lifetimes and
  collision/damage behavior. Equipped item/tool art is visible during attacks.
- Armor absorption/durability, hurt cooldown, death drops, surface respawn,
  hunger, stamina, food, lava damage, and all twelve potion types are integrated
  into the normal player update path. The HUD scales health to upgraded maximum
  health and displays armor and active timed effects.

## Items, tools, stations, and survival loops

- The registry covers all 141 non-tool Java item identities plus the Arcane
  Fertilizer output referenced only by the Java recipe table, including the five
  creative tile items and compatibility Power Glove identity. Tools remain
  non-stackable typed entries: eight kinds, five tiers where applicable, exact
  durability/damage formulas, and local sprites for every combination.
- Recipe groups now match the Java 2.2.4 source: 6 hand recipes, 31 workbench
  stack recipes, 12 workbench tool recipes, 4 furnace, 5 oven, 17 loom, 7 anvil
  stack, 22 anvil tool, and 14 enchanter recipes. Claymores consume their
  matching sword transactionally.
- Workbench, oven, furnace, anvil, enchanter, loom, three lanterns, chest,
  dungeon chest, TNT, bed, composter, statue, and nine spawner kinds can exist
  as local furniture entities. Crafting stations expose scrollable menus;
  containers transfer local inventory; dungeon chests require keys; composters
  make fertilizer; TNT explodes; beds advance time; and statues awaken the
  dungeon boss.
- Village/cave/dungeon generation now materializes its chest and spawner
  occupants. The sky house contains the Air Wizard and the dungeon boss room
  contains the Obsidian Knight statue.
- Tile-item placement, buckets, shovel/hoe/pickaxe ground work, saplings,
  crop planting/growth/fertilizing/watering/harvest, sheep wool, fishing rods,
  and all ore/wood/stone/sand/cloud gathering paths feed the same inventory and
  recipe system.
- Air Wizard and Obsidian Knight fights drop their required progression items;
  totems/poppets resummon them under the original location and active-boss
  restrictions, and the Obsidian Heart raises maximum health.

## Acceptance evidence

- `cargo test -- --test-threads=1`: 46 passed, 0 failed.
- `cargo clippy --all-targets -- -D warnings`: passed.
- `cargo build --release`: passed.
- `target/phase5-previews/phase5-complete.png` verifies the eleven mob sprites,
  depth sorting, boss overlays, and HUD from repository-local assets.
- `target/phase5-previews/phase5-workbench.png` verifies item/tool sprites,
  durability, and the expanded station menu.

The automated acceptance coverage exercises gather/craft/mine components,
armor and potions, farming, structure occupants, projectiles/fuses, dungeon
container unlocking, both boss drops, and every recipe-group count. The normal
survival input path connects those components without debug-only item grants.
