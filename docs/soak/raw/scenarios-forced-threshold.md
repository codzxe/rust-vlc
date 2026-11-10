# Soak scenario-suite report

- Manifest: `/home/z/my-project/wt/w24b/tests/soak/scenarios-forced-threshold.toml` (2 scenario(s))
- Profile: release build, linux/x86_64
- Captured: 2026-11-10T19:23:46Z (UTC)
- Total scenario wall: 2.4 min (144 s)
- Method: every op is a catalog `WireRequest` through the soak dispatch seam onto the real engine stack (access → AVI demux → MJPEG/PCM decode → counting sink) against the committed `tests/fixtures/` media; fixtures' seek targets and track facts are read from their recorded expectation TOMLs, never transcribed.
- Instruments: `live_bytes` (process-global counting allocator — the precise leak signal), `VmRSS` (`/proc/self/status`), fd count (`/proc/self/fd`, count-driven scenarios, Linux only).
- Warmup: the first 15% of each scenario is excluded from the fits (pool sizing, allocator steady state); the series keeps the samples.
- Slopes: least squares over the measured window. Every MB/h figure below is an EXTRAPOLATION from that window, not a 4 h measurement.

## Summary

| scenario | kind | wall | cycles | live slope (MB/h) | RSS slope (MB/h) | verdict |
|---|---|---|---|---|---|---|
| open-close-churn | open-close-churn | 84.4 s | 1000000 | n/a | n/a | FAIL (2 violation(s)) |
| seek-storm | seek-storm | 60.0 s | 1763 | -6.951 | -12.661 | PASS |

## Scenario `open-close-churn` (open-close-churn)

- Driver: 1000000 cycles (configured); wall 84.4 s.
- Fixtures: generated/valid/avi_mjpeg_pcm.avi, generated/malformed/avi_mjpeg_pcm__trunc3.avi.
- Samples: 4082 total, 612 warmup-excluded, measured window 0.0 s.
- Ceiling checked: 50 MB/h; design target 5 MB/h (reported, not asserted (see docs/soak/ for the freeze-or-revise decision)).
- Ops: 1000000 cycles, 1000000 opens (0 error-lane), 0 plays, 0 seeks ok / 0 failed / 0 skipped, 1000000 closes (0 error-lane), 0 playlist passes.
- Decoding: 0 frames, 0 payload bytes at the sinks (the D-6 liveness axis).
- Live-bytes slope: not slope-fitted (this scenario asserts growth budgets, below)
- VmRSS slope: n/a (no measurable samples on this platform or window).
- fd count: baseline 4 → final 4 (max 4); budget 0.
- VmRSS growth: 1712 kB → 1972 kB; budget 0.01 MB.
- Live bytes: 294183 → 294265 (budget 0 B).
- Mean net live-bytes drift: +0.0 B/cycle over the measured cycles (the reopen-stress triage number; negative is memory returning).
- VIOLATIONS:
  - bounded memory: live bytes grew from 294183 to 294265 across 849815 cycles (82 B over the post-warmup baseline, budget 0 B)
  - bounded memory: VmRSS grew from 1712 kB to 1972 kB across 849815 cycles (0.25 MB, budget 0.01 MB)

## Scenario `seek-storm` (seek-storm)

- Driver: duration 1.00 min (configured); wall 60.0 s.
- Fixtures: generated/valid/avi_mjpeg_pcm.avi, generated/valid/avi_mjpeg_2pcm.avi.
- Samples: 62 total, 10 warmup-excluded, measured window 50.0 s.
- Ceiling checked: 0.0001 MB/h; design target 5 MB/h (reported, not asserted (see docs/soak/ for the freeze-or-revise decision)).
- Ops: 1763 cycles, 1763 opens (0 error-lane), 0 plays, 28193 seeks ok / 0 failed / 0 skipped, 1763 closes (0 error-lane), 0 playlist passes.
- Decoding: 711312 frames, 0 payload bytes at the sinks (the D-6 liveness axis).
- Live-bytes slope: -1930.9 B/s = **-6.951 MB/h** (EXTRAPOLATION from a 50.0 s window) — negative (memory returning)
- VmRSS slope: -3.6 kB/s = -12.661 MB/h (EXTRAPOLATION; kernel truth, arenas included).

## Verdict

FAIL — 2 violation(s):

- bounded memory: live bytes grew from 294183 to 294265 across 849815 cycles (82 B over the post-warmup baseline, budget 0 B)
- bounded memory: VmRSS grew from 1712 kB to 1972 kB across 849815 cycles (0.25 MB, budget 0.01 MB)
