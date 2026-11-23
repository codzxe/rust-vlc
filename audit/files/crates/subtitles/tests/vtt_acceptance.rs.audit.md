# AUDIT: crates/subtitles/tests/vtt_acceptance.rs
| Field | Value |
|---|---|
| Auditor | W09-A01 |
| Date | 2026-11-23 |
| Lines / functions | 252 / 5 (4 tests + 1 helper) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (integration test; runs via `cargo test` — baseline audit/runs/baseline/cargo-test-workspace.txt:6143-6149, all 4 tests ok) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | ffmpeg regenerations of 3 fixtures (exit 0, all BYTE-EXACT vs committed); fixture byte-dump cat -A; sed quotes; baseline cite |

## 1. One-paragraph verdict
This is what an acceptance suite should look like, and its provenance claims are TRUE — I regenerated all three "ffmpeg-generated" fixtures with the sandbox's ffmpeg 7.1.5 (`-c:s webvtt`) and each matches the committed file byte-for-byte, including the non-obvious behaviors the doc claims (entities kept for our stripper, `<v>` names dropped by ffmpeg, chronological re-order of an out-of-order SRT, and ffmpeg's webvtt encoder discarding ALL ASS positioning — alignments 7/5/1 + `{\an4}` → settings-free cues). The hand-derived expectations are arithmetically correct (I re-derived every µs value and every damage line number 12/28/28/34/37 against the fixture), the `plain()` sentinel makes a styled payload fail every equality, and the damage-ledger assertion is an exact encounter-ordered vector. The only nits: the suite pins (without comment) a duplicate-CueId quirk, and the ASS source of `styled.vtt` is not committed, so that leg is reproducible only via my reconstruction.

## 2. What actually works (verified)
- Fixture regeneration, all exit 0:
  - `ffmpeg -i w09_basic.srt -c:s webvtt` → `diff` vs fixtures/vtt/basic.vtt: identical (incl. `<v Kim>Speaker line` → `Speaker line` — ffmpeg drops the speaker name, as documented).
  - out-of-order SRT (lead-in cue LAST) → `LONG MATCHES BYTE-EXACT` (ffmpeg re-orders chronologically, as documented).
  - ASS with alignments 7/5/1 + `{\an4}` → `STYLED MATCHES BYTE-EXACT` (ffmpeg emits plain cues, no settings — as documented).
- Baseline: `test ffmpeg_basic_cues_times_text_equal_truth ... ok`, `ffmpeg_from_ass_drops_positioning_plain_cues ... ok`, `ffmpeg_long_form_hours_and_unicode_equal_truth ... ok`, `handcrafted_settings_heavy_equals_truth ... ok` (cargo-test-workspace.txt:6146-6149).
- Arithmetic re-derivation (by hand): `01:02.004` (MM:SS.mmm) = 62_004_000 µs ✓; `01:23:45.678` = 5_025_678_000 µs ✓; `&quot;` stays literal (WebVTT's six named escapes exclude `quot`) ✓.
- Fixture line count (cat -A): REGION header at line 12, cue-4 settings line 28, reversed timestamps line 34, `position:120%` line 37 — all match the asserted damage vector exactly, in order.

## 3. Findings
F-001 [S4]
Where: crates/subtitles/tests/vtt_acceptance.rs:159-170
Code:
```
    let cue = &doc.cues[2];
    assert_eq!(cue.id.get(), 1);
```
What is wrong: the suite pins, without comment, that a non-numeric cue id ("three") mints an AUTO id that COLLIDES with an earlier explicit numeric id (doc ids: 1, 2, 1, 4, 2, 3 — two duplicates). A consumer keying cues by `CueId` gets collisions; nothing in the test or the parser doc flags it. Confidence: HIGH — Verification: read (id sequence follows from the asserts). Fix: a one-line comment, or auto-id offsetting.

F-002 [S4]
Where: crates/subtitles/tests/vtt_acceptance.rs:16-19
What is wrong: `styled.vtt`'s ASS source is not committed (only the .vtt output), so the "from an ASS" leg is not reproducible from the repo — I reconstructed it (byte-exact) but a future reader cannot. Fix: commit the .ass/.srt sources next to the .vtt outputs. Confidence: HIGH — Verification: rg/ls fixtures dir (only .vtt files) + my regeneration.

Zero findings beyond S4 was nearly the case — everything else checked: exact-time equality, exact-positioning equality, exact-damage-vector equality, tag-stripping vs entity-decoding distinction (escaped `&lt;v Kim&gt;` survives as text while a real `<v Fred>` is stripped per vtt/mod.rs:55-66 doc example — consistent), BOM+CRLF tolerance, NOTE/STYLE/REGION handling, `cue_rule_count == 2`, out-of-range percent → cue kept + `BadSetting`.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `plain` | 38-43 | REAL | — | styled payload → sentinel that fails all equalities (honest) |
| `ffmpeg_basic_cues_times_text_equal_truth` | 45-73 | REAL | — | times/text/ids/positioning pinned; no damage |
| `ffmpeg_long_form_hours_and_unicode_equal_truth` | 75-100 | REAL | — | long timestamp form + UTF-8 + `&quot;` literalness |
| `ffmpeg_from_ass_drops_positioning_plain_cues` | 102-117 | REAL | F-002 | 4 cues, default positioning, no damage |
| `handcrafted_settings_heavy_equals_truth` | 119-251 | REAL | F-001 | 6 cues × full settings matrix + exact 5-event damage ledger |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| ffmpeg_basic_cues_times_text_equal_truth | YES | exact µs + exact text incl. entity decode and tag strip; auto-id sequence; default Positioning equality |
| ffmpeg_long_form_hours_and_unicode_equal_truth | YES | long-form hour parsing; UTF-8 payload equality; `&quot;` non-decoding pinned |
| ffmpeg_from_ass_drops_positioning_plain_cues | YES | zip of 4 exact tuples + `Positioning::default()` per cue |
| handcrafted_settings_heavy_equals_truth | YES | per-cue line/position/align asserts + exact encounter-ordered damage vector with line numbers + style-block introspection |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Evidence |
|---|---|---|---|
| "three ffmpeg-generated WebVTT documents (`ffmpeg -c:s webvtt`, 7.1.5)" | L1-2 | REAL | regenerated all 3 byte-exact with sandbox ffmpeg 7.1.5-0+deb13u1 |
| ffmpeg "keeps `&amp;`-family escapes and `<i>`/`<b>` tags" / "drops `<v>` names itself" | L8-11 | REAL | regeneration: `<i>Italic</i>` kept, `<v Kim>` dropped |
| ffmpeg "chronological re-order (lead-in cue first)" | L12-14 | REAL | out-of-order SRT → byte-exact reorder |
| ffmpeg webvtt encoder "drops ALL positioning" from ASS | L16-19 | REAL | ASS alignments+{\an4} → settings-free output, byte-exact |
| "`&quot;` stays literal — WebVTT defines exactly six named escapes" | L14-15 | REAL | fixture + expected text keep `&quot;`; the six (amp,lt,gt,lrm,rlm,nbsp) is the spec's set |
| styled.vtt "from an ASS" | L16 | REAL (reconstruction) / source NOT committed | F-002 |

## 7. Dead / duplicated / unreachable code
None. (The whole `subtitles` crate remains unreachable from shipped apps — see parser.rs audit — but as test evidence this file is exercised in CI.)

## 8. Dependencies & flags
`subtitles` + dev `testkit` (`Must`). Fixtures via `include_str!` (compile-time, no I/O). No features.

## 9. Open questions for the next agent
- vtt parser owner: is the auto-id minting (collisions with explicit numeric ids) intended? Test pins it (F-001).
- `LinePlacement::LineNumber(-1)` (line:-1, "from the bottom") — who consumes negative line numbers downstream?
