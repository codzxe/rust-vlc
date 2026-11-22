# CLAIM AUDIT: crates/playback-engine/tests/model/expected_table.toml

| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Claims extracted | 7 |
| REAL | 6 | REAL-UNPROVEN | 0 | SHALLOW | 0 | STUB | 0 | PAPER-ONLY | 0 | FALSE | 1 | UNREACHABLE | 0 |
| Internal contradictions found | 1 (the header's own stale row-count line vs its own "104 exactly" line and the file body) |
| Prior audit cross-ref | W03-A01 audited this file (audit/files/…expected_table.toml.audit.md) — counts agree (104/33/71); this audit independently re-derived them and adds the input-set and close-row verification. |

## 1. Blunt verdict

This artifact is what it claims to be: the human-review transition
table for the session state machine — exactly 104 `[[pair]]` rows (8
states × 13 inputs, no duplicates, verified by count and by
duplicate-scan), consumed at compile time by `tests/model_check.rs`
(`include_str!` + `parse_artifact`, model_check.rs:240) for the
three-way agreement proof against `state.rs::TRANSITIONS` and the
imperative oracle. Spot-checked rows match the code table, and the
code table's length is pinned at 33 (`state.rs:972`), matching the
file's 33 legal rows. One defect: the header carries a stale
historical row count ("30 legal rows, 66 typed rejections" = 96)
that contradicts both the header's own first claim ("104 entries,
exactly") and the file body (33 + 71 = 104) — the count was not
updated when 3.A3's work-carrier rows landed.

## 2. Claim ledger

| # | Claim (verbatim, ≤ 25 words) | Location | Verdict | Evidence you gathered (code path:line, command + output) |
|---|---|---|---|---|
| 1 | "one entry per (state, input) pair — 8 states x 13 inputs = 104 entries, exactly" | :1-3 | REAL | grep counts: `[[pair]]` = 104, `to =` = 33, `reject =` = 71; states enumerated = 8 (idle, opening, buffering, playing, paused, stopped, ended, error); inputs enumerated = 13; awk duplicate-scan over (state,input): zero duplicates |
| 2 | "tests/model_check.rs loads this file and proves the three-way agreement … for every pair" | :10-15 | REAL | model_check.rs:240 `parse_artifact(include_str!("model/expected_table.toml"))`; model_check.rs is an ungated test that ran in the W00 baseline (1751 pass); W03-A04's audit of model_check.rs: REAL (three-way model check) |
| 3 | "Row count after card 3.C1: 30 legal rows, 66 typed rejections" | :30 | FALSE | the file body has 33 `to` rows and 71 `reject` rows (counts above); 30 + 66 = 96 ≠ 104; the header's own :2 says "104 entries, exactly" |
| 4 | "The close-from-active trio (buffering/playing/paused + close) is typed `settle-first`" | :34-36 | REAL | awk over all `close` rows: buffering/playing/paused → `reject = "settle-first"` (exactly 3 data rows; the 4th grep hit is this header comment itself); idle/opening/stopped/ended/error + close → `to = "idle"` (legal) |
| 5 | "parsed by a hand-rolled std-only reader in the test — playback-engine has a zero-external-dependency contract, C5" | :18-21 | REAL | the parser lives in model_check.rs (W03-A04's audit); playback-engine Cargo.toml has no external deps (W03-A05's manifest audit: "no unsafe/zero external deps verified TRUE") |
| 6 | "this artifact … == the engine's data table (`state.rs::TRANSITIONS` via `apply_input`)" | :10-13 | REAL (spot-checked) | idle+open→opening [] ✓; idle+close→idle [] ✓; opening+open-ready→buffering ["emit-tracks","emit-buffering-enter"] ✓ (matches the TRANSITIONS row read at state.rs:~781+); playing+pause→paused ["freeze-clock"] ✓; `TRANSITIONS.len() == 33` pinned at state.rs:972 == the file's 33 to-rows |
| 7 | "adding, removing, or editing a row is a diff here, visible and arguable in TOML before it is code" | :4-6 | REAL (as design intent, enforced) | any drift between this file and TRANSITIONS fails the model_check test that runs in the merge gate — the artifact cannot silently diverge from the code |

## 3. Contradictions

| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "Row count after card 3.C1: 30 legal rows, 66 typed rejections" (:30) | the same header, line 2: "104 entries, exactly"; the file body: 33 to-rows + 71 reject-rows | the body: 33/71 = 104. The :30 line is a stale historical snapshot (the 2.C2 baseline was 26/70; 3.C1 made it 30/66; 3.A3's work-carriers brought it to 33/71 and the line was never bumped). The parenthetical at :2-3 ("3.C1's four rows + 3.A3's three work-carriers") shows the author knew the history; the numeric line just rotted. |

## 4. Fabrication / inflation indicators

- The stale count at :30 is the only number in the file that cannot be
  reproduced; every other count (104, 8, 13, the trio, the pinned 33)
  verifies. No evidence strings, no platform claims, no "verified"
  language — this is a data artifact with an honest consumer that
  enforces it.

## 5. Matrix rows (if feature-matrix)

N/A — not a feature-matrix file. Relevant cross-ref: this artifact is
the review surface for card 3.C1/3.A3 state-machine rows; any matrix
row citing "state machine transitions" has this file + model_check.rs
as its real evidence chain (the test runs in the merge gate).

## 6. What is this artifact, is it consumed, does it match reality (the assigned questions)

- **What it is**: the declarative oracle of the session state machine —
  one row per (state, input): either a legal transition with its doc 03
  §4.6 side effects, or a typed rejection reason. Its purpose (per
  header) is human review: transition decisions are diffs in TOML.
- **Is it consumed**: YES — `tests/model_check.rs:240` embeds it at
  compile time (`include_str!`) and the model_check test proves
  three-way agreement (artifact == `state.rs::TRANSITIONS` == the
  imperative oracle `tests/model/oracle.rs`) for every one of the 104
  pairs, including effects and typed rejection reasons. That test is
  ungated and ran in the W00 baseline.
- **Does it match reality**: YES for structure and spot-checked
  semantics (104/8/13 exact, no duplicates, 33 == the code's pinned
  table length, 4 rows verified against `state.rs` verbatim, the
  settle-first trio verified exhaustively across all 8 close rows).
  The one mismatch is the header's stale row-count line (claim #3) —
  a comment defect, not a data defect: the data is enforced by a test
  the comment is not.

## 7. Findings

### F-001 [S4] Stale historical row count in the header contradicts the file's own first line and body

Where: crates/playback-engine/tests/model/expected_table.toml:30 (vs :1-3 and the body)
Code (header, verbatim):
```toml
# Row count after card 3.C1: 30 legal rows, 66 typed rejections.
```
What is wrong: the body contains 33 legal and 71 rejection rows (104
total — the header's own line 2 says "104 entries, exactly"). The
:30 line is the post-3.C1 snapshot, never bumped when 3.A3's
work-carrier rows (underrun/eof-reached/drank transitions — visible in
the 13-input list) landed.
Why it matters: the file's entire purpose is to be the accurate human
review surface; a reviewer diffing against the stated counts misjudges
the change surface (96 claimed vs 104 actual). Severity capped at S4
because the consumer test enforces the DATA (a wrong row fails the
merge gate) — only the comment rots.
What correct looks like: "Row count after 3.A3: 33 legal rows, 71
typed rejections (2.C2 baseline 26/70 → 3.C1 30/66 → 3.A3 33/71)."
Confidence: HIGH — Verification: read + grep counts (33/71) + the
state.rs:972 pin of 33.

### F-002 [S4 — informational, no defect] The 4th `settle-first` grep hit is the header itself

Where: crates/playback-engine/tests/model/expected_table.toml:34-36
A naive `grep -c settle-first` returns 4, but only 3 are data rows
(buffering/playing/paused + close); the 4th is this header comment.
Recorded so the next agent grepping this file does not report a
"trio vs four" contradiction that does not exist. Verification: awk
extraction of all `close` rows (8 rows enumerated: 5 legal, 3
settle-first).
