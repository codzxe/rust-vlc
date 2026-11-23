# AUDIT: crates/subtitles/src/srt/damage.rs
| Field | Value |
|---|---|
| Auditor | W09-A10 |
| Date | 2026-11-23 |
| Lines / functions | 213 / 7 (describe, Display, DamageSink::{new,record,into_parts}, 2 tests; plus 20-variant SrtDamageKind) |
| File verdict | UNREACHABLE (real, internally-consumed library code; no shipped entry point) |
| Reachable from shipped entry point? | NO — same crate-level wall (only fuzz/Cargo.toml:50 depends on `subtitles`); inside the crate it IS wired: DamageSink is consumed by srt/parser.rs:41,54, srt/encoding.rs:31,89-201, srt/tags.rs:28,33,101,236 |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | Grep `DamageSink` in crates/subtitles/src (consumers listed above, exit 0); baseline test citation; sed quote verification (exit 0) |

## 1. One-paragraph verdict
The damage ledger does exactly what it claims: a 256-event capped Vec with a saturating dropped-counter (verified by a real cap test), 20 well-documented malformation kinds with exhaustive `const fn` descriptions, and it is genuinely threaded through the live SRT parse path (parser, encoding, tags). Its only defects are that nothing shipped can ever see a single damage event (the crate is not linked by any app), and one weak test whose exhaustiveness claim is aspirational.

## 2. What actually works (verified)
— Baseline (audit/runs/baseline/cargo-test-workspace.txt:5938-5939): `srt::damage::tests::every_kind_describes_itself ... ok`, `srt::damage::tests::ledger_caps_and_counts ... ok`. The cap test records 356 events, asserts exactly 256 kept + 100 dropped with correct first event — a genuine boundary test (hand-checked: 256+100=356 records, cap math correct).

## 3. Findings
F-001 [S2]
Where: crates/subtitles/src/srt/damage.rs:9-13
Code:
```
//! Bounded memory (D-2): the ledger is capped at
//! [`MAX_DAMAGE_EVENTS`] entries; further events are counted in
//! `dropped_damage_events` (a `u32` saturating counter) so a file
//! crafted as "infinite small damage" cannot grow the ledger without
//! bound.
```
What is wrong: nothing in the mechanism — the finding is reachability: the bounded, honest ledger this file implements surfaces to no user in any shipped binary (no app depends on the crate; the SRT parser that feeds it is exercised only by tests/fuzz). The doc-comment contract is real; its audience is zero. Why it matters: this is the 5th file in this domain where the pattern repeats — working plumbing, dead outlet (see vobsub/mod.rs F-001, ass/layout.rs F-001, srt/render.rs F-001, compositing/target.rs F-001). What correct looks like: an app-level consumer (CLI `--check` output, desktop overlay) or an explicit "library-only" scope note in the crate docs. Confidence: HIGH — Verification: read (workspace Cargo.toml greps cited in the other audit files).

F-002 [S4]
Where: crates/subtitles/src/srt/damage.rs:183-212
Code:
```
    fn every_kind_describes_itself() {
        // Exhaustive at the time of writing; new kinds must extend it.
        let kinds = [
            SrtDamageKind::InvalidUtf8,
```
What is wrong: two weaknesses. (1) The comment's "new kinds must extend it" is unenforced — adding a 21st variant compiles fine and the test still passes silently missing it (only `describe()`'s match forces a description, which it must have anyway). (2) `assert_eq!(k.to_string(), k.describe())` (line 210) is a tautology: `Display` is literally implemented as `f.write_str(self.describe())` (lines 111-115), so the assertion can never fail for any variant. The only real check is "description is non-empty". What correct looks like: a `SrtDamageKind::ALL` const array (like AssAlignment::ALL) asserted in the test, dropping the tautological to_string check. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `SrtDamageKind::describe` | 85-108 | REAL | — | Const, exhaustive (compiler-enforced), accurate one-liners. |
| `Display for SrtDamageKind` | 111-115 | REAL | — | Delegates to describe. |
| `SrtDamage` struct | 120-127 | REAL | — | kind + 1-based line_no, 0 = decode-stage. |
| `DamageSink::new` | 141-143 | REAL | — | Default. |
| `DamageSink::record` | 147-153 | REAL | — | Cap + saturating counter — correct. |
| `DamageSink::into_parts` | 156-158 | REAL | — | Consuming split. |
| `ledger_caps_and_counts` | 165-181 | REAL | — | Genuine boundary test. |
| `every_kind_describes_itself` | 183-212 | SHALLOW | S4 | Non-exhaustive-enforced + tautological assertion (F-002). |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ledger_caps_and_counts | YES | Exact 256/100 split + first-event identity. |
| every_kind_describes_itself | NO (weakly) | Only catches an empty description; misses new variants and asserts a tautology (F-002). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the ledger is capped at MAX_DAMAGE_EVENTS ... counted in dropped_damage_events" | damage.rs:9-11 | REAL | record() implementation + cap test (baseline :5939). |
| Field name `dropped_damage_events` | damage.rs:11 | INEXACT | The actual field is `DamageSink::dropped` (line 136), surfaced only via `into_parts()`; SrtDocument's field name differs — cosmetic doc drift. |
| "The SRT parser NEVER aborts a file and NEVER panics" | damage.rs:3-4 | UNVERIFIED here | Owned by parser.rs/encoding.rs (other agents); no counter-evidence found in this file. |

## 7. Dead / duplicated / unreachable code
No dead code inside the file. Crate-level unreachability per F-001.

## 8. Dependencies & flags
None beyond std. No unsafe, no I/O, no features. `Copy + Eq` derivations support the differential ACs' exact-sequence assertions (documented, line 21-22).

## 9. Open questions for the next agent
- srt/parser.rs auditor: does any path record damage with `line_no: 0` other than the decode stage, and does the u32 line counter saturate or wrap on a >4 G-line file?
- Who consumes `SrtDocument.damages` in a UI sense? (Nothing today — tie to F-001.)
