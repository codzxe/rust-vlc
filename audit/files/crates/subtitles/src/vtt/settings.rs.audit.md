# AUDIT: crates/subtitles/src/vtt/settings.rs
| Field | Value |
|---|---|
| Auditor | W09-A06 |
| Date | 2026-11-23 |
| Lines / functions | 312 / 12 (7 prod fns + 1 struct, 5 tests + 1 helper) |
| File verdict | REAL (library) / UNREACHABLE (product) — clean subset parser with spec-conformance gaps at the edges |
| Reachable from shipped entry point? | NO — parse_settings <- vtt/mod.rs:221,253 <- `subtitles` crate root <- no app depends on it (only fuzz/Cargo.toml:50) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 3 |
| Commands run | prebuilt libtest `./target/debug/deps/subtitles-70f89de07410626f vtt::settings` -> 5 passed / 0 failed (exit 0); rg callers + region search (exit 0); sed quote verification |

## 1. One-paragraph verdict
A tidy, tested, damage-ledgered WebVTT cue-settings parser: `line`/`position`/`align` map onto the model's Positioning with correct suffix anchors, percent guards (Percent::new rejects >100), and typed BadSetting/UnsupportedSetting/UnknownSetting events; 5/5 tests pass with exact vectors. The gaps: `region:...` — a CURRENT-spec WebVTT cue setting — is misfiled as an unknown "future revision" setting (rg: no `region` handling anywhere in vtt/), and `position:X%,start|end|auto` (spec-legal values per the position-setting grammar) is rejected as bad, dropping the ENTIRE position including the valid percent. The "first VALID occurrence wins" duplicate policy deviates from the WebVTT parse algorithm's map semantics (first occurrence, valid or not, claims the name) — UNVERIFIED against the spec text offline. And the standing blocker: none of this is reachable from any shipped binary.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-70f89de07410626f vtt::settings` -> `test result: ok. 5 passed; 0 failed` (exit 0): line:0%/-1/-1,end, position:50%/75%,line-right, align:start/center, vertical:rl + size:50% typed-unsupported, 9 bad-value vectors each producing exactly one BadSetting, duplicate-align first-wins, position-suffix-then-align override order, bottom-center default merge.

## 3. Findings
F-001 [S2]
Where: this file + apps/cli/Cargo.toml (absence); fuzz/Cargo.toml:50
Code: `subtitles = { path = "../crates/subtitles" }` (fuzz/Cargo.toml:50; no such row in apps/cli/Cargo.toml)
What is wrong: domain blocker (see tags.rs F-001) — subtitle-vtt matrix row claims `implemented` on 3 platforms for code no shipped binary links. Confidence: HIGH — Verification: ran (rg).
F-002 [S3]
Where: crates/subtitles/src/vtt/settings.rs:90 (absence of a `region` arm)
Code:
```
            other => out.push(DamageEvent::unknown_setting(other, line_no)),
```
What is wrong: `region` is a WebVTT-1.0 cue setting (region:<id> — cues attach to a region with its own viewport/anchor semantics), not a "future revision" name. The parser records it as UnknownSetting and the cue silently renders at the default bottom-center placement instead of inside its region. rg confirms zero region handling in vtt/. What correct looks like: a `region` arm recording UnsupportedSetting (honest), plus a doc note that region-attached cues degrade to default placement. Confidence: HIGH — Verification: read + ran (rg).
F-003 [S3]
Where: crates/subtitles/src/vtt/settings.rs:160-169
Code:
```
    if let Some(suffix) = suffix {
        match suffix {
            "left" | "line-left" => result.horizontal = Some(HorizontalAlign::Left),
            "center" => result.horizontal = Some(HorizontalAlign::Center),
            "right" | "line-right" => result.horizontal = Some(HorizontalAlign::Right),
            _ => {
                out.push(DamageEvent::bad_setting("position", line_no));
                return false;
```
What is wrong: the WebVTT position setting also permits `start`, `end` (writing-direction relative, = left/right for horizontal scripts — the exact mapping `align` already does at line 184/186) and `auto`. A spec-legal `position:50%,start` is classified BadSetting and `return false` DISCARDS the valid 50% percent too (result.position never set). What correct looks like: accept start/end mapped like align; accept/auto-ignore `auto`; on a bad suffix keep the numeric position and drop only the anchor. Confidence: HIGH (grammar from the module's own sibling `align` mapping) — Verification: read; no test covers it.
F-004 [S4]
Where: crates/subtitles/src/vtt/settings.rs:19-20 vs 63-66
Code:
```
//! Per the spec, a duplicate setting name is ignored after its first
//! valid occurrence; settings apply in encounter order so a later
```
```
        match name {
            "line" => {
                if !seen_line && parse_line(value, &mut result, line_no, out) {
                    seen_line = true;
                }
```
What is wrong: "first VALID occurrence wins" means `line:abc line:50%` applies 50% — but the WebVTT parse algorithm stores name->value in a map keyed by name before/while interpreting, so the FIRST occurrence (even invalid) claims the name and the second is skipped. The code's policy contradicts the spec text it cites; spec text itself UNVERIFIED offline (no network; no spec copy in-repo). Impact: sub-percentplacement differences on malformed-then-valid duplicates. Confidence: MEDIUM — Verification: read; UNVERIFIED against spec wording.
F-005 [S4]
Where: crates/subtitles/src/vtt/settings.rs:57-60
Code:
```
    for token in settings.split_ascii_whitespace() {
        let Some((name, value)) = token.split_once(':') else {
            out.push(DamageEvent::bad_setting("", line_no));
            continue;
        };
```
What is wrong: a colon-less token is recorded with an EMPTY setting name — the damage ledger loses the offending token text, making log triage guesswork. Confidence: HIGH — Verification: read.
F-006 [S4]
Where: crates/subtitles/src/vtt/settings.rs:63-89
Code:
```
            "vertical" => {
                if !seen_vertical {
                    seen_vertical = true;
                    out.push(DamageEvent::unsupported_setting(name, line_no));
                }
            }
```
What is wrong: inconsistent duplicate policy: `vertical`/`size` suppress damage for repeat occurrences, but repeated invalid `line:`/`position:` tokens each emit a fresh BadSetting (no seen-flag on the failure path). Cosmetic ledger inconsistency. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SettingsOutcome struct | 30-39 | REAL | — | partial fields, merged by caller |
| parse_settings | 44-94 | REAL | S4 | token loop, seen-flags (F-004/F-006), empty-name damage (F-005) |
| parse_line | 97-135 | REAL | — | %/lineno split, suffix anchors, rejects out-of-range percents |
| parse_position | 138-173 | REAL | S3 | missing start/end/auto suffixes (F-003); numeric+suffix atomicity |
| parse_align | 177-193 | REAL | — | start/end mapped L/R; "middle" accepted (non-spec leniency, harmless) |
| parse_i32 / parse_i64 | 195-207 | REAL | — | length-guarded, no panics |
| to_positioning | 210-221 | REAL | — | bottom-center default merge, tested |
| tests (5) | 223-312 | REAL | — | see §5 |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| subset_maps_to_model | YES | 4 exact placements incl. suffix anchors |
| unsupported_and_unknown_are_typed_notes | YES | exact kinds and count |
| bad_values_damage_not_panic | YES | 9 vectors, exactly-one-damage each (would catch double-record bugs) |
| first_valid_occurrence_wins_and_order_overrides | YES | pins the (spec-deviant) duplicate + override-order policy |
| positioning_merge_default_is_bottom_center | YES | default merge exact |
Gaps: no `position:…,start/end/auto` (F-003), no `region:` (F-002), no invalid-then-valid duplicate case (F-004).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cue settings map to our position model" (subtitle-vtt notes) | feature-matrix.toml:928 | REAL (library) / UNREACHABLE (product) | test run; F-001 |
| "vertical:rl/lr, size:n% — unsupported — typed note, not applied" | settings.rs:17 | REAL | unsupported_and_unknown test |
| "Unknown setting names (future WebVTT revisions)" | settings.rs:4-5 | FALSE for `region` | F-002: region is current-spec |
| "a duplicate setting name is ignored after its first valid occurrence" (attributed to the spec) | settings.rs:19-20 | UNVERIFIED / likely deviant | F-004 |
| mapping table rows line/position/align | settings.rs:11-16 | SHALLOW | position row omits start/end/auto (F-003) |

## 7. Dead / duplicated / unreachable code
No in-file dead code. Whole chain unreachable from shipped builds (F-001).

## 8. Dependencies & flags
std-only (crate dep: media-types); dev testkit. No features, no unsafe. Percent/Positioning come from model::position (another agent's file — the 0..=100 vocabulary rejects off-range values by design).

## 9. Open questions for the next agent
- vtt/mod.rs damage dedup: does the doc-level damage ledger collapse repeated identical events? (F-006 noise depends on it.)
- The `line` LineNumber semantics for negative values (count-from-bottom) — verify the model/renderer actually inverts the anchor; settings.rs only stores the raw number.
