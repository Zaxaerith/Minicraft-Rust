"""Extract original selected upstream blobs without checkout/EOL filters."""
import subprocess
from pathlib import Path

root = Path(__file__).resolve().parents[2]
git = ["git", "--git-dir=" + str(root / ".reference/upstream.git")]
pin = "42ba9192fcc229bcf615453a6504d415c777feaf"
entries = []
for line in subprocess.check_output(git + ["ls-tree", "-r", pin]).decode().splitlines():
    metadata, name = line.split("\t")
    if name.startswith(("src/client/java/", "src/common/java/", "src/client/resources/")):
        entries.append((metadata.split()[2], name))
batch = subprocess.run(git + ["cat-file", "--batch"],
                       input="".join(sha + "\n" for sha, _ in entries).encode(),
                       capture_output=True, check=True).stdout
offset = 0
destination = root / "target/parity/infdev4a/source"
for sha, name in entries:
    end = batch.index(b"\n", offset)
    actual, kind, length = batch[offset:end].decode().split()
    assert actual == sha and kind == "blob"
    start, length = end + 1, int(length)
    path = destination / name
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_bytes(batch[start:start + length])
    offset = start + length + 1
assert offset == len(batch)
print(f"Extracted {len(entries)} original blobs at {pin}")
