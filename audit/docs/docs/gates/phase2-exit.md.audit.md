# CLAIM AUDIT: docs/gates/phase2-exit.md
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Date | 2026-11-24 |
| Claims extracted | 8 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 4 / 2 / 0 / 0 / 0 / 0 / 2 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A dated (2026-10-21) phase-exit record whose honesty structure holds up: the
PASS is explicitly qualified "local-evidence", the two PENDING labels (3-OS CI
on a workflow-scoped token; ADR-003 per-OS verdicts) were real blockers at
the time and are marked as infra, not code — and both have since been
discharged by the tree (the workflows are now live under .github/workflows/
— 11 active files, no .github/workflows-disabled/ remains — and the tauri
shell shipped in the excluded subcrate). The dependency posture claims are
STILL true for the root workspace: root Cargo.lock has no cpal, no wgpu, no
tauri/wry (they live in apps/desktop-tauri's own lock where admitted). The
mock-labeled clauses (audio null device, software compositor) match what
KNOWN-LIMITS.md §4 still says today. The test counts are historical snapshots
(26 first-vertical tests at phase 2; the suite has since grown to 43 test
functions in tests/integration/first-vertical/tests/) — growth, not
contradiction. The unverifiable items are the run claims (fmt/clippy/1329
tests at 021affa) — no git, no cargo-on-PATH in this sandbox.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Verdict: PASS (local-evidence)" with two honest PENDING labels | :3-9 | REAL (as qualified record) | the PENDING items were real (workflows disabled then); the qualification is in the verdict itself |
| 2 | "audio on screen (audible) MET (mock-labeled) ... null device in this sandbox" | table | REAL | KNOWN-LIMITS.md §4 today: "Audio output: null backend only ... no sound on any OS yet" — same posture persists |
| 3 | "video on screen MET (mock-labeled) ... software compositor; wgpu deferred" | table | REAL | root Cargo.lock has no `wgpu` package (rg -> 0); renderer-wgpu ships the software path |
| 4 | "Dependency posture: cpal NOT admitted; wgpu NOT imported; tauri/wry NOT imported" | §3 | REAL (root workspace, still true at HEAD) | rg 'name = "cpal"/"wgpu"/"tauri"/"wry"' Cargo.lock -> 0 hits each; the tauri closure lives in apps/desktop-tauri/Cargo.lock (workspace-EXCLUDED by design) |
| 5 | "workflows are wired (ci.yml + wasm.yml, currently under .github/workflows-disabled/ ...)" | §4.1 | REAL (historical snapshot; since resolved) | .github/workflows-disabled/ does NOT exist at HEAD; 11 active workflows under .github/workflows/; ci.yml:53 "the parked wasm.yml canary is live below as `wasm-build`" — the PENDING was discharged and folded |
| 6 | "tests/integration/first-vertical/** (26 tests)" | §1 table | REAL-UNPROVEN (historical count) | current tree: 43 test fns across the 6 test files (wire_mode 1, journey 7, recovery_matrix 24, class_b 5, evidence 4, baselines_consistency 2) — suite grew; 26-at-phase-2 plausible, not re-runnable |
| 7 | "Baseline: main @ 021affa — fmt CLEAN, clippy -D warnings CLEAN, 1329 passed / 0 failed / 4 ignored" | :11-13 | UNVERIFIED | needs git + cargo (baseline in AGENTS: current 4744 passed — consistent growth, but the 021affa run itself is not re-runnable here) |
| 8 | "Test-count trajectory across the phase: 529 (Phase-1 exit) → 1329 (+800, exact per-merge reconciliations in the worklog)" | §2 footer | UNVERIFIED | worklog.md exists; per-merge reconciliation not re-counted (no git history access) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "wasm.yml ... under .github/workflows-disabled/" (:§4.1) | HEAD: no wasm.yml file; ci.yml:53 says the canary is folded in as `wasm-build` | Both true at their dates; the doc is a snapshot — a reader must not treat §4.1's "currently" as HEAD state |
| "first-vertical (26 tests)" | HEAD: 43 test fns in the crate's tests/ | Historical count; growth is expected across 15 waves |

## 4. Fabrication / inflation indicators
LOW. The doc goes out of its way to label mock-labeled rows, PENDING items,
NOT-SELECTED cards (2.B3), and mock tiers — the anti-inflation genre. The
"MET (mock-labeled)" pattern is the honest form of a stub disclosure. The
one soft spot: "audio on screen (audible) ... MET (mock-labeled)" is generous
phrasing for "the DSP and adapter contract are proven but no audible output
exists" — the label does the honesty work, but a skimmer reads "MET".

## 5. Cross-references verified
- docs/prototypes/video-in-webview.md (cited for ADR-003 to-run list) — exists.
- full_gate.sh exists (canonical gate runner claim) and the AGENTS baseline
  records its current FAIL honestly — the phase2 doc's praise of it ("honest
  exit codes") is borne out by the baseline (it fails loudly, not silently).
- 2.B3 NOT-SELECTED matches the doc-13 vocabulary note in the feature-matrix
  header (:55-57: "not-selected is a wave-CARD status ... NOT a matrix
  status").
