#!/usr/bin/env python3
"""Verify custody of the resources in the official Minicraft+ 2.2.4 JAR."""

from __future__ import annotations

import argparse
import hashlib
from pathlib import Path
import zipfile


ROOT = Path(__file__).resolve().parents[1]
EXPECTED_JAR_SHA256 = "90d534d346eca5da3a200d2c5b6007be919ee94908558c8e75a76c704f9a3f44"
TEXT_SUFFIXES = {".json", ".properties", ".txt"}


def canonical_bytes(name: str, data: bytes) -> bytes:
    """Undo Git's platform line-ending conversion for source text assets."""
    if Path(name).suffix.lower() in TEXT_SUFFIXES:
        return data.replace(b"\r\n", b"\n")
    return data


def main() -> None:
    parser = argparse.ArgumentParser()
    parser.add_argument("jar", type=Path)
    args = parser.parse_args()
    digest = hashlib.sha256(args.jar.read_bytes()).hexdigest()
    if digest != EXPECTED_JAR_SHA256:
        raise SystemExit(f"unexpected official JAR SHA-256: {digest}")

    local_root = ROOT / "assets"
    local = {
        path.relative_to(local_root).as_posix(): canonical_bytes(
            path.relative_to(local_root).as_posix(), path.read_bytes()
        )
        for path in local_root.rglob("*")
        if path.is_file()
    }
    missing: list[str] = []
    mismatched: list[str] = []
    with zipfile.ZipFile(args.jar) as jar:
        names = set(jar.namelist())
        for name, local_data in sorted(local.items()):
            if name not in names:
                missing.append(name)
                continue
            if canonical_bytes(name, jar.read(name)) != local_data:
                mismatched.append(name)

    if missing or mismatched:
        if missing:
            print("missing from JAR:", *missing, sep="\n  ")
        if mismatched:
            print("hash mismatch:", *mismatched, sep="\n  ")
        raise SystemExit("official resource custody audit failed")
    print(f"official JAR sha256 {digest}")
    print(
        f"verified {len(local)} locally copied resource files "
        "(binary byte-for-byte; text after CRLF normalization)"
    )


if __name__ == "__main__":
    main()
