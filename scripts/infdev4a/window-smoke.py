"""Actual Windows renderer/menus/worker, scripted Input, independent processes.

No OS input injection and no claim of a human playthrough. Only this invocation's
exclusively created fixture directory is removed; never pass a player world.
"""
from pathlib import Path
import json
import shutil
import subprocess
import tempfile
import argparse
import threading

parser=argparse.ArgumentParser(description=__doc__)
group=parser.add_mutually_exclusive_group()
group.add_argument("--survival-only",action="store_true")
group.add_argument("--death-only",action="store_true")
options=parser.parse_args()

repo = Path(__file__).resolve().parents[2]
output = repo / "target/validation/current"
output.mkdir(parents=True, exist_ok=True)
exe = repo / "target/release/minicraft-rust.exe"
with tempfile.TemporaryDirectory(prefix="infdev-owned-window-", dir=output) as owned:
    root = Path(owned).resolve()
    assert root.is_relative_to(output.resolve()) and root.name.startswith("infdev-owned-window-")
    modes=("survival-create","survival-reopen") if options.survival_only else (("death-create","death-recover","death-reopen") if options.death_only else ("create","reopen","survival-create","survival-reopen","death-create","death-recover","death-reopen"))
    for mode in modes:
        session_root=root/"death" if mode.startswith("death-") else root/"survival" if mode.startswith("survival-") else root
        with subprocess.Popen([str(exe), "--infdev-window-smoke", mode, str(session_root)],
                              stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, encoding="utf-8") as process:
            outcome={"forced":False}
            def forward():
                for line in process.stdout:
                    print(line, end="", flush=True)
                    if mode=="death-create" and line.strip()=="READY_FOR_FORCED_DEATH_TERMINATION":
                        outcome["forced"]=True
                        process.kill()
            reader=threading.Thread(target=forward,daemon=True)
            reader.start()
            try:
                process.wait(timeout=510 if mode.startswith(("survival-","death-")) else 110)
            except subprocess.TimeoutExpired:
                process.kill();process.wait()
                raise
            finally:
                reader.join(timeout=5)
            if mode=="death-create" and not outcome["forced"]:
                raise RuntimeError("Native death did not reach the durable-checkpoint kill marker")
            if process.returncode and not outcome["forced"]:
                raise RuntimeError(f"Real window {mode} failed: exit={process.returncode}")
    if not options.survival_only and not options.death_only:
        evidence = json.loads((root / "gui-expect.json").read_text(encoding="utf-8"))
        shutil.copyfile(root / "infdev-window.png", output / "infdev-window.png")
        shutil.copyfile(root / "infdev-underground.png", output / "infdev-underground.png")
        (output / "infdev-window-result.json").write_text(json.dumps(evidence, indent=2), encoding="utf-8")
        print("PASS separate real-window v3 create / natural stair chain to -4 and return / layer edits / Save & Quit underground / process restart / five-layer restore")
    if not options.death_only:
        evidence=json.loads((root/"survival/survival-expect.json").read_text(encoding="utf-8"))
        (output/"infdev-survival-result.json").write_text(json.dumps(evidence,indent=2),encoding="utf-8")
        shutil.copyfile(root/"survival/infdev-survival.png",output/"infdev-survival.png")
        print("PASS independent native Survival create/restart: empty inventory, earned resources, tools, furnace/anvil, fuel, natural apple/food, mining underground, return, drops and disk state")
    if not options.survival_only:
        for name in ("infdev-death.png","infdev-recovered.png"):
            shutil.copyfile(root/"death"/name,output/name)
        shutil.copyfile(root/"death/death-expect.json",output/"infdev-death-result.json")
        print("PASS three native death windows: natural inventory, actual forced termination after atomic death, respawn/return/container retrieval, repeated interaction and disk GC/reopen")
