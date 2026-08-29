#!/usr/bin/env python3
"""Create a deterministic, self-contained Minicraft Rust release archive."""

from __future__ import annotations

import argparse
import hashlib
import os
from pathlib import Path
import subprocess
import zipfile


ROOT = Path(__file__).resolve().parents[1]
VERSION = "2.2.4"
DOCUMENTS = [
    "LICENSE",
    "README.md",
]


def arguments() -> argparse.Namespace:
    parser = argparse.ArgumentParser()
    parser.add_argument("--platform", required=True, help="archive platform label")
    parser.add_argument("--binary", required=True, type=Path)
    parser.add_argument("--output", default=ROOT / "dist", type=Path)
    parser.add_argument("--force", action="store_true")
    return parser.parse_args()


def git_revision() -> str:
    try:
        return subprocess.check_output(
            ["git", "rev-parse", "HEAD"], cwd=ROOT, text=True
        ).strip()
    except (OSError, subprocess.CalledProcessError):
        return "unknown"


def zip_info(name: str, executable: bool = False) -> zipfile.ZipInfo:
    info = zipfile.ZipInfo(name, date_time=(1980, 1, 1, 0, 0, 0))
    info.compress_type = zipfile.ZIP_STORED
    info.create_system = 3
    info.external_attr = ((0o755 if executable else 0o644) & 0xFFFF) << 16
    return info


def main() -> None:
    args = arguments()
    binary = args.binary if args.binary.is_absolute() else ROOT / args.binary
    if not binary.is_file():
        raise SystemExit(f"release binary does not exist: {binary}")
    missing = [name for name in DOCUMENTS if not (ROOT / name).is_file()]
    if missing:
        raise SystemExit(f"release documentation is incomplete: {', '.join(missing)}")

    args.output.mkdir(parents=True, exist_ok=True)
    archive = args.output / f"minicraft-rust-{VERSION}-{args.platform}.zip"
    if archive.exists():
        if not args.force:
            raise SystemExit(f"archive already exists: {archive}; pass --force to replace it")
        archive.unlink()

    lock_hash = hashlib.sha256((ROOT / "Cargo.lock").read_bytes()).hexdigest()
    build_info = (
        f"version={VERSION}\n"
        f"platform={args.platform}\n"
        f"git_revision={git_revision()}\n"
        f"cargo_lock_sha256={lock_hash}\n"
        "assets=embedded from the locally copied Minicraft+ 2.2.4 resource set\n"
    ).encode()
    executable_name = "minicraft-rust" + binary.suffix
    prefix = f"minicraft-rust-{VERSION}-{args.platform}"
    entries = [(executable_name, binary.read_bytes(), True)]
    entries.extend((name, (ROOT / name).read_bytes(), False) for name in DOCUMENTS)
    entries.append(("BUILD-INFO.txt", build_info, False))

    with zipfile.ZipFile(archive, "w") as output:
        for name, data, executable in sorted(entries):
            output.writestr(zip_info(f"{prefix}/{name}", executable), data)

    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    print(f"created {archive}")
    print(f"sha256 {digest}")


if __name__ == "__main__":
    main()
