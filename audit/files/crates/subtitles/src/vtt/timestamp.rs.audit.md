# AUDIT: crates/subtitles/src/vtt/timestamp.rs
| Field | Value |
|---|---|
| Auditor | W09-A05 |
| Date | 2026-11-24 |
| Lines / functions | 147 / 3 prod fns + 4 tests |
| File verdict | REAL (small, correct, saturation-safe parser; real in-crate consumer; tests pass) |
| Reachable from shipped entry point? | NO — consumed by vtt/mod.rs:267-269 (the cue `lhs --> rhs` timing parser); crate `subtitles` has no product dependency edge (see eia608.rs audit F-001) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p subtitles --lib vtt::timestamp` → 4 passed / 0 failed (exit 0); sed quote verification |

## 1. One-paragraph verdict
A genuinely correct WebVTT timestamp parser: both legal spellings (`HH:MM:SS.mmm`, `MM:SS.mmm`), greedy prefix split that hands cue settings back to the caller on the same line, 1–3 digit milliseconds right-padded, domain checks (seconds ≤ 59 always, minutes ≤ 59 only when hours present), and a u128 intermediate with `min(i64::MAX)` saturation so no hostile hour count can wrap — the exact opposite defect of vobsub/idx.rs's F-001. All 4 unit tests are behavioral (happy forms, 18 malformed rejections, greedy split with settings remainder, saturation) and pass when run. The only defect is a comment that promises a digit-count check for minutes that the code does not perform, plus one "any value" overstatement in the same doc block.

## 2. What actually works (verified)
— verified by `cargo test -p subtitles --lib vtt::timestamp` → 4 passed: full_forms (incl. `75:00.000` = 4.5e9 µs and `.5`→500 ms padding), rejects_malformed (18 cases incl. `00:00:01.1000`, `--1:00.000`, `1e3:00.000`), greedy_prefix_leaves_settings (exact remainder strings), saturates_at_i64_max (`99999999999999:00:00.000` → `i64::MAX`).

## 3. Findings
F-001 [S4]
Where: crates/subtitles/src/vtt/timestamp.rs:48-49 and 60-63
Code (verbatim):
```
    // Digit-count sanity: sec exactly 1–2 digits (spec: 2; tolerate 1),
    // minutes 1–2 digits when hours are present, milliseconds 1–3.
```
```
    let minutes: u64 = minutes.parse().ok()?;
    if hours.is_some() && minutes > 59 {
        return None;
    }
```
What is wrong: The comment claims minutes are limited to 1–2 digits when hours are present; the code enforces only the value (`> 59`), so `01:000:00.000` (three-digit minutes, value 0) is accepted as 3_600_000_000 µs. Seconds get their explicit `seconds.len() > 2` check (line 50-52) — minutes do not. Second, smaller divergence in the module doc: "in the hours-less `MM:SS.mmm` form … **any value is accepted**" (lines 8-10) is false for a leading field ≥ u64::MAX (~1.8e19 minutes), which fails `u64` parsing and returns `None` instead of saturating the way hours do. Neither case is tested. Blast radius: cosmetic over-tolerance on non-conforming input that already diverges from spec parsers in documented ways. What correct looks like: a `minutes.len() > 2` check next to the seconds check (or fix the comment), and either saturate the leading field or drop "any value". Confidence: HIGH — Verification: read + trace of `01:000:00.000` (parse → 0 ≤ 59 → accepted).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| parse_timestamp | 16-20 | REAL | — | trim + full-consume wrapper |
| parse_timestamp_prefix | 24-34 | REAL | — | greedy `[0-9:.]` scan; empty candidate → None |
| timestamp_us | 37-82 | REAL | S4 | grammar core; F-001 minutes divergence; u128 + min(i64::MAX) saturation verified; `millis > 999` check redundant-but-harmless (ms length already ≤ 3) |
| tests × 4 | 89-145 | REAL | — | all behavioral; gap only at F-001's untested input |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| full_forms | YES | 9 exact µs values incl. both forms + padding + 75-min |
| rejects_malformed | YES | 18 distinct rejection classes with per-case message |
| greedy_prefix_leaves_settings | YES | exact remainder strings pinned |
| saturates_at_i64_max | YES | pins `i64::MAX` clamp |
Gap: no `01:000:00.000` case (F-001), no hours-less leading field beyond u64::MAX.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "milliseconds may be 1–3 digits … we accept fewer and right-pad" | timestamp.rs:5-7 | REAL | full_forms `.5`/`.05` pins (run, passed) |
| "seconds must be `0..=59`; minutes must be `0..=59` when an hours field is present" | timestamp.rs:7-8 | REAL | rejects_malformed `00:60.000` / `01:60:00.000` (run, passed) |
| "the µs total saturates at `i64::MAX` rather than overflowing" | timestamp.rs:11-12 | REAL | saturates_at_i64_max (run, passed); u128 intermediate confirmed by read |
| "minutes 1–2 digits when hours are present" | timestamp.rs:48-49 | FALSE | F-001: only the value is checked |
| "any value is accepted" (hours-less leading field) | timestamp.rs:8-10 | FALSE at the u64 parse bound | F-001 second half |

## 7. Dead / duplicated / unreachable code
`millis > 999` (67-69) is provably dead given `ms_part.len() ≤ 3` — defensive, harmless. Module itself unreachable from product binaries (header field).

## 8. Dependencies & flags
Std-only; one small `format!` per parse for ms padding (fine outside hot loops; VTT files parse once). No unsafe, no panics — `split_at`/`rsplit_once` are byte-safe on ASCII delimiters.

## 9. Open questions for the next agent
- vtt/mod.rs:267-269 builds the cue interval from these µs values — if a saturated `i64::MAX` start meets a normal end, check how `CueInterval::new` reports `EndBeforeStart` there (not my file).
