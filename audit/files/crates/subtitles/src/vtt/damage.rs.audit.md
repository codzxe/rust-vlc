# AUDIT: crates/subtitles/src/vtt/damage.rs
| Field | Value |
|---|---|
| Auditor | W09-A09 |
| Date | 2026-11-30 |
| Lines / functions | 158 / 5 prod (bad_setting, unknown_setting, unsupported_setting, unsupported_block, truncate_name) + Display |
| File verdict | UNREACHABLE (real, small, correct; used by the parser; taxonomy covered by mod.rs tests) |
| Reachable from shipped entry point? | NO — constructors are `pub(super)`, consumed by vtt/mod.rs (OrphanBlock/TimingLineNotSecond/BadTimestamp/ReversedInterval/IdOverflow/unsupported_block) and vtt/settings.rs (bad_setting:59, unsupported_setting:81, unknown_setting:90); all inside `crates/subtitles`, sole tree-wide dependent `fuzz/Cargo.toml:50` (non-member) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/subtitles-85f02b91c21cc80b vtt` → 22 passed 0 failed (exit 0); `rg 'MAX_DAMAGE|damage.len' crates/subtitles/src/vtt/{mod,settings}.rs` → no cap found (exit 1) |

## 1. One-paragraph verdict
A clean, small damage-taxonomy module: the constructors are all reachable from the parser, `truncate_name` is correct (char-boundary loop can stop at 0, no underflow; 16-byte cap verified by read), Display covers all 8 variants, and the taxonomy is exercised end-to-end by the 22 passing vtt tests (BadTimestamp/ReversedInterval/OrphanBlock/TimingLineNotSecond/UnsupportedSetting/IdOverflow asserted on real parses in vtt/mod.rs:375-393). Two real complaints: the doc promises hostile inputs "cannot balloon the ledger", but only each event's NAME is capped — the event COUNT is uncapped (`doc.damage` is a bare `Vec`, mod.rs:99, no `MAX_DAMAGE_EVENTS` unlike the sibling srt module's 256-entry cap), so a settings-heavy hostile file produces an O(input) event vec with a per-event String allocation; and the sole `bad_setting` call site passes a hardcoded empty name, making the name parameter and its truncation path dead in practice and the diagnostic uselessly vague.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-85f02b91c21cc80b vtt` → `test result: ok. 22 passed; 0 failed` (includes the damage-ledger assertions in vtt/mod.rs tests). `truncate_name` semantics re-derived by read: `is_char_boundary(0)` is always true so the loop cannot underflow; multibyte case (3-byte chars, byte 16 mid-char) steps back to a boundary.

## 3. Findings
F-001 [S3]
Where: damage.rs:20-21 (claim), mod.rs:99 (`pub damage: Vec<DamageEvent>`), settings.rs:59-90 (per-token pushes)
Code: (verbatim)
```
//! Setting/block names stored in events are truncated at a char
//! boundary to ≤16 bytes so hostile inputs cannot balloon the ledger.
```
What is wrong: the truncation bounds each event's name, not the ledger's SIZE. Every whitespace token on a cue timing line that is not `name:value`, or has an unknown name, pushes an event (settings.rs:59, 90); `doc.damage` has no cap anywhere in the vtt module (`rg 'MAX_DAMAGE|damage.len'` over mod.rs/settings.rs → no hits). The sibling srt damage module caps at `MAX_DAMAGE_EVENTS = 256` with overflow counting (srt/damage.rs:17,148-150) — vtt has no equivalent, so a hostile one-line file with millions of setting tokens produces millions of events (each with a String allocation): a memory amplification the doc claims cannot happen. What correct looks like: the srt-style cap + `Overflow`-style event, or at minimum a doc that admits linear growth.
Confidence: HIGH — Verification: read + rg (exit 1 on cap search); no executable measurement (UNVERIFIED by run — would need a crafted 10 MB file and a memory probe).
F-002 [S4]
Where: damage.rs:34-41, settings.rs:58-60
Code: (verbatim)
```
    pub(super) fn bad_setting(name: &str, line_no: u32) -> Self {
        Self {
            line_no,
            kind: DamageKind::BadSetting {
                name: truncate_name(name),
            },
        }
    }
```
```
        let Some((name, value)) = token.split_once(':') else {
            out.push(DamageEvent::bad_setting("", line_no));
            continue;
```
What is wrong: the only call site hardcodes `""` — the offending token (the actual malformed setting text) is discarded, the `name` parameter is pointless, and the Display output is "setting `` malformed or out of domain" with no diagnostic content. The `truncate_name` path for `BadSetting` is dead in practice.
Confidence: HIGH — Verification: read + rg (single call site).
F-003 [S4]
Where: damage.rs:75-85
Code: (verbatim)
```
fn truncate_name(name: &str) -> String {
    if name.len() <= 16 {
        name.to_owned()
    } else {
        let mut end = 16;
        while !name.is_char_boundary(end) {
            end -= 1;
        }
        name[..end].to_owned()
    }
}
```
What is wrong: no test anywhere exercises the >16-byte or multibyte-boundary path — `rg truncate|\\b16\\b` over tests/vtt_fuzz.rs and tests/vtt_acceptance.rs finds only unrelated `bytes.truncate` calls. The function is correct by read, but the "char boundary" safety property (the one thing that could panic if rewritten naively) is unpinned.
Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| bad_setting | 34-41 | REAL | S4 | sole caller passes "" (F-002) |
| unknown_setting | 44-51 | REAL | — | called settings.rs:90 |
| unsupported_setting | 54-61 | REAL | — | called settings.rs:81 |
| unsupported_block | 64-71 | REAL | — | called mod.rs:187 (REGION) |
| truncate_name | 75-85 | REAL | S4 | correct; untested boundary path (F-003) |
| Display for DamageKind | 136-158 | REAL | — | all 8 variants, accurate text |

## 5. Test honesty
No tests in this file; the taxonomy is tested indirectly via vtt/mod.rs tests (executed, 22/22 pass) — those assert kinds on real parses and would fail if production were broken. `truncate_name` specifically: untested (F-003).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the parser never panics and never aborts on a malformed region" | damage.rs:1-2 | REAL | parser pushes events instead of returning early (mod.rs:193-236) |
| names "truncated at a char boundary to ≤16 bytes" | damage.rs:20-21 | REAL | truncate_name (read) |
| "so hostile inputs cannot balloon the ledger" | damage.rs:21 | FALSE (count-wise) | F-001: no event cap in vtt |
| taxonomy covers malformed/unsupported/noted | damage.rs:7-18 | REAL | all constructors called; kinds asserted in tests |

## 7. Dead / duplicated / unreachable code
- `bad_setting`'s name parameter + its truncation path: effectively dead (F-002). File otherwise unreachable from shipped entry points (crate-level).

## 8. Dependencies & flags
- std only (`core::fmt::Display`); no unsafe; no features. Per-event String allocation (≤16 bytes).

## 9. Open questions for the next agent
- vtt/mod.rs owner: is the uncapped damage Vec intentional (O(input) contract)? If the srt cap was deliberate policy, vtt diverges from it silently — worth a Lead decision record.
