# AUDIT: tests/integration/first-vertical/evidence/transcripts/corrupt-noidx.json
| Field | Value |
|---|---|
| Auditor | W16-A08 (recovered by W16-RR1) |
| Date | 2026-11-24 |
| Kind | Committed evidence artifact (transcript/1 JSON) |
| File verdict | REAL-UNPROVEN (coherent recorded transcript; currency vs fresh run unproven) |
| Reachable from shipped entry point? | N/A — test evidence; read by tests/evidence.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256/ledger cross-check; Read full file; rg consumers; baseline citation |

## 1. Verdict
Genuine recorded transcript of the `corrupt-noidx` scenario: one command (session.open on the
class-B fixture generated/malformed/avi_mjpeg_pcm__noidx0.avi), 18 events walking
Idle→Opening→Buffering→Playing→Ended with final position == duration. Ledger digest verified.
Currency vs a fresh run is UNPROVEN: the orchestrator baseline's currency test failed at `open`
(fail-fast) and never reached this id.

## 2. What actually works (verified)
- sha256=fb90a57a…d77b, size 3834 — both present in evidence/ledger.json per my python3 re-hash
  (and the baseline-passing `ledger_matches_the_artifacts`, cargo-test-workspace.txt:2765).
- The scenario it records runs green fresh in the baseline: `test class_b_noidx ... ok`
  (cargo-test-workspace.txt:2755) — the underlying engine behavior is real, only the committed
  bytes' currency is unproven.
- Accounting is internally consistent: staging_frames 16 × 320×240×4B = 4_915_200 == staging_bytes
  (json:184-186); video_presented 16.

## 3. Findings
F-001 [S3]
Where: tests/integration/first-vertical/tests/evidence.rs:37-68 + this artifact
Code: (baseline cargo-test-workspace.txt:2773-2774)
thread 'committed_transcripts_match_fresh_runs' (27977) panicked at tests/integration/first-vertical/tests/evidence.rs:47:9:
assertion `left == right` failed: the committed transcript for `open` drifted from a fresh run — regenerate with: cargo test -p first-vertical --test evidence -- --ignored regen
What is wrong: the committed-transcript currency gate fails at the first scenario id, so this
file's byte-currency vs a fresh run has zero recorded proof (the loop aborts before `corrupt-noidx`).
Why it matters: this JSON is committed as doc 10 §7 evidence; a silent drift would be invisible.
What correct looks like: regenerate the evidence set; the loop then reaches this id.
Confidence: HIGH — Verification: read baseline log.

## 4. What was checked and how
Full JSON read; digest/size vs ledger (python3); event walk (seq 1-19, gap at 7 — same harness
property as every transcript); command/event/gen consistency; fixture name matches the registered
MANIFEST.toml row `avi_mjpeg_pcm__noidx0.avi` (generated/MANIFEST.toml:126-131); arithmetic
check of accounting block. Nothing executes from this file itself.
