# AUDIT: crates/subtitles/src/vtt/damage.rs
| Field | Value |
|---|---|
| Auditor | W09-A09 |
| Date | 2026-11-21 |
| Lines / functions | 158 / 8 (4 constructors, 1 helper, enum + Display) |
| File verdict | UNREACHABLE — fully wired taxonomy, consumers' tests baseline-executed green; no shipped reachability |
| Reachable from shipped entry point? | NO (crate census, cross-ref layout.rs F-001) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | `rg "DamageEvent|DamageKind|bad_setting|…|truncate" crates/subtitles/src/vtt` (full caller census); read of vtt/mod.rs:130-249 (constructor sites + line_no closure); quote verifies via Read tool; baseline: orchrun-test-workspace-nofailfast.log.txt:5516-5537 (vtt lib tests ok), :5673-5681 (vtt_acceptance 4/0 — the ffmpeg fixtures + settings_heavy), :5683-5690 (vtt_fuzz 3/0), :5540 (lib 267/0) |

## 1. One-paragraph verdict
A small, disciplined damage taxonomy that does exactly what it documents: every one of the 9 `DamageKind` variants has a real production constructor (verified site-by-site in vtt/mod.rs and vtt/settings.rs), the ≤16-byte char-boundary name truncation is correctly implemented (walk-back cannot reach 0 because boundaries occur at most 4 bytes apart), and `Display` covers all 9 variants with unit-carrying messages. Its consumers' tests are baseline-executed green — the vtt lib tests (orchrun log :5516-5537), the ffmpeg-fixture acceptance suite (:5673-5681) and the fuzz suite (:5683-5690). The file has no tests of its own, its boundary-truncation path is asserted nowhere in the crate, and nothing outside the never-shipped crate can reach it.

## 2. What actually works (verified)
— Constructor census: `bad_setting` ← settings.rs:59,119,128,149,153,157,166,188; `unknown_setting` ← settings.rs:90; `unsupported_setting` ← settings.rs:81,87; `unsupported_block("REGION", …)` ← mod.rs:187; `BadTimestamp`/`ReversedInterval`/`TimingLineNotSecond`/`OrphanBlock`/`IdOverflow` ← mod.rs:193-237. No variant is paper-only.
— `truncate_name` walk-back verified: worst case `end` stops at ≥13 for 4-byte chars (boundaries every ≤4 bytes), never 0; result is ≤16 bytes on a char boundary — matches doc :20-21.
— `IdOverflow` doc ("auto id was assigned instead") matches mod.rs:232-240 (`auto_id.saturating_add(1)` then `CueId::new`).
— 1-based line-number contract honored by the only producers (`line_no = |i| (block_base + i) as u32`, mod.rs:172, block_base = 1-based :162).
— Executed in baseline: vtt lib tests green (orchrun-test-workspace-nofailfast.log.txt:5516-5537, incl. `malformed_blocks_are_damages_not_panics` matching 7 taxa), vtt_acceptance 4/0 (:5673-5681: the record's "3 real ffmpeg -c:s webvtt fixtures + settings_heavy handcrafted"), vtt_fuzz 3/0 (:5683-5690).

## 3. Findings

### F-001 [S2]
Where: crates/subtitles/src/vtt/damage.rs:25-30
Code:
```
pub struct DamageEvent {
    /// 1-based line number where the event was detected.
    pub line_no: u32,
    /// What happened.
    pub kind: DamageKind,
}
```
What is wrong: Crate-level unreachability (cross-ref layout.rs F-001: the `subtitles` crate's only reverse-dependency is fuzz/Cargo.toml:50; no shipped binary links it). The "dirty-world diagnosis" this ledger offers can be surfaced to exactly zero users.
Why it matters: The entire D-4 diagnostic design ships nowhere.
What correct looks like: Pipeline wiring exposing damage ledgers (CLI/UI surfaces), or UNREACHABLE status in the matrix.
Confidence: HIGH — Verification: rg census.

### F-002 [S3] — CORRECTED v2 (was S2 "consumers' tests never executed")
Where: crates/subtitles/src/vtt/damage.rs:1-158 + audit/runs/baseline/INDEX.md
Code:
```
//! Damage events (card 5.M3): the parser never panics and never
//! aborts on a malformed region — every skipped or degraded region of
//! the document is reported as a typed event with the 1-based line
//! number where it was detected (D-4: dirty-world diagnosis, the same
```
What is wrong: The v1 claim — "the VTT suite … has no execution evidence in audit/runs/baseline/" — was FALSE: `orchrun-test-workspace-nofailfast.log.txt` ran the full subtitles workspace member green: vtt lib tests :5516-5537, `vtt_acceptance` 4/0 at :5673-5681 (ffmpeg fixtures + settings_heavy), `vtt_fuzz` 3/0 at :5683-5690, lib 267/0 at :5540. The genuine defect is the evidence INDEX: INDEX.md's verdict table cites only the two fail-fast runs (#3, #4), both of which stop at packages alphabetically before `subtitles`, and lists none of the 10 orchrun-* artifacts — breaking the "INDEX.md lists everything" contract (full chain in layout.rs F-002).
Why it matters: The matrix's "implemented ×3 platforms" for subtitle-vtt IS backed by executed tests — but only an auditor who found the unlisted artifact knows that; the indexed evidence trail says otherwise.
What correct looks like: INDEX.md rows for orchrun-*; --no-fail-fast in the headline runs.
Confidence: HIGH — Verification: read of all three baseline logs.

### F-003 [S4]
Where: crates/subtitles/src/vtt/damage.rs:75-85
Code:
```
fn truncate_name(name: &str) -> String {
    if name.len() <= 16 {
        name.to_owned()
    } else {
        let mut end = 16;
        while !name.is_char_boundary(end) {
            end -= 1;
        }
```
What is wrong: The >16-byte truncation path is asserted nowhere: settings.rs tests only `matches!(… DamageKind::UnsupportedSetting { .. })` (settings.rs:265-269), mod.rs tests match kinds, and no test feeds a >16-byte or multi-byte setting/block name to verify the char-boundary walk or the ≤16-byte result. `Display` strings (:136-158) are likewise never asserted.
Why it matters: The one function with a loop-and-index subtlety (the kind that panics on `end -= 1` at 0 if a future edit changes the cap semantics) is the one with zero direct or indirect assertions; a regression would ship silently.
What correct looks like: A 5-line unit test: 17-byte ASCII name → 16 bytes; a CJK name crossing the boundary → char boundary; empty name.
Confidence: HIGH — Verification: rg census of all `DamageKind` assertions in the crate.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DamageEvent` (struct) | 25-30 | REAL | — | pub fields; produced only by vtt internals + mod.rs direct literals |
| `bad_setting` | 34-41 | REAL | — | 8 production call sites (settings.rs) |
| `unknown_setting` | 44-51 | REAL | — | 1 site (settings.rs:90) |
| `unsupported_setting` | 54-61 | REAL | — | 2 sites (vertical/size, settings.rs:81,87) |
| `unsupported_block` | 64-71 | REAL | — | 1 site (REGION, mod.rs:187) |
| `truncate_name` | 75-85 | REAL | S4 | correct by read; boundary path unasserted anywhere (F-003) |
| `DamageKind` (enum, 9 variants, non_exhaustive) | 91-134 | REAL | — | all 9 constructed in production code (§2) — no paper variants |
| `Display for DamageKind` | 136-158 | REAL | — | all 9 covered; µs units in ReversedInterval message; no assertions on strings |

## 5. Test honesty
No tests in this file. Indirect coverage exists in vtt/mod.rs tests (:356-393 — kind matching for 7 of 9 taxa via a hostile fixture) and settings.rs tests (:265-287). Would those fail if damage.rs broke? Yes for kind selection (they match variants), no for name truncation and Display (F-003). All executed green in the baseline no-fail-fast run (orchrun log :5516-5537).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every skipped or degraded region … reported as a typed event with the 1-based line number" | module doc :1-5 | REAL | constructor census §2; line_no closure mod.rs:172 |
| "vertical:, size:, REGION blocks — recorded, never silently ignored" | module doc :11-15 | REAL | settings.rs:81,87 + mod.rs:185-188 |
| "Setting/block names stored in events are truncated at a char boundary to ≤16 bytes" | module doc :20-21 | REAL | :75-85 verified by read (unasserted — F-003) |
| matrix subtitle-vtt "implemented ×3", "92/0 at card, 7003 fuzz inputs" | docs/feature-matrix.toml:920-927 | REAL (tests) / UNREACHABLE (shipped) | current-tree suite executed green in baseline (orchrun :5516-5537, :5673-5690); "92/0 at card" is card-time scoping, current count differs (22 lib + 4 acceptance + 3 fuzz); crate unshipped |

## 7. Dead / duplicated / unreachable code
- Nothing dead inside the file (all variants + constructors used). The file as a whole is unreachable from shipped builds (F-001).
- Cross-crate duplication note (for the Lead): this is the THIRD parallel damage-ledger design in the crate (ass/parser.rs `AssDamage` 8 variants, srt damage, this) — same pattern, three vocabularies; `#[non_exhaustive]` doc :88 admits the unification debt ("the Lead's 5.M1 unification").

## 8. Dependencies & flags
- std-only (`core::fmt`). No features, no unsafe, no panics (boundary walk verified safe), no allocations beyond the ≤16-byte name.

## 9. Open questions for the next agent
- A07 (vtt/mod.rs): the `Err(_) if !raw.is_empty() && raw.bytes().all(ascii_digit)` branch (:232) — confirm a NEGATIVE all-digit id ("-5") takes the plain auto-id path (my read says yes: '-' fails `is_ascii_digit`), i.e. no false IdOverflow.
- A03 (vtt/fuzz.rs): does the fuzz harness ever assert ledger bounds (event count ≤ block count)? My census says no.
