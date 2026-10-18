"""Real Windows CLI: cross boundaries, restart, kill dirty process, reopen.

Uses an exclusively created temporary test directory and removes only that
invocation's fixture. Never opens any supplied/player world.
"""
import subprocess
import tempfile
from pathlib import Path

repo = Path(__file__).resolve().parents[2]
binary = repo / "target/debug/minicraft-parity.exe"
output = repo / "target/validation/current"
output.mkdir(parents=True, exist_ok=True)
with tempfile.TemporaryDirectory(prefix="infdev-owned-smoke-", dir=output) as owned:
    root = Path(owned)
    world = root / "infdev4a-v1/Checkpoint42"

    def run(args, commands):
        result = subprocess.run(
            [str(binary), *map(str, args)], input=commands, text=True,
            encoding="utf-8", capture_output=True, timeout=20, check=True,
        )
        print(result.stdout, end="")
        return result.stdout

    run(["infdev-create", root, "Checkpoint42", 42],
        "move -65 -1\nset rock 731\nsave\nunload\nview\nquit\n")
    text = run(["infdev-open", world], "view\nquit\n")
    assert "position=(-65,-1)" in text and "cell=Rock data=731" in text
    manifest = (world / "manifest.json").read_bytes()

    process = subprocess.Popen(
        [str(binary), "infdev-open", str(world)], stdin=subprocess.PIPE,
        stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True, encoding="utf-8",
    )
    try:
        process.stdin.write("set sand 23\n")
        process.stdin.flush()
        statuses = 0
        for _ in range(12):
            line = process.stdout.readline()
            if not line:
                raise AssertionError("explorer exited before dirty edit")
            if line.startswith("seed="):
                statuses += 1
            if statuses == 2:
                break
        assert statuses == 2
        process.kill()  # Windows TerminateProcess, no checkpoint/Drop cleanup.
        process.wait(timeout=10)
    finally:
        if process.poll() is None:
            process.kill()
            process.wait(timeout=10)
        process.stdin.close()
        process.stdout.close()
        process.stderr.close()
    assert (world / "manifest.json").read_bytes() == manifest
    text = run(["infdev-open", world], "view\nquit\n")
    assert "cell=Rock data=731" in text  # unsaved 23 must not leak into checkpoint
    duplicate = subprocess.run(
        [str(binary), "infdev-create", str(root), "Checkpoint42", "43"],
        text=True, capture_output=True, timeout=10,
    )
    assert duplicate.returncode != 0
    assert "731" in text
    print("PASS: negative-boundary movement, unload, process restart, forced termination recovery, duplicate-target refusal")
