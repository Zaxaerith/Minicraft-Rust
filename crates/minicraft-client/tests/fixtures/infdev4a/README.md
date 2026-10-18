# Infdev4A selected Java reference samples

`noise-coordinates.tsv` comes from the original `LevelNoise` and `Simplex` at
Infdev3 `42ba9192fcc229bcf615453a6504d415c777feaf`. Regenerate with
`pwsh -NoProfile -File scripts/infdev4a/run.ps1` from the repository root.
The driver verifies three selected Git blobs without EOL normalization and
compiles the original two noise sources unchanged; no game Java checkout edits.

50 rows: five signed seeds, ten global positions including negative boundaries
and i32 extrema. Chunk size is read from the verified ChunkManager declaration.
Coordinates use actual Java Math.floorDiv/floorMod; this does not claim to
execute the complete ChunkManager, biomes, World assembly or Infdev Save.
Height is compared as exact promoted double bit patterns, not rounded decimals.
Prototype material thresholds are a separate Rust extension without Java goldens.

GPL-3.0 upstream/project attribution applies; original Simplex credits K.jpg's
OpenSimplex2. No large Java sources, classes, resources or worlds are committed.
