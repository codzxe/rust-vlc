# Soak scenario-suite report

- Manifest: `/home/z/my-project/wt/w24b/tests/soak/scenarios-sandbox.toml` (5 scenario(s))
- Profile: release build, linux/x86_64
- Captured: 2026-11-10T18:50:07Z (UTC)
- Total scenario wall: 3.6 min (216 s)
- Method: every op is a catalog `WireRequest` through the soak dispatch seam onto the real engine stack (access → AVI demux → MJPEG/PCM decode → counting sink) against the committed `tests/fixtures/` media; fixtures' seek targets and track facts are read from their recorded expectation TOMLs, never transcribed.
- Instruments: `live_bytes` (process-global counting allocator — the precise leak signal), `VmRSS` (`/proc/self/status`), fd count (`/proc/self/fd`, count-driven scenarios, Linux only).
- Warmup: the first 15% of each scenario is excluded from the fits (pool sizing, allocator steady state); the series keeps the samples.
- Slopes: least squares over the measured window. Every MB/h figure below is an EXTRAPOLATION from that window, not a 4 h measurement.

## Summary

| scenario | kind | wall | cycles | live slope (MB/h) | RSS slope (MB/h) | verdict |
|---|---|---|---|---|---|---|
| open-close-churn | open-close-churn | 216.4 s | 2000000 | n/a | n/a | PASS |

## Scenario `open-close-churn` (open-close-churn)

- Driver: 2000000 cycles (configured); wall 216.4 s.
- Fixtures: generated/valid/avi_mjpeg_pcm.avi, generated/malformed/avi_mjpeg_pcm__trunc3.avi.
- Samples: 4090 total, 613 warmup-excluded, measured window 0.0 s.
- Ceiling checked: 50 MB/h; design target 5 MB/h (reported, not asserted (see docs/soak/ for the freeze-or-revise decision)).
- Ops: 2000000 cycles, 2000000 opens (0 error-lane), 0 plays, 0 seeks ok / 0 failed / 0 skipped, 2000000 closes (0 error-lane), 0 playlist passes.
- Decoding: 0 frames, 0 payload bytes at the sinks (the D-6 liveness axis).
- Live-bytes slope: not slope-fitted (this scenario asserts growth budgets, below)
- VmRSS slope: n/a (no measurable samples on this platform or window).
- fd count: baseline 4 → final 4 (max 4); budget 4.
- VmRSS growth: 1720 kB → 1972 kB; budget 8 MB.
- Live bytes: 295117 → 295117 (budget 524288 B).
- Mean net live-bytes drift: +0.0 B/cycle over the measured cycles (the reopen-stress triage number; negative is memory returning).

## Verdict

PASS — every scenario stayed inside its configured liveness and memory budgets.
