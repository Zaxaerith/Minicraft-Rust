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

The seven compact JSON files in `output` are copied to
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
and builds tiny isolated levels; no GUI, natural flower generation, full world
load, random combat or rendered-frame equivalence is claimed. The Java shovel
bug is recorded rather than made into an expected Rust item-loss behavior.
The driver/harness and upstream-derived fixtures retain GPL-3.0 attribution.

Another source-level discrepancy is intentionally not adopted: in
`Player.tick` the `drop-one` / `drop-stack` branch condition is inverted relative
to its comment and the inventory menu: plain Q drops the whole held stack,
whereas Shift-Q decrements it by one. Rust uses Q for one and Shift-Q for the
whole stack consistently, deducting the referenced inventory count exactly once.
This is a documented item-conservation/input fix, not a claim of Java branch
parity. The small Java fixture does not invoke this keyboard branch.
