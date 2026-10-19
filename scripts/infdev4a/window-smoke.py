"""Actual Windows renderer/menus/worker, scripted Input, two separate processes.

No OS input injection and no claim of a human playthrough. Only this invocation's
exclusively created fixture directory is removed; never pass a player world.
"""
from pathlib import Path
import json
import shutil
import subprocess
import tempfile

repo = Path(__file__).resolve().parents[2]
output = repo / "target/validation/current"
output.mkdir(parents=True, exist_ok=True)
exe = repo / "target/release/minicraft-rust.exe"
with tempfile.TemporaryDirectory(prefix="infdev-owned-window-", dir=output) as owned:
    root = Path(owned).resolve()
    assert root.is_relative_to(output.resolve()) and root.name.startswith("infdev-owned-window-")
    for mode in ("create", "reopen"):
        result = subprocess.run([str(exe), "--infdev-window-smoke", mode, str(root)],
                                capture_output=True, text=True, encoding="utf-8", timeout=110)
        print(result.stdout, end="")
        if result.returncode:
            print(result.stderr)
            raise RuntimeError(f"Real window {mode} failed: exit={result.returncode}")
    evidence = json.loads((root / "gui-expect.json").read_text(encoding="utf-8"))
    shutil.copyfile(root / "infdev-window.png", output / "infdev-window.png")
    shutil.copyfile(root / "infdev-underground.png", output / "infdev-underground.png")
    (output / "infdev-window-result.json").write_text(json.dumps(evidence, indent=2), encoding="utf-8")
    print("PASS separate real-window v3 create / natural stair chain to -4 and return / layer edits / Save & Quit underground / process restart / five-layer restore")
