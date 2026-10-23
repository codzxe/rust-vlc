#!/usr/bin/env python3
"""Lumen performance-baseline harness (card 3.F1, docs/12-performance-
benchmarking.md §4.1 Phase-2/Phase-3 capture).

MEASURES (nothing here is a target — Phase 3 claims none):

  startup   process wall for `lumen --help` / `lumen --version` (N>=20,
            median + p95), plus the doc-12 M-01 signal: spawn -> first
            `system.get-engine-info` response over --control-stdio, and a
            best-effort COLD variant (posix_fadvise DONTNEED on the binary
            before each run; the loader/shared-lib pages cannot be evicted
            from userspace, so cold rows are an UPPER bound on warmth).
  probe     `session.open` write -> `media.info-ready` event line read,
            through the real --control-stdio --json NDJSON session on the
            committed fixtures. Host-measured at command-layer receipt
            (doc 12 §1.5 — the engine's ts_ns envelope field is not in the
            v1 catalog), so the number includes pipe transit + serde. The
            MP4 fixture measures the probe's TYPED REFUSAL latency (the
            format is identified, D-018 ships no demuxer for it) — an
            honest refusal-timing row, not a media-info row.
  decode    decode-bench repeated runs (N>=10): wall_s (process wall:
            open + full-file decode-to-null) and decode_s (the packet
            loop). This is the card's prescribed time-to-first-frame /
            first-sound family: the v1 catalog has no
            `video.first-frame-presented` event and no `audio.first-sound`
            counter (doc 12 §2.4 items 2-3 unlanded), and the CLI engine
            is poll-advanced (no wall-clock playback), so a per-frame TTF
            is NOT measurable through the contract in this build. What IS
            real: whole-file open+decode wall, decode-only wall, and the
            per-frame mean derived from them.
  seek      scripted `playback.seek` (percent 0.5, then 0.25 in the same
            session) -> response line and `timing.seek-completed` event
            line, per --control-stdio session. The dispatch awaits the
            terminal internally, so the wire-visible response latency
            covers the whole native seek flow.
  memory    peak RSS per process via /proc/<pid>/status `VmHWM`
            tight-poll sampling (kernel high-water of the process's own
            image; `os.wait4`'s ru_maxrss is UNUSABLE from a Python parent
            — the fork's pre-exec Python image contaminates the child's
            signal->maxrss, measured: wait4(/bin/true)=~13 MiB vs the
            child's real ~2 MiB). Scenarios: decode-bench, a full
            scripted session (open + 2 seeks + stop + close), --help as
            the process floor. Sample counts ride the raw rows.
  stats     evidence probe for the M-09/M-10 counter surface: TWO
            requests, each on its own fresh live session —
            `system.get-stats` (doc 12 §2.4 item 4's [PROPOSAL]: in the
            wire catalog at all?) and `system.get-state-snapshot` (the
            nearest cataloged surface: served by this host?). Both
            answers recorded verbatim; the counters are NOT reachable
            through the command layer in this build (DNF, honest).

HONESTY RULES (wave-5 brief, binding): every number in docs/perf/
baseline.md comes from a raw file written by THIS script during the
recorded run; medians/p95 are nearest-rank over the raw rows; failures
are recorded as failures, never dropped; no warmup samples are discarded.

USAGE (see docs/perf/baseline.md for the committed run):

  python3 tools/perf/baseline.py all                     # full capture
  python3 tools/perf/baseline.py startup --runs 25
  python3 tools/perf/baseline.py probe --runs 20
  python3 tools/perf/baseline.py decode --runs 12
  python3 tools/perf/baseline.py seek --runs 20
  python3 tools/perf/baseline.py memory --runs 5
  python3 tools/perf/baseline.py stats
  python3 tools/perf/baseline.py environment            # fingerprint only

Common flags: --binary PATH (default <repo>/../target/release/lumen i.e.
the shared CARGO_TARGET_DIR release build), --raw-dir docs/perf/raw,
--fixtures (repeatable; default = the three committed class-A parents).
"""

from __future__ import annotations

import argparse
import csv
import datetime as dt
import json
import os
import platform
import subprocess
import sys
import time
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parent.parent.parent


def resolve_default_binary() -> Path:
    """The release `lumen` to drive, by default: the shared
    CARGO_TARGET_DIR of the main tree when running inside a worktree
    (../target/release/lumen), else the repo's own target dir. The
    --binary flag overrides; $LUMEN_PERF_BINARY wins over both."""
    env = os.environ.get("LUMEN_PERF_BINARY")
    if env:
        return Path(env)
    candidates = [REPO_ROOT / "target" / "release" / "lumen"]
    if REPO_ROOT.parent.name == "worktrees":
        # a git worktree: the main tree one level up carries the shared
        # CARGO_TARGET_DIR the wave agents build into
        candidates.insert(
            0, REPO_ROOT.parent.parent / "target" / "release" / "lumen")
    for candidate in candidates:
        if candidate.exists():
            return candidate
    return candidates[0]


DEFAULT_BINARY = resolve_default_binary()
DEFAULT_RAW_DIR = REPO_ROOT / "docs" / "perf" / "raw"
DEFAULT_FIXTURES = [
    "avi_mjpeg_pcm.avi",
    "avi_mjpeg_2pcm.avi",
    "mp4_mjpeg_pcm.mp4",
]
FIXTURES_DIR = REPO_ROOT / "tests" / "fixtures" / "generated" / "valid"

NS_PER_MS = 1_000_000


# ---------------------------------------------------------------------------
# Small utilities
# ---------------------------------------------------------------------------

def now_ms() -> float:
    """Monotonic clock, milliseconds (time.perf_counter_ns)."""
    return time.perf_counter_ns() / NS_PER_MS


def median(values: list[float]) -> float:
    ordered = sorted(values)
    middle = len(ordered) // 2
    if len(ordered) % 2:
        return ordered[middle]
    return (ordered[middle - 1] + ordered[middle]) / 2.0


def p95(values: list[float]) -> float:
    """Nearest-rank 95th percentile: the ceil(N*0.95)-th smallest
    value (1-based). N=20 -> rank 19 (second-worst); N=12 -> rank 12
    (worst); N=5 -> rank 5 (worst). Nothing is interpolated."""
    ordered = sorted(values)
    rank = max(1, -(-len(ordered) * 95 // 100))  # ceil(N*0.95), 1-based
    return ordered[rank - 1]


def stats_row(values: list[float]) -> dict[str, float]:
    return {
        "n": len(values),
        "median": round(median(values), 3),
        "p95": round(p95(values), 3),
        "min": round(min(values), 3),
        "max": round(max(values), 3),
    }


def write_csv(path: Path, header: list[str], rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("w", newline="", encoding="utf-8") as handle:
        writer = csv.DictWriter(handle, fieldnames=header)
        writer.writeheader()
        writer.writerows(rows)
    print(f"raw: {path.relative_to(REPO_ROOT)} ({len(rows)} rows)")


def append_jsonl(path: Path, record: dict) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with path.open("a", encoding="utf-8") as handle:
        handle.write(json.dumps(record, sort_keys=True) + "\n")


def run_status_text(code: int) -> str:
    return "ok" if code == 0 else f"exit{code}"


# ---------------------------------------------------------------------------
# Environment fingerprint (honest, read from /proc)
# ---------------------------------------------------------------------------

def read_first(path: str, field: str) -> str:
    """First /proc line whose field name (before ':') is exactly
    `field` — a prefix match would confuse `model` with `model name`."""
    try:
        with open(path, encoding="utf-8") as handle:
            for line in handle:
                if line.split(":", 1)[0].strip() == field:
                    return line.split(":", 1)[1].strip()
    except OSError:
        pass
    return "?"


def environment_snapshot(binary: Path) -> dict:
    cpu_model = read_first("/proc/cpuinfo", "model name")
    cpu_family = read_first("/proc/cpuinfo", "cpu family")
    cpu_model_id = read_first("/proc/cpuinfo", "model")
    cores = os.cpu_count() or -1
    mem_total_kb = read_first("/proc/meminfo", "MemTotal")
    kernel = platform.release()
    stat = binary.stat()
    try:
        import hashlib
        md5 = hashlib.md5(binary.read_bytes()).hexdigest()
    except OSError:
        md5 = "?"
    try:
        git_head = subprocess.run(
            ["git", "rev-parse", "HEAD"],
            cwd=REPO_ROOT, capture_output=True, text=True, check=True,
        ).stdout.strip()
    except (OSError, subprocess.CalledProcessError):
        git_head = "?"
    return {
        "captured_utc": dt.datetime.now(dt.timezone.utc).isoformat(),
        "hostname": platform.node() or "?",
        "kernel": kernel,
        "cpu": {
            "model_name": cpu_model,
            "family": cpu_family,
            "model_id": cpu_model_id,
            "logical_cores": cores,
        },
        "mem_total_kb": mem_total_kb,
        "python": platform.python_version(),
        "binary": {
            "path": str(binary),
            "bytes": stat.st_size,
            "md5": md5,
            "mtime_utc": dt.datetime.fromtimestamp(
                stat.st_mtime, dt.timezone.utc).isoformat(),
        },
        "git_head": git_head,
        "clock": "time.perf_counter_ns (monotonic)",
        "timing_note": (
            "shared 2-core sandbox; other wave agents may run concurrently — "
            "medians+raw are the record, single rows are not"
        ),
    }


def cmd_environment(args: argparse.Namespace) -> dict:
    snap = environment_snapshot(args.binary)
    out = args.raw_dir / "environment.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(snap, indent=2, sort_keys=True) + "\n", encoding="utf-8")
    print(f"raw: {out.relative_to(REPO_ROOT)}")
    for key, value in snap.items():
        print(f"  {key}: {value}")
    return snap


# ---------------------------------------------------------------------------
# Family 1 — process startup
# ---------------------------------------------------------------------------

def time_argv(binary: Path, argv: list[str]) -> tuple[float, int]:
    """Wall of one full process run (spawn -> exit)."""
    start = now_ms()
    code = subprocess.run(
        [str(binary), *argv], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    ).returncode
    return now_ms() - start, code


def evict_binary_pages(binary: Path) -> None:
    """Best-effort cold-run prep: posix_fadvise(DONTNEED) over the
    binary. Userspace cannot evict the loader / shared-lib / rust std
    pages, so 'cold' here is an upper bound on true cold."""
    try:
        fd = os.open(binary, os.O_RDONLY)
        try:
            os.posix_fadvise(fd, 0, 0, os.POSIX_FADV_DONTNEED)
        finally:
            os.close(fd)
    except OSError as failure:
        print(f"  fadvise failed: {failure}", file=sys.stderr)


def time_ready_signal(binary: Path) -> tuple[float, int]:
    """Doc-12 M-01: spawn -> first successful `system.get-engine-info`
    response line, over --control-stdio (the machine surface)."""
    start = now_ms()
    proc = subprocess.Popen(
        [str(binary), "--control-stdio"],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, text=True, bufsize=1,
    )
    try:
        assert proc.stdin is not None and proc.stdout is not None
        proc.stdin.write('{"requestId": 1, "cmd": "system.get-engine-info"}\n')
        proc.stdin.flush()
        line = proc.stdout.readline()
        ready = now_ms() - start
        ok = '"error"' not in line and '"engine-info"' in line
        proc.stdin.close()
        proc.stdout.close()
        code = proc.wait()
        # A real bool: the predecessor returned an int exit-code-style
        # 0/1 here, and the caller's `status = "ok" if ok else ...`
        # read 0 (success) as falsy — every ready row was mislabeled
        # bad-response and silently dropped from the summary.
        return ready, bool(ok and code == 0)
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()


def cmd_startup(args: argparse.Namespace) -> None:
    rows: list[dict] = []
    for mode in ("--help", "--version"):
        for run in range(1, args.runs + 1):
            wall, code = time_argv(args.binary, [mode])
            rows.append({
                "mode": mode, "variant": "warm", "run": run,
                "wall_ms": round(wall, 3), "status": run_status_text(code),
            })
    for run in range(1, args.cold_runs + 1):
        evict_binary_pages(args.binary)
        wall, code = time_argv(args.binary, ["--version"])
        rows.append({
            "mode": "--version", "variant": "cold-fadvise", "run": run,
            "wall_ms": round(wall, 3), "status": run_status_text(code),
        })
    for run in range(1, args.runs + 1):
        ready, ok = time_ready_signal(args.binary)
        rows.append({
            "mode": "ready(engine-info)", "variant": "warm", "run": run,
            "wall_ms": round(ready, 3), "status": "ok" if ok else "bad-response",
        })
    write_csv(
        args.raw_dir / "startup.csv",
        ["mode", "variant", "run", "wall_ms", "status"],
        rows,
    )


# ---------------------------------------------------------------------------
# Family 2 — probe latency (open -> media.info-ready)
# ---------------------------------------------------------------------------

OPEN_REQUEST = (
    '{{"requestId": 1, "cmd": "session.open", "args": {{"source": '
    '{{"kind": "file", "path": "{path}"}}}}}}\n'
)


def probe_once(binary: Path, fixture: Path) -> dict:
    """One fresh --control-stdio --json session: write the open, stamp
    the response and the media.info-ready event at line receipt."""
    proc = subprocess.Popen(
        [str(binary), "--control-stdio", "--json"],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, text=True, bufsize=1,
    )
    try:
        assert proc.stdin is not None and proc.stdout is not None
        t_write = now_ms()
        proc.stdin.write(OPEN_REQUEST.format(path=fixture))
        proc.stdin.flush()
        t_write_done = now_ms()
        response_ms = media_info_ms = ""
        outcome = "open-error"
        # The dispatch is synchronous: response first, then the events
        # the open's pumps produced, then EOF teardown output.
        deadline = time.monotonic() + 30.0
        while time.monotonic() < deadline:
            line = proc.stdout.readline()
            if not line:
                break
            t_line = now_ms()
            if '"requestId":1,' in line:
                response_ms = round(t_line - t_write_done, 3)
                if '"error"' in line:
                    outcome = "typed-refusal"
                    break
                outcome = "accepted"
            elif '"media.info-ready"' in line:
                media_info_ms = round(t_line - t_write_done, 3)
                outcome = "media-info"
                break
        proc.stdin.close()
        proc.stdout.close()
        code = proc.wait()
        return {
            "fixture": fixture.name,
            "response_ms": response_ms,
            "media_info_ms": media_info_ms,
            "outcome": outcome,
            "write_ms": round(t_write_done - t_write, 3),
            "exit": run_status_text(code),
        }
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()


def cmd_probe(args: argparse.Namespace) -> None:
    rows = []
    for name in args.fixtures:
        fixture = FIXTURES_DIR / name
        for run in range(1, args.runs + 1):
            row = probe_once(args.binary, fixture)
            row["run"] = run
            rows.append(row)
            if row["outcome"] not in ("media-info", "typed-refusal"):
                print(f"  !! {name} run {run}: outcome={row['outcome']}")
    write_csv(
        args.raw_dir / "probe.csv",
        ["fixture", "run", "write_ms", "response_ms", "media_info_ms",
         "outcome", "exit"],
        rows,
    )


# ---------------------------------------------------------------------------
# Family 3 — time-to-first-frame / first-sound proxy (decode-bench)
# ---------------------------------------------------------------------------

def decode_once(binary: Path, fixture: Path) -> dict:
    start = now_ms()
    proc = subprocess.run(
        [str(binary), "decode-bench", str(fixture)],
        capture_output=True, text=True, timeout=120,
    )
    wall_ext_ms = now_ms() - start
    row = {
        "fixture": fixture.name,
        "status": run_status_text(proc.returncode),
        "wall_ext_ms": round(wall_ext_ms, 3),
        "bench_wall_s": "", "bench_decode_s": "",
        "v_frames": "", "v_decoded": "", "a_frames": "", "a_decoded": "",
        "bytes": "",
    }
    for line in proc.stdout.splitlines():
        if line.startswith("BENCH "):
            fields = dict(
                token.split("=", 1) for token in line.split()[1:] if "=" in token
            )
            row.update({
                "bench_wall_s": fields.get("wall_s", ""),
                "bench_decode_s": fields.get("decode_s", ""),
                "v_frames": fields.get("v_frames", ""),
                "v_decoded": fields.get("v_decoded", ""),
                "a_frames": fields.get("a_frames", ""),
                "a_decoded": fields.get("a_decoded", ""),
                "bytes": fields.get("bytes", ""),
            })
            break
    if proc.returncode != 0:
        # keep the typed refusal's first line — the honest DNF evidence
        row["refusal"] = proc.stderr.strip().splitlines()[0] if proc.stderr.strip() else ""
    return row


def cmd_decode(args: argparse.Namespace) -> None:
    rows = []
    for name in args.fixtures:
        fixture = FIXTURES_DIR / name
        for run in range(1, args.runs + 1):
            row = decode_once(args.binary, fixture)
            row["run"] = run
            rows.append(row)
            if row["status"] != "ok":
                print(f"  !! {name} run {run}: {row.get('refusal', row['status'])}")
    write_csv(
        args.raw_dir / "decode.csv",
        ["fixture", "run", "status", "wall_ext_ms", "bench_wall_s",
         "bench_decode_s", "v_frames", "v_decoded", "a_frames", "a_decoded",
         "bytes", "refusal"],
        rows,
    )


# ---------------------------------------------------------------------------
# Family 4 — seek latency (scripted seek -> seek-completed)
# ---------------------------------------------------------------------------

SEEK_SCRIPT = (
    OPEN_REQUEST  # requestId 1 (has the {path} placeholder, braces doubled)
    + '{{"requestId": 2, "cmd": "playback.seek", '
    '"args": {{"target": {{"kind": "percent", "fraction": 0.5}}}}}}\n'
    + '{{"requestId": 3, "cmd": "playback.seek", '
    '"args": {{"target": {{"kind": "percent", "fraction": 0.25}}}}}}\n'
    + '{{"requestId": 4, "cmd": "playback.stop"}}\n'
    + '{{"requestId": 5, "cmd": "session.close"}}\n'
)


def seek_once(binary: Path, fixture: Path) -> dict:
    """One fresh session: open (auto-plays during the settle, lands
    wherever the machine-mode open leaves it), then two percent seeks.
    Stamps: each request write-done -> its response line, and write-done
    -> the `timing.seek-completed` event line that follows it."""
    proc = subprocess.Popen(
        [str(binary), "--control-stdio", "--json"],
        stdin=subprocess.PIPE, stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL, text=True, bufsize=1,
    )
    try:
        assert proc.stdin is not None and proc.stdout is not None
        script = SEEK_SCRIPT.format(path=fixture)
        lines = script.splitlines(keepends=True)
        row: dict = {"fixture": fixture.name, "open_response_ms": "",
                     "seek1_response_ms": "", "seek1_event_ms": "",
                     "seek2_response_ms": "", "seek2_event_ms": "",
                     "outcome": "incomplete", "exit": ""}
        # Interleave writes and reads exactly like a controller would:
        # write open, wait for its response, then the seeks.
        proc.stdin.write(lines[0])
        proc.stdin.flush()
        t0 = now_ms()
        seek_t: dict[int, float] = {}
        pending = {2, 3}
        state = "open"
        deadline = time.monotonic() + 60.0
        while time.monotonic() < deadline:
            line = proc.stdout.readline()
            if not line:
                break
            t_line = now_ms()
            if state == "open" and '"requestId":1,' in line:
                row["open_response_ms"] = round(t_line - t0, 3)
                if '"error"' in line:
                    row["outcome"] = "open-error"
                    break
                state = "seek1"
                proc.stdin.write(lines[1])
                proc.stdin.flush()
                seek_t[2] = now_ms()
            elif state == "seek1":
                if '"timing.seek-failed"' in line:
                    row["outcome"] = "seek1-failed-event"
                    break
                if '"requestId":2,' in line:
                    row["seek1_response_ms"] = round(t_line - seek_t[2], 3)
                    if '"error"' in line:
                        row["outcome"] = "seek1-error"
                        break
                elif '"timing.seek-completed"' in line:
                    row["seek1_event_ms"] = round(t_line - seek_t[2], 3)
                    state = "seek2"
                    proc.stdin.write(lines[2])
                    proc.stdin.flush()
                    seek_t[3] = now_ms()
            elif state == "seek2":
                if '"timing.seek-failed"' in line:
                    row["outcome"] = "seek2-failed-event"
                    break
                if '"requestId":3,' in line:
                    row["seek2_response_ms"] = round(t_line - seek_t[3], 3)
                    if '"error"' in line:
                        row["outcome"] = "seek2-error"
                        break
                elif '"timing.seek-completed"' in line:
                    row["seek2_event_ms"] = round(t_line - seek_t[3], 3)
                    row["outcome"] = "completed"
                    break
        for extra in lines[3:]:
            try:
                proc.stdin.write(extra)
            except OSError:
                break
        proc.stdin.flush()
        proc.stdin.close()
        proc.stdout.close()
        row["exit"] = run_status_text(proc.wait())
        return row
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()


def cmd_seek(args: argparse.Namespace) -> None:
    rows = []
    for name in args.fixtures:
        fixture = FIXTURES_DIR / name
        for run in range(1, args.runs + 1):
            row = seek_once(args.binary, fixture)
            row["run"] = run
            rows.append(row)
            if row["outcome"] != "completed":
                print(f"  !! {name} run {run}: outcome={row['outcome']}")
    write_csv(
        args.raw_dir / "seek.csv",
        ["fixture", "run", "open_response_ms", "seek1_response_ms",
         "seek1_event_ms", "seek2_response_ms", "seek2_event_ms",
         "outcome", "exit"],
        rows,
    )


# ---------------------------------------------------------------------------
# Family 5 — peak memory (/proc VmHWM sampling)
# ---------------------------------------------------------------------------

def peak_rss_once(binary: Path, argv: list[str], stdin_script: str | None) -> dict:
    """Peak RSS via /proc/<pid>/status `VmHWM` tight-poll sampling.

    Why not os.wait4's ru_maxrss: the child is forked from THIS Python
    process, and the kernel folds the pre-exec (Python image) mm's
    high-water into the child's signal->maxrss at execve — measured on
    this machine: `wait4(/bin/true)` reports ~13 MiB (the Python image)
    while /proc VmHWM for the same child reports its own ~1-2 MiB. The
    VmHWM of the post-exec image is the process's own kernel-truth
    peak; the sampling loop reads it as fast as /proc answers (a read
    is ~0.03 ms; the committed raw rows carry the sample count), with
    one final read after exit-detection — the only race is peak growth
    inside that last sub-millisecond window."""
    proc = subprocess.Popen(
        [str(binary), *argv],
        stdin=subprocess.PIPE if stdin_script is not None else subprocess.DEVNULL,
        stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
    )
    start = now_ms()
    try:
        if stdin_script is not None:
            assert proc.stdin is not None
            proc.stdin.write(stdin_script.encode())
            proc.stdin.flush()
            proc.stdin.close()
        status_path = f"/proc/{proc.pid}/status"
        peak_kib = 0
        samples = 0
        while True:
            try:
                with open(status_path, encoding="ascii") as handle:
                    for line in handle:
                        if line.startswith("VmHWM:"):
                            value = int(line.split()[1])
                            if value > peak_kib:
                                peak_kib = value
                            samples += 1
                            break
            except OSError:
                break  # process gone: /proc entry removed
            if proc.poll() is not None:
                try:
                    with open(status_path, encoding="ascii") as handle:
                        for line in handle:
                            if line.startswith("VmHWM:"):
                                value = int(line.split()[1])
                                if value > peak_kib:
                                    peak_kib = value
                                samples += 1
                                break
                except OSError:
                    pass
                break
        code = proc.wait()
        return {
            "peak_rss_kib": peak_kib,
            "hwm_samples": samples,
            "wall_ms": round(now_ms() - start, 3),
            "status": run_status_text(code),
        }
    finally:
        if proc.poll() is None:
            proc.kill()
            proc.wait()


def cmd_memory(args: argparse.Namespace) -> None:
    rows = []
    scenarios: list[tuple[str, list[str], str | None]] = [
        ("help", ["--help"], None),
    ]
    for name in args.fixtures:
        fixture = FIXTURES_DIR / name
        scenarios.append((f"decode-bench:{name}", ["decode-bench", str(fixture)], None))
        scenarios.append((
            f"session:{name}",
            ["--control-stdio", "--json"],
            SEEK_SCRIPT.format(path=fixture),
        ))
    for label, argv, script in scenarios:
        for run in range(1, args.runs + 1):
            row = peak_rss_once(args.binary, argv, script)
            row.update({"scenario": label, "run": run})
            rows.append(row)
            if row["status"] != "ok":
                print(f"  !! {label} run {run}: {row['status']}")
    write_csv(
        args.raw_dir / "memory.csv",
        ["scenario", "run", "peak_rss_kib", "hwm_samples", "wall_ms", "status"],
        rows,
    )


# ---------------------------------------------------------------------------
# Family 6 — stats-surface evidence (M-09/M-10 counters)
# ---------------------------------------------------------------------------

def cmd_stats(args: argparse.Namespace) -> None:
    """Evidence probe for the M-09/M-10 counter surface. Two requests,
    each on its own fresh live session (the first unknown command ends
    the machine-mode loop, so they cannot share one):

      1. `system.get-stats` — the doc-12 §2.4 item-4 proposal: is it in
         the wire catalog at all?
      2. `system.get-state-snapshot` — the nearest CATALOGED surface:
         it exists in the enum but is it SERVED by this host?

    Both answers are recorded verbatim (stdout AND stderr — the unknown
    variant's typed refusal lands on stderr): the measured proof that
    dropped-frame / underrun counters are (or are not) reachable through
    the command layer in this build."""
    out_path = args.raw_dir / "stats-probe.jsonl"
    if out_path.exists():
        out_path.unlink()
    fixture = FIXTURES_DIR / args.fixtures[0]
    for request_id, request in (
        (1, '{"requestId": 1, "cmd": "system.get-stats"}\n'),
        (2, '{"requestId": 2, "cmd": "system.get-state-snapshot"}\n'),
    ):
        proc = subprocess.Popen(
            [str(args.binary), "--control-stdio", "--json"],
            stdin=subprocess.PIPE, stdout=subprocess.PIPE,
            stderr=subprocess.PIPE, text=True, bufsize=1,
        )
        try:
            assert proc.stdin is not None and proc.stdout is not None
            proc.stdin.write(OPEN_REQUEST.format(path=fixture))
            proc.stdin.write(request)
            proc.stdin.flush()
            proc.stdin.close()  # EOF after the requests: the CLI answers, then exits
            lines: list[str] = []
            deadline = time.monotonic() + 30.0
            while time.monotonic() < deadline and len(lines) < 200:
                line = proc.stdout.readline()
                if not line:
                    break
                lines.append(line.rstrip("\n"))
            stderr_text = proc.stderr.read() if proc.stderr is not None else ""
            code = proc.wait()
            append_jsonl(out_path, {
                "requested": request.split('"cmd": ')[1].split('"')[1],
                "context": f"live session on {fixture.name}",
                "exit": run_status_text(code),
                "stdout_lines": lines,
                "stderr": stderr_text.strip().splitlines(),
                "observed_response": next(
                    (line for line in lines
                     if f'"requestId":{request_id}' in line), None),
            })
            print(f"raw: {out_path.relative_to(REPO_ROOT)} [{request_id}/2]")
            for line in lines:
                print(f"  | {line[:200]}")
            for line in stderr_text.strip().splitlines():
                print(f"  ! {line[:200]}")
        finally:
            if proc.poll() is None:
                proc.kill()
                proc.wait()


# ---------------------------------------------------------------------------
# Summary (medians/p95 printed from the raw rows — same numbers the doc
# tables carry; the CSVs are the truth, this is a convenience view)
# ---------------------------------------------------------------------------

def load_csv(path: Path) -> list[dict]:
    if not path.exists():
        return []
    with path.open(newline="", encoding="utf-8") as handle:
        return list(csv.DictReader(handle))


def numeric(row: dict, key: str) -> float | None:
    value = row.get(key, "")
    if value in ("", None):
        return None
    try:
        return float(value)
    except ValueError:
        return None


def cmd_summary(args: argparse.Namespace) -> None:
    summary: dict[str, dict] = {}

    rows = load_csv(args.raw_dir / "startup.csv")
    if rows:
        per: dict[tuple[str, str], list[float]] = {}
        for row in rows:
            if row["status"] == "ok":
                per.setdefault((row["mode"], row["variant"]), []).append(
                    float(row["wall_ms"]))
        summary["startup_ms"] = {
            f"{mode}/{variant}": stats_row(values)
            for (mode, variant), values in sorted(per.items())
        }

    rows = load_csv(args.raw_dir / "probe.csv")
    if rows:
        per: dict[tuple[str, str], list[float]] = {}
        for row in rows:
            if row["outcome"] == "media-info":
                per.setdefault((row["fixture"], "media_info_ms"), []).append(
                    float(row["media_info_ms"]))
            if row["outcome"] == "typed-refusal":
                per.setdefault((row["fixture"], "refusal_response_ms"), []).append(
                    float(row["response_ms"]))
        summary["probe_ms"] = {
            f"{fixture}/{signal}": stats_row(values)
            for (fixture, signal), values in sorted(per.items())
        }

    rows = load_csv(args.raw_dir / "decode.csv")
    if rows:
        per: dict[tuple[str, str], list[float]] = {}
        for row in rows:
            if row["status"] != "ok":
                continue
            for key in ("bench_wall_s", "bench_decode_s"):
                value = numeric(row, key)
                if value is not None:
                    per.setdefault((row["fixture"], key), []).append(value * 1000.0)
            # derived, labeled as such: whole-file decode / decoded frames
            decode_s = numeric(row, "bench_decode_s")
            frames = numeric(row, "v_decoded")
            if decode_s is not None and frames:
                per.setdefault((row["fixture"], "per_frame_ms_derived"), []).append(
                    decode_s * 1000.0 / frames)
        summary["decode_ms"] = {
            f"{fixture}/{key}": stats_row(values)
            for (fixture, key), values in sorted(per.items())
        }

    rows = load_csv(args.raw_dir / "seek.csv")
    if rows:
        per: dict[tuple[str, str], list[float]] = {}
        for row in rows:
            if row["outcome"] != "completed":
                continue
            for key in ("seek1_event_ms", "seek2_event_ms",
                        "seek1_response_ms", "seek2_response_ms",
                        "open_response_ms"):
                value = numeric(row, key)
                if value is not None:
                    per.setdefault((row["fixture"], key), []).append(value)
        summary["seek_ms"] = {
            f"{fixture}/{key}": stats_row(values)
            for (fixture, key), values in sorted(per.items())
        }

    rows = load_csv(args.raw_dir / "memory.csv")
    if rows:
        per: dict[str, list[float]] = {}
        for row in rows:
            # Refusal-path rows (exit65 on the MP4) are real processes
            # that really peaked — keep them, tagged by their status, so
            # the doc's refusal row carries measured data instead of a
            # silent drop.
            key = row["scenario"] if row["status"] == "ok" \
                else f"{row['scenario']} [{row['status']}-refusal]"
            per.setdefault(key, []).append(float(row["peak_rss_kib"]))
        summary["peak_rss_kib"] = {
            scenario: stats_row(values) for scenario, values in sorted(per.items())
        }

    out = args.raw_dir / "summary.json"
    out.parent.mkdir(parents=True, exist_ok=True)
    out.write_text(json.dumps(summary, indent=2, sort_keys=True) + "\n",
                   encoding="utf-8")
    print(f"raw: {out.relative_to(REPO_ROOT)}")
    print(json.dumps(summary, indent=2, sort_keys=True))


FAMILY_DEFAULT_RUNS = {
    "startup": 25,
    "probe": 20,
    "decode": 12,
    "seek": 20,
    "memory": 5,
}


def cmd_all(args: argparse.Namespace) -> None:
    """Full capture with each family's documented default run count
    (the committed run) — families not controllable through --runs here
    on purpose: the record must be reproducible with one command."""
    cmd_environment(args)
    for family, runner in (("startup", cmd_startup), ("probe", cmd_probe),
                           ("decode", cmd_decode), ("seek", cmd_seek),
                           ("memory", cmd_memory)):
        args.runs = FAMILY_DEFAULT_RUNS[family]
        args.cold_runs = 5
        runner(args)
    cmd_stats(args)
    cmd_summary(args)


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> int:
    # Common flags live on BOTH the main parser (real defaults) and every
    # subparser (SUPPRESS defaults), so `--binary X all` and `all --binary X`
    # both work — the suppressed form only writes the attribute when the flag
    # is actually given, so a pre-family value is never clobbered by the
    # subparser's default (the classic argparse trap).
    def common_flags(target: argparse.ArgumentParser, suppress: bool):
        target.add_argument(
            "--binary", type=Path,
            default=argparse.SUPPRESS if suppress else DEFAULT_BINARY,
            help=f"lumen binary (default {DEFAULT_BINARY})")
        target.add_argument(
            "--raw-dir", type=Path,
            default=argparse.SUPPRESS if suppress else DEFAULT_RAW_DIR,
            help=f"raw-data output dir (default {DEFAULT_RAW_DIR})")
        target.add_argument(
            "--fixtures", nargs="+",
            default=argparse.SUPPRESS if suppress else DEFAULT_FIXTURES,
            help="fixture file names under tests/fixtures/generated/valid/")

    parser = argparse.ArgumentParser(
        description="Lumen perf baseline capture (card 3.F1) — measures only, "
                    "no targets.")
    common_flags(parser, suppress=False)
    sub = parser.add_subparsers(dest="family", required=True)

    def add(name: str, runner, runs: int, help_text: str, **extra):
        cmd = sub.add_parser(name, help=help_text)
        common_flags(cmd, suppress=True)
        if runs:
            cmd.add_argument("--runs", type=int, default=runs,
                             help=f"repetitions per row (default {runs})")
        for flag, default in extra.items():
            cmd.add_argument(f"--{flag.replace('_', '-')}", type=int,
                             default=default)
        cmd.set_defaults(runner=runner)
        return cmd

    add("all", cmd_all, 0, "full capture: every family + environment")
    add("environment", cmd_environment, 0, "environment fingerprint only")
    add("startup", cmd_startup, 25, "process startup (--help/--version/ready)",
        cold_runs=5)
    add("probe", cmd_probe, 20, "open -> media.info-ready latency")
    add("decode", cmd_decode, 12, "decode-bench wall_s/decode_s runs")
    add("seek", cmd_seek, 20, "scripted seek -> seek-completed latency")
    add("memory", cmd_memory, 5, "peak RSS (VmHWM /proc sampling) per scenario")
    for name, runner, help_text in (
        ("stats", cmd_stats, "system.get-stats surface evidence"),
        ("summary", cmd_summary, "recompute summary.json from raw CSVs"),
    ):
        cmd = sub.add_parser(name, help=help_text)
        common_flags(cmd, suppress=True)
        cmd.set_defaults(runner=runner)

    args = parser.parse_args()
    for flag, default in (("binary", DEFAULT_BINARY),
                          ("raw_dir", DEFAULT_RAW_DIR),
                          ("fixtures", DEFAULT_FIXTURES)):
        if not hasattr(args, flag):
            setattr(args, flag, default)
    if not args.binary.exists():
        parser.error(f"binary not found: {args.binary} (build it first)")
    args.runner(args)
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
