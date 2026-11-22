# AUDIT: crates/playback-engine/tests/soak/main.rs

| Field | Value |
|---|---|
| Auditor | W03-A05 |
| Date | 2026-11-15 (static analysis; no cargo per adaptations) |
| Lines / functions | 436 / 1 soak `#[test]` + 3 config-parse `#[test]`s + 7 config/harness fns (4 sibling modules: dispatch/memory/report/scenario — not in my assignment) |
| File verdict | REAL-UNPROVEN (a genuine env-configured soak harness with a real nightly workflow) |
| Reachable from shipped entry point? | N/A (test binary). The nightly workflow exists: `.github/workflows/soak-nightly.yml` (verified by LS + rg; whether it has ever run green is wave-17 territory). The default `cargo test` run executes the 2-minute CI-short soak. |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:1 |
| Commands run | full-file read; LS of .github/workflows + rg for SOAK_MINUTES (workflow exists, line 1-6); W00 baseline cross-reference (the default 2-min soak ran inside the 1751-pass suite) |

## 1. One-paragraph verdict

A real soak harness, not a ritual: the open → play → seek → track-switch → reopen loop runs through the command layer for a **real wall duration** (env-configured, garbage refuses loudly — never silently defaults), memory is sampled at a fixed cadence on two instruments (the process-global counting allocator's `live_bytes` plus `VmRSS` from /proc), the first 15% (bounded 1 s..50%) is warmup whose samples are excluded from the least-squares slope fit, and the measured-window slope must stay under a generous 50 MB/h ceiling (the nightly tightens to 5 MB/h via env, honestly labeled `[PROPOSAL until first real measurement]`). Liveness is asserted (D-6): ≥3 cycles, seek and switch skip rates ≤ 25%, at least one seek and one switch completed — "a soak that skips its ops is a ritual, not a measurement" is written into the asserts. The failure message names the top-suspect phase by growth per cycle. The sibling modules (dispatch/memory/report/scenario) carry the machinery. One honest gap: the slope assertion is conditional on a fitted slope existing, so degenerate short-run configs can pass without any leak check.

## 2. What actually works (verified)

- Config parsing is total and loud: `parse_minutes`/`parse_sample_ms`/`parse_leak_mb_per_h` refuse garbage, zero, negative, and out-of-range values with error sentences (97-158), pinned by three parse tests (378-434).
- The liveness asserts (246-286) and the phase-ranked triage in the failure message (356-367) make a green run meaningful and a red run actionable.
- The nightly workflow exists (`.github/workflows/soak-nightly.yml`, with a scenarios tier referencing tests/soak/scenarios-nightly.toml).
- W00 baseline: the default 2-minute soak ran green inside the workspace suite (adds ~2 min; the 1751 count includes `soak_loops_stay_memory_bounded`) — cited, not re-run (ADAPTATION).

## 3. Findings

### F-001 [S4] — The leak assertion silently no-ops when no slope can be fit; the ≥2-samples guard is itself gated to runs ≥ 0.5 min

Where: crates/playback-engine/tests/soak/main.rs:300-320.

Code (300-312, verbatim):
```rust
    if minutes >= 0.5 {
        assert!(
            measured.len() >= 2,
            "the measured window holds {} samples — too few to fit a line at {minutes} min",
            measured.len()
        );
    }

    // ---- THE assertion: the generous ceiling on live bytes ----
    if let Some(slope) = live_slope {
```

What is wrong: `least_squares_slope` presumably answers `None` with fewer than 2 points; in that case the ceiling assertion (the test's entire purpose) is skipped and the test passes as a no-op. The ≥2-samples guard that would catch this only runs when `minutes >= 0.5` — so `SOAK_MINUTES=0.4` (24 s) with a coarse `SOAK_SAMPLE_MS` (e.g. 60000) can produce a one-sample measured window, skip both asserts, and report green. Default configs cannot hit this (2 min / 1 s cadence), and short smoke runs are the documented use of fractional minutes — but a "leak check skipped: N samples" failure or notice would be honest where a silent pass is not.

Confidence: HIGH — Verification: read (slope-None path inferred from the Option handling at 309; the memory module is a sibling file, UNVERIFIED).

## 4. Per-function table

| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (config table, discipline) | 1-49 | REAL-UNPROVEN | — | honest about the [PROPOSAL] 5 MB/h target |
| `ALLOC` global + `live_bytes` | 69-76 | REAL-UNPROVEN | — | the counting allocator instrument |
| consts (CEILING/WARMUP/MIN_CYCLES/MAX_SKIP_RATE) | 81-88 | REAL-UNPROVEN | — | generous-by-design |
| `parse_minutes` / `parse_sample_ms` / `parse_leak_mb_per_h` / accessors / `report_path` | 97-176 | REAL-UNPROVEN | — | loud parsing, all pinned by tests |
| `soak_loops_stay_memory_bounded` | 187-354 | REAL-UNPROVEN | S4 | warmup exclusion, liveness, slope fit + ceiling, markdown report; F-001 conditional |
| `triage_summary` | 357-367 | REAL-UNPROVEN | — | phase-ranked top suspect in failures |
| parse tests ×3 | 378-434 | REAL-UNPROVEN | — | garbage/zero/negative/out-of-range refusal pinned |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `soak_loops_stay_memory_bounded` | YES (default config) | a gross leak breaks the slope ceiling; a stalled loop breaks liveness; the skip-rate caps catch a ritualized scenario — F-001 names the degenerate-config hole |
| parse tests ×3 | YES | exact defaults + refusal matrix |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "wired as the scheduled job `.github/workflows/soak-nightly.yml`" | main.rs:20-21 | REAL (file exists) | LS + rg; whether the schedule ever fired is wave-17 |
| "the nightly tightens it to the 5 MB/h design target" `[PROPOSAL]` | main.rs:21-22 | REAL-UNPROVEN | honestly labeled; no real 4 h measurement blessed it yet (UNVERIFIED — worklog/CI runs are other waves) |
| "Liveness is asserted (D-6) … a soak that skips its ops is a ritual" | main.rs:40-42 | REAL-UNPROVEN | asserts at 246-286 |

## 7. Dead / duplicated / unreachable code

None in this file. The `#[cfg(test)] mod tests` inside a test binary is redundant-looking but harmless (unit tests of the config parsers inside the integration binary).

## 8. Dependencies & flags

`testkit` (CountingGlobalAlloc, serialize_tests); sibling modules dispatch/memory/report/scenario (not my files). Env: SOAK_MINUTES / SOAK_SAMPLE_MS / SOAK_LEAK_MB_PER_H / SOAK_REPORT.

## 9. Open questions for the next agent

- Has soak-nightly.yml ever run (schedule history, run logs)? A nightly that never fired is PAPER-ONLY evidence for the "4 h bounded memory" claim — wave 17.
- `tests/soak/dispatch.rs`, `memory.rs` (least_squares_slope None semantics — F-001 depends on it), `report.rs`, `scenario.rs` — other agents; the slope-None inference should be confirmed there.
- `tests/soak_scenarios/main.rs` is my file #28 — audited next; the scenarios tier shares this harness's discipline.
