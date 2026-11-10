# Soak scenario-suite report

- Manifest: `/home/z/my-project/wt/w24b/tests/soak/scenarios-sandbox.toml` (5 scenario(s))
- Profile: release build, linux/x86_64
- Captured: 2026-11-10T18:10:37Z (UTC)
- Total scenario wall: 8.0 min (480 s)
- Method: every op is a catalog `WireRequest` through the soak dispatch seam onto the real engine stack (access → AVI demux → MJPEG/PCM decode → counting sink) against the committed `tests/fixtures/` media; fixtures' seek targets and track facts are read from their recorded expectation TOMLs, never transcribed.
- Instruments: `live_bytes` (process-global counting allocator — the precise leak signal), `VmRSS` (`/proc/self/status`), fd count (`/proc/self/fd`, count-driven scenarios, Linux only).
- Warmup: the first 15% of each scenario is excluded from the fits (pool sizing, allocator steady state); the series keeps the samples.
- Slopes: least squares over the measured window. Every MB/h figure below is an EXTRAPOLATION from that window, not a 4 h measurement.

## Summary

| scenario | kind | wall | cycles | live slope (MB/h) | RSS slope (MB/h) | verdict |
|---|---|---|---|---|---|---|
| playlist-cycle | playlist-cycle | 480.0 s | 66886 | 0.038 | 0.403 | PASS |

## Scenario `playlist-cycle` (playlist-cycle)

- Driver: duration 8.00 min (configured); wall 480.0 s.
- Fixtures: generated/valid/avi_mjpeg_pcm.avi, generated/valid/avi_mjpeg_2pcm.avi, generated/malformed/avi_mjpeg_pcm__noidx0.avi.
- Samples: 482 total, 73 warmup-excluded, measured window 407.0 s.
- Ceiling checked: 50 MB/h; design target 5 MB/h (reported, not asserted (see docs/soak/ for the freeze-or-revise decision)).
- Ops: 66886 cycles, 66886 opens (0 error-lane), 66886 plays, 0 seeks ok / 0 failed / 0 skipped, 66886 closes (0 error-lane), 22295 playlist passes.
- Decoding: 6242720 frames, 383873656800 payload bytes at the sinks (the D-6 liveness axis).
- Live-bytes slope: 10.5 B/s = **0.038 MB/h** (EXTRAPOLATION from a 407.0 s window) — within the 5 MB/h design target
- VmRSS slope: 0.1 kB/s = 0.403 MB/h (EXTRAPOLATION; kernel truth, arenas included).

## Verdict

PASS — every scenario stayed inside its configured liveness and memory budgets.
