from pathlib import Path

root = Path(r"C:\MYAPPLY\Code\Rust\minicraft-rust")
src = (root / "src/world/tile.rs").read_text(encoding="utf-8")
start = src.index("pub fn damage_tile(")
end = src.index("pub fn tile_placement(")
damage = src[start:end].rstrip() + "\n"
rest = src[:start] + src[end:]

# strip original module header uses
lines = rest.splitlines(True)
body_start = 0
for i, line in enumerate(lines):
    if line.startswith("//!") or line.startswith("use ") or line.strip() == "":
        continue
    body_start = i
    break
body = "".join(lines[body_start:])

header = """//! Stable tile identities, registry metadata, and tile-local helpers.
use serde::{Deserialize, Serialize};

use crate::item::ItemId;

"""
(root / "crates/minicraft-client/src/level/tile/mod.rs").write_text(header + body, encoding="utf-8")

root_tile = """//! Level-bound tile helpers (damage against a Level); identities live in client.
use super::Level;
pub use minicraft_client::level::tile::{
    Tile, connection_mask, connects, find_stair_site, is_tile_item, solid, tile_placement,
    underlay_layers,
};

""" + damage
(root / "src/world/tile.rs").write_text(root_tile, encoding="utf-8")
print("ok", body.count("\\n"), damage.count("\\n"))
