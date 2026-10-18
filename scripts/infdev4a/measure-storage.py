"""Windows-only, explicit storage timing + process-memory sampling.

Uses unique throwaway disk worlds owned by the diagnostic Rust process. No OS
cache flush is attempted: timings represent warm-cache local operation, not a
cold disk benchmark. Run after building minicraft-parity in Release.
"""
import argparse
import csv
import ctypes
from ctypes import wintypes
from pathlib import Path
import subprocess
import sys
import threading
import time


class MemoryCounters(ctypes.Structure):
    _fields_ = [
        ("cb", wintypes.DWORD),
        ("PageFaultCount", wintypes.DWORD),
        ("PeakWorkingSetSize", ctypes.c_size_t),
        ("WorkingSetSize", ctypes.c_size_t),
        ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPagedPoolUsage", ctypes.c_size_t),
        ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
        ("QuotaNonPagedPoolUsage", ctypes.c_size_t),
        ("PagefileUsage", ctypes.c_size_t),
        ("PeakPagefileUsage", ctypes.c_size_t),
        ("PrivateUsage", ctypes.c_size_t),
    ]


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--exe", type=Path, default=Path("target/release/minicraft-parity.exe"))
    parser.add_argument("--output", type=Path, default=Path("target/validation/current"))
    args = parser.parse_args()
    if sys.platform != "win32":
        parser.error("Windows x64 process memory sampling is required")
    args.output.mkdir(parents=True, exist_ok=True)
    executable = args.exe.resolve(strict=True)
    csv_path = args.output / "infdev-storage-metrics.csv"
    # A failed/interrupted measurement cannot inherit an old complete CSV.
    csv_path.write_text("status=running\n", encoding="utf-8")
    kernel = ctypes.WinDLL("kernel32", use_last_error=True)
    psapi = ctypes.WinDLL("psapi", use_last_error=True)
    kernel.OpenProcess.argtypes = [wintypes.DWORD, wintypes.BOOL, wintypes.DWORD]
    kernel.OpenProcess.restype = wintypes.HANDLE
    kernel.CloseHandle.argtypes = [wintypes.HANDLE]
    psapi.GetProcessMemoryInfo.argtypes = [wintypes.HANDLE, ctypes.POINTER(MemoryCounters), wintypes.DWORD]
    psapi.GetProcessMemoryInfo.restype = wintypes.BOOL

    process = subprocess.Popen(
        [str(executable), "infdev-storage-measure", str(args.output.resolve())],
        stdout=subprocess.PIPE, stderr=subprocess.STDOUT, text=True, encoding="utf-8",
    )
    handle = kernel.OpenProcess(0x410, False, process.pid)
    if not handle:
        process.kill()
        raise ctypes.WinError(ctypes.get_last_error())
    lock = threading.Lock()
    current = None
    peaks = {}
    rows = []

    def sample():
        counters = MemoryCounters()
        counters.cb = ctypes.sizeof(counters)
        if not psapi.GetProcessMemoryInfo(handle, ctypes.byref(counters), counters.cb):
            return
        with lock:
            if current:
                previous = peaks.setdefault(current, [0, 0])
                previous[0] = max(previous[0], counters.WorkingSetSize)
                previous[1] = max(previous[1], counters.PrivateUsage)

    def monitor():
        while process.poll() is None:
            sample()
            time.sleep(0.01)

    sampler = threading.Thread(target=monitor, daemon=True)
    sampler.start()
    try:
        with (args.output / "infdev-storage-metrics.log").open("w", encoding="utf-8") as log:
            for line in process.stdout:
                log.write(line)
                log.flush()
                print(line, end="", flush=True)
                fields = line.rstrip().split("\t")
                if fields[0] == "INFDEV_STORAGE_STAGE":
                    with lock:
                        current = tuple(fields[1:4])
                    sample()
                elif fields[0] == "INFDEV_STORAGE_RESULT":
                    sample()
                    key = tuple(fields[1:4])
                    with lock:
                        memory = peaks.get(key, [0, 0]).copy()
                    if not all(memory):
                        raise RuntimeError("Could not sample child process memory")
                    rows.append([*key, fields[4], *memory])
        if process.wait() != 0 or len(rows) != 10:
            raise RuntimeError(f"Storage diagnostic failed: exit={process.returncode}, rows={len(rows)}")
        with csv_path.open("w", encoding="utf-8", newline="") as output:
            writer = csv.writer(output)
            writer.writerow(["dataset", "saved_chunks", "operation", "milliseconds", "sampled_working_set_bytes", "sampled_private_bytes"])
            writer.writerows(rows)
        print(f"Completed: {csv_path}")
    finally:
        if process.poll() is None:
            process.kill()
            process.wait()
        sampler.join(timeout=1)
        kernel.CloseHandle(handle)


if __name__ == "__main__":
    main()
