//! Shared ASCII structure stamping used by LevelGen and later decorate passes.
use super::tile::Tile;

pub fn draw(
    map: &mut [Tile],
    width: usize,
    height: usize,
    center_x: usize,
    center_y: usize,
    rows: &[&str],
    tile_for: impl Fn(char) -> Option<Tile>,
) {
    let pattern_width = rows.first().map_or(0, |row| row.chars().count());
    let pattern_height = rows.len();
    // This intentionally preserves Structure#setData's 2.2.4 rotation.
    for (row, line) in rows.iter().enumerate() {
        for (column, key) in line.chars().enumerate() {
            let Some(tile) = tile_for(key) else { continue };
            let x = center_x as i32 - pattern_width as i32 / 2 + row as i32;
            let y = center_y as i32 - pattern_height as i32 / 2 + column as i32;
            if x >= 0 && y >= 0 && x < width as i32 && y < height as i32 {
                map[x as usize + y as usize * width] = tile;
            }
        }
    }
}

pub fn ornate_lava_pool(map: &mut [Tile], width: usize, height: usize, x: usize, y: usize) {
    draw(
        map,
        width,
        height,
        x,
        y,
        &[
            "WWWDWWW", "WOOOOOW", "WOLLLOW", "DOLLLOD", "WOLLLOW", "WOOOOOW", "WWWDWWW",
        ],
        |key| match key {
            'L' => Some(Tile::Lava),
            'W' => Some(Tile::ObsidianWall),
            'O' => Some(Tile::OrnateObsidian),
            'D' => Some(Tile::ObsidianDoor),
            _ => None,
        },
    );
}

pub fn dungeon_lock(map: &mut [Tile], width: usize, height: usize, x: usize, y: usize) {
    draw(
        map,
        width,
        height,
        x,
        y,
        &["WWWWW", "WOOOW", "WOOOW", "WOOOW", "WWWWW"],
        |key| match key {
            'W' => Some(Tile::ObsidianWall),
            'O' => Some(Tile::ObsidianFloor),
            _ => None,
        },
    );
}
