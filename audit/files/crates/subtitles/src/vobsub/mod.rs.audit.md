# AUDIT: crates/subtitles/src/vobsub/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A10 |
| Date | 2026-11-23 |
| Lines / functions | 616 / 13 (7 prod incl. 1 Display + 6 fns, 2 test helpers, 5 tests) |
| File verdict | UNREACHABLE (real, unit-tested pair-decode code; NO shipped entry point depends on crates/subtitles) |
| Reachable from shipped entry point? | NO — grep of every workspace Cargo.toml: only `fuzz/Cargo.toml:50` depends on `subtitles`; apps/cli, apps/desktop, apps/desktop-tauri, apps/web and every crates/* Cargo.toml have zero references; crates/playback-engine contains no "subtitle" token at all |
| Findings | S0: 0 S1: 1 S2: 2 S3: 2 S4: 0 |
| Commands run | Grep `subtitles` in **/*.toml (only fuzz/Cargo.toml:50); grep apps/*/Cargo.toml (no match, exit 1); Grep `subtitle` in crates/playback-engine/src (no match); sed quote verifications (exit 0); `cargo test -p subtitles vobsub::tests` BLOCKED (cargo not on PATH in sandbox) — tests cited from audit/runs/baseline/cargo-test-workspace.txt |

## 1. One-paragraph verdict
The IDX/SUB pair decode is genuine, carefully written reference-derived code (palette application, grayscale fallback, transparent-border crop, damage ledger, half-open spans) with honest, strong unit tests — and none of it is reachable from any shipped binary, because nothing in the workspace depends on the `subtitles` crate except the fuzz harness. The module's two hard contracts are both breakable by hostile input: "never panics" fails on unchecked i64 timestamp arithmetic (`start_ms * 1_000_000` overflows from an idx `hh` field that idx.rs parses as unbounded i64), and "no unbounded allocation" fails because 12-bit SPU dimensions allocate up to 67 MiB of RGBA per cue from a ~3 KB RLE payload, with an unbounded cue count (duplicate `id: X, index: 0` stream declarations re-decode the same fragment N times).

## 2. What actually works (verified)
— Baseline `cargo test -p subtitles` (audit/runs/baseline/cargo-test-workspace.txt:5735-6171): all 5 `vobsub::tests::*` pass (`pair_decode_end_to_end`, `multi_pes_accumulation`, `bad_filepos_and_missing_palette_are_typed`, `grayscale_fallback_matches_reference_shape`, `cue_contract_render`), lib total "267 passed; 0 failed", plus `vobsub_fuzz_survival_mutations_and_truncations ... ok` in tests/dvb_vobsub_fuzz_survival.rs. Local re-run blocked: `cargo` not on PATH in this sandbox (not a repo defect).

## 3. Findings
F-001 [S2]
Where: crates/subtitles/src/vobsub/mod.rs:1-3 and 129-131
Code:
```
//! VobSub (IDX/SUB) — the DVD sub-picture subtitle pair (card 5.N2,
//! ADR-012 self-build; carriage already landed: the PS demuxer
//! routes `0xBD` substream `0x20..=0x3F` as `"dvd_subtitle"`).
```
What is wrong: "carriage already landed" is tag-only. crates/demux/src/ps/mod.rs:171 labels the stream `"dvd_subtitle"`, but no crate routes those bytes to `parse_vobsub`: the only consumer of `subtitles` in the whole workspace is `fuzz/Cargo.toml:50`. Grep of apps/cli, apps/desktop, apps/desktop-tauri, apps/web Cargo.tomls: zero matches; crates/playback-engine/src contains no "subtitle" token. The doc header implies the decode is wired into playback; it is not. Why it matters: docs/feature-matrix.toml:1042 claims VobSub `status = { windows = "implemented", linux = "implemented", macos = "implemented" }` while no shipped binary can decode a single VobSub cue. What correct looks like: either a playback-engine subtitle-track path calling `vobsub_cues`, or the header/matrix saying "library only, unwired". Confidence: HIGH — Verification: read (greps above, exit codes 0/1).

F-002 [S2]
Where: crates/subtitles/src/vobsub/mod.rs:262-268
Code:
```
    let start_ms = entry.timestamp_ms + spu.start_ms;
    let end_ms = spu.end_ms.map(|e| entry.timestamp_ms + e);
    Some(VobSubCue {
        stream_index: 0,
        lang: String::new(),
        start_ns: start_ms * 1_000_000,
        end_ns: end_ms.map(|ms| ms * 1_000_000),
```
What is wrong: unchecked i64 arithmetic on file-derived values. idx.rs:200-206 parses `hh` as an unbounded `i64` (`timestamp: 10000000:00:00:000` → `timestamp_ms = 3.6e13`); `start_ms * 1_000_000 = 3.6e19 > i64::MAX (9.22e18)` overflows: panic in debug builds (where `cargo test`/fuzz run), silent wrap to garbage/negative ns in release. This directly falsifies the module contract at lines 23-24: "It never panics; every malformation ... is typed data." What correct looks like: `checked_mul`/`saturating_mul` with a `VobSubDamage::TimestampOverflow` variant. Confidence: HIGH — Verification: read + arithmetic verified (python3: 10_000_000*3600*1000*1_000_000 > 2**63-1 → True); not executed against the binary (cargo unavailable).

F-003 [S1]
Where: crates/subtitles/src/vobsub/mod.rs:257-261
Code:
```
    let mut rgba = Vec::with_capacity(w * h * 4);
    for &index in &indices {
        let c = rgba_palette[index as usize];
        rgba.extend_from_slice(&[c.r, c.g, c.b, c.a]);
    }
```
What is wrong: unbounded output amplification from header-declared dimensions. spu.rs:222-224 derives `w = x2-x1+1`, `h = y2-y1+1` from 12-bit fields (≤4096 each), allocates `vec![0u8; w*h]` (≤16.7M) BEFORE any RLE validation, and spu.rs:340+ `decode_run` treats a nibble value <4 as "fill rest of line" — so a ~2-4 KB SPU legitimately fills a 4096×4096 grid. This line then converts it to up to 67,108,864 bytes of RGBA per cue. The 128 KiB cap (mod.rs:167) bounds only the INPUT accumulation, not the decoded output. Cue count is unbounded: an idx with N `id: X, index: 0` stream declarations all pointing `filepos: 0` re-decodes the same fragment into N cues (mod.rs:158-165 matches on `want = 0x20 + stream.index` for every stream). A ~4 KB hostile idx+sub pair demands N×67 MiB (1000 streams ≈ 67 GiB) → OOM. The doc comment (lines 39-40, "unbounded allocation (SPU accumulation capped at 128 KiB)") states the cap as if it bounded allocation overall; it does not. Why it matters: this is exactly the "unbounded alloc from header" class; today the blast radius is the fuzz harness and `cargo test` (no shipped consumer), which is the only reason this is not S0. What correct looks like: a decoded-area cap (e.g., reject w*h > k×input-size or a fixed pixel budget) recorded as `VobSubDamage::Spu{..}`. Confidence: HIGH — Verification: read (dimension math from spu.rs:222-224, fill-line rule at spu.rs:343-346 quoted in analysis); OOM not executed.

F-004 [S3]
Where: crates/subtitles/src/vobsub/mod.rs:161-171
Code:
```
            let next_bound = bounds.iter().find(|&&fp| fp > entry.filepos).copied();
            let span_end = next_bound.unwrap_or(u64::MAX);
            let mut acc: Vec<u8> = Vec::new();
            for (ss, offset, _pts, payload) in &fragments {
                if *ss == want && (*offset as u64) >= entry.filepos && (*offset as u64) < span_end {
                    acc.extend_from_slice(payload);
```
What is wrong: O(entries × fragments) — the fragment list is re-scanned from index 0 for every entry, and `bounds.iter().find` is O(entries) per entry, so a full-movie pair (~2-5k entries × ~2-5k fragments) does millions of comparisons plus a fresh `Vec::new()` per entry. Not per-frame, parse-time only, so S3 not S2. What correct looks like: sort fragments once by offset and advance a cursor per entry (both lists are already ordered). Confidence: HIGH — Verification: read.

F-005 [S3]
Where: crates/subtitles/src/vobsub/mod.rs:391-394
Code:
```
        let u = Rational::new(u32::from(cue.x), ew).unwrap_or(Rational::ZERO);
        let v = Rational::new(u32::from(cue.y), eh).unwrap_or(Rational::ZERO);
        let layout = CueLayout {
            anchor: Anchor::new(u, v).unwrap_or(Anchor::BOTTOM_CENTER),
```
What is wrong: `Anchor::new` rejects fractions outside [0,1] (compositing/layout.rs:96-97, `CueError::AnchorOutOfRange`); an SPU placed outside the idx canvas (x/y up to 4095 from 12-bit fields vs. default 720×480 at line 383) silently teleports the cue to `BOTTOM_CENTER` instead of recording typed damage — the exact class of silent misplacement the damage-ledger design exists to prevent. Note also `Rational::new` can only fail on `den == 0` and `ew/eh` are `.max(1)`, so the `unwrap_or(Rational::ZERO)` arms are dead code. What correct looks like: map out-of-range anchors to a `VobSubDamage`/`CueError` entry, drop the dead fallbacks. Confidence: HIGH — Verification: read (Anchor::new semantics verified in compositing/layout.rs:86-97).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Display for VobSubDamage` | 73-88 | REAL | — | Plain formatting; unreachable from shipped app like everything here. |
| `VobSubDoc::is_clean` | 122-127 | REAL | — | damage.is_empty(). |
| `parse_vobsub` | 129-209 | REAL-UNPROVEN | S2/S1 | Pair walk works (baseline tests); O(n²) scan (F-004); overflow (F-002) + amplification (F-003) live in its callees' outputs. |
| `render_cue` | 213-276 | REAL | S2 | Palette/alpha math matches reference (`a: spu.alpha[i].saturating_mul(17)`); F-002 overflow at 262-268. |
| `guess_palette` | 281-334 | REAL | — | Mirrors ffmpeg guess_palette shape; `j.min(levels.len()-1)` guard; test asserts white level. |
| `crop_borders` | 339-374 | REAL | — | Border crop correct incl. w=1/last-column edge cases (x1 loop bound `w-1` is safe because an all-transparent bitmap returns None at 353-355); O(w*h) scans. |
| `vobsub_cues` | 382-425 | REAL | S3 | Contract reduce works (baseline `cue_contract_render`); F-005 anchor teleport; `z.saturating_add(i as u16)` can collide across batches but monotone within one. |
| `build_spu` (test) | 437-490 | REAL | — | Hand-rolled SPU/PES builder, debug_assert on offset arithmetic. |
| `wrap_pes` (test) | 492-502 | REAL | — | PES 0xBD wrapper. |
| `pair_decode_end_to_end` | 504-532 | REAL | — | Strong: crop, palette, timing (1024/5120 ms), forced flag. |
| `multi_pes_accumulation` | 534-559 | REAL | — | Verifies split-PES accumulation + BadFilepos span logic. |
| `bad_filepos_and_missing_palette_are_typed` | 561-574 | REAL | — | Slightly loose first-match assertion but the `contains` check is exact. |
| `grayscale_fallback_matches_reference_shape` | 576-596 | REAL | — | Checks p[2]=white, p[0].a=0. |
| `cue_contract_render` | 598-615 | REAL | — | Asserts anchor rationals, z, half-open span (`active_at(start)`, `!active_at(end)`). |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| pair_decode_end_to_end | YES | Byte-exact rgba, crop rect, and ms→ns timing assertions. |
| multi_pes_accumulation | YES | Split-PES + second-entry span both asserted. |
| bad_filepos_and_missing_palette_are_typed | YES (weakly) | Exact `contains(BadFilepos{0x200})`; first-match arm is loose. |
| grayscale_fallback_matches_reference_shape | YES | Exact Rgba equality on level map output. |
| cue_contract_render | YES | Rational equality + half-open span semantics. |
None cover: overflow timestamps (F-002), oversized dimensions (F-003), out-of-canvas anchors (F-005) — the fuzz survival test (tests/dvb_vobsub_fuzz_survival.rs) mutates one embedded pair, so those classes are untested.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "carriage already landed: the PS demuxer routes 0xBD ... as dvd_subtitle" | mod.rs:2-3 | MISLEADING | Demux labels only (ps/mod.rs:171); no data path to this crate (F-001). |
| "It never panics; every malformation ... is typed data" | mod.rs:23-24 | FALSE for hostile idx | F-002 overflow arithmetic. |
| "Never ... unbounded allocation" | mod.rs:39-40 | FALSE | F-003: input cap ≠ output cap; 67 MiB/cue from ~3 KB. |
| VobSub "implemented" on windows/linux/macos | docs/feature-matrix.toml:1042 | PAPER-ONLY for shipped app | No shipped binary links this crate (F-001). |

## 7. Dead / duplicated / unreachable code
Entire module is dead code from the shipped-binary perspective (only fuzz/Cargo.toml:50 links the crate). Dead arms: `unwrap_or(Rational::ZERO)` at mod.rs:391-392 (den is `.max(1)`). `spu.indices.clone()` at mod.rs:252 duplicates up to 16.7M bytes for forced cues when a borrow would do.

## 8. Dependencies & flags
Uses crate::compositing (bitmap/layout/target), media-types time. No feature gates in this file; crate has no consumers gating it either way. No unsafe, no I/O (verified by read).

## 9. Open questions for the next agent
- compositing/layout.rs: does `place_cue` clamp `SafeMargin::TitleSafe` against `reference` extents? If yes, DVD absolute positions (720×480 canvas) get shifted by title-safe insets — verify against the 9-alignment grid test claims.
- dvb/mod.rs shares the same Anchor pattern — same F-005 teleport and F-003 amplification classes likely apply there.
- vobsub/fuzz.rs and idx.rs/spu.rs are other agents' files; F-002/F-003 roots live there (idx.rs:197-207 unbounded hh; spu.rs:264 pre-decode alloc).
