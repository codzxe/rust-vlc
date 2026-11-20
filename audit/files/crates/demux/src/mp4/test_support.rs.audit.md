# Audit — crates/demux/src/mp4/test_support.rs

| Field | Value |
|---|---|
| Auditor | Devil-Auditor W05-A02 |
| Date | 2026-11-16 |
| Lines | 570 (verified `wc -l`) |
| Functions | 24: `boxb`, `fullbox`, `fullbox_f`, `u32b`, `ftyp`, `mvhd`, `tkhd`, `mdhd`, `hdlr`, `stsd_entry_with`, `trak`, `udta_cover`, `moov_body`, `moov_body_with_udta`, `progressive`, `progressive_with_entry_children`, `progressive_with_cover`, `fragmented`, `chapter_track`, `chap_ref`, `progressive_with_chapters`, `mem_stream` (pub: 8) + 2 data structs (`SampleSpec`, `TrackSpec`) |
| File verdict | **REAL** — a genuine byte-level MP4/MOV muxer emulation that feeds the real parser through the real `SourceStream` seam; no fake in-memory structures, no shared logic with the demuxer |
| Reachable | YES — `pub mod test_support;` (mod.rs:101, NOT `#[cfg(test)]`-gated: compiles into the production library by design); callers: tests.rs (38 tests), fuzz.rs:46 (via `mem_stream`), fuzz/fuzz_targets/fuzz_demux_mp4.rs:19 (via fuzz harness), apps/cli/src/containers/mp4.rs:637 (inside a `#[test]`) |
| Findings | S0:0 S1:0 S2:0 **S3:1** **S4:4** (5 total) |
| Commands run | `wc -l`; full `Read`; `sed -n '1,10p'`, `'188,199p'`, `'562,570p'`, `'543,548p'`, `'413,422p'` (quote verification); `rg -n "chap_ref\("` repo-wide (dead-fn proof); `rg -n "test_support"` crates/demux/tests/ (consumer map); `sed -n '100,126p' crates/demux/src/avi/test_support.rs` (seam verification) |

## 1 One-paragraph verdict

This is a REAL byte-stream builder, the opposite of a parser-bypassing mock: it emits conformant ISO-BMFF bytes (plain/full boxes with correct big-endian size+kind headers, `ftyp`/`moov`(`mvhd`/`tkhd`/`mdhd`/`hdlr`/`minf`/`stbl` with `stts`/`stsc`/`stsz`/`stco`/`stss`/`ctts`/`edts`/`elst`/`tref`/`chap`), fragmented shapes (`moof`/`mfhd`/`traf`/`tfhd`/`tfdt`/`trun` + `mvex`/`trex` with two-pass `data_offset` patching), and the iTunes cover ladder (`udta/meta/ilst/covr/data`) — with ffmpeg-style two-pass `stco` offset patching so the parser's offset-following paths are genuinely exercised. The demuxer consumes these bytes through the real `SourceStream` trait (avi `BytesStream`, avi/test_support.rs:121-126), so nothing bypasses the parser. Critically for the circularity question: expected values in tests.rs are hand-computed literals; this builder shares ZERO functions with the production parser (`rg` for `test_support` in mod.rs: only the module declaration). Defects are hygiene, not honesty: ~60 lines of copy-pasted two-pass offset code across two builders, a double-built `moof` (first draft discarded after measuring its length), three vestigial writes, one dead public function (`chap_ref` — zero callers repo-wide), and a header doc whose consumer list ("integration suites") does not match the actual consumers.

## 2 What actually works (verified)

- **Real bytes, real parsing**: `mem_stream` (570-line file's 566-570) delegates to `crate::avi::test_support::mem_stream` → `BytesStream::new(bytes)` boxed as `Box<dyn SourceStream>` (avi/test_support.rs:121-126). The demuxer's `Mp4Demuxer::new(stream)` parses these bytes through the same read path as file input.
- **Box emission is spec-shaped**: `boxb` writes `size(4)+kind(4)+payload` (42-48); `fullbox` prepends version/flags (51-65); `ftyp` writes major brand + version field + compat brands (71-80); `tkhd` sets flags `0x0…3` (enabled|in_movie — 111-115) with the correct comment about disabled tracks.
- **Two-pass stco patching mirrors ffmpeg muxers** (328-347): pass 1 with zero offsets measures the moov length, pass 2 rewrites with real `mdat` offsets — exercising the parser's offset-following, not hand-fed positions.
- **Sample tables are complete and distinct-valued**: stts run-length encodes distinct deltas (189-204), stsz carries per-sample sizes (211-217), stco carries per-sample offsets, stss is omitted when all/none sync (231) — so the parser's "absent stss ⇒ all sync" default is genuinely testable.
- **ctts uses per-sample runs** (239-247), exercising v0 signed-offset parsing.
- **The elst encoding is spec-correct even where the parser is not**: `media_time.to_be_bytes()[4..8]` (172) writes the low 32 bits of the i64 — the correct v0 signed-i32 encoding; seg_dur is written in movie-timescale units per the TrackSpec doc (34-35). The unit bug is downstream (see tests.rs F-001), not here.
- **Fragmented layout is real**: `tfhd` flags `0x020_000` (default-base-is-moof), `tfdt` v0 bases, `trun` flags `0x000_0F01` (data-offset+duration+size+flags+cts all present, 466/502), `tfdt` ladder accumulated across fragments (507-512) — the parser's moof walk, trun field parsing and default-base arithmetic are all exercised.
- **Cover ladder**: `udta_cover` (266-283) builds the exact `udta/meta(FullBox)/ilst/covr/data` chain with `wkt` + locale — matching the parser's `cover_in_meta` both-offsets-tries shape (mod.rs:1918-1927).

## 3 Findings

### T-001 [S3] Copy-pasted two-pass offset blocks + a fully discarded first moof draft
Where: `crates/demux/src/mp4/test_support.rs:328-355` (`progressive_with_entry_children`) vs `368-401` (`progressive_with_cover`); `446-468` vs `481-504` (`fragmented`'s moof2/moof3)
Code (446-449 + 481-484, the duplicated build):
```rust
        let mut moof2 = Vec::new();
        moof2.extend_from_slice(&fullbox(b"mfhd", 0, &u32b(f as u32 + 1)));
        for (ti, t) in tracks.iter().enumerate() {
```
…then at 481:
```rust
        let mut moof3 = Vec::new();
        moof3.extend_from_slice(&fullbox(b"mfhd", 0, &u32b(f as u32 + 1)));
        for (ti, t) in tracks.iter().enumerate() {
```
What's wrong: `fragmented` builds the ENTIRE moof twice — `moof2` (with a placeholder `data_offset`) is wrapped and used only for `moof_box.len()` (469, 484), then `moof3` rebuilds every traf/trun with real offsets. The traf/trun emission blocks (454-467 and 490-503) are near-identical 14-line copies. Likewise the 28-line two-pass offset patching in `progressive_with_entry_children` (328-347 + 345-354) is repeated verbatim inside `progressive_with_cover` (368-393). Any future box-layout change must be edited in 2 (or 4) places; a divergence silently produces fixtures that test a different shape than intended.
Why it matters: fixture-builder divergence is a test-integrity hazard — the suite would keep passing while the fixture stopped representing the intended format shape. Also ~60 wasted lines per call are O(file) clones, though test-only.
What correct looks like: extract `fn mdat_offsets(tracks, start) -> Vec<Vec<u64>>` and build the traf/trun once with a placeholder, patching the 4-byte `data_offset` field in place (the same two-pass trick `progressive` already uses for stco — patch, not rebuild).
Confidence: HIGH (line-identical blocks compared from the full Read).

### T-002 [S4] Vestigial writes: the stts placeholder, `let _ = t;`
Where: `crates/demux/src/mp4/test_support.rs:189-199` and `421`
Code (verified `sed -n '188,199p'`):
```rust
        // stts: one run per distinct delta.
        let mut stts = Vec::new();
        stts.extend_from_slice(&u32b(1)); // patched below per-run
        let mut runs: Vec<(u64, u64)> = Vec::new();
        for s in &spec.samples {
            match runs.last_mut() {
                Some((count, delta)) if *delta == s.dur => *count += 1,
                _ => runs.push((1, s.dur)),
            }
        }
        stts.clear();
```
Code (421, verified `sed -n '413,422p'`):
```rust
        mvex.extend_from_slice(&fullbox(b"trex", 0, &trex));
        let _ = t;
    }
```
What's wrong: the `stts.extend(&u32b(1))` write is dead (cleared two lines later — the comment "patched below per-run" describes a patch that happens by full rebuild, not patch); `let _ = t;` exists only to silence an unused-variable warning inside a loop whose iterator binding `t` is otherwise unused (trex fields come from `i`, not `t`).
What correct looks like: delete both; bind `for (i, _t) in tracks.iter().enumerate()`.
Confidence: HIGH.

### T-003 [S4] Dead public function `chap_ref` — zero callers repo-wide
Where: `crates/demux/src/mp4/test_support.rs:543-548`
Code (verified `sed -n '543,548p'`):
```rust
/// The `tref/chap` reference bytes for a chapter track id (kept
/// public for tests that splice their own shapes).
#[must_use]
pub fn chap_ref(track_id: u32) -> Vec<u8> {
    boxb(b"tref", &boxb(b"chap", &u32b(track_id)))
}
```
What's wrong: `rg -n "chap_ref\("` over crates/, apps/, tools/ finds ZERO callers (tests set the `TrackSpec.chap_ref` FIELD, never this function; the doc's promised "tests that splice their own shapes" were never written). Because the module is not `#[cfg(test)]`-gated, this dead function ships in the production library's public API.
What correct looks like: delete it, or gate the whole module `#[cfg(any(test, feature = "test-fixture"))]`-style so dead test surface can't leak into release builds.
Confidence: HIGH (repo-wide grep; the only hits are the definition itself and the field of the same name).

### T-004 [S4] Header consumer claim does not match actual consumers
Where: `crates/demux/src/mp4/test_support.rs:3-6`
Code (verified `sed -n '1,10p'`):
```rust
//! In-crate byte builders for the MP4 tests (card 5.C1).
//!
//! Public like [`crate::wav::test_support`] on purpose: the
//! integration suites + fuzz harness need deterministic shapes. This
//! is test-support surface, not player surface — nothing in the
//! demuxer itself uses it.
```
What's wrong: the actual consumers are (a) tests.rs (in-crate unit), (b) fuzz.rs → fuzz target, (c) `apps/cli/src/containers/mp4.rs:637` — a `#[test]` inside the CLI app crate. The demux crate's OWN integration suites (`corpus_mp4.rs`, `mp4_fixtures.rs`) do NOT import `mp4::test_support` (they open committed fixture files from disk; `rg test_support crates/demux/tests/` hits only avi/ivf/mkv suites). "Nothing in the demuxer itself uses it" is TRUE (verified — mod.rs has no test_support reference beyond the `pub mod` declaration), but "the integration suites" is imprecise, and the CLI app (a shipped binary crate) does compile this surface in.
Why it matters: the module is compiled into every non-test build of the demux library and reachable from apps — that is player-surface code by linkage, contradicting "not player surface" as written.
What correct looks like: correct the doc ("unit tests, the fuzz harness, and app-level tests") and cfg-gate the module for release builds.
Confidence: HIGH (consumer list grep-verified).

### T-005 [S4] `fragmented` synthesizes filler samples past the spec's supply without any test covering that lane
Where: `crates/demux/src/mp4/test_support.rs:434-437` (`map_or(vec![0xAA; 4], ...)`), 460, 496, 510 (dur default 1024)
Code (434-437):
```rust
                let bytes = t
                    .samples
                    .get(idx)
                    .map_or(vec![0xAA; 4], |s| s.bytes.clone());
```
What's wrong: when `frags * per_frag` exceeds the sample count, the builder fabricates 4-byte `0xAA` samples with trex-default durations — a synthetic lane NO test exercises (both fragmented tests supply exact counts: 8=4×2, 8=2×4). The behavior is undocumented in the module header and untested; a future test relying on it would inherit an unstated fixture semantic.
What correct looks like: either assert supply == demand in the builder (`assert!(tracks.iter().all(|t| t.samples.len() >= frags * per_frag))`) or document the filler lane in the module docs.
Confidence: HIGH on existence; the no-coverage claim verified by reading both fragmented tests (tests.rs:226-271).

## 4 Per-function/per-test table

This file contains no tests. Function-level honesty table (builders only; every consumer is a real-parser test):

| Function | Lines | Feeds real parser? | Notes |
|---|---|---|---|
| `boxb` / `fullbox` / `fullbox_f` / `u32b` | 42-69 | indirect | spec-correct headers |
| `ftyp` | 71-80 | YES (probe + brand walk) | writes 5 compat brands incl. major twice — ffmpeg-ish |
| `mvhd` | 82-95 | YES (movie_timescale parse) | duration u32 cast — fine for fixture sizes |
| `tkhd` | 97-116 | YES (track_id + flags) | flags=3 enabled|in_movie, correct |
| `mdhd` | 118-127 | YES (timescale/duration/lang) | `0x55C4` "und" language, correct |
| `hdlr` | 129-136 | YES (handler fourcc) | correct |
| `stsd_entry_with` | 143-150 | YES (entry size/kind + pcmC children) | 28-byte v0 audio base |
| `trak` | 155-261 | YES (whole trak walk) | T-002 vestige at 189-199 |
| `udta_cover` | 266-283 | YES (cover_in_meta) | exact iTunes ladder |
| `moov_body` / `moov_body_with_udta` | 288-308 | YES | |
| `progressive` | 313-315 | YES | delegates |
| `progressive_with_entry_children` | 323-356 | YES (pcmC children) | duplicated pass-2 (T-001) |
| `progressive_with_cover` | 361-402 | YES | duplicated pass-2 (T-001) |
| `fragmented` | 408-515 | YES (moof/trun/tfdt walk) | T-001 double moof; T-005 filler |
| `chapter_track` | 519-541 | YES (text trak + titles) | QT text sample: u16 len + utf8, correct |
| `chap_ref` | 546-548 | — | DEAD (T-003) |
| `progressive_with_chapters` | 554-560 | YES | clones specs, sets chap_ref=2 |
| `mem_stream` | 566-570 | YES (the seam itself) | delegates to avi BytesStream |

## 5 Doc claims touching this file

- test_support.rs:3-6 header (quoted in T-004): "the integration suites + fuzz harness need deterministic shapes … nothing in the demuxer itself uses it." VERDICT: HALF-TRUE — "nothing in the demuxer itself" verified TRUE; "integration suites" FALSE for the demux crate's own integration suites (they don't import it); the surface IS compiled into shipped library+app binaries.
- `docs/formats/container-mp4.md:61` — `"Fixtures: 7 valid + 14 malformed derivatives through fuzz + typed passes"`. VERDICT: IMPRECISE — the committed tree has 8 valid mp4-family parents in tests/fixtures/generated/valid (mp4_3gp_h263.3gp, mp4_chapters.mp4, mp4_edts.mp4, mp4_frag.mp4, mp4_frag_sidx.mp4, mp4_h264_aac.mp4, mp4_mjpeg_pcm.mp4, mp4_qt.mov) and 24 malformed derivatives in tests/fixtures/generated/malformed (21 `.mp4` + 2 `.3gp` + 1 `.mov`, counted via `ls | rg`). The claimed 14 matches neither count; stale or hand-waved. The suites themselves (mp4_fixtures.rs) are real.
- `docs/feasibility/sections/subsystems.md:124` — "`fuzz_demux_mp4` seed corpus = these fixtures". VERDICT: PARTIAL — fuzz/corpus/fuzz_demux_mp4 holds 4 seeds (seed_mp4_chapters.mp4, seed_mp4_frag.mp4, seed_mp4_h264_aac.mp4, seed_mp4_trunc0.mp4; verified `ls`) vs the 8 committed valid fixtures; trunc0 is a derivative, not a parent. Coverage directionally true, count-wise partial.
- `docs/formats/container-mp4.md:24` — "`EXTENSIONS = [...]` (mp4/mod.rs:138, verified this audit)". VERDICT: consistent with mod.rs:138-140 (path-existence level; the "verified this audit" refers to a prior in-repo audit, not this one).

## 6 Dead/duplicated code

- `chap_ref` (546-548): DEAD, zero callers (T-003).
- stts placeholder write (190): DEAD vestige (T-002).
- `let _ = t;` (421): vestige (T-002).
- moof2 draft (446-478): built and discarded except `.len()` (T-001).
- Two-pass offset block duplicated between `progressive_with_entry_children` and `progressive_with_cover` (T-001).
- No shared code with the parser exists (the circularity check): `rg "test_support"` against mod.rs returns only the `pub mod` declaration — verified.

## 7 Open questions

1. Why is the module compiled into release builds instead of `#[cfg(test)]` + a test-only feature? The stated reason (doc-comment: examples/fuzz need it) is real for the fuzz target, but the fuzz target is itself never run (see fuzz.rs audit) — the cost (test surface in the shipped library) is currently paid for an unused benefit.
2. Were the fuzz corpus seeds (4 files, 2026-11-16 16:19 timestamps) generated by this builder or copied from tests/fixtures/generated? `seed_mp4_h264_aac.mp4` (13 703 B) differs in size from `tests/fixtures/generated/valid/mp4_h264_aac.mp4`; provenance UNVERIFIED (no generator script found in fuzz/).
