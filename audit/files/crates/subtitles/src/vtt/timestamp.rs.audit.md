# AUDIT: crates/subtitles/src/vtt/timestamp.rs
| Field | Value |
|---|---|
| Auditor | W09-A01 |
| Date | 2026-11-21 |
| Lines / functions | 147 / 3 production fns + 4 tests |
| File verdict | UNREACHABLE (real, test-executed in baseline; no shipped entry point links the crate) |
| Reachable from shipped entry point? | NO. In-crate chain: `vtt/mod.rs:265-274` `split_timing` → `timestamp::parse_timestamp` (start) + `parse_timestamp_prefix` (end, settings ride behind) — the cue-timing path of `vtt::parse`. Crate consumed only by `fuzz/Cargo.toml:50` + own tests; no edge from apps/cli or apps/desktop-tauri. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | Read tool (whole file) + Read vtt/mod.rs:240-294 (caller context); `rg -n "parse_timestamp|timestamp_us"` → caller map (vtt/mod.rs:267-269 only); every doc-comment tolerance claim traced to its enforcing line; arithmetic hand-checks (u128 saturation, right-padding); baseline log :5534-5537 → 4/4 `vtt::timestamp::tests` ok |

## 1. One-paragraph verdict
The cleanest file of my six: a 3-function timestamp grammar with every module-doc tolerance decision implemented exactly as written — 1–3 digit milliseconds right-padded (:53-66), seconds bounded 0..=59 in both forms (:57-59), minutes bounded only when hours are present so the spec-legal `75:00.000` long-cue form parses (:61-63), and unbounded hours saturating to `i64::MAX` through a u128 intermediate instead of wrapping (:77-81, pinned by its own test). All four tests ran green in the baseline and each targets a distinct branch; the 18-case rejection table is the strongest negative pin in the crate. The single defect is a comment that claims a minutes digit-count check ("minutes 1–2 digits when hours are present") that the code never performs — `01:000:00.000` is accepted.

## 2. What actually works (verified)
— Baseline: `vtt::timestamp::tests::{full_forms, rejects_malformed, greedy_prefix_leaves_settings, saturates_at_i64_max}` 4/4 ok (:5534-5537), inside the 267/0 lib run; exercised end-to-end by `vtt::tests::*` (7/7, :5523-5529) and `tests/vtt_acceptance.rs` 4/4 (:5673) — the 3-ffmpeg-fixture + handcrafted shape matches the matrix evidence string exactly.
— Saturation math verified: hours ≤ u64::MAX ⇒ total ≤ ~6.5e28 < u128::MAX; `.min(i64::MAX as u128)` clamps; test pins `99999999999999:00:00.000` → `Some(i64::MAX)`.
— Right-padding verified: `format!("{ms_part:0<3}")` — `.5`→"500" (500 ms), `.05`→"050" (50 ms), both pinned (:99-100).
— Greedy class split verified against adversarial shapes: `12:34.567junk` → `(754_567_000, "junk")`; glued settings `00:00:04.000position:50%` stop at 'p'; `1.2:3.4` dies at `minutes.parse` (embedded dot); `--1:00.000` dies on the empty candidate; `1e3:00.000` dies on no-dot.
— Panic-free trace: no indexing, no unwrap (testkit `Must` in tests only), all parses `.ok()?`.

## 3. Findings

### F-001 [S4]
Where: crates/subtitles/src/vtt/timestamp.rs:48-49 (absent check; contrast :50-55 and :61)
Code:
```
    // Digit-count sanity: sec exactly 1–2 digits (spec: 2; tolerate 1),
    // minutes 1–2 digits when hours are present, milliseconds 1–3.
    if seconds.is_empty() || seconds.len() > 2 {
```
What is wrong: the comment promises "minutes 1–2 digits when hours are present", but the only minutes validation is the VALUE check `hours.is_some() && minutes > 59` (:61-63). `parse_timestamp("01:000:00.000")` — minutes `"000"`, 3 digits, value 0 — is accepted as 1 h 0 m 0 s. Seconds (:50-52) and milliseconds (:53-55) get real digit-count checks; minutes gets none.
Why it matters: comment-code drift in a file otherwise exact about its tolerances; the undocumented acceptance (`01:000:00.000`) is a dirty-world tolerance nobody signed off on. No user-visible harm (value still correct), hence S4.
What correct looks like: `if hours.is_some() && (minutes.is_empty() || minutes.len() > 2) { return None; }` — or fix the comment to say minutes digit count is unbounded.
Confidence: HIGH — Verification: read (all three digit-count sites compared; no other minutes length check exists in the file).

Notes (below finding threshold, checked and cleared):
— `if millis > 999` (:67-69) is unreachable: ms_part is length-checked to ≤3 digits (:53) and class-filtered to ASCII digits, so ≤ 999 always. Harmless defensive code.
— Doc "any value is accepted" for the hours-less leading field is bounded by u64::MAX parseability (a 20+-digit minute field returns None); pedantic, not a defect.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_timestamp` | 14-20 | REAL | — | full-consume contract; trims |
| `parse_timestamp_prefix` | 24-34 | REAL | — | greedy digit/:/. class scan; empty candidate → None; feeds settings to vtt/mod.rs:269 |
| `timestamp_us` | 36-82 | REAL | S4 | F-001 (comment-only minutes digit claim); grammar 2-or-3 fields; all value bounds enforced; u128 saturation correct; dead `millis > 999` check |
| tests ×4 | 84-146 | REAL | — | 9 positives incl. `75:00.000` and padding; 18 negatives each targeting a branch; exact `rest` strings; saturation pin |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| full_forms | YES | both spellings, whitespace, zero, 75-min form, both padding cases — exact µs values |
| rejects_malformed | YES | 18 shapes, each hits a distinct rejection branch (length, range, field-count, class, double-dot) |
| greedy_prefix_leaves_settings | YES | exact (value, rest) pairs incl. glued junk and the None cases |
| saturates_at_i64_max | YES | pins the clamp-vs-wrap policy |
| (absent) `01:000:00.000` | — | accepted today (F-001) — no pin either way |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Parse WebVTT: cues, settings subset, NOTE/STYLE blocks" | feature-matrix.toml `subtitle-vtt` description | REAL (in-crate) | this file is the timing half; vtt::tests 7/7 + settings 5/5 + text 4/4 + timestamp 4/4 in baseline |
| "3 real ffmpeg -c:s webvtt fixtures + settings_heavy handcrafted" | same row, evidence | REAL | tests/vtt_acceptance.rs 4/4 (:5673): 3 ffmpeg_* + handcrafted_settings_heavy — matches |
| "7003 fuzz inputs zero panics" | same row, evidence | REAL-UNPROVEN (here) | vtt_fuzz 3/3 ok (:5698-5700 area, incl. `large_document_under_200kb_is_survivable` — the VTT lane, unlike ASS fuzz-lite, does generate 200 KB documents); exact 7003 count lives in vtt/fuzz.rs (another agent) |
| status implemented x3 / web not_started | same row | UNREACHABLE (per-platform) | no Cargo.toml edge from any shipped binary to `subtitles` |
| Tolerance list (1–3 digit ms right-padded; sec ≤59; min ≤59 only with hours; hours any digits, saturate) | module doc :5-12 | REAL | every clause traced to its enforcing line (:53-66, :57-59, :61-63, :77-81) except the minutes digit-count comment (F-001) |

## 7. Dead / duplicated / unreachable code
Dead: `millis > 999` guard (:67-69, provably unreachable). No duplication (vobsub/idx.rs has its own unrelated `parse_timestamp` — different signature, different module, no collision). Crate unreachable from shipped binaries.

## 8. Dependencies & flags
std-only; no unsafe; no features; `pub(super)` visibility — module-private to vtt. `testkit::Must` in tests only.

## 9. Open questions for the next agent
- vtt/mod.rs owner: `split_timing` (:266) splits on the FIRST "-->" — a malicious line `a-->b-->c` puts `b-->c` in rhs; the greedy class then rejects it (b is not in the class) → whole line None. Confirm that is the intended damage path (I traced it as safe, no panic).
- vtt/fuzz.rs owner: confirm the 7003-input count and whether the mutation alphabet can reach the saturation branch.
