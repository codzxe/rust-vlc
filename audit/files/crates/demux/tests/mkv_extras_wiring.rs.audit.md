# AUDIT: crates/demux/tests/mkv_extras_wiring.rs
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-23 |
| Lines / functions | 961 / 24 (15 builder/helpers + 9 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | The TEST is real; the API it pins (tags/attachments/chapters/chapter_targets/title/track_uid on MkvDemuxer) has NO shipped consumer: apps/cli/src/containers/mkv.rs harvest_facts consumes only `track_entries()` (mkv.rs:207); the ASS font hookup is a PROPOSAL (subtitles/src/font/mod.rs:262) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | `cargo test -p demux --test mkv_extras_wiring` -> exit 0, 9 passed / 0 failed in 0.13s (ffmpeg+ffprobe+DejaVu font present, generated lane RAN); grep for API consumers across crates/ and apps/ -> none outside demux; sed quote verification |

## 1. One-paragraph verdict
A genuine byte-level wiring suite: hand-built full MKV files (real EBML vint/id builders, correct Cues-before-clusters + extras-after layout, exactly ffmpeg's element order) pin that Tags/Attachments/Chapters/Info.Title/TrackUID are parsed at `start()` and retained on demuxer getters, that chapter targets convert ns→scale ticks correctly, that a Xiph-lacing lie recovers exactly once without livelock, and that a degenerate attachment folds damage into the counter. The ffmpeg-generated differential lane (real title metadata, real attached TTF asserted byte-exact, cover JPEG, ffmetadata chapters cross-checked against ffprobe) also runs and passes in this sandbox. The adversarial content is modest (3 malformed shapes) but real and regression-pinned. The damning context: none of the pinned getter surface is consumed by any shipped code — the "wiring" stops at the demuxer API, and the file's own comments sell the font attachment as "the ASS stack's input seam" while the ASS font adapter is still a `[PROPOSAL]` in the subtitles crate. The suite honestly pins demuxer-level behavior; the surrounding rhetoric implies more end-to-end wiring than exists.

## 2. What actually works (verified)
— verified by `cargo test -p demux --test mkv_extras_wiring` -> `test result: ok. 9 passed; 0 failed; 0 ignored` (my run, exit 0; matches baseline cargo-test-workspace.txt:2338-2351). The generated lane's `println!("PASS w3i-wiring.mkv: ...")` test passed with ffmpeg 7.x + DejaVuSansMono-Oblique.ttf present in this sandbox (verified via `which ffmpeg` / `ls /usr/share/fonts/truetype/dejavu/`).
— verified by grep: all getters exist (mkv/mod.rs:200-317: segment_info/track_entries/tags/attachments/chapters/title/track_uid/chapter_targets/damage_bytes) and `FileData::Inline`/`NonInline` (attachments.rs:73-81).

## 3. Findings
F-001 [S3]
Where: crates/demux/tests/mkv_extras_wiring.rs:254, 390, 913
Code:
```
    // Attachments: a font (the ASS stack's input, issue #17) and a cover.
```
What is wrong: The comment claims the attached font is "the ASS stack's input" — no ASS/subtitles code consumes `demuxer.attachments()`; the font hookup is an unrealized proposal (crates/subtitles/src/font/mod.rs:262: "[PROPOSAL]: the attachment hookup lands with the demux card]"). Why it matters: the suite's "wiring" framing reads as end-to-end; in reality the retained extras are demuxer-side metadata only — apps/cli/src/containers/mkv.rs:112-114 itself admits "Chapters/Editions/ordered-chapters: demuxer-side metadata only". What correct looks like: comments that say "future ASS seam", not present-tense "the ASS stack's input". Confidence: HIGH — Verification: read + grep (zero consumers of tags()/attachments()/chapters()/chapter_targets()/title()/track_uid() outside the demux crate).

F-002 [S3]
Where: crates/demux/tests/mkv_extras_wiring.rs:809-815
Code:
```
    let Some(font_path) = system_font() else {
        return;
    };
    if Command::new("ffmpeg").arg("-version").output().is_err() {
        eprintln!("SKIP: ffmpeg absent (generated wiring lane)");
        return;
    }
```
What is wrong: The only real-ffmpeg differential lane degrades to a vacuous pass (early `return`, eprintln only, no #[ignore], no failure) on machines without fonts or ffmpeg. Why it matters: the lane is the suite's headline ("an ffmpeg-generated differential lane", header line 5); on such machines the baseline count "9 passed" includes a test that verified nothing. It ran here, so the evidence is real in this sandbox — but the gate is environmental, not enforced. What correct looks like: a skipped test must be `#[ignore]`-visible or the harness must record the skip count. Confidence: HIGH — Verification: read + ran (passed genuinely here).

F-003 [S4]
Where: crates/demux/tests/mkv_extras_wiring.rs:840-841
Code:
```
    if !out.exists() {
        let status = Command::new("ffmpeg")
```
What is wrong: The generated MKV is cached in a fixed temp dir and reused if present — a stale artifact from an older generation code path keeps being asserted forever. Why it matters: the differential silently drifts from the generator. What correct looks like: regenerate or validate a recorded digest first. Confidence: MEDIUM — Verification: read (no staleness observed; content deterministic per ffmpeg version).

F-004 [S4]
Where: crates/demux/tests/mkv_extras_wiring.rs:824-831
Code:
```
        let status = Command::new("ffmpeg")
            .args(["-y", "-v", "error", "-f", "lavfi", "-i"])
```
What is wrong: `Command::status()` with no timeout — a hung ffmpeg hangs the suite indefinitely. Test-harness robustness only. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/demux/tests/mkv_extras_wiring.rs:816-821
Code:
```
    let dir = std::env::temp_dir().join("lumen-w3i-mkv-wiring");
    std::fs::create_dir_all(&dir).must();
```
What is wrong: Fixed shared temp dir; concurrent runs of the suite race on the same generated files (write/use interleaving between processes). Deterministic content limits the damage. Confidence: MEDIUM — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| id_bytes | 40-44 | REAL | — | correct marker-form id trim |
| size_vint | 47-57 | REAL | — | shortest vint, marker bit; 8-octet max |
| el / el_uint / el_str / cat | 60-86 | REAL | — | byte-truth element builders |
| ebml_header | 89-99 | REAL | — | DocType matroska v2 |
| simple_block | 105-113 | REAL | — | 16-bit signed rel TC (spec-correct), keyframe flag 0x80 |
| cluster | 116-122 | REAL | — | |
| track_entry | 126-134 | REAL | — | TrackUID optional |
| cues | 138-154 | REAL | — | offsets relative to Segment payload |
| mkv_file | 159-166 | REAL | — | sized Segment |
| info | 169-178 | REAL | — | scale 1ms, f64 duration BE, title |
| F64Bytes | 181-189 | REAL | — | trait for one call site (odd shape, works) |
| drain | 192-202 | REAL | — | .must() on every event — errors fail |
| tags_attachments_chapters_wire_through_start_retained | 211-453 | REAL | — | the namesake: full pin incl. nested SimpleTag, uid-77 scope join, ns→tick conversion (1500/3900), walk untouched |
| title_falls_back_to_the_global_title_tag | 457-486 | REAL | — | fallback + honest None |
| track_uids_are_stable_across_reopens | 490-523 | REAL | — | reopen identity, out-of-range None |
| multi_edition_chapters_retain_all_and_target_the_default | 528-604 | REAL | — | 3 editions, hidden atom not a target |
| chapter_targets_seek_lands_at_or_before | 608-671 | REAL | — | cue-indexed landing pinned exactly (1000 ticks) |
| unknown_size_tags_master_parses_and_playback_reaches_eos | 676-701 | REAL | — | 0xFF unknown-size streaming shape |
| damaged_block_recovers_once_and_still_reaches_eos | 709-749 | REAL | — | Xiph-lace lie, recoveries == 1 (livelock regression pin) |
| degenerate_attachment_damage_folds_into_the_counter | 753-784 | REAL | — | "attached-file-without-data" damage row |
| system_font | 792-805 | REAL | — | loud-ish skip (see F-002) |
| generated_lane_wires_title_font_cover_and_chapters | 807-961 | REAL | — | ffprobe cross-check + byte-exact font/cover |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| 8 hand-built | YES | Values pinned absolutely (uids, ns→ticks math, exact tick landings, damage reason strings); breaking retention, unit conversion, or the once-only recovery fails asserts. The cues placeholder-width trick (line 317-319) is explicitly self-checked. |
| generated_lane | YES (when it runs) | ffprobe independently confirms the title; font/cover asserted byte-exact against the source files; vacuous-pass risk only when env lacks ffmpeg/fonts (F-002). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "full-file hand-built vectors ... parsed and retained on the demuxer at start()" | header lines 1-7 | REAL | 9/9 pass, my run |
| "ffmpeg-generated differential lane ... real attached TTF font" | header line 5-7 | REAL (this sandbox) | lane ran; font byte-exact assert passed |
| "the ASS stack's input, issue #17" (font) | line 254 | SHALLOW | no consumer; subtitles font adapter is [PROPOSAL] (subtitles/src/font/mod.rs:262) |
| "the mid-playback skip fix" / window-leak regression | line 707-708 | REAL-UNPROVEN | regression story unverifiable from this file; the pinned invariant (1 recovery, EOS) is verified |

## 7. Dead / duplicated / unreachable code
None dead in-file. The pinned getter surface is currently unreachable from shipped entry points (see F-001) — dormant API, not dead code.

## 8. Dependencies & flags
Depends on `demux::avi::test_support::mem_stream` (line 24) — cross-module reuse of the AVI test seam for MKV; works but is a layering smell (MKV tests reaching into avi::test_support). No feature gates. External tools: ffmpeg/ffprobe (env-gated).

## 9. Open questions for the next agent
- mkv/mod.rs owner: is `chapter_targets()`'s Vec allocation per call (mkv/mod.rs:276) hot anywhere? No current caller, so moot until UI wiring lands.
- Who closes issue #17 (ASS font hookup)? If it lands, F-001 downgrades.
