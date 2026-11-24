# CLAIM AUDIT: docs/soak/soak-report-20261110.md (L1-279)
| Field | Value |
|---|---|
| Auditor | W19-A09 |
| Date | 2026-11-24 |
| Claims extracted | 10 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 8 / 0 / 0 / 0 / 0 / 0 / 2 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Every headline number in this synthesis cross-checks against the committed per-scenario raw reports to the digit: 73,834 play-loop cycles, 246,309 seeks / 0 failed, 2,000,000 churn cycles at 216.4 s, 295,117→295,117 live bytes, 1,720→1,972 kB RSS, and the post-warmup arithmetic (2,000,000−300,246=1,699,754; 150,000−22,533=127,467) recomputes exactly. The doc volunteers its own epistemics: slopes are "EXTRAPOLATION from a ≤407 s window, not a 4 h measurement", §8 lists what did NOT run (CI billing-blocked, LSAN, aarch64, non-Linux RSS, real audio), and it corrects the wave briefing's wrong "aarch64" label to the measured x86_64. The only weakness is the shared one of all self-authored perf evidence: the raw reports are produced by the in-tree harness itself — internal consistency is proven here, external re-execution was not possible (cargo build budget).

## 2. Claim ledger
| # | Claim (verbatim, ≤25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "the five-scenario bounded soak RAN for real, in-sandbox, all green" | :3-5 | REAL-UNPROVEN (internally REAL) | Raw reports exist with matching numbers; re-execution not possible here (no release build; cargo budget). Internal consistency fully verified (below). |
| 2 | play-loop: 480.0 s, 73,834 cycles, −0.058 MB/h, 5,439,151 frames / 337.2 GB | §2 | REAL (vs raw) | scenarios-sandbox__play-loop.md: cycles 73834, frames 5439151, payload 337187809200 B, slope −16.1 B/s = −0.058 MB/h (arithmetic checks: 16.1×3600/1e6 = 0.0580). |
| 3 | seek-storm: 15,395 cycles, 246,309 seeks (0 failed), +0.060 MB/h | §2 | REAL (vs raw) | scenarios-sandbox__seek-storm.md:24 "15395 cycles … 246309 seeks ok / 0 failed". |
| 4 | open-close-churn: 2,000,000 cycles, 216.4 s, 295,117→295,117 B, VmRSS 1,720→1,972 kB | §2 | REAL (vs raw) | scenarios-sandbox__open-close-churn.md:20,24,29-30 exact match. |
| 5 | "cycles measured… 1,699,754 post-warmup (of 2.0M)" via tag 300,246→2,000,000; 127,467 of 150k | §2 | REAL (arithmetic) | 2,000,000−300,246=1,699,754 ✓; 150,000−22,533=127,467 ✓ (recomputed). |
| 6 | "482 samples each, 73 warmup-excluded, measured window 407.0 s" | §2 | REAL (vs raw) | play-loop.md: "Samples: 482 total, 73 warmup-excluded, measured window 407.0 s". |
| 7 | "5 MB/h design target is FROZEN on the measured numbers" | header | REAL (policy) | §3 records the freeze decision with measured slopes ~0.04-0.06 vs 5 target — 100× headroom. |
| 8 | "nightly 4 h tier and the LSAN job are wired and billing-blocked" | header, §8 | REAL (labels) | §8 blocked list explicit: CI billing, LSAN disk pressure, 4h walls, aarch64, non-Linux RSS, real audio. |
| 9 | Honest env correction: "the wave briefing called this sandbox 'aarch64' — the machine is x86_64" | env row | REAL | Self-correcting label, raw reports all record linux/x86_64. |
| 10 | Raw evidence committed under docs/soak/raw/ (7 files incl. calibration + forced-threshold) | header | REAL | ls: all 7 files present + scripts/soak-scenarios.sh + tests/soak/{scenarios-sandbox.toml, scenarios.schema.json, …}. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "all green / PASS" | §8 blocked list | No contradiction — PASS claims are scoped to the bounded tier that ran; blocked items labeled. |

## 4. Fabrication / inflation indicators
None. Numbers reconcile; extrapolations are labeled extrapolations; negative slopes (memory returning) are reported as-is. The shared-harness self-measurement caveat (raw produced by in-tree soak_scenarios suite, not an independent observer) is the only meta-concern — flagged, not a finding against this doc.
Commands run: ls docs/soak/raw + scripts + tests/soak; rg/ sed over raw reports; python arithmetic recomputation of 1,699,754/127,467 and MB/h conversions. Exit 0.
Findings: S2: 0 · S3: 0 · S4: 0.
