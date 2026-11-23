# AUDIT: crates/demux/src/mp4/test_support.rs
| Field | Value |
|---|---|
| Auditor | W05-A08 |
| Date | 2025-11-23 |
| Lines / functions | 570 / 22 fns + 2 spec structs |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (by design: nothing in the release demuxer calls it) — but it IS compiled into every release build (`pub mod test_support;` mp4/mod.rs:101, not cfg-gated). Consumers: `mp4/tests.rs:9` (same-crate #[cfg(test)]), `mp4/fuzz.rs:46` (mem_stream only), `apps/cli/src/containers/mp4.rs:637` (CLI #[cfg(test)], mem_stream only). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 4 |
| Commands run | `sed -n` quotes (exit 0, verified); `./target/debug/deps/demux-036dd705535b2fb3 mp4` → 38 passed / 0 failed (exit 0, 0.17s); `rg` consumer searches (exit 0; note: one timed-out `rg -rln` was a self-inflicted `-r`eplace flag misuse, re-run via Grep tool) |

## 1. One-paragraph verdict
The builders are real and byte-correct: box headers, mvhd/tkhd/mdhd/hdlr field layouts (counted: mvhd v0 body exactly 96 B, tkhd 80 B, mdhd 20 B, language 0x55C4="und"), two-pass stco offset patching with identical moov byte lengths, one-chunk-per-sample stsc/stsz/stts run-length, elst v0 with correct negative media_time two's complement, ISO FullBox `meta`, trun flag bits 0x0F01 with field order duration/size/flags/cts and the spec's bit-15 non-sync flag 0x8000, and per-track data_offsets relative to the moof start. The 38 unit tests that consume them round-trip through the real `Mp4Demuxer` and assert packet counts, tfdt dts ladders, and keyframe patterns — I ran them, all pass. The defects are honesty-shaped, not correctness-shaped: the module doc's justification for shipping this in release builds is false for the builders, one pub function is dead, `trak` cannot represent a zero-sync track, and `fragmented` silently fabricates filler samples.

## 2. What actually works (verified)
— verified by `./target/debug/deps/demux-036dd705535b2fb3 mp4` -> `test result: ok. 38 passed; 0 failed` (exit 0). Builder-consuming tests assert real parser output: `fragmented_demuxes_moofs_with_tfdt_ladder` (8 packets, dts 0/1024/2048/4096, keyframes at samples 0 and 4 — the builder's trun/stss/tfdt encoding is pinned by the demuxer's decode of it); `progressive`/`progressive_with_chapters`/`progressive_with_cover` shapes parse with exact track counts. Byte layouts independently re-derived by hand-count against ISO/IEC 14496-12 field tables (see per-function notes).

## 3. Findings
F-001 [S3]
Where: crates/demux/src/mp4/test_support.rs:3-6
Code:
```
//! Public like [`crate::wav::test_support`] on purpose: the
//! integration suites + fuzz harness need deterministic shapes. This
//! is test-support surface, not player surface — nothing in the
//! demuxer itself uses it.
```
What is wrong: the justification is false for the ~550 lines of builders. Integration suites do NOT use them: `tests/mp4_fixtures.rs:20` imports only `Mp4Demuxer, fuzz` and reads ffmpeg-authored files (`generated/valid/mp4_chapters.mp4` etc.); the fuzz harness uses only `mem_stream` (`fuzz.rs:46`); the only cross-crate consumer, `apps/cli/src/containers/mp4.rs:637`, also uses only `mem_stream` (inside `#[cfg(test)]`). The builders are consumed exclusively by same-crate `#[cfg(test)]` code (`mp4/tests.rs:9`), which a `#[cfg(test)] mod test_support` would serve. Why it matters: the release lane compiles dead test builders into every shipped binary on a doc claim that does not hold; the honest framing is "public for mem_stream's cross-crate test users; builders are crate-test-only". What correct looks like: gate the builders `#[cfg(test)]` (mem_stream can stay pub), or fix the doc to state the real consumers. Confidence: HIGH — Verification: rg over crates/, apps/, tools/ (zero builder consumers outside mp4/tests.rs).
F-002 [S4]
Where: crates/demux/src/mp4/test_support.rs:231-238
Code:
```
        if !syncs.is_empty() && syncs.len() < spec.samples.len() {
            let mut stss = Vec::new();
            stss.extend_from_slice(&u32b(syncs.len() as u32));
            for s in &syncs {
                stss.extend_from_slice(&u32b(*s as u32));
            }
            stbl.extend_from_slice(&fullbox(b"stss", 0, &stss));
        }
```
What is wrong: stss is omitted both when ALL samples are sync (correct: absent stss = all sync, ISO §8.6.2) and when NO sample is sync — the latter produces a file asserting every sample is a keyframe, the exact opposite of the TrackSpec. A `TrackSpec` with all `sync: false` cannot be expressed. Currently latent: no test builds an all-non-sync track (rg `sync: false` — zero hits; test sync arrays are mixed). Why it matters: a future test of non-sync seek behavior using an all-false spec would silently assert all-keyframe semantics. What correct looks like: emit an explicit empty-count stss (count=0) for the zero-sync case, or document the limitation. Confidence: HIGH — Verification: read + rg.
F-003 [S4]
Where: crates/demux/src/mp4/test_support.rs:432-438 (and 459-464)
Code:
```
                let bytes = t
                    .samples
                    .get(idx)
                    .map_or(vec![0xAA; 4], |s| s.bytes.clone());
                list.push(bytes.len() as u32);
                mdat.extend_from_slice(&bytes);
```
What is wrong: when `frags * per_frag > samples.len()`, `fragmented` fabricates 4-byte `0xAA` filler samples (dur 1024 from the trex default, non-sync, cts 0) — undocumented in the builder doc ("per-fragment trun runs (sizes, durations and flags explicit; defaults ride trex)" says nothing about synthesizing samples). No current caller hits it: both call sites (`mp4/tests.rs:233` 4×2 vs 8 samples; `:260` 2×2 vs 4+4) supply exact counts. Why it matters: a future test that miscounts silently tests fabricated media. What correct looks like: assert `frags * per_frag <= samples.len()` or document the fill behavior. Confidence: HIGH — Verification: read + call-site count check.
F-004 [S4]
Where: crates/demux/src/mp4/test_support.rs:189-199
Code:
```
        let mut stts = Vec::new();
        stts.extend_from_slice(&u32b(1)); // patched below per-run
        let mut runs: Vec<(u64, u64)> = Vec::new();
```
(then) `stts.clear();` at :198 before rebuilding. What is wrong: the initial `extend` is a dead write — the comment says "patched below per-run" but the code clears and rebuilds instead. Harmless; the comment lies about the mechanism. What correct looks like: drop the dead extend + stale comment. Confidence: HIGH — Verification: read.
F-005 [S4]
Where: crates/demux/src/mp4/test_support.rs:8-10 vs 143-150, 256
Code:
```
//! The builders emit the layouts ffmpeg's muxers produce (two-pass
//! offset patching, one-chunk-per-sample stsc, mvex/trex defaults)
//! so the fixtures exercise the same paths the corpus does.
```
vs `stsd_entry_with` (audio-shaped base for every handler, incl. `vide`/`avc1`) and `minf.extend_from_slice(&fullbox(b"vmhd", 0, &[0u8; 8]))` (vmhd even for `soun` tracks — ffmpeg writes smhd). What is wrong: the claim holds for the box-ladder strategy but not the sample entries: ffmpeg writes 78-byte VisualSampleEntry for video and smhd for audio; these builders write a 28-byte audio-shaped base and vmhd for all tracks. The function-local comment (138-141) honestly discloses the entry simplification ("the parser only needs the entry's size and kind") and the parser indeed reads no visual fields (rg width/height in mp4/*.rs — only pcmC format-size mapping at mod.rs:496). tkhd width/height are zero. Why it matters: "the fixtures exercise the same paths the corpus does" is overstated — corpus files carry real VisualSampleEntry bytes these fixtures never produce. What correct looks like: scope the module-doc claim to the ladder, or emit handler-appropriate entries. Confidence: HIGH — Verification: read + rg + hand-count of entry bytes.
F-006 [S3]
Where: crates/demux/src/mp4/test_support.rs:543-548
Code:
```
/// The `tref/chap` reference bytes for a chapter track id (kept
/// public for tests that splice their own shapes).
#[must_use]
pub fn chap_ref(track_id: u32) -> Vec<u8> {
    boxb(b"tref", &boxb(b"chap", &u32b(track_id)))
}
```
What is wrong: zero callers anywhere (rg `chap_ref(` over crates/, apps/, tools/ — only the TrackSpec field `chap_ref` is used, e.g. `mp4/tests.rs:382 audio.chap_ref = Some(2)`). The doc's "kept public for tests that splice their own shapes" describes consumers that do not exist. Dead pub fn compiled into the release lane. What correct looks like: delete it, or use it in the one test that hand-splices. Confidence: HIGH — Verification: rg (exit 0, no callers). Note: `#[must_use]` byte-verified via Read tool (sed output mangles the literal `[m` — tool artifact, see log).

## 4. Per-function table (mandatory for files >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `boxb` | 42-48 | REAL | — | `as u32` size cast wraps >4 GiB payloads — unreachable at test scale |
| `fullbox` / `fullbox_f` | 51-65 | REAL | — | correct 24-bit flag split |
| `u32b` | 67-69 | REAL | — | |
| `ftyp` | 71-80 | REAL | — | major duplicated in compat list — legal |
| `mvhd` | 82-95 | REAL | — | v0 body counted = 96 B ✓; duration `as u32` (test scale) |
| `tkhd` | 97-116 | REAL | — | v0 body 80 B ✓; flags 3 = enabled+in_movie ✓; width/height 0 |
| `mdhd` | 118-127 | REAL | — | 20 B ✓; 0x55C4 = "und" ✓ |
| `hdlr` | 129-136 | REAL | — | NUL-terminated empty name ✓ |
| `stsd_entry_with` | 143-150 | REAL | S4 | audio-shaped for every handler (F-005); channels/samplesize/rate offsets verified (8/10/16) |
| `trak` | 155-261 | REAL | S4 | stts RLE ✓, stsc 1/chunk ✓, stsz ✓, stco ✓, stss (F-002), ctts (1,cts) pairs + `as u32` two's complement ✓, elst v0 low-32 of i64 ✓, tref/chap ✓, vmhd-always (F-005) |
| `udta_cover` | 266-283 | REAL | — | ISO FullBox meta + mdir hdlr + covr/data ✓ |
| `moov_body` | 288-301 | REAL | — | mvhd(1000,0) hardcoded movie timescale |
| `moov_body_with_udta` | 304-308 | REAL | — | |
| `progressive` | 313-315 | REAL | — | delegate |
| `progressive_with_entry_children` | 323-356 | REAL | — | two-pass offsets; pass-1/pass-2 moov lengths identical (fixed 4-B fields) ✓; mdat start = len+8 ✓ |
| `progressive_with_cover` | 361-402 | REAL | — | same two-pass + constant udta ✓ |
| `fragmented` | 408-515 | REAL | S4 | trex defaults, tfhd 0x020000 default-base-is-moof ✓, tfdt v0 ✓, trun 0x0F01 field order ✓, non-sync 0x8000 (bit 15) ✓, data_offset = moof_size+8+track_start ✓, moof2/moof3 same length ✓; filler fabrication (F-003) |
| `chapter_track` | 519-541 | REAL | — | QuickTime text sample = u16 len + text ✓ |
| `chap_ref` | 546-548 | DEAD | S3 | F-006 zero callers |
| `progressive_with_chapters` | 554-560 | REAL | — | hardcoded chapter track id 2 — matches doc and progressive()'s 1..N ids |
| `mem_stream` | 566-570 | REAL | — | delegates to avi seam; consumed by fuzz.rs:46 + CLI tests |

## 5. Test honesty
This file IS test support; its own honesty = do the builders generate what they claim? Yes for box layouts (verified: 38 consumer tests pass through the real demuxer; hand-checked field offsets/lengths). Gaps: F-002/F-003 latent misexpression paths, F-006 dead helper advertised as used. The consumer tests are mutation-sensitive (packet counts, dts values, keyframe flags asserted numerically).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "integration suites + fuzz harness need deterministic shapes" (why public) | :3-4 | FALSE (for builders) | F-001: suites use ffmpeg files; fuzz uses mem_stream only |
| "nothing in the demuxer itself uses it" | :5-6 | REAL | rg: no production caller |
| "builders emit the layouts ffmpeg's muxers produce" | :8-10 | SHALLOW (ladder yes, entries no) | F-005 |
| "kept public for tests that splice their own shapes" (chap_ref) | :543-544 | FALSE | F-006: zero callers |
| "audio-shaped: the parser only needs the entry's size and kind" | :139-140 | REAL | rg: parser reads no visual fields; only pcmC size map (mod.rs:496) |

## 7. Dead / duplicated / unreachable code
`chap_ref` (F-006). The whole module is dead weight in the release lane beyond `mem_stream` (F-001). No duplication with other modules' builders (each format has its own).

## 8. Dependencies & flags
No feature gates; `pub mod test_support` unconditional (mp4/mod.rs:101). Deps: `std::sync::Arc`, `plugin_api::SourceStream`, `crate::avi::test_support` (mem_stream delegation). No unsafe/unwrap/expect/panic (house rules upheld).

## 9. Open questions for the next agent
- mp4/mod.rs owner: confirm whether the release profile strips these dead builders (LTO/gc-sections); if not, F-001 is also a (small) binary-size line item.
- Whoever owns wav/adts/ts/ps test_support twins: same false-justification pattern? (ps is my next file; wav/adts/ts are not.)
