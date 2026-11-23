# AUDIT: crates/demux/src/mkv/mod.rs
| Field | Value |
|---|---|
| Auditor | W05-A06 |
| Date | 2026-11-23 |
| Lines / functions | 1045 / 28 (incl. trait impl) |
| File verdict | REAL (one S1 hang + one S2 shipped wrong-behavior; core walk/seek/recovery logic genuine and mostly tested) |
| Reachable from shipped entry point? | YES — `apps/cli/src/containers/mod.rs:57 "matroska" => mkv::open(stream, opts)` → `apps/cli/src/containers/mkv.rs:147 MkvDemuxer::new`; desktop lane reaches the same host via `apps/desktop` → `ln::host::CliHost` (lumen-cli dep). NOT reachable in the web wasm lane (apps/web/src/open.rs:25 wires `AviDemuxer` only — zero mkv refs). |
| Findings | S0: 0 S1: 1 S2: 1 S3: 3 S4: 4 |
| Commands run | `cargo test -p demux --lib mkv::` → 50 passed / 0 failed; standalone probe `audit/scratch/probe6_mkv_w05a06.rs` (rustc vs target/debug/deps rlibs) → lacing/keyframe/damage cases printed, `livelock` case **killed by watchdog exit 124**; `probe7_mkv_wasm32_sim.rs` → panic "slice index starts at 2 but ends at 1"; ffprobe cross-check of scratch lacing fixtures (see cluster.rs audit) |

## 1. One-paragraph verdict
This is a real, carefully-structured Matroska core: bounded windows, typed truncation-vs-contained-lie classification, window restore on every error exit, compute-then-commit seek, damage honesty counters, and a start() walk that correctly hunts trailing metadata (ffmpeg's layout). But it ships one executed hang and one executed wrong-flag defect: any block whose declared size exceeds the 4 MiB window but stays under the 32 MiB block gate makes `next_event()` spin forever re-reading the identical window (probe killed by an 8 s watchdog, exit 124 — S1), and every `BlockGroup` packet is emitted with `keyframe: false` because the "derive from ReferenceBlock absence" the comments promise is never performed (executed: `key=[false]` — S2). The lacing frames this file emits come from `cluster.rs` and inherit the W04-A06 lacing inversion (adjudicated in the cluster.rs audit file: all three codings read the count byte as literal frame count; ffmpeg-legal laces misparse silently — S1, executed here too).
## 2. What actually works (verified)
- `cargo test -p demux --lib mkv::` → `test result: ok. 50 passed; 0 failed`.
- probe6 `control` case: 3 MiB single-frame block in one window → `packets=1 sizes=[3145728]` — window bounds, block parse, pool fill, EOS all correct.
- probe6 cases fixed/xiph/ebml + BlockGroup + block-before-Timecode executed and printed (outputs below) — the demuxer walk, track mapping (`by_number`, unknown track damage-skipped), and recovery paths all run.
- Bounds discipline read-verified: block past cluster end → `error::truncated` (639-643); size > 32 MiB → `bad_structure` (644-646); unknown-size inside cluster → `bad_structure` (632-636); slice indices in `next_block_packets` all guarded by u64 window math before any `as usize` (619-672).
## 3. Findings
F-001 [S1]
Where: crates/demux/src/mkv/mod.rs:647-664 (with gate at 644-646)
Code:
```
            if data_end > window.base + window.data.len() as u64 {
                // Spans past the window but inside the cluster: slide
                // so the block's head is at the window start.
                let from = window.cursor;
                let to = window.end.min(from + MAX_CLUSTER_WINDOW);
```
```
            if size > MAX_BLOCK_BYTES {
                break Err(error::bad_structure(data_start));
            }
```
What is wrong: the slide sets `base = cursor` and re-reads `[cursor, cursor+4 MiB)` — but `cursor` never advances past the block header until the block parses. For a block with `hdr_len + size > MAX_CLUSTER_WINDOW` (4 MiB) and `size <= MAX_BLOCK_BYTES` (32 MiB) — a range the code explicitly accepts — the second iteration parses the same header at offset 0, `data_end > base + data.len()` is true again (size still > 4 MiB), and the slide re-issues the identical read: infinite loop, one seek + 4 MiB read + 4 MiB alloc per iteration. Executed: `timeout 8 ./probe6_bin livelock` → killed, **exit 124**, no output past the previous case; control 3 MiB block parses fine. Why it matters: `next_event()` never returns — 100 % CPU hang on any file with a >4 MiB block (large intra frames, laced high-rate audio blocks, or a 100-byte-crafted header declaring 5 MiB); the player freezes with no error, no damage note, no EOS. The demuxer's own gate says sizes up to 32 MiB are legal input. What correct looks like: slide must make progress — e.g. grow the window to cover the block (cap window reads at min(block span, MAX_BLOCK_BYTES)) or stream the frame directly without requiring it to fit a window (the pool read at 810-813 is already positional). Confidence: HIGH — Verification: ran (watchdog kill, exit 124).

F-002 [S2]
Where: crates/demux/src/mkv/mod.rs:751-753 (claim), 825 (use); crates/demux/src/mkv/cluster.rs:58-59 (claim), 217 (value)
Code:
```
                    // BlockDuration/ReferenceBlock: the keyframe
                    // derivation is ReferenceBlock-absence; duration
                    // rides DefaultDuration.
```
```
                keyframe: block.keyframe,
```
What is wrong: the comment claims the caller derives BlockGroup keyframes from ReferenceBlock absence; the caller never looks for `ID_REFERENCE_BLOCK` (the walk at 723-755 only handles `ID_BLOCK`) and `parse_block_with(..., simple=false)` sets `keyframe: simple && flags & FLAG_KEYFRAME != 0` = always false. Executed: BlockGroup block with no ReferenceBlock → `key=[false]`; identical payload as SimpleBlock with the keyframe bit → `key=[true]`. Why it matters: mkvmerge writes video with references as BlockGroups — in such files *every* video packet is marked non-keyframe, so any downstream keyframe gating (seek-join, drop-until-keyframe) sees a stream with zero keyframes. What correct looks like: track ReferenceBlock presence in the 723-755 loop and OR it into the block's keyframe before emit. Confidence: HIGH — Verification: ran.

F-003 [S3]
Where: crates/demux/src/mkv/mod.rs:731-737
Code:
```
                    let cstart = at + chdr_len;
                    let cend = cstart + csize as usize;
                    if cend > payload.len() {
                        return Err(error::truncated(
```
What is wrong: `csize` is an unbounded u64 (vint up to 2^56−1) truncated by `as usize` *before* the bound check. On a 32-bit-usize target (wasm32-unknown-unknown — `apps/web` depends on `demux` and is the wasm32 composition root) a crafted child size of 0xFFFF_FFFF wraps the add (`cstart.wrapping_add(0xFFFF_FFFF) = cstart − 1`), passes `cend > payload.len()`, and the subsequent `&payload[cstart..cend]` panics — executed natively as the exact expression with u32 standing for usize: `panicked ... slice index starts at 2 but ends at 1` (audit/scratch/probe7_mkv_wasm32_sim.rs). On 64-bit lanes the sum cannot wrap (≤ 2^56 + 32 MiB) and the check works. Reachability today: NOT reachable from a shipped entry point — apps/web/src/open.rs:25 wires only `AviDemuxer` (zero `mkv` references in apps/web/src), CLI/desktop are 64-bit — so this is a latent portability landmine, not a live S1. What correct looks like: `u64` math for `cend`, then `usize::try_from` after the bound check (the pattern `next_block_packets` already uses). Confidence: HIGH — Verification: ran (native simulation of the exact expression; wasm execution UNVERIFIED — no wasm runtime in sandbox).

F-004 [S3]
Where: crates/demux/src/mkv/mod.rs:74-80 (docs) vs 802-806 (behavior)
Code:
```
/// Per-packet pool ceiling; larger frames spill unpooled (block-
/// bounded by [`MAX_BLOCK_BYTES`]).
pub const MAX_PACKET_BYTES: usize = 4 * 1024 * 1024;
```
```
            if frame.len > MAX_PACKET_BYTES * 4 {
                // Beyond every sane ceiling: contained refusal.
                self.cluster_damage += frame.len as u64;
                continue;
            }
```
What is wrong: the documented contract is "larger frames spill unpooled, block-bounded by MAX_BLOCK_BYTES (32 MiB)"; the code drops any frame > 16 MiB (`MAX_PACKET_BYTES * 4`) as damage instead. A 17 MiB single-frame block is accepted by the 32 MiB block gate, walked, then silently discarded (only a `Recovered` note at cluster end). The 4× multiplier is undocumented and contradicts both constants' docs. What correct looks like: one ceiling — either spill up to MAX_BLOCK_BYTES or lower MAX_BLOCK_BYTES to the frame ceiling and document it. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/demux/src/mkv/mod.rs:552-556 (claim) — no matching accounting code
Code:
```
    /// Where blocks start inside a just-loaded window: the byte
    /// offset just past the `Timecode` element (blocks may legally
    /// precede it in malformed files; the spec mandates Timecode
    /// first — files that violate that lose those leading blocks to
    /// the skip, damage-counted).
```
What is wrong: "damage-counted" is false — `block_start_after_timecode` just returns the post-Timecode offset; leading blocks are never parsed and never touch `cluster_damage`. Executed: cluster `[SimpleBlock][Timecode][SimpleBlock]` → 1 packet, `recovered=0 damage=0`. Why it matters: the module's honesty counter under-reports skipped payload, and the doc overstates it. Confidence: HIGH — Verification: ran.

F-006 [S3]
Where: crates/demux/src/mkv/mod.rs:82-84, 854-868
Code:
```
/// The no-cues linear seek scan budget (bytes of cluster headers
/// walked before the honest `TooSlow` refusal).
pub const MAX_SEEK_SCAN_BYTES: u64 = 512 * 1024 * 1024;
```
```
            scanned += 12;
            if scanned > MAX_SEEK_SCAN_BYTES {
                break Err(error::scan_too_slow(MAX_SEEK_SCAN_BYTES, scanned));
            }
```
What is wrong: the budget counts 12 bytes per *element header*, not bytes walked — refusing only after ≈44.7 M header parses, each of which is a positional seek+read (`parse_header_at` → `read_prefix_at`). On a crafted file of tiny top-level elements the "bounded" scan performs tens of millions of I/O round trips before refusing. Same shape in `open()`'s start walk (one 12-byte positional read per top-level element across the whole segment — thousands of seeks on a cluster-dense movie, vs ffmpeg's sequential buffered walk). Correct would count actual bytes skipped (element extents) or cap iterations. Confidence: MEDIUM — Verification: read (I/O amplification not re-measured; both loops read as described).

F-007 [S4]
Where: crates/demux/src/mkv/mod.rs:841-850 (and cues.rs `relative_pos`, dead here)
Code:
```
        if let Some(index) = self.cues.as_ref() {
            if let Some(point) = index.landing_before(target_ticks) {
                let pos = self
                    .segment_data_start
                    .checked_add(point.cluster_pos)
```
What is wrong: (a) `CueRelativePosition` is parsed into `CuePoint.relative_pos` ("narrows inside it", cues.rs:32-34) but `landing` uses only `cluster_pos` — the field is dead at this seam and re-emission starts at the cluster head; (b) a target before the first cue point yields `None` → `seek_out_of_range` instead of clamping to the segment start (ffmpeg lands at cluster 0). Correct behavior for the common "seek to 0" case with cues that start at t>0 is a typed refusal. Confidence: HIGH — Verification: read (landing_before(-1) → None pinned by cues.rs test at 207).

F-008 [S4]
Where: crates/demux/src/mkv/mod.rs:739-749
Code:
```
                    if cid == ebml::ID_BLOCK {
                        let mut block = cluster::parse_block_with(
```
What is wrong: a BlockGroup with multiple `Block` children (illegal but parseable) overwrites `packets` — earlier blocks' packets vanish with no damage note (last-wins). Also `BlockDuration` is ignored entirely (packet duration always the track DefaultDuration). Both are spec-relevant elements on a legal path (mkvmerge writes BlockDuration for subtitles). Confidence: HIGH — Verification: read.

Cross-reference (root cause lives in cluster.rs, this file emits its output): `emit_block` iterates `block.frames` produced by `cluster::parse_block_with`; the lacing count semantics there are inverted vs Matroska/ffmpeg (count byte read as literal frame count, not frames−1). Executed in this probe: ffmpeg-legal fixed count=2 → `packets=2 sizes=[3,3]` (expect 3×2B), xiph count=2 → `[4,12]` (expect [4,5,6]), ebml count=2 → `[3,8]` (expect [3,3,4]) — all with `damage=0 recovered=0` (silent). Full adjudication in audit/files/crates/demux/src/mkv/cluster.rs.audit.md (S1, confirms W04-A06's escalation).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| probe | 89-93 | REAL | — | 4-byte EBML magic; wired via media-probe MkvSignature |
| ClusterWindow (struct) | 96-107 | REAL | — | data/base/cursor/end/timecode |
| MkvDemuxer::new | 163-190 | REAL | — | typed Unsupported for unknown-length/unseekable/empty source |
| doctype / segment_info / track_entries / cue_index | 192-215 | REAL | — | diagnostics getters; track_entries consumed by CLI harvest (mkv.rs:199) |
| tags / attachments / chapters / title / track_uid | 217-264 | REAL, UNREACHABLE-shipped | S4 | zero shipped consumers (W04-A08 sweep re-confirmed by rg: only demux tests) |
| chapter_targets | 276-313 | REAL, UNREACHABLE-shipped | S4 | ns→ticks; `atom.start_ns / scale` (296, 301) divides by `timestamp_scale`, and parse accepts a written 0 (header.rs:415 `Some(scale) if scale <= u64::from(u32::MAX)`); start() then fails at time_base() (440) but `chapters` is already populated — calling chapter_targets() on the failed demuxer would panic (÷0). No started-check, but zero shipped callers → latent |
| damage_bytes / pool_bytes | 316-325 | REAL | — | honesty observables |
| open | 340-466 | REAL | S4 | trailing-metadata walk genuine; per-element positional reads (F-006); unknown-size child handling sane |
| collect_tags / collect_attachments / collect_chapters | 471-495 | REAL | — | merge in file order, damage folded |
| load_next | 499-550 | REAL | — | window load, cluster skip, unknown-size tolerance |
| block_start_after_timecode | 557-579 | REAL | S4 | F-005 false "damage-counted" doc; `at += hdr_len + size as usize` unchecked add (wrap on crafted sizes — release wraps, no panic here since loop just terminates) |
| next_block_packets | 590-693 | REAL | S1 | F-001 livelock; window restore on all exits verified |
| packets_from_element | 697-760 | REAL | S2/S3/S4 | F-002 keyframe, F-003 csize cast, F-008 multi-Block/BlockDuration |
| emit_block | 763-835 | REAL | S3 | F-004 frame ceiling; DefaultDuration cadence math unit-correct (ns ÷ ns/tick); unknown-track damage-skip good; inherits lacing frames (cross-ref S1) |
| landing | 840-884 | REAL | S3/S4 | F-006 scan budget, F-007 relative_pos dead / before-first-cue refusal |
| target_ticks | 888-896 | REAL | — | Missing start = 0 |
| parse_header_mem | 902-906 | REAL | — | in-memory twin of parse_header_at |
| Demuxer::start | 909-915 | REAL | — | idempotent; clone of track table |
| Demuxer::next_event | 917-1001 | REAL | S1 | F-001 surfaces here; truncation-vs-recovery classification (979-987) coherent |
| Demuxer::seek | 1003-1013 | REAL | — | pure query per trait contract (plugin-api demux.rs:68-73 "Does **not** move the read position") — `_pos` discard is CORRECT |
| Demuxer::seekability | 1015-1024 | REAL | — | cues → Fast, else SlowLinear |
| Demuxer::reanchor | 1026-1044 | REAL | — | compute-then-commit (D4), Discontinuity pushed, eos reset |
## 5. Test honesty
No tests in this file (mkv tests live in sibling modules + tests/). Sibling/ integration coverage judged in their own audits; the 50 lib tests that exercise this file's paths pass.
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Cluster payloads are walked in bounded windows" | mod.rs:27-28 | REAL with a hole | window is bounded; the slide logic livelocks on >4 MiB blocks (F-001, executed) |
| "larger payloads spill unpooled but stay block-bounded by MAX_BLOCK_BYTES" | mod.rs:30-31, 74-76 | FALSE for >16 MiB frames | F-004: dropped as damage, not spilled |
| "the 5.X2b steady state is zero-alloc per packet" | mod.rs:31-37 | REAL | read path uses read_prefix_into_at into pool slot; scratch recycled (607-613, 715, 772) — except the F-001 loop's per-iteration 4 MiB Vec |
| "files that violate that lose those leading blocks to the skip, damage-counted" | mod.rs:554-556 | FALSE | F-005 executed: damage=0 |
| "keyframe derivation is ReferenceBlock-absence" | mod.rs:751-753 | FALSE | F-002 executed: key=[false] |
| "VP8/VP9 and AV1 WebM video are demuxed (packets + timestamps + seek) but NOT decodable" | mod.rs:19-23 | REAL-UNPROVEN | decode-side claim; demux side passes CodecID through (header.rs lane) — not re-executed here |
| EXTENSIONS is "the R3 registry row" | mod.rs:67-68 | MISLEADING | zero readers of demux::mkv::EXTENSIONS; media-probe keeps its own list (signature/mod.rs, registry_roundtrip.rs:248-250) — drift risk |
## 7. Dead / duplicated / unreachable code
- `EXTENSIONS` (mod.rs:68): zero consumers repo-wide (rg over crates/ apps/ tools/); the probe registry duplicates the list.
- tags()/attachments()/chapters()/title()/track_uid()/chapter_targets(): zero shipped consumers (test-only surface; confirmed W04-A08 + re-rg).
- `let _ = payload;` at 833: explicit dead parameter of emit_block (the pool re-reads positionally) — documented, harmless.
## 8. Dependencies & flags
- In-crate: mkv/{ebml,header,cluster,cues,error,tags,attachments,chapters}, `crate::avi::riff::PayloadSlots` (cross-family reuse of the AVI pool — layering reuse, noted, not a violation).
- std-only (HashMap, VecDeque); no feature gates; no unsafe; no unwrap/expect in production paths (the `unwrap_or` at 953 is post-extend, provably Some).
## 9. Open questions for the next agent
- Does any real corpus file (tests/fixtures corpus lane) contain a >4 MiB block that would have tripped F-001? The corpus lane is machine-local (W04-A08) — check when corpus-v4 is present.
- The engine side: does playback-engine gate on `Packet::keyframe` (F-002 blast radius)? If not, downgrade F-002's user impact; the field is still wrong at the API.
- mkv/fuzz.rs: does its input generator ever emit element sizes in the (4 MiB, 32 MiB] range with a matching cluster end? If yes the fuzz lane should have caught F-001 — check the harness bounds (fuzz.rs is another lane's file).
