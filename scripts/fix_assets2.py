from pathlib import Path
import re

root = Path(r"C:\MYAPPLY\Code\Rust\minicraft-rust")
p = root / "crates/minicraft-client/src/assets.rs"
t = p.read_text(encoding="utf-8")
# File lives at crates/minicraft-client/src/assets.rs
# include_bytes relative: ../../../../assets
t = re.sub(
    r'include_bytes!\("(?:\.\./)+assets/',
    'include_bytes!("../../../../assets/',
    t,
)
# CARGO_MANIFEST_DIR is crates/minicraft-client → ../../assets
t = re.sub(r'"/(?:\.\./)+assets/', '"/../../assets/', t)
p.write_text(t, encoding="utf-8")
print("include ok", t.count('include_bytes!("../../../../assets/'))
print("manifest ok", t.count('"/../../assets/'))
print("bad include", len(re.findall(r'include_bytes!\("\.\./assets/', t)))
print("bad manif", len(re.findall(r'"/\.\./assets/', t)))
