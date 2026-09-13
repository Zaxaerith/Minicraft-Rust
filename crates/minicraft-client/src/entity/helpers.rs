//! Java Entity / Level pure helpers: distance, standability, item drop RNG.
use crate::level::tile::Tile;

/// Java Entity.isWithin distance check (squared form).
pub fn squared_distance(x: i32, y: i32, other_x: i32, other_y: i32) -> i32 {
    let dx = x - other_x;
    let dy = y - other_y;
    dx * dx + dy * dy
}

/// Player Mob xr=4 yr=3 corner standability (Java Entity.moveX/Y tile probes).
/// Keeps the historical y+4 corner used by World::can_stand.
pub fn can_stand(
    tiles: &[Tile],
    data: &[u16],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
) -> bool {
    [(-4, -3), (4, -3), (-4, 4), (4, 4)]
        .into_iter()
        .all(|(offset_x, offset_y)| tile_free(tiles, data, width, height, x + offset_x, y + offset_y))
}

/// Java Entity.moveX/Y tile corners for an xr/yr hitbox (Furniture.push).
pub fn furniture_can_stand(
    tiles: &[Tile],
    data: &[u16],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
    xr: i32,
    yr: i32,
) -> bool {
    [(-xr, -yr), (xr, -yr), (-xr, yr), (xr, yr)]
        .into_iter()
        .all(|(offset_x, offset_y)| tile_free(tiles, data, width, height, x + offset_x, y + offset_y))
}

fn tile_free(
    tiles: &[Tile],
    data: &[u16],
    width: usize,
    height: usize,
    x: i32,
    y: i32,
) -> bool {
    let tile_x = x.div_euclid(16);
    let tile_y = y.div_euclid(16);
    if tile_x < 0 || tile_y < 0 || tile_x >= width as i32 || tile_y >= height as i32 {
        return false;
    }
    let index = tile_x as usize + tile_y as usize * width;
    !tiles[index].solid(data[index])
}

// --- Java ItemEntity drop RNG (id-seeded splitmix64, not level Random). ---

pub(crate) fn split_mix(mut value: u64) -> u64 {
    value = value.wrapping_add(0x9E37_79B9_7F4A_7C15);
    value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    value ^ (value >> 31)
}

pub(crate) fn unit(value: u64) -> f64 {
    ((value >> 11) as f64 + 1.0) / ((1_u64 << 53) as f64 + 1.0)
}

pub(crate) fn gaussian(first: u64, second: u64) -> f64 {
    (-2.0 * unit(first).ln()).sqrt() * (std::f64::consts::TAU * unit(second)).cos()
}
