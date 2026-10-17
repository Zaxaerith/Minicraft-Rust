# Genuine Java 2.3.0-dev fixtures

Produced by `scripts/parity230/run.ps1` / `Oracle230.java` by compiling and running
unmodified Minicraft+ source commit `4dc00332e24a727b96be7b885d5cbbdd9c85263c`,
whose internal version is `2.3.0-dev2`. Full provenance, replay instructions and
scope are in `scripts/parity230/README.md`. The dependency jar is used after all
fresh source classes and fixed resources, so its older game implementation does
not supply oracle results. Data and source attribution: Minicraft+ / GPL-3.0.

The files contain real method outputs, not Rust test expectations transcribed as
Java results. Compare semantic values; JSON object member order is immaterial.
Java tile IDs are intentionally distinct from append-only compatible Rust IDs.
`operations.json` also records the upstream Violet-shovel wrong-drop bug; Rust
preserves the harvested variant rather than reproducing that data-loss bug.


Phase 2 adds `generated-flora.json` (actual raw LevelGen surface counts, hashes,
and normal-128 flower coordinates) and `watering-flora.json` (actual item-use
reachability for all19 species; Java's time-seeded interactions are not a seeded
cross-language trace). The 1/60 denominator is source evidence, not measured
frequency. Full replay TSV files remain ignored under target. Existing Phase1
fixtures, item/tile IDs and all 2.2.4 goldens are unchanged.


Phase3 `Migration230.java` invokes the official public `new Save(worldname)`:
`migration-base.zip` stores six actual raw LevelGen maps (per-layer seeds42..47)
with a controlled arrival patch; `migration-colors.zip` adds real held/inventory,
flower/wool/Torch/Sign/stair, Bed/DyeVat/chest/death-chest and passive-mob state;
`migration-instances.zip` contains genuinely different WateringCan states plus
an unsupported level3 Zombie, and must be refused. The ZIPs are small **test
packaging** of unchanged actual Save files, not a supported user archive format.
The generator does not run a complete GUI session or claim full Level constructor
structure parity. Exact compressed bytes may differ due advancement timestamps;
replay comparisons concern saved semantics and recorded controlled fields.

`migration-registry.json` records the actual Items/Tiles registration and Java
array-to-depth mapping. `migration-operations.json` executes official
Load.loadInventory and records that the saved held item moves into inventory,
without restoring held selection. `migration-unlocks.json` comes from official
Save.writeUnlocks after the actual achievement API. `generated-depths.json`
contains all six raw LevelGen name/data hashes for128 seeds42/256 and256/512
seed42; complete row-major TSVs remain under target.

The strict transactional import profile accepts exactly dev2, rejects unsupported
instances/entities/aliases rather than deleting them, and publishes into a new
native directory. The source directory is snapshotted read-only. See the root
Phase3 audit for current restrictions, replay and command-line instructions.


Phase3 additionally includes `migration-score60.zip`, written by official Save
with the legitimate Score60 setting and12345 remaining ticks. The color fixture
includes actual locked/unlocked DungeonChests with exact saved contents, one
Key, Ornate Wood tile/item, and the prior colored gameplay samples.
`ornate-wood.json` runs real placement on Hole/Water/Cloud/Lava and Iron Axe /
Iron Pickaxe mining at surface/sky depth, including one remaining durability,
stamina and drops. It also reads the upstream workbench Recipe and sprite key.
The isolated224 driver now writes `dungeon-chests.txt` through Save.writeEntity
for regression of the identical legacy chest encoding (see224 fixture README).
