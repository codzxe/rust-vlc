# AUDIT: crates/codecs/src/audio/mp3/mod.rs
| Field | Value |
|---|---|
| Auditor | W07-A01 |
| Date | 2026-11-21 |
| Lines / functions | 121 / 3 (1 probe fn + 2 tests; plus module docs, 7 module decls, 5 re-export groups) |
| File verdict | REAL (module root; the probe fn itself is UNREACHABLE dead code — F-001) |
| Reachable from shipped entry point? | YES for the re-exported surface (call chain: `lumen open x.mp3` → apps/cli/src/containers/mod.rs `"mp3" => es::open_mpeg_audio(...)` → demux `RawMp3Demuxer` → pipeline_host.rs:261-263 `"mp3" \| "mp1"` arm → `Mp3Decoder`; `mp2`/`mp1` same lane). `is_mp3_stream` itself: NO — zero non-test callers (rg over crates+apps+fuzz: only mod.rs:80 and its own tests). Executed: baseline test-workspace.log.txt:439-440 (both in-file tests `ok`), mp3 family green `:409-445`, differential suites ran `:1159-1175`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 0 |
| Commands run | `rg -n "is_mp3_stream" crates apps fuzz --type rust` (no callers outside tests); `rg -n "mp3/decoder.rs\|mp3/header.rs" audit/waves/*.tsv` (submodule ownership: decoder.rs A04, header.rs A06, xing.rs A10, id3.rs A09, seek.rs A07); `sed -n '80,140p' apps/cli/src/containers/es.rs` (the real mp3 open path uses `RawMp3Demuxer`, not this probe); `rg -n "codecs" crates/demux/Cargo.toml` (no dependency — demux cannot call this crate); `rg -n "BITRATES\|table_bitrate" crates/demux/src/es/mp3.rs crates/codecs/src/audio/mp3/header.rs` (verbatim table duplication proven); Read on the full file. |

## 1. One-paragraph verdict
The module root is an honest map — its submodule docs, re-exports, and two tests all check out (the synthetic-frame test pins the exact decode shape 1 frame/2ch/44.1k/1152×2, and the probe test pins 6 accept/reject cases; both green in baseline) — but it carries two real structural defects. First, `is_mp3_stream` is dead code: the doc says "the future MP3 demuxer / registry row will call" it, but that demuxer already exists and ships (`demux/src/es/mp3.rs::RawMp3Demuxer` behind the containers "mp3" arm) and cannot call it (the demux crate does not and may not depend on codecs) — the "future" arrived and bypassed this probe. Second, precisely because of that layering, the repo contains TWO verbatim copies of the MPEG-audio header tables and parse (`codecs/…/mp3/header.rs:118-143` and `demux/src/es/mp3.rs:126-180` — `BITRATES_V2_L23` and `table_bitrate` byte-identical), while this module's doc claims sole ownership of the "raw MPEG-audio elementary stream" format slice — a drift hazard directly on the shipped MP3 lane.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/test-workspace.log.txt:439-440` → `audio::mp3::tests::{a_minimal_synthetic_stream_decodes_end_to_end, probe_accepts_id3_and_frame_headers_only}` `ok`; the full mp3 family `:409-445` green; `tests/mp3_bitexact.rs` + `tests/mp3_corpus_differential.rs` + `tests/mp3_fuzz_survival.rs` executed in the same green run (`:1159-1175`).
— Verified by arithmetic (read): the synthetic frame `[0xFF,0xFB,0x90,0x00] + 413 zeros` is a correctly constructed MPEG-1 Layer III 128 kbps/44.1 kHz stereo frame — 144×128000/44100 = 417 bytes — so the test's asserted shape (1 frame, 2 ch, 44 100 Hz, 1152×2 samples) exercises real header math, not a fixture shortcut.
— Verified by `rg`: every re-export listed (66-74) exists in its submodule and the `pub mod fuzz` visibility is justified (fuzz/fuzz_targets/fuzz_codec_mp3.rs:19 and tests/mp3_fuzz_survival.rs:14 call into the crate).

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/audio/mp3/mod.rs:76-85
Code:
```
/// Whether `data` begins like a raw MPEG-audio stream: an ID3v2 tag
/// or a valid frame header at offset 0 (the probe the future MP3
/// demuxer / registry row will call; doc 04 §2.2 "MP3 raw stream").
#[must_use]
pub fn is_mp3_stream(data: &[u8]) -> bool {
    if data.get(..3) == Some(b"ID3") {
        return true;
    }
    parse_frame_header(data).is_ok()
}
```
What is wrong: The function is dead in the shipped tree — zero non-test callers — and its justification is stale: the "future MP3 demuxer" already exists and ships (`crates/demux/src/es/mp3.rs::RawMp3Demuxer`, wired through `apps/cli/src/containers/mod.rs` "mp3" arm → `es::open_mpeg_audio`), but it CANNOT call this probe: `crates/demux/Cargo.toml` has no `codecs` dependency (layering — demux L2 sits below codecs L3), so the demuxer carries its own probe logic (`demux.es.no-sync` scan). The shipped MP3 lane never runs `is_mp3_stream`.
Why it matters: Dead `pub` API advertised as the lane's probe is false inventory: a reader auditing MP3-format sniffing will read this function and believe it is in the path (the doc says the demuxer "will call" it); in reality the shipped path's probe lives in another crate entirely and this one influences nothing.
What correct looks like: Either delete the probe (the demuxer owns sniffing now), or move format sniffing into a layer both crates can share (with F-002's tables) and update the doc to name the real caller.
Confidence: HIGH — Verification: read + rg (callers enumerated; demux Cargo.toml checked).

### F-002 [S3]
Where: crates/codecs/src/audio/mp3/mod.rs:5-17 (with crates/demux/src/es/mp3.rs:126-180 and crates/codecs/src/audio/mp3/header.rs:118-143)
Code:
```
//! **The raw MPEG-audio elementary stream**: the ID3v2.2/2.3/2.4
//! tag ([`id3`]), the ISO 11172-3/13818-3 frame headers — sync,
//! version/layer, bitrate/samplerate tables, channel modes, mode
```
What is wrong: The module claims ownership of the raw MPEG-audio format slice, but the repository carries TWO independent implementations of it: this crate's `header.rs` (BITRATES_V1_L1/L2/L3, BITRATES_V2_L1/V2_L23, `table_bitrate`, `parse_frame_header`) and the demux crate's `es/mp3.rs` (the same const names, byte-identical tables — `BITRATES_V2_L23: [u32; 14] = [8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160]` appears verbatim at header.rs:131 and es/mp3.rs:138 — plus its own `table_bitrate`, `table_sample_rate`, `parse_head`). The duplication is forced by the layering boundary (demux cannot depend on codecs), but nothing documents that split here, and the two copies can drift silently.
Why it matters: A table or geometry fix applied to one copy (e.g., an MPEG-2.5 edge case in `samples_per_frame`) will not reach the other; the demuxer would then mint track timing/geometry from stale math while the decoder decodes with corrected math — a real desync class on the shipped MP3/MP2 lane (the demuxer's harvest feeds `AudioPipeline`'s negotiated rate).
What correct looks like: Extract the ISO tables + header parse into the lowest shared layer (media-types or a tiny shared crate both may depend on), or at minimum document in BOTH files that they are parallel copies of the same spec tables with a cross-reference and a shared conformance test.
Confidence: HIGH — Verification: read + rg/sed (tables compared byte-for-byte; both files read).
Note: header.rs itself is W07-A06's assignment and es/mp3.rs belongs to the demux waves — this finding is filed here because mod.rs's ownership claim is the contradicted doc.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs + decls + re-exports | 1-74 | REAL | F-002 | Map verified against submodules; `pub mod fuzz` visibility justified by external callers (verified). |
| `is_mp3_stream` | 79-85 | UNREACHABLE (dead) | F-001 | Total, no panic (`get(..3)` + typed parse); 3-byte "ID3" prefix accepts any text starting "ID3" — weak but documented as "begins like"; irrelevant while dead. |
| test `probe_accepts_id3_and_frame_headers_only` | 91-100 | REAL | — | 6 cases incl. the mislabeled-FLAC corpus guard — would fail if the probe widened to `fLaC`/RIFF. |
| test `a_minimal_synthetic_stream_decodes_end_to_end` | 102-120 | REAL | — | Hand-built valid frame (417 B = 144×128000/44100, verified); asserts frames=1, 2ch, 44.1 kHz, 1152×2 — would fail on any geometry or gapless regression; `Err => unreachable!` makes an engine refusal a hard failure (brittle-by-design, pinned green). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| probe_accepts_id3_and_frame_headers_only (91) | YES | Direct true/false matrix on the probe. |
| a_minimal_synthetic_stream_decodes_end_to_end (102) | YES | Exact shape assertions through the real `decode_stream` pipeline (walk → decode → gapless). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The raw MPEG-audio elementary stream: … the ISO 11172-3/13818-3 frame headers … ([`header`])" — module owns the slice | module docs 5-17 | PARTLY FALSE | The demux crate carries a parallel verbatim copy (F-002) — ownership is split, undocumented. |
| "the probe the future MP3 demuxer / registry row will call" | 76-78 | FALSE | The demuxer exists, ships, and cannot call it (layering) — F-001. |
| "no panic on any input — every decode/parse/seek path is bounds-checked" | module docs 37-42 | REAL-UNPROVEN (here) | Submodule-level claim; header/id3/xing/seek/decoder are other agents' files; the in-file probe and tests are panic-free (read); fuzz survival green in baseline. |
| "The differential compares against ffmpeg on the same gapless window … records the PCM MD5 plus the max sample delta" | module docs 44-53 | REAL | `tests/mp3_bitexact.rs` + `tests/mp3_corpus_differential.rs` exist and ran green (baseline :1159-1175); the empirical ffmpeg claim (`frames × spf − delay − padding`) is the suites' premise — executed, not re-derived by me. |

## 7. Dead / duplicated / unreachable code
- `is_mp3_stream`: dead pub fn (F-001).
- Table/parse duplication codecs↔demux (F-002) — the structural item.
- No other dead items; all re-exports have consumers (pipeline_host, mp2 module, tests, fuzz).

## 8. Dependencies & flags
Module rides `codec-mp3` (default-on; `codec-mp2` implies it — Cargo.toml:77). Engine `symphonia-bundle-mp3 =0.6.1` used only in `decoder.rs`. No unsafe in this file. The demux crate's parallel copy is the boundary consequence noted in F-002.

## 9. Open questions for the next agent
- W07-A06 (header.rs) + demux-wave owner (es/mp3.rs): decide the F-002 consolidation owner; a shared conformance test asserting both table sets stay identical would be the minimal guard.
- W07-A04 (decoder.rs): the gapless window (delay/padding trim) is this module's headline feature — verify the trim math and the Xing frame-count inconsistency typed refusal (`xing_frame_count_inconsistency_is_typed` green, baseline :415).
