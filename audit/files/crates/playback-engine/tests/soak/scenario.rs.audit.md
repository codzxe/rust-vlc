# AUDIT: crates/playback-engine/tests/soak/scenario.rs

| Field | Value |
|---|---|
| Auditor | W03-A04 |
| Date | 2026-11-22 |
| Lines / functions | 624 / 9 production fns + 1 in-file test |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (soak-harness scenario script) — invoked by the soak driver (`soak/main.rs`, another agent's file); `cargo test -p playback-engine --test soak`. Execution UNVERIFIED (no cargo). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | Read; cross-check of the liveness-bound claim against soak/main.rs:255-284 (skip rates + `>= 1` floors); sed verification. No cargo (adaptation). |

## 1. One-paragraph verdict

The soak's deterministic cycle script: fixture plans parsed from the committed expectation TOMLs (seek targets and track ids read, never transcribed — with a parent-fallback for dirty fixtures), a pure-function-of-cycle-index op rotation (absolute/relative/percent seeks over mid-media targets, there-and-back switches), per-phase live-byte attribution around every op, and a per-cycle frames-must-advance liveness assert whose failure message is the doctrine itself ("a ritual, not a soak"). The header documents an honest measurement history (the old op order skipped the seek 1597/1597 — 100%) that motivated moving the ops to land mid-media. One real gap: the op-liveness floor for seeks/switches lives in the driver as `>= 1` — a soak that completes a single seek and skips every other one passes that floor; the header's own history shows near-total skip is the realistic degradation mode.

## 2. What actually works (verified)

- `FixturePlan::from_expectation` (86-114): parses the fixture's own recorded truth; the dirty fixture's missing `[seek]`/`[tracks]` sections are filled from its RECORDED parent (93-101), not hand-copied; missing-everything is a typed sentence (103-112).
- The digit-filter parsers (153-190) handle the TOML's `_`-separated numbers and `id =` fields inside inline tables; the in-file test (597-623) pins the parsed values exactly (targets `[0, 500_000, 1_000_000, 1_999_000]`, audio ids, the multi-track `switch_to = Some(2)`, the dirty fixture's parent-inherited facts) — the parse is part of the proof surface, as claimed.
- `run_cycle` (386-500): every op is dispatched through the command layer (`lumen_soak_dispatch`), errors abort the cycle with a sentence naming op+fixture (a soak that errors its ops measures nothing — the doc says so and the code does so), skip lanes are counted per op with the state reason documented (448-449, 461-463), and the phases attribute live-byte deltas per op (Open/Seek/Switch/Play/Close).
- The per-cycle liveness assert (570-573): engine lifetime frames must advance — a cycle that decodes nothing fails loudly.
- `seek_request`'s rotation (332-352) exercises all three wire seek shapes (Absolute/Relative/Percent); `mid_media_targets` (357-360) drops the end-adjacent target with `.max(1)` guarding the single-target case.
- Bounded everything: `ADVANCE_POLLS = 3`, `PLAY_TO_END_POLLS = 1024`; the dirty lane's Error open short-circuits to close with the exit counted (418-424).

## 3. Findings

### F-001 [S3] The op-liveness floor for seeks/switches is `>= 1` — near-total skip passes
Where: crates/playback-engine/tests/soak/scenario.rs:17-18 (the history), 29-35 (the doctrine) vs soak/main.rs:280-284 (the actual floor)
Code (scenario.rs:16-18):
```rust
//! the skip lanes
//! counted and bounded by the liveness asserts (D-6). Measured with
//! ops-after-advance: 1597/1597 cycles skipped the seek (100%).
```
Code (main.rs:280-284, cross-file):
```rust
        counts.seeks_ok + counts.seeks_failed >= 1,
        ...
        counts.switches_ok >= 1,
```
What is wrong: The scenario file's doctrine says skips are "bounded by the liveness asserts". The liveness assert IN this file covers frames only (570-573); the op-level floor lives in the driver and is satisfied by ONE completed seek (or one failed-but-terminal seek) and ONE completed switch across the entire run. The header's own recorded history — 1 597/1 597 seeks skipped under the old op order — is precisely the degradation mode this floor cannot catch: a regression that re-introduces near-total skipping (e.g., the open settle draining to Ended before the op lands again) leaves `seeks_ok >= 1` true on a lucky cycle.
Why it matters: The soak is the long-run leak measurement; if its seek/switch lanes silently degrade to rituals, the per-seek/per-switch engine paths (mint, flush, decoder cache) stop being soaked while the report still lists them as loop ops. The skip-rate computations exist in the driver (main.rs:255-274) — whether their thresholds bind meaningfully is that file's audit; the `>= 1` floor itself is the weak link visible from here.
What correct looks like: A per-run floor tied to the playable-cycle count (e.g., `seeks_ok + seeks_failed >= playable_cycles / 2`), with the skip-rate threshold asserted at a documented number.
Confidence: HIGH on the floor's weakness (read); MEDIUM on the driver thresholds (main.rs lines seen via rg only — that file is another agent's).
### F-002 [S4] The expectation parser's digit-filter accepts malformed values silently
Where: crates/playback-engine/tests/soak/scenario.rs:157-163
Code:
```rust
                    let digits: String = item
                        .chars()
                        .filter(|c| c.is_ascii_digit() || *c == '-')
                        .collect();
                    if !digits.is_empty() {
                        seek_targets_us.push(digits.parse::<i64>().must());
                    }
```
What is wrong: `targets_us = [0, 5x00_000]` parses as 500000 — any non-digit garbage inside a number is silently dropped rather than rejected. Same pattern for the track ids and `to_track_id` (175-189). A typo'd expectation TOML yields a wrong-but-plausible target instead of a parse failure.
Why it matters: The file's own doctrine is "the parsing is part of the proof surface" (75-76); a lenient parser weakens that surface. Hygiene — the committed TOMLs are pinned by the in-file test (597-623), which limits today's exposure to edits.
What correct looks like: Reject items containing characters outside `[0-9_-]` with a sentence naming the line.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Notes |
|---|---|---|---|
| `FixturePlan::from_expectation` | 86-114 | REAL | Parent fallback; typed missing-fact errors. |
| `FixturePlan::path` | 120-122 | REAL | Existence-checked via testkit. |
| `parse_expectation` | 127-201 | REAL | S4 F-002 digit filter; sections seek/tracks/switch_plan. |
| `parent_of` | 205-221 | REAL | Naming-convention mapping; typed error. |
| `fixture_rotation` | 230-248 | REAL | Two valid parents + one dirty truncation. |
| `CycleCounts` (struct) | 256-295 | REAL | Fixed-size counters, documented lanes. |
| `open_request`/`seek_request`/`switch_request`/`close_request` | 313-376 | REAL | Wire-shape builders; three seek shapes. |
| `mid_media_targets` | 357-360 | REAL | `.max(1)` single-target guard. |
| `run_cycle` | 386-500 | REAL | The cycle; per-op error sentences; phase attribution; skip lanes counted. |
| `in_playback` | 503-508 | REAL | Playing|Buffering per the table. |
| `switch_selectors` | 513-528 | REAL | Multi-track there-and-back / deselect-reselect fallback. |
| `advance` | 531-535 | REAL | Bounded. |
| `close_cycle` | 538-555 | REAL | Phase-attributed. |
| `finish_cycle` | 562-586 | REAL | The frames liveness assert + exit bookkeeping. |
| fixture_rotation_parses_the_recorded_truth (test) | 596-623 | REAL | Exact parsed-value pins; serialize_tests. |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| fixture_rotation_parses_the_recorded_truth | YES | Exact target/id/switch pins incl. the dirty fixture's parent-inherited facts. |

The scenario's own honesty: op errors abort with sentences; skips counted never hidden; the 100%-skip history is disclosed rather than buried.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "seek targets and track ids are read from the fixtures' own expectation files, never transcribed" (3-6) | module doc | REAL | Parse + the in-file pin test. |
| "with the skip lanes counted and bounded by the liveness asserts (D-6)" (16-17) | module doc | SHALLOW | Counted: yes (this file). Bounded: the frames assert is here; the op floor is `>= 1` in the driver (F-001). |
| "A soak that skips its ops is a ritual, not a measurement — the scenario fails loudly on that" (34-35) | module doc | SHALLOW | True for op ERRORS and frame-dead cycles; NOT true for op SKIPS below the `>= 1` floor (F-001). |
| "Measured with ops-after-advance: 1597/1597 cycles skipped the seek (100%)" (17-18) | module doc | UNVERIFIED | Historical run figure; motivates the current op order. Honest disclosure either way. |
| "the parsing is part of the proof surface" (75-76) | comment | SHALLOW | Real pins, but the parser is lenient (F-002). |

## 7. Dead / duplicated / unreachable code

None.

## 8. Dependencies & flags

commands, playback-engine, testkit; parent modules soak/dispatch (another agent) and soak/memory (another agent). `run_cycle`'s `live: fn() -> isize` parameter keeps the allocator dependency injectable. No feature gates.

## 9. Open questions for the next agent

- soak/main.rs auditor: what are the skip-rate thresholds at 255-274, and are they asserted or printed? F-001's severity rises to S2 if they are printed only.
- soak/dispatch.rs auditor: does `lumen_soak_dispatch` share the scenarios_b host's parallel-dispatch hazard (my host.rs F-001) — "a faithful in-test mirror of apps/cli's lumen_dispatch playing subset" (report.rs:80-83) is the same disclosed-but-shipped gap.
