from pathlib import Path
import re

root = Path(r"C:\MYAPPLY\Code\Rust\minicraft-rust")
src = (root / "src/assets.rs").read_text(encoding="utf-8")
src = src.replace('include_bytes!("../assets/', 'include_bytes!("../../assets/')
src = re.sub(
    r'(env!\("CARGO_MANIFEST_DIR"\),\s*\n\s*)"/assets/',
    r'\1"/../../assets/',
    src,
)
src = re.sub(
    r'(env!\("CARGO_MANIFEST_DIR"\),\s*)"/assets/',
    r'\1"/../../assets/',
    src,
)
out = root / "crates/minicraft-client/src/assets.rs"
out.write_text(src, encoding="utf-8")
text = out.read_text(encoding="utf-8")
print("bad /assets/", len(re.findall(r'"/assets/', text)))
print("include ../../", text.count('include_bytes!("../../assets/'))
print("include ../assets", text.count('include_bytes!("../assets/'))
print("manifest ../../assets", text.count('/../../assets/'))
print("manifest /assets/", text.count('"/assets/'))
