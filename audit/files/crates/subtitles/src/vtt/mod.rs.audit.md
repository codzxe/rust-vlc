# AUDIT: crates/subtitles/src/vtt/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A07 |
| Date | 2026-11-21 |
| Lines / functions | 423 (279 production + 144 tests) / 8 production units + 7 tests |
| File verdict | UNREACHABLE (real, differential-tested parser; zero shipped-entry-point callers) |
| Reachable from shipped entry point? | **NO.** `subtitles::vtt::parse` is called only by `crates/subtitles/src/vtt/fuzz.rs:29` and the crate's own tests (`vtt_acceptance.rs`, `vtt_fuzz.rs`, `roundtrip_matrix.rs`, `fuzz_survival.rs`). `rg -n "subtitles" crates/demux/Cargo.toml crates/playback-engine/Cargo.toml apps/cli/Cargo.toml` → no dependency entries. The closest the shipped tree comes to subtitles: `crates/demux/src/ts/mod.rs:365,382` maps TS stream type `0x90` to the string `"pgssub"`/`MediaKind::Subtitle` — track *detection* only; no decode path exists. `lumen engine-info` features: no subtitle lane. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `cat crates/subtitles/src/vtt/mod.rs` (full read); `cat crates/subtitles/src/vtt/damage.rs crates/subtitles/src/vtt/fuzz.rs` (ledger-cap question; name truncation only); `cat crates/subtitles/src/vtt/timestamp.rs` (split_timing's two parse entry points); `sed -n '173,190p;191,206p'` (quote verification); `sed -n '1,80p' docs/formats/sub-vtt.md` (record claims); `rg -n "Doc-tests subtitles" -A 5 audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (doctests: 2/2 ok); `rg -n "deps/subtitles|test result" .../orchrun-test-workspace-nofailfast.log.txt` (lib 267/0, vtt_acceptance 4/0, vtt_fuzz 3/0). |

## 1. One-paragraph verdict
A clean, genuinely linear WebVTT block walker — unlike the SRT parser it never rescans, so the "one pass" claim actually holds here — with typed damage for every degradation, ffmpeg-fixture differential evidence, and honest unsupported-setting notes. Two defects: block classification by bare `starts_with("NOTE"/"STYLE"/"REGION")` silently eats legal cue identifiers that begin with those keywords (no damage event at all for NOTE*/STYLE*), and the damage ledger has no event cap, contradicting the fuzz harness's own "the damage ledger is capped" bound claim while SRT caps at 256 and DVB at 64. As with the whole crate: nothing shipped can call it.

## 2. What actually works (verified)
— Header gate, BOM/CRLF tolerance, blank-block walk, id policy (numeric verbatim, overflow→damage+auto, non-numeric→auto from 1), timing split with greedy end-timestamp, settings projection, tag-stripped text: verified by reading, and by baseline `orchrun-test-workspace-nofailfast.log.txt` → `subtitles` lib `267 passed`, `vtt_acceptance 4 passed`, `vtt_fuzz 3 passed`, doctests `2 passed` (the doctest in this file's module docs is one of them).
— `split_timing` is genuinely one-pass (each block visited once; `parse_block` is O(block)); no SRT-style resync quadratic.

## 3. Findings

### F-001 [S3] `starts_with("NOTE"/"STYLE"/"REGION")` without the spec delimiter — cue identifiers beginning with those keywords are silently dropped
Where: crates/subtitles/src/vtt/mod.rs:173-189
Code:
```rust
    if first.starts_with("NOTE") {
        return; // comment block: skipped whole
    }
    if first.starts_with("STYLE") {
        let css = block[1..].join("\n");
```
What is wrong: WebVTT cue identifiers are arbitrary strings (spec: any characters without `-->` and without a newline); the spec distinguishes a comment block by `NOTE` followed by a space/tab/end-of-line, and style/region blocks by the line being exactly `STYLE`/`REGION`. This parser classifies any block whose first line merely *starts with* the keyword. Consequences, traced:
- a cue with id `NOTES-1` → treated as a comment → **cue dropped, zero damage events**;
- a cue with id `STYLE-2` → its timing+text lines are collected as CSS into `style_blocks` → **cue dropped, zero damage events** (and `cue_rule_count` polluted);
- a cue with id `REGIONS` → `UnsupportedBlock` damage (visible, but the kind lies — the block was a legal cue).
Why it matters: legal documents lose cues with no diagnostic trail for two of the three keywords — the exact "silent misrendering" the module doc (:34-36) promises never happens ("typed notes, never silent").
What correct looks like: `first == "NOTE" || first.starts_with("NOTE ") || first.starts_with("NOTE\t")` (and exactly `==` for STYLE/REGION), else fall through to cue parsing.
Confidence: HIGH on the code behavior (trivial trace); MEDIUM on spec exactness (spec text not available in sandbox — stated from knowledge; no browser conformance suite in tree).
### F-002 [S3] Damage ledger has no event cap — contradicts the fuzz-harness bound claim and the crate's own ledger discipline
Where: crates/subtitles/src/vtt/mod.rs:193, 201, 215, 222-228, 234 (every `doc.damage.push`), claim at vtt/fuzz.rs:20-21
Code:
```rust
    let Some(timing_idx) = timing_idx else {
        doc.damage.push(DamageEvent {
            line_no: line_no(0),
            kind: DamageKind::OrphanBlock,
        });
```
Claim (vtt/fuzz.rs:20-22):
```rust
//! # Bounds (MP §12 / D-2)
//!
//! Parse: one pass, output ≤ O(input); the damage ledger is capped
```
What is wrong: SRT caps its ledger at 256 events with a dropped-count (`srt/damage.rs`, `MAX_DAMAGE_EVENTS`), DVB at 64 (`dvb/mod.rs:77`) — VTT pushes into an unbounded `Vec`. `vtt/damage.rs` truncates only the *name strings* (≤16 bytes, "so hostile inputs cannot balloon the ledger" — damage.rs:16-17), which bounds per-event size, not event count. A hostile 25 MB VTT of orphan two-line blocks yields ~10⁶ `DamageEvent`s (tens of MB) — linear, so not a hang, but "capped" is false and the crate-wide discipline (a ledger a UI can safely display) is broken by this one parser.
Why it matters: the fuzz harness's "never allocates without a ceiling" contract is stated over a false premise; any future consumer rendering the ledger (the documented purpose: "callers can surface the degradation honestly") gets an unbounded report.
What correct looks like: the SRT `DamageSink` pattern (cap + `dropped_damage_events` counter) — `VttDocument` already has the field shape to copy (`SrtDocument.dropped_damage_events`).
Confidence: HIGH — Verification: read (all push sites listed; no cap check anywhere in mod.rs/settings.rs call path).
### F-003 [S4] Header gate over-accepts: `starts_with("WEBVTT")` with no delimiter check
Where: crates/subtitles/src/vtt/mod.rs:139-141
Code:
```rust
    if lines.is_empty() || !lines[0].starts_with("WEBVTT") {
        return Err(VttError::MissingHeader);
    }
```
What is wrong: the spec requires the magic followed by a space, tab, or end-of-line; `WEBVTTGarbage` is accepted as a valid WebVTT document. Over-tolerance only (nothing is lost — the rest parses normally), but the "one fatal" gate is looser than documented ("does not begin with the `WEBVTT` header line", :104-105).
Why it matters: a non-VTT file that happens to start with those bytes is parsed instead of rejected.
What correct looks like: `lines[0] == "WEBVTT" || lines[0].starts_with("WEBVTT ") || lines[0].starts_with("WEBVTT\t")`.
Confidence: HIGH — Verification: read; spec-exactness MEDIUM (same caveat as F-001).
### F-004 [S4] Empty cue text accepted silently — undocumented divergence from the SRT parser's `EmptyCueText` damage
Where: crates/subtitles/src/vtt/mod.rs:252-259
Code:
```rust
    let text = text::decode_cue_text(&block[timing_idx + 1..].join("\n"));
    let positioning = settings::to_positioning(&outcome);
    doc.cues.push(Cue::new(
```
What is wrong: a cue with no text lines produces `CueText::Plain("")` with no damage event (tested and pinned: `glued_separator_and_empty_cue_text`, :400-408, `assert_eq!(doc.damage.len(), 0)`), while the SRT parser records `EmptyCueText` and drops the cue. Neither the module doc nor sub-vtt.md mentions the divergence. Defensible (the spec does not forbid empty cues) but undocumented and inconsistent across the two text parsers a consumer would use side by side.
What correct looks like: one documented policy — either both damage or both accept, with a line in both module docs.
Confidence: HIGH — Verification: read both parsers + both records.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct StyleBlock` | 84-90 | REAL | — | Data carrier; `cue_rule_count` = `css.matches("::cue").count()` — substring counting, so `::cue` inside a comment/string also counts (cosmetic; count is advisory). |
| `struct VttDocument` | 94-102 | REAL | — | Cues + damage + style blocks. Derives Default. |
| `enum VttError` + `Display` + `Error` | 106-124 | REAL | — | One variant, `#[non_exhaustive]`, correct message. |
| `parse` | 132-165 | REAL | S4 (F-003) | BOM strip, CRLF normalize (suffix `\r` only — a lone `\r` line separator is NOT normalized; split('\n') leaves it embedded → such files misparse blocks. WebVTT requires LF/CRLF so this is spec-legal input handling, noted not filed). Header gate over-accepts (F-003). Walk is linear; blocks visited once. |
| `parse_block` | 170-260 | REAL | S3 (F-001, F-002) | Keyword classification (F-001); `timing_idx > 1` → `TimingLineNotSecond` (correct one-id-line rule); id policy correct incl. `IdOverflow` with digit-check; `CueInterval::new` failure → `ReversedInterval` damage (interval rejected, cue dropped — same as SRT). Empty text accepted (F-004). All damage pushes uncapped (F-002). |
| `split_timing` | 265-274 | REAL | — | First `-->` split; full-parse left (trailing junk on the left ⇒ `BadTimestamp`), greedy-prefix right (settings ride behind). Sound; delegates to timestamp.rs (audited there by another agent; its u128/saturation core verified green in its tests). |
| `is_blank` | 276-278 | REAL | — | Space/tab only + vacuous-empty ⇒ blank. Stricter than SRT's `trim()` (Unicode NBSP is not blank here) — spec-consistent (only ASCII whitespace per spec). |
| `mod tests` (7 tests) | 280-423 | REAL | — | See §5. |

## 5. Test honesty (7 tests, lines 288-422)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `header_forms_and_empty_documents` | Yes | Header variants, BOM+CRLF, three MissingHeader rejects. |
| `basic_cue_with_id_and_settings` | Yes | Exact µs, text decode, all three settings mapped, zero damage. |
| `ids_numeric_nonnumeric_and_absent` | Yes | Verbatim/auto id policy (also pins the id-collision behavior 1,1,2 — documented). |
| `note_style_region_blocks` | Yes | NOTE skip, STYLE collection + `::cue` count=2, REGION damage. Would NOT catch F-001 (no id starting with a keyword is tested). |
| `malformed_blocks_are_damages_not_panics` | Yes | Seven damage kinds asserted with exact line number (`damage[6].line_no == 20`). Would NOT catch F-002 (no cap assertion). |
| `glued_separator_and_empty_cue_text` | Yes | Glued `-->` µs asserts; pins the F-004 empty-text acceptance. |
| `multiline_text_and_line_numbers` | Yes | Entity+tag decode across lines; 75-minute hours-less form. |
| Coverage gaps (mutation reasoning) | — | No test: (a) id lines starting with NOTE/STYLE/REGION (F-001 passes all tests); (b) any ledger-cap behavior (F-002 passes); (c) lone-CR separators. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "status = implemented (win/linux/macos)" | matrix row `subtitle-vtt` (feature-matrix.toml:923); sub-vtt.md:2-3 | UNREACHABLE (implemented-at-crate, unreachable-in-product) | Code+tests real (this audit; no-fail-fast baseline: lib 267/0, vtt_acceptance 4/0, vtt_fuzz 3/0, doctests 2/0); zero shipped callers (Reachability row); "Phase-6 network-hls consumes the parser" (sub-vtt.md) is a next step, not a present fact. |
| "Parse: one pass, output ≤ O(input)" | vtt/fuzz.rs:20 | REAL | Walk is linear (no resync rescan; contrast SRT F-001). |
| "the damage ledger is capped" | vtt/fuzz.rs:21 | FALSE | F-002: no event cap; only per-event name truncation (damage.rs:16-17). |
| "NOTE blocks skipped whole; STYLE blocks collected tolerantly …; REGION blocks typed-unsupported notes" | sub-vtt.md:12-15; mod.rs:1-5 | SHALLOW | True for exact-keyword blocks; F-001 makes all three swallow legal cue ids (NOTE/STYLE with no damage at all). |
| "`vertical:`/`size:` settings and REGION blocks are known WebVTT this parser does not apply — typed notes, never silent misrendering" | mod.rs:34-36 | REAL (for settings) / PARTLY FALSE (for blocks) | Settings: verified typed (test :385-392). "Never silent" is contradicted by F-001's silent NOTE*/STYLE* cue loss. |
| "Cue text → CueText::Plain: ALL tags stripped … `&quot;` stays literal" | sub-vtt.md:26-29 | REAL (at this layer) | text.rs owns the decode; its unit tests + vtt_acceptance pin it (green in no-fail-fast baseline). Full verify is text.rs owner's. |
| "the ONLY fatal is a missing header" | sub-vtt.md:10-11 | REAL | Only `VttError::MissingHeader` exists; every other path is damage. |

## 7. Dead / duplicated / unreachable code
- Unreachable from shipped entry points (Reachability row).
- No duplication inside this file; the damage-event shape parallels srt/damage.rs by design (different cap policy — F-002).

## 8. Dependencies & flags
- Std-only; no feature flags; no unsafe; no `#[allow]`. Clippy-clean per baseline #5.

## 9. Open questions for the next agent
- To the `timestamp.rs` owner: mod docs claim the hours-less `75:00.000` form is "spec-permitted" (timestamp.rs:8-10). My recollection of the HTML/WebVTT timestamp grammar is minutes 00–59 in BOTH forms (only hours may be arbitrarily long) — if so, `75:00.000` should be `BadTimestamp` and the doc line is false. UNVERIFIED (no spec text in sandbox); worth a check against the HTML Standard's "parse a timestamp".
- To the `settings.rs` owner: duplicate-setting "first-valid-wins" is claimed in sub-vtt.md — I did not re-verify the merge order here (parse_settings mutates doc.damage; cap question F-002 applies to its pushes too).
- Wave-level: `subtitle-vtt` matrix row evidence cites "7003 fuzz inputs zero panics" — the fuzz budget keeps inputs <200 KB, which is why F-002's uncapped ledger never surfaced.
