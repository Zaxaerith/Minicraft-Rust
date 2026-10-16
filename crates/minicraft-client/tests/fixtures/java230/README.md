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
