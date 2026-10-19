//! Deterministic completion of the Java parent-StairsDown -> child-StairsUp
//! rule. Build a bounded halo before cropping: a chunk's load order never
//! changes stairs or writes into an already saved neighboring chunk.
use super::{Cell, Ground, paged::World, terrain::layers::sample_layer};

struct Grid {
    x: i128,
    y: i128,
    width: usize,
    cells: Vec<Option<Cell>>,
}
impl Grid {
    fn cell(&self, x: i128, y: i128) -> Option<Cell> {
        let dx = usize::try_from(x - self.x).ok()?;
        let dy = usize::try_from(y - self.y).ok()?;
        if dx >= self.width || dy >= self.width {
            return None;
        }
        self.cells[dy * self.width + dx]
    }
    fn down(&self, x: i128, y: i128) -> bool {
        self.cell(x, y)
            .is_some_and(|c| c.ground == Ground::StairsDown)
    }
}

/// Original Structure.dungeonGate has a symmetric 5x5 Tile template. Its
/// Lantern and finite-world center Boss room are outside this flight engine.
fn gate(dx: i128, dy: i128) -> Ground {
    match (dx.abs(), dy.abs()) {
        (2, 0) | (0, 2) => Ground::ObsidianDoor,
        (2, _) | (_, 2) => Ground::ObsidianWall,
        _ => Ground::Obsidian,
    }
}

/// Seed is the explicit LevelGen seed on every depth, as in the raw Oracle;
/// this does not claim the full finite World.initWorld nextLong seed assembly.
pub fn generate(seed: i64, depth: i8, cx: i64, cy: i64) -> Result<Vec<Cell>, String> {
    if !(-4..=0).contains(&depth) {
        return Err("legal Infdev depths are -4..0".into());
    }
    let x = cx.checked_mul(64).ok_or("chunk x outside i64 tiles")?;
    let y = cy.checked_mul(64).ok_or("chunk y outside i64 tiles")?;
    x.checked_add(63).ok_or("chunk x end overflow")?;
    y.checked_add(63).ok_or("chunk y end overflow")?;
    // Cave landing radius=1; dungeon gate radius=2. Include the complete
    // ancestor influence, including across chunk boundaries and i64 edges.
    let halo = i128::from(depth.unsigned_abs()) + i128::from(depth == -4);
    let mut parent = Grid {
        x: i128::from(x) - halo,
        y: i128::from(y) - halo,
        width: 64 + 2 * halo as usize,
        cells: Vec::new(),
    };
    for iy in 0..parent.width {
        for ix in 0..parent.width {
            let at = i64::try_from(parent.x + ix as i128)
                .ok()
                .zip(i64::try_from(parent.y + iy as i128).ok());
            parent.cells.push(
                at.map(|(x, y)| sample_layer(seed, 0, x, y).map(|s| s.cell))
                    .transpose()?,
            );
        }
    }
    for layer in (depth..0).rev() {
        let radius = if layer == -4 { 2 } else { 1 };
        let mut child = Grid {
            x: parent.x + radius as i128,
            y: parent.y + radius as i128,
            width: parent.width - 2 * radius,
            cells: Vec::new(),
        };
        for iy in 0..child.width {
            for ix in 0..child.width {
                let gx = child.x + ix as i128;
                let gy = child.y + iy as i128;
                let Some((x, y)) = i64::try_from(gx).ok().zip(i64::try_from(gy).ok()) else {
                    child.cells.push(None);
                    continue;
                };
                let mut cell = sample_layer(seed, layer, x, y)?.cell;
                if parent.down(gx, gy) {
                    cell = Cell {
                        ground: Ground::StairsUp,
                        data: 0,
                    };
                } else if layer == -4 {
                    // Stable global x-major/y-major ordering for overlapping
                    // gates; reassert every stair center above so no gate can
                    // erase a return connection (explicit Rust safety rule).
                    for dx in -2..=2 {
                        for dy in -2..=2 {
                            if parent.down(gx + dx, gy + dy) {
                                cell = Cell {
                                    ground: gate(dx, dy),
                                    data: 0,
                                };
                            }
                        }
                    }
                } else if cell.ground != Ground::StairsDown
                    && (-1..=1).any(|dx| (-1..=1).any(|dy| parent.down(gx + dx, gy + dy)))
                {
                    // Java setAreaTiles(..., Dirt) preserves existing stairs.
                    cell = Cell {
                        ground: Ground::Dirt,
                        data: 0,
                    };
                }
                child.cells.push(Some(cell));
            }
        }
        parent = child;
    }
    debug_assert_eq!(parent.width, 64);
    parent
        .cells
        .into_iter()
        .map(|c| c.ok_or_else(|| "invalid chunk address".into()))
        .collect()
}

/// Only real paired stairs can move the explorer; loading a target or failing
/// validation never changes the active position/depth or rewrites stair tiles.
pub fn travel(world: &mut World) -> Result<(), String> {
    if !world.is_layered() {
        return Err("v2 is surface-only; create an independent v3 world".into());
    }
    let (x, y) = world.position();
    let source = world.cell(x, y)?;
    let (target, expected) = match source.ground {
        Ground::StairsDown => (world.depth() - 1, Ground::StairsUp),
        Ground::StairsUp => (world.depth() + 1, Ground::StairsDown),
        _ => return Err("stand on stairs, then press PICKUP (V/P)".into()),
    };
    if !(-4..=0).contains(&target) {
        return Err("this experimental world has depths -4..0 only".into());
    }
    if world.cell_at(target, x, y)?.ground != expected {
        return Err("saved stair counterpart differs; terrain was not overwritten".into());
    }
    world.visit_layer(target, x, y)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn canonical_parent_stairs_connect_on_every_depth_in_any_generation_order_and_at_i64_edges() {
        let mut links = 0;
        for (cx, cy) in [(945, 877), (-1, -1), (i64::MIN / 64, i64::MAX / 64)] {
            let mut forward = Vec::new();
            for depth in (-4..=0).rev() {
                forward.push((depth, generate(42, depth, cx, cy).unwrap()));
            }
            for &(depth, ref expected) in forward.iter().rev() {
                assert_eq!(&generate(42, depth, cx, cy).unwrap(), expected);
                if depth == 0 {
                    continue;
                }
                let parent = &forward.iter().find(|(d, _)| *d == depth + 1).unwrap().1;
                for (index, cell) in parent.iter().enumerate() {
                    if cell.ground == Ground::StairsDown {
                        assert_eq!(
                            expected[index],
                            Cell {
                                ground: Ground::StairsUp,
                                data: 0
                            },
                            "depth={depth} chunk=({cx},{cy}) cell={index}"
                        );
                        links += 1;
                    }
                }
            }
        }
        assert!(links > 5, "natural candidates must exercise real links");
        assert!(generate(42, -5, 0, 0).is_err());
        assert!(generate(42, 1, 0, 0).is_err());
        assert!(generate(42, -1, i64::MAX, 0).is_err());
    }
}
