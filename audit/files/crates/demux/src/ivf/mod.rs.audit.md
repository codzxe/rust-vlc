# AUDIT: crates/demux/src/ivf/mod.rs
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-21 |
| Lines / functions | 703 / 39 items (5 consts, 1 fn probe, 3 structs + impls, 30 fns/methods incl. the `Demuxer` impl) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES. `lumen` CLI `open` → `apps/cli/src/demux_host.rs` `open_source` → probe names `ivf` (`crates/media-probe/src/signature/phase5.rs:202` `DKIF` magic) → `apps/cli/src/containers/mod.rs:55` `"ivf" => ivf::open(stream, opts)` → `apps/cli/src/containers/ivf.rs:82` `IvfDemuxer::new`. Executed in baseline: `containers::ivf::tests` 6/6 pass incl. `generated_av1_lane_decodes_the_full_stream` (orchrun-test-workspace-nofailfast.log.txt:2796-2801); demux unit suite 22/22 (`:1691-1713`); `tests/ivf_fixtures.rs` 5/5 (`:2078-2089`, ffmpeg-authored bytes, payload-md5 differential). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | Full-file Read (703 lines, verbatim) · `rg -n "IvfDemuxer" crates apps` (9 files: CLI host + demux tests + codecs AV1/VP9 differentials — no engine-internal callers besides the host) · Read `crates/plugin-api/src/demux.rs:1-140` (trait contract: `seek` = pure query that "never moves anything", `reanchor` = commit half, invariant D4) · Read `crates/demux/src/avi/riff.rs:203-412` (`SourceReader::read_prefix_at/into`: absolute seek + short-read re-ask loop, `Ok(0)` = EOF; position moves) and `:490-691` (`PayloadSlots`: 2×4 MiB slots, spill beyond `slot_cap`) · `rg -n "sticky\|fail\(" crates/demux/src/adts/mod.rs` + `sed -n '485,545p;660,720p'` (ADTS precedent: identical determinism-based sticky on walk errors; but ADTS *does* length-check its prefix read at `:531-532`) · Read `crates/input/src/buffered.rs:186-260,300-395` (`confirm_eof`: shrink-below-declared → typed `input.file.truncated`) + `crates/input/src/file.rs:207-260` (`FileStream` defers to BufferedReader) · Read `crates/plugin-api/src/access.rs:24-99` (trait: "short reads are data", `Ok(0)` only at end of stream; `size()` advisory-trusted) · `rg -n "ivf" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (executed evidence above) · `sed -n '337,350p' docs/feature-matrix.toml` (row `container-ivf`: implemented w×l×m) · Read `crates/media-types/src/time.rs:57-97` (one tick = num/den seconds → `tick_base` correct) · `rg -n "track.clone\(\)" crates/demux/src` (ADTS/ES/IVF clone per packet) + Read `crates/demux/src/avi/mod.rs:620-665` (AVI's W5-14 lane caches `TrackId`+`TimeBase`, no per-packet clone) |

## 1. One-paragraph verdict
This is a real, complete, honestly-documented IVF demuxer: strict 32-byte header validation with typed refusals for every structural lie, a positional frame walk whose arithmetic I traced end-to-end (ceiling check *before* the size addition, truncation check before the pool read, `saturating`/`max` discipline on pts and duration), a bounded header-only seek scan (`seek` is a genuine pure query per the trait contract — position save/restore around the scan, walk untouched; `reanchor` is the commit half with D4-ordered compute-then-commit), and a payload pool whose `pool_bytes()` observable is what the docs claim. The "sticky truncation" claim holds observably (walk is not advanced past a refused frame, so replays mint the identical typed error — same mechanism as the ADTS precedent, pinned by `walk_refuses_frame_beyond_file_sticky`). Defects: one unchecked slice on the frame-header read (`&raw[4..12]` panics if a `SourceStream` serves `Ok(0)` before its declared size — the ADTS sibling length-checks exactly this read, every in-tree source defends, but the plugin seam does not), one per-packet `Track` clone (a `CodecDescriptor(String)` heap alloc per frame — drift from the AVI lane's cached-base pattern and the crate's own "zero allocations per packet" bar), an error-priority inversion in `seek`'s position restore, and a triplicated dead fallback base.

## 2. What actually works (verified)
— Header validation (`IvfHeader::parse`, :175-203): magic, ≥32-byte length, exact header_len, non-zero dims/rate/scale — every refusal typed; pinned by `header_refuses_*` tests, executed 22/22 (orchrun :1691-1713).
— Timing model (:232-241): `TimeBase::new(scale, rate)` gives one tick = scale/rate seconds (media-types time.rs:57-90, `num/den`); `tick_nanos` computes in u128 with no overflow. `clean_walk_passes_the_container_truth` asserts pts/base/duration exactly (tests.rs:158-171) — executed.
— Walk arithmetic (`step`, :438-498): `walk < len` guaranteed before `len - walk` (no underflow); the 16 MiB ceiling is checked (:457-461) **before** `payload_end = payload_at + size` (:462-463) so no u64 overflow is possible; truncation refusal (:464-469) precedes the pool read. Zero-size frames emit empty pooled-static payloads (tests.rs:190-213).
— Ceiling refusal is allocation-free: `walk_refuses_ceiling_sized_frames_before_allocating` asserts `pool_bytes() == 4` after a 16 MiB+1 claim (tests.rs:304-318) — executed; `fuzz_lite_wild_declared_sizes_are_refused_not_allocated` asserts the bound over 256 rounds (tests.rs:536-553) — executed.
— `seek` is a true pure query (trait contract plugin-api/demux.rs:73-75, 93-98): scan, restore position, never touch `walk`/`pending`/`eos`; pinned by `seek_lands_on_the_last_frame_at_or_before_the_target` (:361-408, "A pure query: the walk position never moved"). `reanchor` commits with Discontinuity and clears `pending` (:694-701); D4 failure-atomicity holds (all fallible work precedes the first mutation :694-698).
— Sticky replay of walk failures: `frame_beyond_file` leaves `walk` unchanged, so `next_event` re-reads the same header and mints the identical code — asserted (tests.rs:296-300), executed; identical mechanism to the ADTS precedent (adts/mod.rs:670-676 returns raw Err, no `fail()`, same determinism).
— End-to-end through the shipped CLI: `containers::ivf::tests::generated_av1_lane_decodes_the_full_stream` opens via the real `open_source` path and decodes — executed, baseline orchrun :2797. Real-ffmpeg-bytes differential: `tests/ivf_fixtures.rs` (`frame_counts_pts_ladders_and_payload_md5_match_ffmpeg`, 5/5, orchrun :2078-2089).

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/ivf/mod.rs:421-428
Code:
```
        let raw = self.reader.read_prefix_at(
            usize::try_from(FRAME_HEADER_BYTES).unwrap_or(usize::MAX),
            at,
        )?;
        let mut size = [0u8; 4];
        size.copy_from_slice(&raw[0..4]);
        let mut pts = [0u8; 8];
        pts.copy_from_slice(&raw[4..12]);
```
What is wrong: `read_prefix_at` returns "whatever the stream actually carries there" (riff.rs:297-312) — a short `Vec` when the source reports EOF early. The slices `&raw[0..4]` and `&raw[4..12]` are unchecked: if `raw.len() < 12` they panic (index out of range). The caller guards (`step` :443, `scan_landing` :512) only check against `reader.length()`, which is `stream.size()` trusted verbatim at `SourceReader::new` (riff.rs:227). A `SourceStream` impl whose `size()` over-reports and which then serves `Ok(0)` (the trait permits short reads as data, access.rs:36-38) reaches this panic — in `step` **and** in `scan_landing`, i.e. on open-to-EOF and on every seek. The demuxer contract says "Panics are contract violations" (plugin-api/demux.rs:51).
Why it matters: Every in-tree source defends this today (input's `BufferedReader::confirm_eof` types shrink-below-declared as `input.file.truncated`, buffered.rs:407-416; streaming's `RangedResource` types known-length shortfalls inside `fetch_window`, ranged.rs:888-891) — so no shipped lane triggers it. But `SourceStream` is a public plugin seam (doc 06 kinds 1-2), `SourceReader::new` performs no verification that a source can actually deliver `size()` bytes, and the crate's own sibling demuxer length-checks exactly this class of read (`adts/mod.rs:533-535`: `if full.len() < hlen as usize { return Err(error::truncated(...)) }` before slicing). One hostile or buggy plugin source = panic in the demux worker thread. Rated S2 for the unchecked-slice-on-input pattern per the checklist ("panics on malformed input"), with the explicit caveat that no in-tree path reaches it.
What correct looks like: After the read, `if raw.len() < FRAME_HEADER_BYTES as usize { return Err(error::truncated(at + FRAME_HEADER_BYTES, self.reader.length())); }` — the ADTS pattern, minted with this module's own `error::truncated`.
Confidence: MEDIUM — Verification: read (all in-tree `SourceStream` impls traced: input/file.rs:207-260, input buffered.rs, streaming/ranged.rs:836-905; none can produce the short read, so the panic is UNVERIFIED by execution — reachable only through a contract-violating third-party impl).

### F-002 [S3]
Where: crates/demux/src/ivf/mod.rs:473-477
Code:
```
        let track = self
            .track
            .clone()
            .ok_or_else(|| self.fail(error::not_started()))?;
        let base = track.timing.base;
```
What is wrong: A full `Track` clone per frame. `Track` contains `codec: CodecDescriptor` — a `String` newtype (media-types/ids.rs:573) — so every packet pays one heap allocation, plus Vec/Option clones, for two values that are immutable after `start` (`track.id`, `track.timing.base`). The crate's own performance bar is "zero allocations per packet" (riff.rs:510-512, the 7-c slot-cap policy's stated goal), and the AVI movi lane was specifically reworked (W5-14) to mint packets from cached `TrackId` + `TimeBase` with no per-packet clone (avi/mod.rs:635, 648-649). IVF (and ADTS/ES, same idiom) never got that treatment; no zero-alloc test covers the IVF walk (`tests/zero_alloc_read_chain.rs` is AVI-scoped).
Why it matters: 30–60 small allocations per second on the only IVF lane that exists (decode-through-CLI); small in absolute terms but a measured-defect class in this codebase (the memory-audit register's V-1 counts per-packet allocation events as findings), and the pattern will be copy-pasted into every future demuxer card.
What correct looks like: Cache `TrackId` and `TimeBase` (both `Copy`) at `start` time and mint `Packet { track: self.track_id, ... }` as AVI does.
Confidence: HIGH — Verification: read; AVI contrast read (avi/mod.rs:620-665); zero-alloc test scope verified by rg.

### F-003 [S4]
Where: crates/demux/src/ivf/mod.rs:663-665
Code:
```
        let result = self.scan_landing(target_ticks);
        self.reader.seek_to(save)?;
        let (pts, _) = result?;
```
What is wrong: If `scan_landing` failed *and* the position restore also fails, the `?` on `seek_to` discards the scan's error and surfaces the restore error — the diagnostic the caller needs (seek-out-of-range vs IO) is replaced by a secondary failure. The crate's own discipline for this exact situation is the opposite: `peek_chunk_header_into` restores best-effort and keeps the original error ("the original error wins", riff.rs:381-396).
Why it matters: Error-priority inversion on an already-failing path hides the root cause; the fix is the established `let _ = self.reader.seek_to(save);` + `result?` shape used 60 lines away in riff.rs.
What correct looks like: `if let Err(restore) = self.reader.seek_to(save) { /* log/ignore per the riff.rs precedent */ } let (pts, _) = result?;` — or at minimum restore after propagating.
Confidence: HIGH — Verification: read; contrast riff.rs:383-397.

### F-004 [S4]
Where: crates/demux/src/ivf/mod.rs:543-547 (also 657-661, 688-692 — three copies)
Code:
```
        let base = self
            .track
            .as_ref()
            .map(|t| t.timing.base)
            .unwrap_or(TimeBase { num: 1, den: 1_000 });
```
What is wrong: The same five-line fallback (with the same four-line comment) is pasted into `landing_span`, `seek`, and `reanchor`. The fallback arm is dead in all three: every entry path is `!started`-guarded and `start` refuses before `self.track` can be `None` while `started == true`. `TimeBase { num: 1, den: 1_000 }` also bypasses `TimeBase::new`'s validation by struct literal — the pattern the codebase itself calls "degenerate struct-literal bases" when defending against it (time.rs:529-530).
Why it matters: Triplicated dead code with a raw struct-literal base is exactly the drift the "8.A1 sweep" comments say was already cleaned once; three copies means three chances for the next edit to miss one.
What correct looks like: One private `fn track_base(&self) -> TimeBase { self.track.as_ref().map(|t| t.timing.base).unwrap_or(TimeBase { num: 1, den: 1_000 }) }` (or assert the invariant and drop the fallback).
Confidence: HIGH — Verification: read; all three entry guards traced (:649-651, :681-683, and `landing_span`'s only callers are those two).

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (contract: timing/codec-honesty/damage/seek/scope) | 1-89 | REAL | — | Every concrete claim cross-checked below; the "sticky truncation" and "pure query" claims verified against code + tests. |
| `use` set (`avi::header::DamageLog`, `avi::riff::{PayloadSlots, SourceReader}`) | 91-99 | REAL | — | Cross-family reuse of the AVI reader/pool — deliberate, documented. |
| `mod error/fuzz/test_support/tests` decls | 101-106 | REAL | — | `test_support` is NOT cfg(test) — production module (see its own audit file). |
| `HEADER_BYTES` = 32 | 108-111 | REAL | — | Matches ffmpeg/libvpx; != 32 refused typed. |
| `FRAME_HEADER_BYTES` = 12 | 113-114 | REAL | — | 4 size + 8 pts. |
| `MAX_FRAME_BYTES` = 4 MiB | 116-118 | REAL | — | Pool slot cap (2 slots ⇒ 8 MiB pool bound, matches `pool_bytes` doc). |
| `FRAME_CEILING_BYTES` = 16 MiB | 120-122 | REAL | — | Refusal bound checked before any allocation (test pins pool==4 after refusal). |
| `MAX_SEEK_SCAN_FRAMES` = 2_000_000 | 124-126 | REAL | — | Scan budget; 2M×12-byte reads worst case — bounded, documented SlowLinear. |
| `EXTENSIONS` = ["ivf"] | 128-129 | REAL | — | media-probe duplicates the string instead of importing (phase5.rs:206) — see phase5 audit. |
| `probe` | 132-135 | REAL | — | 4-byte magic; length-guarded. Executed (probe test). |
| `IvfHeader` struct | 140-163 | REAL | — | Field-for-field the on-disk layout (offsets verified against parse). |
| `IvfHeader::parse` | 175-203 | REAL | — | Length/magic/header_len/dims typed refusals; LE field reads at correct offsets (4,6,8-11,12,14,16,20,24); offset 28-32 ignored as documented. Slicing safe: len ≥ 32 checked first. |
| `IvfHeader::codec` | 209-215 | REAL | — | AV01→"av1", VP90→"vp9", else None (refusal path). |
| `IvfHeader::descriptor` | 221-224 | REAL | — | `CodecDescriptor::new(...).ok()` — swallows the registry-validation Err into None; acceptable (names are static literals). |
| `IvfHeader::tick_base` | 232-234 | REAL | — | `TimeBase::new(scale, rate)` = num scale/den rate ⇒ tick = scale/rate s — verified against media-types semantics + test (:69-71). |
| `IvfHeader::tick_nanos` | 239-241 | REAL | — | u128 math, `u64::try_from(...).ok()` — no panic, None on overflow (doc honest). |
| `IvfStreamInfo` struct + accessor wiring | 246-265 | REAL | — | ffprobe-parity fields; `stream_info` (:353-366) copies from header, None before start / unsupported fourcc. |
| `StickyShape` + `of` + `mint` | 270-289 | REAL | — | Replayable error shape; used for start failures; walk failures replay by determinism (see §7 note). |
| `IvfDemuxer` struct | 297-316 | REAL | — | All state bounded; `pending` VecDeque drained per event (≤1 packet + 1 recovered pending at a time — verified in step/next_event). |
| `IvfDemuxer::new` | 325-341 | REAL | — | Delegates source-capability refusals to SourceReader::new (typed Open-stage; empty/unknown-length/unseekable refused — test :351-356). |
| `header` / `stream_info` / `frames_demuxed` / `damage_total` / `pool_bytes` | 345-387 | REAL | — | Pure accessors; `#[must_use]`. |
| `track_duration` | 393-408 | REAL | — | (max_pts + 1) × tick in u128 with saturating ops; None before first packet / on overflow; max-never-last pts anchor correct for regressing hostile ladders. Test pins 3×tick (:182-186). |
| `fail` | 412-415 | REAL | — | Sets sticky + returns. |
| `frame_header_at` | 420-434 | REAL | S2 | F-001 (unchecked slices on short read). pts i64 gate before use (:429-432) — hostile u64 pts refused typed (test :321-332). |
| `step` | 438-498 | REAL | S3 | F-002 (per-frame Track clone). Order of checks correct: fragment→ceiling→truncation→read; `walk` advanced only on success (sticky-by-determinism). |
| `scan_landing` | 506-527 | REAL | — | Bounded (2M frames), header-only (no payload reads — each iteration reads exactly 12 bytes), last-pts-≤-target landing with first-frame clamp; `at + 12 <= len` guard; sizes not ceiling-checked here — harmless (scan-only, walk is the truth; a liar frame is unreachable past its own claim in both). No-frames file → typed out-of-range (test :452-468). |
| `target_ticks` | 531-534 | REAL | — | Unknown stamp → None → caller's 0 (documented). |
| `landing_span` | 538-552 | REAL | S4 | F-004 copy #1. |
| `Demuxer::start` | 556-616 | REAL | — | Sticky on failure (test asserts replay :271-275); re-entry after success returns the same track; header-only file refused `no_frames` (documented ffmpeg divergence); track minted with container-exact base, span end honestly Missing. |
| `Demuxer::next_event` | 618-643 | REAL | — | pending-first, eos latch, loop-until-event; sticky minted before anything; not-started typed. |
| `Demuxer::seek` | 645-667 | REAL | S4 | F-003 (restore-error priority). Pure-query contract honored (position restored, walk untouched) — pinned by test :381-383. |
| `Demuxer::seekability` | 669-675 | REAL | — | SlowLinear after clean start; None when sticky — honest. |
| `Demuxer::reanchor` | 677-702 | REAL | — | D4 compute-then-commit (mutations only after `?`-free landing); Discontinuity queued; eos reset; test :411-449 (incl. negative-target clamp). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `header_parse_roundtrips_the_field_set` | YES | Field-by-field equality incl. base (1,30) and tick_nanos — kills offset/endianness mutations. |
| `probe_detects_dkif_and_rejects_the_rest` | YES | 4 positive/negative cases incl. empty. |
| `header_refuses_short_buffers_typed` / `..._bad_magic_typed` / `..._structural_lies_typed` | YES | Exact error-code equality per mutation site (len, magic, header_len, dims, rate, scale). |
| `clean_walk_passes_the_container_truth` | YES | pts ladder, base, per-packet duration, keyframe=false, dts=Missing, payload bytes, frames/damage counters, duration=(2+1)×tick — mutation of any of these fails an assert. |
| `walk_passes_vfr_gaps_and_zero_size_frames_through` | YES | Gap pts [0,0,5] + zero-length payload pass-through. |
| `declared_frame_count_is_advisory_the_walk_is_truth` | YES | Declares 99, carries 2 — trust-the-walk pinned. |
| `trailing_fragment_is_recovered_then_clean_end` | YES | Recovered event payload (5, reason) + damage_total — bounded-skip semantics pinned. |
| `start_refuses_every_malformed_shape_typed` | YES | Code-per-shape + sticky replay asserted (kills the "re-reads damage on retry" class). |
| `walk_refuses_frame_beyond_file_sticky` | YES | Terminal code + replay equality — pins determinism-sticky. Would NOT catch a mutation that advances `walk` past a refused frame *and* keeps the same first-error code — but the replay assert reads the same position, so an advance breaks it. |
| `walk_refuses_ceiling_sized_frames_before_allocating` | YES | `pool_bytes() == 4` — kills "allocate then refuse". |
| `walk_refuses_i64_impossible_pts` | YES | u64::MAX pts → typed `pts-overflow`. |
| `trait_calls_before_start_refuse_typed` | YES | next_event/seek/seekability pre-start + empty-source refusal at Open stage. |
| `seek_lands_on_the_last_frame_at_or_before_the_target` | YES | Landing table + pure-query assert (walk unmoved, 5 packets after) + clamp-past-end + clamp-before-start. |
| `reanchor_resumes_the_walk_from_the_landing` | YES | Discontinuity count, packet window [3,4,5], negative re-clamp yields all 6. |
| `seek_out_of_range_only_when_no_frame_exists` | YES | Degenerate header+fragment file → typed out-of-range. |
| `fuzz_lite_*` (3 tests) | YES (for panic/hang/alloc-class bugs) | 512 bit-flip rounds with determinism cross-check every 64, every truncation prefix, 256 wild-size rounds with pool bound assert. Executed in baseline. Not a semantics oracle — the fixture differentials are. |
| `tests/ivf_fixtures.rs` (integration, 5) | YES | Real ffmpeg-authored bytes: stream params vs ffprobe, frame counts/pts ladders/payload **md5** vs ffmpeg, truncation derivatives, crafted beyond-file refusal. Strongest evidence in the family. |
| `containers::ivf::tests` (CLI, 6) | YES | Real `open_source` path incl. full-stream AV1 decode and `--start-time` skip. |
All 33 executed, 0 failed (orchrun :1691-1713, :2078-2089, :2796-2801).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Demux IVF elementary AV1 streams" implemented (win/linux/macos) | docs/feature-matrix.toml row `container-ivf` (:337-345) | REAL | Executed: demux unit 22/22 + ivf_fixtures 5/5 (md5 vs ffmpeg) + CLI lane decode 6/6 in baseline (orchrun cites above). |
| "fixture-only evidence — ZERO ivf corpus lanes; VP8 typed v0 refusal" | same row, evidence string | REAL | VP8 refusal pinned (`start_refuses_every_malformed_shape_typed` VP80 case); no corpus lane exists (the demux suite's own comment, ivf.rs:198-199 "no `.ivf` file exists in the corpus (verified)"). Honest self-limitation. |
| "one tick = scale/rate seconds … packet pts are the container's own numbers, unrescaled" | module docs :18-33 | REAL | `tick_base` + `clean_walk` pts/base asserts. |
| "every packet carries keyframe = false — never invented; dts is Missing" | module docs :49-56 | REAL | tests.rs:172-174. |
| "a frame whose declared size runs past end-of-stream is the sticky typed truncation" | module docs :64-66; error.rs:99-101 | REAL | Observable replay pinned (tests.rs:296-300); mechanism is determinism (walk not advanced), same as the ADTS precedent — verified by tracing both. |
| "a size above the payload ceiling is the typed ResourceLimit refusal before any allocation is trusted" | module docs :66-68 | REAL | pool_bytes()==4 assert (tests.rs:317). |
| "seek/reanchor run a bounded header-only scan (SlowLinear … never reads payload bytes)" | module docs :70-78 | REAL | scan_landing reads exactly 12 bytes/iteration (:513); SeekSupport::SlowLinear; pure-query pinned. |
| "The declared frame count is advisory … never trusted to bound reads" | module docs :86-89 | REAL | Advisory test + walk reads only what exists. |

## 7. Dead / duplicated / unreachable code
- F-004's triple-pasted dead fallback (all three arms unreachable behind `!started` guards).
- The `StickyShape` mechanism is used only for `start` failures; walk-step failures (frame_too_large/frame_beyond_file/pts_overflow) bypass `fail()` and replay by determinism. Behaviorally equivalent for `next_event` (pinned by test) and — traced — no reanchor path can skip a terminal frame_beyond_file damage (a frame overrunning EOF is by definition the last claim; the scan lands at-or-before it). Consistent with ADTS. Recorded as observation, not a finding.
- Nothing unreachable: the module is compiled unconditionally (demux lib.rs) and wired through the CLI host.

## 8. Dependencies & flags
- Reuses `avi::header::DamageLog` and `avi::riff::{SourceReader, PayloadSlots}` — cross-family coupling of IVF onto the AVI card's reader; deliberate (comment :98-99), and the shared reader is where F-001's defense belongs.
- No feature gates anywhere in the module (rg over mod.rs: none) — the IVF lane ships in every build of `demux`.
- `PayloadSlots::new(2, MAX_FRAME_BYTES)`: payloads in (4 MiB, 16 MiB] spill to a fresh exact-size Vec per frame (riff.rs:654-670) — unpooled by design; `pool_bytes()` stays honest ("bounded by 2 × MAX_FRAME_BYTES"). Legal 4K-AV1 frames up to 16 MiB pay one alloc + copy each; bounded, documented in riff.rs, noted here for the record (no finding).

## 9. Open questions for the next agent
- Should `SourceReader` itself grow a `read_exact_at` (typed truncation on short read) so every demuxer stops hand-rolling the ADTS-style length check? F-001 would then be structurally closed family-wide (adts, ivf, and whichever future cards copy this idiom).
- `crates/codecs/tests/{av1,vp9}_differential.rs` construct `IvfDemuxer` directly — if those suites assert demux behavior, they double as reachability evidence; a codecs-wave agent should confirm they ran in baseline (av1_differential appears at orchrun :850).
