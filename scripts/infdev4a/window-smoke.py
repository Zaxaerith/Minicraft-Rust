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
import time
import hashlib
from datetime import datetime, timezone

parser=argparse.ArgumentParser(description=__doc__)
group=parser.add_mutually_exclusive_group()
group.add_argument("--survival-only",action="store_true")
group.add_argument("--death-only",action="store_true")
group.add_argument("--hunt-only",action="store_true")
group.add_argument("--combat-only",action="store_true")
group.add_argument("--environment-only",action="store_true")
parser.add_argument("--environment-night",action="store_true", help="after the independent short PASS, wait for first night at the original clock rate once, then reopen again")
options=parser.parse_args()
if options.environment_night and not options.environment_only:
    parser.error("--environment-night requires --environment-only")

repo = Path(__file__).resolve().parents[2]
output = repo / "target/validation/current"
output.mkdir(parents=True, exist_ok=True)
exe = repo / "target/release/minicraft-rust.exe"
trace=[]
exe_hash=hashlib.sha256(exe.read_bytes()).hexdigest() if options.environment_only else None
with tempfile.TemporaryDirectory(prefix="infdev-owned-window-", dir=output) as owned:
    root = Path(owned).resolve()
    assert root.is_relative_to(output.resolve()) and root.name.startswith("infdev-owned-window-")
    modes=("environment-create","environment-reopen") if options.environment_only else ("combat-create","combat-death","combat-recover") if options.combat_only else ("hunt-create","hunt-reopen") if options.hunt_only else ("survival-create","survival-reopen") if options.survival_only else (("death-create","death-recover","death-reopen") if options.death_only else ("create","reopen","survival-create","survival-reopen","death-create","death-recover","death-reopen","hunt-create","hunt-reopen"))
    if options.environment_night:
        modes+=("environment-night","environment-night-reopen")
    if options.environment_only:
        (output/"environment-processes.json").write_text(json.dumps({"requestedModes":modes,"processes":[],"status":"running"},indent=2),encoding="utf-8")
    for mode in modes:
        session_root=root/"environment" if mode.startswith("environment-") else root/"combat" if mode.startswith("combat-") else root/"hunt" if mode.startswith("hunt-") else root/"death" if mode.startswith("death-") else root/"survival" if mode.startswith("survival-") else root
        with subprocess.Popen([str(exe), "--infdev-window-smoke", mode, str(session_root)],
                              stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, encoding="utf-8") as process:
            started=time.monotonic()
            outcome={"mode":mode,"pid":process.pid,"startedUtc":datetime.now(timezone.utc).isoformat(),
                     "exeSha256":exe_hash,"forced":False,"timeout":False,"lastOutput":None}
            print(f"START {mode} pid={process.pid} utc={outcome['startedUtc']}",flush=True)
            def forward():
                for line in process.stdout:
                    outcome["lastOutput"]=line.strip()
                    print(f"[{mode} +{time.monotonic()-started:.1f}s] {line}", end="", flush=True)
                    if (mode=="death-create" and line.strip()=="READY_FOR_FORCED_DEATH_TERMINATION") or (mode=="combat-death" and line.strip()=="READY_FOR_FORCED_COMBAT_TERMINATION"):
                        outcome["forced"]=True
                        process.kill()
            reader=threading.Thread(target=forward,daemon=True)
            reader.start()
            try:
                process.wait(timeout=1230 if mode=="environment-night" else 510 if mode.startswith(("environment-","survival-","death-","hunt-","combat-")) else 110)
            except subprocess.TimeoutExpired:
                outcome["timeout"]=True
                print(f"SCRIPT_TIMEOUT {mode}: explicit controller termination",flush=True)
                process.kill();process.wait()
                raise
            finally:
                reader.join(timeout=5)
                if mode.startswith("environment-"):
                    outcome.update({"exitCode":process.returncode,"elapsedSeconds":round(time.monotonic()-started,3),
                                    "endedUtc":datetime.now(timezone.utc).isoformat(),
                                    "pass":process.returncode==0 and not outcome["forced"] and not outcome["timeout"]})
                    trace.append(outcome)
                    (output/"environment-processes.json").write_text(json.dumps({"requestedModes":modes,"processes":trace},indent=2),encoding="utf-8")
                    # Keep compact evidence even if the long wait is interrupted;
                    # never copy a world or recycle a player's directory.
                    for artifact in session_root.glob("environment-*"):
                        if artifact.is_file() and artifact.suffix in (".json",".png"):
                            name=f"{mode}-{artifact.name}" if "failure" in artifact.name else artifact.name
                            shutil.copyfile(artifact,output/name)
                    print(f"END {mode}: exit={process.returncode} timeout={outcome['timeout']} elapsed={outcome['elapsedSeconds']}s",flush=True)
                    if mode=="environment-reopen" and outcome["pass"]:
                        print("PASS SHORT native acquisition/battle -> normal Escape Save & Quit -> independent process: exact pre-tick clock/inventory/position/ALL entity pages; menu pause and GC",flush=True)
            if mode in ("death-create","combat-death") and not outcome["forced"]:
                raise RuntimeError("Native death did not reach the durable-checkpoint kill marker")
            if process.returncode and not outcome["forced"]:
                for name in ("combat-failure.json","combat-expect.json","combat-death-expect.json"):
                    if (session_root/name).is_file():
                        shutil.copyfile(session_root/name,output/name)
                raise RuntimeError(f"Real window {mode} failed: exit={process.returncode}")
    if options.environment_only:
        assert [entry["mode"] for entry in trace if entry["pass"]]==list(modes)
        if options.environment_night:
            evidence=json.loads((output/"environment-night-time.json").read_text(encoding="utf-8"))
            assert evidence["naturalClock"] and evidence["after"]["day_tick"]>=48600
            print("PASS SEPARATE native original-rate day/evening/night, normal Save & Quit and independent night restart")
        else:
            print("Night waiting NOT RUN by this short-only command; use --environment-night explicitly")
        raise SystemExit(0)
    if options.combat_only:
        evidence=json.loads((root/"combat/combat-expect.json").read_text(encoding="utf-8"))
        assert evidence["hurt"] and evidence["cloth"]>0
        for name in ("combat-expect.json","combat-death-expect.json","combat-result.json","infdev-combat.png","infdev-combat-death.png"):
            shutil.copyfile(root/"combat"/name,output/name)
        print("PASS native combat: earned sword, natural underground zombie, injury/kill/drop, chunk crossing, exact pre-tick restart and fatal enemy contact/chest recovery after forced termination")
        raise SystemExit(0)
    if not options.survival_only and not options.death_only and not options.hunt_only:
        evidence = json.loads((root / "gui-expect.json").read_text(encoding="utf-8"))
        shutil.copyfile(root / "infdev-window.png", output / "infdev-window.png")
        shutil.copyfile(root / "infdev-underground.png", output / "infdev-underground.png")
        (output / "infdev-window-result.json").write_text(json.dumps(evidence, indent=2), encoding="utf-8")
        print("PASS separate real-window v3 create / natural stair chain to -4 and return / layer edits / Save & Quit underground / process restart / five-layer restore")
    if not options.death_only and not options.hunt_only:
        evidence=json.loads((root/"survival/survival-expect.json").read_text(encoding="utf-8"))
        (output/"infdev-survival-result.json").write_text(json.dumps(evidence,indent=2),encoding="utf-8")
        shutil.copyfile(root/"survival/infdev-survival.png",output/"infdev-survival.png")
        print("PASS independent native Survival create/restart: empty inventory, earned resources, tools, furnace/anvil, fuel, natural apple/food, mining underground, return, drops and disk state")
    if not options.survival_only and not options.hunt_only:
        for name in ("infdev-death.png","infdev-recovered.png"):
            shutil.copyfile(root/"death"/name,output/name)
        shutil.copyfile(root/"death/death-expect.json",output/"infdev-death-result.json")
        print("PASS three native death windows: natural inventory, actual forced termination after atomic death, respawn/return/container retrieval, repeated interaction and disk GC/reopen")

    if options.hunt_only or not options.survival_only and not options.death_only:
        evidence=json.loads((root/"hunt/hunt-expect.json").read_text(encoding="utf-8"))
        assert evidence["leather"]>0 and evidence["wool"]>0 and evidence["earnedPork"]>0, "current passive-animal EXE/evidence required"
        shutil.copyfile(root/"hunt/hunt-expect.json",output/"infdev-hunt-result.json")
        shutil.copyfile(root/"hunt/infdev-hunt.png",output/"infdev-hunt.png")
        print("PASS native hunt: naturally spawned cow/pig/sheep, earned leather/wool/pork, axe, real combat/drop/pickup/food, chunk crossing, disk state restart and GC")
