"""Start time and battery proxies for the five ways to write a BUI app.

Each app in bench/apps (and the Rust example `bench_app`) shows the same small
UI and prints "ready" once its first frame is on screen. For every binding
this script:

* launches it N times and takes the median wall time from launch to "ready",
  the CPU time it used to get there, and its memory then;
* keeps one launch open and idle for a while and measures the CPU it uses.

Laptops can't report energy per process, so the battery numbers are proxies:
CPU time spent starting up, and CPU used while the window sits idle (an idle
UI that uses no CPU lets the processor sleep).

    py -3 bench/run.py [runs=10] [idle_seconds=10]

Needs: the buipy extension built for this Python (cmake --build <dir> --target
buipy), the Node addon (cd bindings/node && npm run build), and the Rust
example built with `cargo build --release --example bench_app` in
bindings/rust, and the gpui comparison with `cargo build --release --bins` in
bindings/rust/bench. Every BUI app runs twice, with BUI_GPU=0 (CPU renderer)
and BUI_GPU=1 (GPU renderer). Windows only (process times come from the Win32
API).
"""
from __future__ import annotations

import ctypes
import ctypes.wintypes as wt
import glob
import json
import os
import statistics
import subprocess
import sys
import threading
import time

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
APPS = os.path.join(ROOT, "bench", "apps")

k32 = ctypes.WinDLL("kernel32", use_last_error=True)
psapi = ctypes.WinDLL("psapi")


class PMC(ctypes.Structure):
    _fields_ = [("cb", wt.DWORD), ("PageFaultCount", wt.DWORD), ("PeakWorkingSetSize", ctypes.c_size_t),
                ("WorkingSetSize", ctypes.c_size_t), ("QuotaPeakPagedPoolUsage", ctypes.c_size_t),
                ("QuotaPagedPoolUsage", ctypes.c_size_t), ("QuotaPeakNonPagedPoolUsage", ctypes.c_size_t),
                ("QuotaNonPagedPoolUsage", ctypes.c_size_t), ("PagefileUsage", ctypes.c_size_t),
                ("PeakPagefileUsage", ctypes.c_size_t), ("PrivateUsage", ctypes.c_size_t)]


def _cpu_hz() -> float:
    """The CPU's rated clock (the rate cycle counts tick at)."""
    import winreg
    with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, r"HARDWARE\DESCRIPTION\System\CentralProcessor\0") as k:
        return winreg.QueryValueEx(k, "~MHz")[0] * 1e6


CPU_HZ = _cpu_hz()


def cpu_name() -> str:
    import winreg
    with winreg.OpenKey(winreg.HKEY_LOCAL_MACHINE, r"HARDWARE\DESCRIPTION\System\CentralProcessor\0") as k:
        return winreg.QueryValueEx(k, "ProcessorNameString")[0].strip()


def cpu_seconds(handle) -> float:
    """CPU time of a process (all threads, user + kernel) in seconds, from its
    exact cycle count (GetProcessTimes only counts 15.6 ms scheduler ticks)."""
    cycles = ctypes.c_ulonglong()
    if not k32.QueryProcessCycleTime(handle, ctypes.byref(cycles)):
        return float("nan")
    return cycles.value / CPU_HZ


def memory_mb(handle) -> float:
    m = PMC()
    m.cb = ctypes.sizeof(PMC)
    psapi.GetProcessMemoryInfo(handle, ctypes.byref(m), m.cb)
    return m.WorkingSetSize / 1048576


def find_pyd() -> str | None:
    hits = glob.glob(os.path.join(ROOT, "build*", "**", "buipy*.pyd"), recursive=True)
    tag = f"cp{sys.version_info.major}{sys.version_info.minor}"
    hits = [h for h in hits if tag in os.path.basename(h)]
    return os.path.dirname(max(hits, key=os.path.getmtime)) if hits else None


def variants():
    py_env = dict(os.environ, PYTHONPATH=os.pathsep.join(filter(None, [os.path.join(ROOT, "bindings", "python"),
                                                                       find_pyd()])))
    js_env = dict(os.environ, NODE_PATH=os.path.join(ROOT, "bindings", "node_modules"), NODE_ENV="production")
    rust_exe = os.path.join(ROOT, "bindings", "rust", "target", "release", "examples", "bench_app.exe")
    bench_bin = os.path.join(ROOT, "bindings", "rust", "bench", "target", "release")
    py = [sys.executable, "-X", "frozen_modules=on"]
    ours = [
        ("tkinter syntax (Python)", py + [os.path.join(APPS, "tk_app.py")], py_env),
        ("Qt syntax (Python)", py + [os.path.join(APPS, "qt_app.py")], py_env),
        ("Blender components (Python)", py + [os.path.join(APPS, "components_app.py")], py_env),
        ("Rust (HTML + CSS)", [rust_exe], dict(os.environ)),
        ("JavaScript (React on Node.js)", ["node", os.path.join(APPS, "js_app.js")], js_env),
        # the same gpui-syntax source on blenderui, gpui and gpui-fast
        ("gpui scene: blenderui", [os.path.join(bench_bin, "bui-settings.exe")], dict(os.environ)),
    ]
    out = []
    for name, cmd, env in ours:
        for renderer, flag in (("CPU", "0"), ("GPU", "1")):
            out.append((f"{name}, {renderer}", cmd, dict(env, BUI_GPU=flag)))
    out.append(("gpui scene: gpui", [os.path.join(bench_bin, "gpui-settings.exe")], dict(os.environ)))
    out.append(("gpui scene: gpui-fast", [os.path.join(bench_bin, "gpui-fast-settings.exe")], dict(os.environ)))
    return out


def launch(cmd, env, timeout=30.0):
    """Start `cmd`; returns (process, seconds to "ready", CPU seconds then, MB then)."""
    t0 = time.perf_counter()
    p = subprocess.Popen(cmd, env=env, stdout=subprocess.PIPE, stderr=subprocess.PIPE, text=True)
    got = threading.Event()

    def read():
        for line in p.stdout:
            if line.strip() == "ready":
                got.set()
                return

    threading.Thread(target=read, daemon=True).start()
    if not got.wait(timeout):
        err = p.stderr.read() if p.poll() is not None else ""
        p.kill()
        raise RuntimeError(f"{cmd[-1]}: no first frame within {timeout}s\n{err[-2000:]}")
    dt = time.perf_counter() - t0
    h = int(p._handle)  # the Popen handle has query rights
    return p, dt, cpu_seconds(h), memory_mb(h)


def stop(p):
    p.kill()
    p.wait()


def bench(name, cmd, env, runs, idle_s):
    launch(cmd, env)[0].kill()  # warm the file cache
    starts, cpus, mems = [], [], []
    for _ in range(runs):
        p, dt, cpu, mem = launch(cmd, env)
        starts.append(dt * 1000)
        cpus.append(cpu * 1000)
        mems.append(mem)
        stop(p)
    # idle: one window left alone; skip a second for deferred startup work
    p, *_ = launch(cmd, env)
    h = int(p._handle)
    time.sleep(1.0)
    c0, t0 = cpu_seconds(h), time.perf_counter()
    time.sleep(idle_s)
    idle = (cpu_seconds(h) - c0) / (time.perf_counter() - t0) * 100
    mem_idle = memory_mb(h)
    stop(p)
    return {
        "name": name,
        "start_ms": round(statistics.median(starts), 1),
        "start_best_ms": round(min(starts), 1),
        "start_cpu_ms": round(statistics.median(cpus), 1),
        "memory_mb": round(statistics.median(mems), 1),
        "memory_idle_mb": round(mem_idle, 1),
        "idle_cpu_pct": round(idle, 4),
        "idle_cpu_ms_per_min": round(idle / 100 * 60_000, 1),
    }


def main():
    runs = int(sys.argv[1]) if len(sys.argv) > 1 else 10
    idle_s = float(sys.argv[2]) if len(sys.argv) > 2 else 10.0
    results = []
    for name, cmd, env in variants():
        try:
            r = bench(name, cmd, env, runs, idle_s)
        except Exception as e:  # noqa: BLE001 - report and go on
            print(f"{name}: FAILED: {e}", file=sys.stderr)
            continue
        print(json.dumps(r), flush=True)
        results.append(r)
    out = os.path.join(ROOT, "bench", "results.json")
    with open(out, "w", encoding="utf-8") as f:
        json.dump({"runs": runs, "idle_seconds": idle_s, "python": sys.version.split()[0], "cpu": cpu_name(),
                   "results": results}, f, indent=2)
    print(f"wrote {out}")


if __name__ == "__main__":
    main()
