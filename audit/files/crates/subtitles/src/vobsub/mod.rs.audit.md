# AUDIT: crates/subtitles/src/vobsub/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A08 |
| Date | 2026-11-21 |
| Lines / functions | 616 / 7 prod (2 types+Display, 5 fns) + 7 test fns |
| File verdict | UNREACHABLE (real, in-crate-tested library code; zero shipped callers) |
| Reachable from shipped entry point? | NO — `parse_vobsub`/`vobsub_cues` callers: only `crates/subtitles/tests/dvb_vobsub_fuzz_survival.rs:36,292,305,369` and in-file unit tests. `subtitles` crate is a dependency of NOTHING but `fuzz/Cargo.toml:50` (rg over all Cargo.toml); `rg "subtitle|vobsub|CompositedCue" apps/` → no matches. `lumen` CLI and desktop-tauri never link it. |
| Findings | S0: 0 S1: 1 S2: 2 S3: 3 S4: 1 |
| Commands run | `rg "lumen_subtitles\|subtitles::"` (23 hits: docs, fuzz, crate-internal only); `rg subtitles **/Cargo.toml` (only fuzz/); `rg "subtitle|vobsub|..." apps/**/*.rs` (0); Read spu.rs + idx.rs full (invariants: `indices.len()==w*h` at spu.rs:264, w,h ≤ 4096 from 12-bit coords spu.rs:215-218, `index` parsed as unclamped u8 idx.rs:136-140, timestamp components unclamped i64 idx.rs:200-206); `sed -n` quote verifies; baseline grep: subtitles v0.1.0 compiled in test-workspace.log.txt:15, no subtitles failures in #3/#4. |

## 1. One-paragraph verdict
Real, carefully written pair-decode logic (idx↔sub accumulation, palette/grayscale, crop, contract reduce) with genuinely non-tautological unit tests — and it is dead code in the shipped product: no binary, app, or engine crate depends on the subtitles crate, so not one VobSub cue can ever be decoded by `lumen`. Within the dead code there are three contract violations of its own "never panics / never unbounded allocation" promise (mod.rs:19-27, 39-40): a crafted-but-grammatical <1 MB idx/sub pair allocates multiple GiB of cue RGBA (fill-run expansion × unbounded entry count), an idx `index:` ≥ 224 overflows `0x20u8 + index` (panic in dev-profile builds), and an 8-digit-hour idx timestamp overflows `start_ms * 1_000_000` (same class). The 5.N2 record's "zero unbounded allocations" evidence claim cannot prove what it says.

## 2. What actually works (verified)
— verified by code trace + baseline: `parse_vobsub` accumulation span logic (bounds across all streams, mod.rs:149-155, 161-162) matches the documented reference psize semantics; `crop_borders` (mod.rs:339-374) indexing is safe (`spu.indices.len()==w*h` guaranteed by spu.rs:264 + both `decode_field` calls fill bounded rows) and its loop bounds are sound (all-transparent case returns None at :353-355). Baseline run #3 (`audit/runs/baseline/test-workspace.log.txt`, subtitles compiled :15, workspace 1681 passed / 1 failed — failure is `apps/desktop/tests/zero_alloc_transport.rs`, not subtitles) executes this crate's tests green, but in `--release` (overflow-checks OFF — see F-002/F-003).

## 3. Findings

### F-001 [S1]
Where: crates/subtitles/src/vobsub/mod.rs:251-257 (with :157-200, :382-389)
Code:
```
    let (x, y, w, h, indices) = if spu.forced {
        (spu.x, spu.y, spu.width, spu.height, spu.indices.clone())
    } else {
        crop_borders(spu, transparent)?
    };

    let mut rgba = Vec::with_capacity(w * h * 4);
```
What is wrong: unbounded output allocation from file-controlled counts. SPU dims are 12-bit (w,h ≤ 4096, spu.rs:215-218,256-257), a forced SPU skips the crop, and one 2-byte fill run covers a whole 4096-px line (spu.rs `decode_run` v<4 → usize::MAX → `take = room`), so a ~8 KiB well-formed SPU yields `indices` 16.7 MB and `rgba` 64 MiB (8192× amplification). Every cue's full RGBA is retained in `VobSubDoc.cues` (:157-200) with no count or byte cap — an ~800 KB crafted .idx/.sub pair (100 entries × 8 KiB SPUs) allocates ≈6.4 GB.
Why it matters: direct violation of the module contract "Never: … unbounded allocation (SPU accumulation capped at 128 KiB)" (:39-40) — the cap bounds INPUT only — and of the fuzz harness contract "output ≤ O(input) + the 128 KiB cap" (vobsub/fuzz.rs:30-32). OOM/abort on grammatically valid input; the in-tree survival harness is the reachable consumer that would die.
What correct looks like: cap decoded pixels per SPU (e.g. reject w*h > the reference's 720×576-class bounds or a fixed 4 MiB pixel budget), cap total decoded-cue bytes, and/or stream cues instead of accumulating `VobSubDoc.cues`.
Confidence: HIGH — Verification: read (spu.rs fill-run + 12-bit dims + forced-skip-crop traced; arithmetic checked).

### F-002 [S2]
Where: crates/subtitles/src/vobsub/mod.rs:159
Code:
```
    for stream in &idx.streams {
        let want = 0x20u8 + stream.index;
```
What is wrong: `IdxStream.index` is parsed as an unclamped `u8` (idx.rs:136-140, `.parse::<u8>().ok().unwrap_or(0)`) though documented "0..32" (idx.rs:53-54). For `index: 224..=255` the addition overflows u8: panic in dev-profile builds (overflow-checks on — root Cargo.toml has no `overflow-checks` override, so plain `cargo test`/`cargo run` dev builds panic), silent wrap in the baseline's --release profile. For `index: 32..=223` `want` lands outside `0x20..=0x3F` and every entry degrades to a misleading `BadFilepos` damage event instead of a typed bad-index event.
Why it matters: violates the module's own "It never panics; every malformation … is typed data" (:23-24). No test can reach it: the survival suite's idx mutation alphabet is `b"0123456789abcdefx:,#\n\t\xff"` over a 1-digit index (dvb_vobsub_fuzz_survival.rs:406), so index stays ≤ 9.
What correct looks like: validate `index <= 0x1F` at parse (typed `IdxDamage`) or use `checked_add`/`u32` here.
Confidence: HIGH — Verification: read (idx.rs parse + root Cargo.toml profile grep).

### F-003 [S2]
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
What is wrong: `timestamp_ms` is unbounded i64 from the idx (`hh:mm:ss:ms` components parse as bare i64, idx.rs:200-206 — e.g. `timestamp: 10000000:00:00:000` parses fine). `start_ms * 1_000_000` overflows i64 for start_ms > 9.2e12 (≈292 years): dev-profile panic; release wraps negative, after which `Timestamp::new(negative, ns_base).as_nanos()` errors (media-types time.rs:475 `u64::try_from` fails) and `vobsub_cues` returns Err — dropping the ENTIRE document (see F-005).
Why it matters: second "never panics" contract violation on loose-but-parseable idx text; the release-path failure mode is total cue loss from one bad line.
What correct looks like: `checked_mul` → typed `VobSubDamage::BadTimestamp`, per-cue skip, not a whole-document abort.
Confidence: HIGH — Verification: read (idx.rs:197-207 + media-types time.rs:461-480 traced).

### F-004 [S3]
Where: crates/subtitles/src/vobsub/mod.rs:160-171
Code:
```
        for entry in &stream.entries {
            let next_bound = bounds.iter().find(|&&fp| fp > entry.filepos).copied();
            let span_end = next_bound.unwrap_or(u64::MAX);
            let mut acc: Vec<u8> = Vec::new();
            for (ss, offset, _pts, payload) in &fragments {
```
What is wrong: O(entries × fragments) — for every idx entry the FULL fragment list is re-scanned, plus an O(E) linear `find` over `bounds` per entry (O(E²)). A multi-stream rip (idx files in the wild reach tens of thousands of entries; the .sub correspondingly) does ~E×F redundant iterations; at 1e5×1e5 that is 1e10 comparisons on one open().
Why it matters: quadratic hot path on the one API this module exists for; fine for unit-test sizes, pathological for real DVD rips.
What correct looks like: fragments sorted by offset (they already are — `extract_spu_pes` scans linearly), then a per-stream cursor/binary search over `bounds`.
Confidence: HIGH — Verification: read (loop structure + extract_spu_pes emission order).

### F-005 [S3]
Where: crates/subtitles/src/vobsub/mod.rs:382-425
Code:
```
pub fn vobsub_cues(doc: &VobSubDoc, z: u16) -> Result<Vec<CompositedCue>, CueError> {
    let extent = doc.size.unwrap_or((720, 480));
    let mut cues = Vec::new();
    for (i, cue) in doc.cues.iter().enumerate() {
```
(and :417-422 `cues.push(CompositedCue::new(span, bitmap, layout, z.saturating_add(i as u16))?);`)
What is wrong: (a) `vobsub_cues` ignores `cue.stream_index`/`cue.lang` — a multi-language idx (the format's whole point: `id: en`, `id: de`, …) composites EVERY language's cues simultaneously, overlapping; there is no selection seam (the reference shows one stream, per `langidx`). (b) The `?` on `CompositedCue::new` makes the render all-or-nothing: one cue with a typed conversion failure (e.g. the negative timestamp of F-003, reachable via a legal `delay: -…` line, idx.rs:186-192) discards every cue in the document — the opposite of the per-entry damage-ledger design.
Why it matters: any future engine glue that wires this API renders wrong (all languages stacked) or loses all subtitles because one line was bad.
What correct looks like: a `stream: Option<u8>` filter parameter (default `lang_idx`), and per-cue typed skip recorded into a damage ledger instead of `?`.
Confidence: HIGH — Verification: read.

### F-006 [S4]
Where: crates/subtitles/src/vobsub/mod.rs:389
Code:
```
        let bitmap = CueBitmap::from_rgba(cue.width, cue.height, cue.height, cue.rgba.clone())?;
```
What is wrong: full-cue RGBA copy per cue (`clone`) on the only contract-reduce path; parse already owns the bytes but `vobsub_cues` borrows `doc`, forcing the copy. (Third arg is `baseline` — `cue.height` = bottom row — correct usage, verified against bitmap.rs:105-110.)
Why it matters: doubles peak memory of the largest allocation class in the crate (the F-001 bitmaps) for no semantic gain.
What correct looks like: `vobsub_cues(doc: VobSubDoc, …)` taking ownership and moving `rgba`.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `VobSubDamage` (+`Display`) | 59-88 | REAL | — | data + fmt, correct; 4 variants match the ledger uses |
| `VobSubCue` | 90-111 | REAL | — | plain data; `end_ns: Option` open-ended per reference |
| `VobSubDoc`/`is_clean` | 113-127 | REAL | — | trivial, correct |
| `parse_vobsub` | 129-209 | REAL | S1/S2/S3 | F-001 aggregate accumulation, F-002 overflow at :159, F-004 quadratic; span/psize logic itself traced correct |
| `render_cue` | 211-276 | REAL | S1/S2 | F-001 alloc :257, F-003 overflow :267-268; palette math (α×17, &0xFF_FFFF) matches the documented reference formula; colormap 4-bit → palette[16] indexing safe |
| `guess_palette` | 278-334 | REAL-UNPROVEN | — | ffmpeg `guess_palette` port (level maps 0x00/0x55/0x80/0xAA/0xFF); traced self-consistent (j can't exceed levels.len()); NOT diffable against ffmpeg source in-tree — fidelity UNVERIFIED |
| `crop_borders` | 336-374 | REAL | — | bounds-sound (all-transparent → None :353-355; x1≤x2 guaranteed by a non-transparent column); returns cropped indices |
| `vobsub_cues` | 376-425 | REAL | S3/S4 | F-005 both halves, F-006; ns_base literal construction comment is honest (coprime 1/1e9) |
| tests: `build_spu`,`wrap_pes` | 437-502 | REAL | — | builders mirroring spu.rs; `debug_assert` on cmds2 arithmetic |
| `pair_decode_end_to_end` | 504-532 | REAL | — | asserts crop rect, palette RGBA, exact ms math ((90<<10)/90=1024, (450<<10)/90=5120) — would fail under crop/palette/timing mutation |
| `multi_pes_accumulation` | 534-559 | REAL | — | split-SPU + second-entry BadFilepos; would fail if span logic broke |
| `bad_filepos_and_missing_palette_are_typed` | 561-574 | REAL | — | weak first assert (matches either variant) but `contains(BadFilepos)` pins the load-bearing fact |
| `grayscale_fallback_matches_reference_shape` | 576-596 | REAL | — | single-color case only; multi-level map untested |
| `cue_contract_render` | 598-615 | REAL | — | pins anchor rationals (360/720, 240/480), z, reference extent, half-open active_at |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| pair_decode_end_to_end | YES | exact rect/pixel/timing literals; mutation of crop off-by-one, alpha×17, or (date<<10)/90 breaks an assert |
| multi_pes_accumulation | YES | two-fragment SPU only decodes if accumulation spans fragments; second entry must yield BadFilepos |
| bad_filepos_and_missing_palette_are_typed | YES (partially) | `contains(BadFilepos 0x200)` is the strong half; first `matches!` accepting either variant is weak |
| grayscale_fallback_matches_reference_shape | YES for the 1-color level; NO for 2-4-color level maps | only `LEVEL_MAP[0]` is pinned |
| cue_contract_render | YES | anchor = x/extent rationals asserted exactly |
| NOT covered by any test | — | F-001 (no max-dim SPU exists in any seed), F-002 (mutation alphabet caps index ≤ 9), F-003 (mutation can't build 8-digit hours), F-005 multi-stream render (single-stream seeds only) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Parse VobSub (IDX/SUB)" status implemented win/linux/macos, evidence "230/0 gates" | docs/feature-matrix.toml `subtitle-vobsub` (:1035-1046) | UNREACHABLE (code REAL, tests real; no shipped entry point links the crate — see header) | rg callers: tests only; Cargo.toml dependency graph: fuzz only |
| "bounded by … the 128 KiB cap" / "Never: … unbounded allocation" | mod.rs:26-27,39-40; docs/formats/sub-vobsub.md "Working subset" | FALSE (for output) | F-001: input cap only; output uncapped |
| "zero panics/hangs/unbounded allocations" (survival evidence) | docs/formats/sub-vobsub.md Evidence | SHALLOW | survival suite asserts per-cue stride only (dvb_vobsub_fuzz_survival.rs:298-304); its seeds/mutation alphabet cannot synthesize max-dimension SPUs or big indexes — it cannot disprove F-001/F-002/F-003 |
| "carriage already landed: the PS demuxer routes 0xBD substream 0x20..=0x3F as dvd_subtitle" | mod.rs:1-3; sub-vobsub.md "Carriage" | REAL (label only) | crates/demux/src/ps/mod.rs:171 `PrivateKind::Spu => "dvd_subtitle"` — but nothing downstream consumes the track |
| "next entry on the same stream implicitly replaces it" (open-ended semantics) | mod.rs:33-35 | REAL-UNPROVEN | open-ended `end_ns: None` implemented; the replacement semantics live in the (nonexistent) consumer — nothing implements "replaces at its timestamp" |

## 7. Dead / duplicated / unreachable code
The entire module is production-dead (crate-level unreachability, see header). In-file: `VobSubCue.forced`/`stream_index`/`lang` fields are populated but never read by any consumer (`vobsub_cues` ignores stream/lang entirely — F-005). `build_spu` here duplicates `vobsub_build_spu` in tests/dvb_vobsub_fuzz_survival.rs:228-259 (third copy of the SPU builder grammar across the crate — spu.rs:457 has the original).

## 8. Dependencies & flags
Module uses `crate::compositing::{bitmap, layout, target}` + `media-types::time` only — matches boundaries.toml `[crates.subtitles] internal_deps = ["media-types"]`. No feature flags. No unsafe (workspace `unsafe_code = deny`).

## 9. Open questions for the next agent
- dvb/mod.rs (A06) and pgs (A05/A10): do their reduce functions share the F-001 amplification pattern (uncapped w×h from file-controlled dims) and the F-005 all-or-nothing `?`? Compare `dvb_cues`/`pgs_cues`.
- Whoever audits `vobsub/fuzz.rs` (A05): the "output ≤ O(input)" bound claim is refuted by F-001 — cross-file contradiction to record.
