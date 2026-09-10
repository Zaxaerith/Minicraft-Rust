from pathlib import Path
import re

root = Path(r"C:\MYAPPLY\Code\Rust\minicraft-rust")
p = root / "crates/minicraft-client/src/assets.rs"
t = p.read_text(encoding="utf-8")
# include_bytes is relative to src/ (this file's directory): ../../../assets → project root assets
t = re.sub(
    r'include_bytes!\("(?:\.\./)+assets/',
    'include_bytes!("../../../assets/',
    t,
)
# CARGO_MANIFEST_DIR = crates/minicraft-client → ../../assets
t = re.sub(r'"/(?:\.\./)+assets/', '"/../../assets/', t)
p.write_text(t, encoding="utf-8")
print("include ../../../", t.count('include_bytes!("../../../assets/'))
print("manifest /../../", t.count('"/../../assets/'))
print("bad include", t.count('include_bytes!("../../assets/'))
print("bad manif", t.count('"/../../assets/'), "only good")
# verify path exists
assert (root / "assets/pack.png").is_file()
print("pack.png exists", (root / "assets/pack.png").stat().st_size)
