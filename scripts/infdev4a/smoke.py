"""Real Windows processes: all storage generations, upgrade, GC and termination.
Only the invocation's exclusively created temporary fixtures are ever removed.
"""
import subprocess
import tempfile
import argparse
from pathlib import Path

repo = Path(__file__).resolve().parents[2]
parser = argparse.ArgumentParser(description=__doc__)
parser.add_argument("--release", action="store_true", help="use the existing Release binary")
options = parser.parse_args()
binary = repo / "target" / ("release" if options.release else "debug") / "minicraft-parity.exe"
output = repo / "target/validation/current"
output.mkdir(parents=True, exist_ok=True)

def run(args, commands, check=True):
    result = subprocess.run([str(binary), *map(str,args)], input=commands, text=True,
                            encoding="utf-8", capture_output=True, timeout=30, check=check)
    print(result.stdout, end="")
    if not check:
        assert result.returncode != 0, "expected safe refusal"
    return result.stdout

def snapshot(root):
    return {str(p.relative_to(root)):p.read_bytes() for p in root.rglob("*") if p.is_file()}

with tempfile.TemporaryDirectory(prefix="infdev-owned-smoke-", dir=output) as owned:
    root=Path(owned)
    for create,namespace,name,ground,data in [
        ("infdev-create","infdev4a-v1","Legacy42","rock",731),
        ("infdev-surface-create","infdev-v2","Surface42","flower",11),
        ("infdev-layers-create","infdev-v3","Layers42","iron",731),
    ]:
        world=root/namespace/name
        commands="move -65 -1\n"
        if namespace=="infdev-v3":commands+="visit-depth -2 -65 -1\n"
        commands+=f"set {ground} {data}\nsave\nunload\nview\nsave\n"
        if namespace!="infdev4a-v1":commands+="gc\n"
        run([create,root,name,42],commands+"quit\n")
        text=run(["infdev-open",world],"view\nquit\n")
        assert "position=(-65,-1)" in text and f"data={data}" in text
        if namespace=="infdev-v3":assert "depth=-2" in text
        manifest=world/("manifest.json" if namespace=="infdev4a-v1" else "checkpoint.json")
        checkpoint=manifest.read_bytes()
        process=subprocess.Popen([str(binary),"infdev-open",str(world)],stdin=subprocess.PIPE,
                                 stdout=subprocess.PIPE,stderr=subprocess.PIPE,text=True,encoding="utf-8")
        try:
            process.stdin.write("set sand 23\n");process.stdin.flush()
            statuses=0
            for _ in range(12):
                line=process.stdout.readline()
                assert line,"process exited before dirty edit"
                if line.startswith("seed="):statuses+=1
                if statuses==2:break
            assert statuses==2
            process.kill();process.wait(timeout=10)
        finally:
            if process.poll() is None:process.kill();process.wait(timeout=10)
            process.stdin.close();process.stdout.close();process.stderr.close()
        assert manifest.read_bytes()==checkpoint
        text=run(["infdev-open",world],"view\nquit\n")
        assert f"data={data}" in text
        run([create,root,name,43],"",check=False)
        if namespace!="infdev4a-v1":
            manifest.write_bytes(b"truncated checkpoint")
            text=run(["infdev-open",world],"view\nquit\n")
            assert "RECOVERED" in text and f"data={data}" in text
            assert (world/"gc-recovery.hold").exists()
            objects=snapshot(world/"chunks")|{"nodes/"+k:v for k,v in snapshot(world/"nodes").items()}
            run(["infdev-open",world],"gc\n",check=False)
            assert objects==snapshot(world/"chunks")|{"nodes/"+k:v for k,v in snapshot(world/"nodes").items()}
    source=root/"infdev4a-v1/Legacy42"
    before=snapshot(source)
    text=run(["infdev-upgrade",source,root,"Converted42"],"view\nsave\ngc\nquit\n")
    assert "LegacyHeightV1/paged-v2" in text and "data=731" in text
    assert snapshot(source)==before
    run(["infdev-upgrade",source,root,"Converted42"],"",check=False)
    print("PASS v1/v2/v3: boundary/restart/forced termination/duplicate refusal; paged GC+corrupt recovery pin; independent underground checkpoint; read-only copy upgrade")
