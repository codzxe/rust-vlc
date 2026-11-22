# AUDIT: crates/demux/src/mkv/mod.rs

| Field | Value |
|---|---|
| Auditor | W05-A03 |
| Date | 2026-11-12 |
| Lines / functions | 1045 / 32 (incl. trait impl) |
| File verdict | REAL (with one S1 correctness bug on valid >4 MiB clusters) |
| Reachable from shipped entry point? | YES — `lumen` CLI → `apps/cli/src/containers/mod.rs:57` `("matroska" => mkv::open(stream, opts))` → `apps/cli/src/containers/mkv.rs:147` `MkvDemuxer::new(stream)` → `Demuxer::start`/`next_event`/`seek`/`reanchor` (all in this file). |
| Findings | S0:0 S1:1 S2:3 S3:3 S4:1 |

Commands run (adaptation: no cargo/npm/node; static analysis + `sed -n` quote verification only):
- `sed -n 'a,bp' crates/demux/src/mkv/mod.rs` for every quote below (all verified verbatim)
- Read cross-file evidence: `crates/demux/src/mkv/ebml.rs` (full), `crates/demux/src/mkv/cues.rs` (full), `crates/demux/src/mkv/header.rs` (scale/codec sections), `crates/plugin-api/src/demux.rs` (full), `crates/demux/src/avi/riff.rs:588-687` (PayloadSlots::fill), `apps/cli/src/containers/mkv.rs`, `apps/cli/src/pipeline_host.rs:376`, `apps/web/Cargo.toml`, `apps/web/src/open.rs`
- Run-dependent claims cite the W00 baseline (`audit/waves/wave-00/BASELINE-SUMMARY.md`) or are marked UNVERIFIED.

## 1. One-paragraph verdict

This file is the real MKV/WebM demuxer core — a genuine EBML walk with bounded windows, a packet pool, typed refusals, damage accounting and cue-based seek. The parsing discipline is far above the repo's stub average: every declared size is checked against its enclosing bound, allocations are capped (`MAX_BLOCK_BYTES` 32 MiB, pool ceiling 4 MiB, cue table 65 536), and timestamp math is checked. But it contains one S1 correctness bug that aborts playback of valid files: a block header straddling the 4 MiB cluster-window boundary is misclassified as stream truncation (F-001), and clusters larger than 4 MiB are common in high-bitrate MKV remuxes and live WebM. Around it sit a latent division-by-zero in a public accessor (F-002), a cue-table ceiling that kills file open instead of the documented degrade (F-003), and an unbudgeted open-time walk (F-004).

## 2. What actually works (verified)

- `seek()` correctly does NOT move the walk — verified against the trait contract: `crates/plugin-api/src/demux.rs:72-75` "Does **not** move the read position" and lines 93-98 make `reanchor` the acting half. The compute-then-commit split (comment "D4" at mod.rs:1031-1034) is implemented as documented.
- Vint parsing cannot overflow: `ebml.rs:179-244` caps id vints at 4 octets and size vints at 8, rejects zero lead bytes; `ElementHeader::declared_end` uses `checked_add` (ebml.rs:418-424).
- Window-load arithmetic cannot underflow: `load_next:517-519` `(window_end - el.data_pos)` is safe because `parse_header_at` guarantees `data_pos ≤ bound` (full-header-parse requirement) and `window_end ≥ data_pos` (hdr_len ≤ 12 < MAX_CLUSTER_WINDOW).
- Allocations are bounded end to end: block size refused past 32 MiB (line 644), frames past 16 MiB damage-skipped (line 802), pool spill `vec![0u8; want]` in `riff.rs:660` is called only with `want ≤ 16 MiB` from `emit_block`.
- Unknown-length/unseekable/empty sources refused typed at construction (ebml.rs:282-290).
- Codec map (header.rs:779-848) recognizes V_MPEG4/ISO/AVC, ASP, HEVC, V_MS/VFW/FOURCC(+FFV1 sniff), VP8/VP9, AV1, MJPEG, FLAC, Opus, Vorbis, ALAC, AC3/EAC3, DTS, TrueHD, A_AAC*, A_MPEG/L3, L2, PCM INT/FLOAT, S_TEXT/{UTF8,ASS,SSA,WEBVTT}; unknown CodecIDs pass through raw for the decode seam to refuse.
- Verified by reading the CLI wiring (`apps/cli/src/containers/mkv.rs:147`) — actual decode-through-CLI not re-run here; W00 baseline ran the release build (PASS) but exercised MP4, not MKV.

## 3. Findings

### F-001 [S1] Block header straddling the 4 MiB window boundary is misclassified as stream truncation → valid files abort playback
Where: crates/demux/src/mkv/mod.rs:599-631 (bug), amplified fatal at 973-987
Code:
```rust
            if window.cursor >= window.base + window.data.len() as u64 && window.cursor < window.end
            {
                let from = window.cursor;
                let to = window.end.min(from + MAX_CLUSTER_WINDOW);
```
and the failure arm:
```rust
            let Some((id, size, hdr_len, unknown)) = parse_header_mem(&window.data[offset..])
            else {
                // Trailing partial header: the cluster's declared end
                // crosses the stream — typed truncation.
                break Err(error::truncated(
```
What is wrong: The window slides only when `cursor` is **at or past** the window's data end. When the cursor sits **inside** the window with fewer bytes remaining than the next header needs (headers are 2-12 bytes), `parse_header_mem` returns `None` and the code mints a `Truncated` error — even though the cluster continues past the window (`cursor < window.end`) and all bytes exist in the file. `parse_header_at` (ebml.rs:450-451) solves exactly this by reading a 12-byte window; the in-window walk has no equivalent.
Why it matters: Any cluster larger than `MAX_CLUSTER_WINDOW` (4 MiB) is walked in consecutive windows (doc comment at lines 70-71 promises "huge or unknown-size clusters walk in consecutive windows"). A block header whose start falls within the last 11 bytes of a window edge triggers the spurious `Truncated`. In `next_event:973-987` that error is **fatal** whenever `reader.length() <= segment_end` — the normal case for any file whose Segment is the last element (`segment_end = min(declared_end, length)` equals `length`). Result: a valid MKV whose cluster exceeds 4 MiB — a 40 Mbps 4K remux with ~5 s clusters (~25 MiB clusters), or a live-style WebM with an unknown-size cluster clamped to segment end — hard-fails demux mid-playback. Where `length > segment_end` (trailing bytes) it degrades to skipping the rest of the cluster (up to megabytes of packet loss).
What correct looks like: Slide (or extend the read) when the remaining window bytes cannot hold a maximal header — e.g. `if window.data.len() - offset < 12 && window.end > window.base + window.data.len() as u64 { slide }` — or carry a 12-byte overlap between windows the way `es/annexb.rs:76-80` documents ("no start code or header straddles two windows unseen"). A regression test needs a cluster > 4 MiB with a header forced to the boundary; no test in `crates/demux/tests/` constructs a cluster larger than `MAX_CLUSTER_WINDOW` (verified by grep — no test references it).
Confidence: HIGH — Verification: read (control-flow derivation through `load_next:516-522` window sizing, the slide at 599/647, and the fatal-propagation guard at 979-987; execution blocked: no cargo in this audit lane).

### F-002 [S2] `chapter_targets` divides by `timestamp_scale` with no zero guard — division-by-zero panic on a malformed file
Where: crates/demux/src/mkv/mod.rs:296 and 301
Code:
```rust
            let Some(start_ticks) = i64::try_from(atom.start_ns / scale).ok() else {
                continue;
            };
            let end = atom
                .end_ns
                .and_then(|ns| i64::try_from(ns / scale).ok())
```
What is wrong: `scale = u64::from(self.info.timestamp_scale)` (line 277). `parse_info` deliberately stores a zero `TimestampScale` ("a zero `TimestampScale` is stored as-is", header.rs:386-388) and surfaces it later via `SegmentInfo::time_base()` — so `open()` parses **Chapters before** validating the scale (chapters collected at mod.rs:425, scale validated at mod.rs:440). `chapter_targets` has no `started` check and no zero guard; with scale 0 the integer division panics. The same file guards the identical hazard elsewhere with `checked_div` (see quote below) — the guard was simply not applied here.
Why it matters: A file with `TimestampScale=0` fails `start()` but leaves `chapters` populated; any caller that retains the errored demuxer and calls this `pub fn` panics (`attempt to divide by zero`). Current reachability: **no shipped caller** — grep shows only `crates/demux/tests/mkv_extras_wiring.rs` (4 call sites) and docs; the doc comment at 270-274 admits "the command layer does not exist yet, so nothing downstream reads it today". So: latent pub-API panic, unreachable from shipped entry points today.
What correct looks like: `scale.checked_recip`-style guard: `let scale = u64::from(...); if scale == 0 { return Vec::new(); }` or route through `self.base`.
Contrast (verified, mod.rs:790-793):
```rust
                // A zero scale is the typed honest absence (checked
                // division refuses it rather than panicking).
                let scale = u64::from(self.info.timestamp_scale);
                let ticks = ns.checked_div(scale)?;
```
Confidence: HIGH — Verification: read (panic path derived; `parse_info` zero-storage and `time_base` refusal read in header.rs:109-120, 384-419).

### F-003 [S2] Cue-table ceiling is fatal at open, contradicting the documented degrade-to-`SlowLinear`
Where: crates/demux/src/mkv/mod.rs:416-422
Code:
```rust
                ebml::ID_CUES => {
                    self.cues = Some(cues::parse_cues(
                        &mut self.reader,
                        &child,
                        self.segment_end,
                    )?);
                }
```
What is wrong: `cues::parse_cues` returns `Err(cue_table_too_large)` past 65 536 entries (cues.rs:91-96). The `?` here propagates it out of `open()` → `start()` fails → **the whole file is refused**. cues.rs's own contract (lines 10-14) says the demuxer "degrades to `SeekSupport::SlowLinear` — never an unbounded allocation". The code does not degrade; it refuses to open the file at all.
Why it matters: A valid (if unusually dense) or hostile file with >65 536 cue points — ~18 h at one cue per second, or trivially crafted — cannot be opened at all, when the correct behavior (and the documented one) is to drop the index and fall back to the linear scan that `landing()` already implements. The refusal also contradicts the file's own damage-tolerance posture (degenerate tracks are skipped, not fatal).
What correct looks like: `match cues::parse_cues(...) { Ok(ix) => self.cues = Some(ix), Err(e) if is_too_large(&e) => { /* damage-count, fall back */ } Err(e) => return Err(e) }`.
Confidence: HIGH — Verification: read (`parse_cues` ceiling and error kind read in cues.rs:91-96; doc claim at cues.rs:8-14).

### F-004 [S2] `open()`'s top-level walk has no budget — unbounded positional reads + per-element allocations on hostile files
Where: crates/demux/src/mkv/mod.rs:377-433
Code:
```rust
        while child_pos < self.segment_end {
            let Some(child) = ebml::parse_header_at(&mut self.reader, child_pos, self.segment_end)?
            else {
                break;
            };
```
What is wrong: The walk visits **every** top-level Segment child to the segment end (the comment at 334-339 admits the early break was removed so trailing metadata is found). Each iteration costs one `parse_header_at` = one positional seek + one 12-byte read + one fresh `Vec<u8>` allocation (ebml.rs:336-348 `read_prefix_at` → `vec![0u8; want]`). There is no element-count or byte budget — unlike the sibling no-cues seek scan, which is budgeted (`MAX_SEEK_SCAN_BYTES`, lines 82-84, 865-867).
Why it matters: A hostile ~100 MB file of micro-elements (each ≥ 2 bytes) forces on the order of millions of seek+read+alloc operations inside `start()` before a single packet is produced — an unbounded work amplification at open, with no typed `TooSlow` escape. Honest files (thousands of clusters) pay a bounded, acceptable cost; the asymmetry is a DoS seam the codebase itself guards against elsewhere.
What correct looks like: A scanned-bytes/element budget in this loop with the same `scan_too_slow` refusal (or: stop the walk once first cluster + Info + Tracks are known and fetch trailing Tags/Attachments/Chapters lazily on first access).
Confidence: MEDIUM — Verification: read (cost model derived from ebml.rs read path; no execution to measure amplification).

### F-005 [S3] `size as usize` narrowing truncates on 32-bit targets (wasm32 compiles this crate; no shipped wiring reaches it)
Where: crates/demux/src/mkv/mod.rs:573 and 731-737
Code:
```rust
            at += hdr_len + size as usize;
```
and
```rust
                    let cstart = at + chdr_len;
                    let cend = cstart + csize as usize;
                    if cend > payload.len() {
```
What is wrong: EBML sizes are u64 (up to 2^56). On a 32-bit target `as usize` takes the low 32 bits. In `packets_from_element` a crafted `csize` whose low 32 bits make `cstart + (csize as usize)` wrap below `cstart` passes the `cend > payload.len()` check and then `&payload[cstart..cend]` (line 741) panics with start > end; in `block_start_after_timecode` a wrapped `at` can re-scan window bytes (adversarial cycling). On 64-bit these paths are exact — `next_block_packets` line 644 rejects `size > MAX_BLOCK_BYTES` (32 MiB, fits u32) before its `as usize` at line 666 — but the BlockGroup loop (731-737) and the timecode scan (573) apply **no ceiling** before the cast.
Why it matters: `apps/web/Cargo.toml:64` depends on `demux` unconditionally for the wasm32-unknown-unknown target, so this code is compiled 32-bit today. It is UNREACHABLE from a shipped entry point right now because `apps/web/src/open.rs` wires only AVI (line 25 `use demux::avi::AviDemuxer`; other probe ids get the "no web-host wiring yet" refusal at open.rs:243). Latent panic the moment MKV is wired on web.
What correct looks like: `usize::try_from(size)` with the typed `bad_structure`/`truncated` refusal, exactly as cues.rs:124 already does (`usize::try_from(child.size).map_err(...)`).
Confidence: MEDIUM — Verification: read (cast semantics; wasm32 dependency and AVI-only wiring verified in apps/web).

### F-006 [S3] `Recovered.skipped` reports the cumulative file total, not this recovery's bytes — budget accounting double-counts
Where: crates/demux/src/mkv/mod.rs:930-937 (same pattern at 963-969 and 988-997)
Code:
```rust
                if self.cluster_damage > 0 {
                    self.damage_total += self.cluster_damage;
                    self.cluster_damage = 0;
                    return Ok(DemuxEvent::Recovered {
                        skipped: self.damage_total,
                        reason: "unknown-track-or-oversize-blocks",
                    });
                }
```
What is wrong: `skipped` is `self.damage_total` — the running total across the whole file. The trait defines the field as "Bytes or blocks skipped by the resynchronization" for **this** event and says "the engine counts it against the demux recovery budget" (plugin-api/src/demux.rs:152-158). A second damaged cluster re-reports the first cluster's bytes; an engine summing `skipped` over events inflates the budget consumption.
Why it matters: Recovery-budget observability is wrong for every file with more than one damaged cluster; a budget-capped engine would prematurely (or never) trip. The per-event value (`skipped` local at line 990 in the Err arm) is even computed and then discarded in favor of the cumulative.
What correct looks like: report `skipped: this_event_delta`.
Confidence: HIGH — Verification: read (trait field semantics at plugin-api/src/demux.rs:154-158).

### F-007 [S3] Doc comment's decode-availability claim is stale/false for AV1 (and understates the registry)
Where: crates/demux/src/mkv/mod.rs:17-23
Code:
```rust
//! the time of this card the registry decodes **Vorbis, Opus (audio)
//! and MJPEG/H.264-in-avcC (video)** — **VP8/VP9 and AV1 WebM video
//! are demuxed (packets + timestamps + seek) but NOT decodable**
//! (typed `Unsupported` at the decode seam, never a silent skip at
```
What is wrong: The shipped CLI decode registry lands av1: `apps/cli/src/pipeline_host.rs:376-377` — "no decoder for `{other}` in this build (landed: mjpeg, pcm, flac, mp3, mp2, vorbis, opus, aac-lc, alac, adpcm, av1, h264)". So "AV1 ... NOT decodable" is false for the current build, and the "Vorbis, Opus and MJPEG/H.264" list omits flac/mp3/mp2/aac-lc/alac/adpcm. (VP8/VP9 genuinely remain outside the landed list — that part holds.)
Why it matters: This is exactly the class of stale capability claim the audit exists to flag; the demuxer's own header maps `V_AV1 → "av1"` (header.rs:807) so an AV1 WebM track is demuxed and decodable end-to-end in the CLI build, contradicting the comment a reader would use to predict behavior.
What correct looks like: Point at the registry as the single source of truth without enumerating it (the sentence's own rule), or keep the enumeration in sync.
Confidence: HIGH — Verification: read (pipeline_host landed-codec list; descriptor map in header.rs). Cross-check vs W00 baseline: the baseline's "no decoder in this build (first vertical: mjpeg + pcm)" quotes `apps/cli/src/bench.rs:334`, which hardcodes only mjpeg+pcm for the **bench** lane — that message is bench-specific, not the play path; the play path's list is pipeline_host.rs:376.

### F-008 [S4] Dead defensive code, unused parameter, unused cue field
Where: crates/demux/src/mkv/mod.rs:941-947, 854+882, 833, and 840-848
Code:
```rust
                Ok(Some(mut packets)) => {
                    if packets.is_empty() {
```
and
```rust
        let _ = payload;
        Ok(packets)
```
What is wrong: (a) The `packets.is_empty()` branch in `next_event` is unreachable: `next_block_packets` line 679-685 `continue`s on empty before ever breaking with `Some(empty)`. (b) `landing`'s `let save = self.walk; … self.walk = save;` (lines 854, 882) restores a value the scan never modifies — misleading "restore" code that suggests movement that does not happen. (c) `emit_block`'s `payload` parameter is accepted and explicitly discarded (`let _ = payload;`). (d) `CuePoint.relative_pos` is parsed and stored (cues.rs:148, 180) but `landing` ignores it — seek precision inside large clusters is discarded (correct-but-coarser, and dead data at this seam).
Why it matters: Noise that hides real invariants; the dead empty-check in particular suggests a state that cannot occur, which future edits may "fix" incorrectly.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `probe` | 89-93 | REAL | — | 4-byte EBML magic check; `header.len() >= 4` guard prevents indexing panic. Reached via CLI container registry sniff. |
| `struct ClusterWindow` | 96-107 | REAL | — | Plain data holder. |
| `MkvDemuxer::new` | 163-190 | REAL | — | Defers all parsing to `start`; `TimeBase::new(1,1)` placeholder replaced in `open`. Refuses nothing itself (reader does). |
| `doctype` | 193-196 | REAL | — | Accessor. |
| `segment_info` | 199-202 | REAL | — | Accessor. |
| `track_entries` | 206-209 | REAL | — | Accessor; the CLI host consumes it (`containers/mkv.rs:13`). |
| `cue_index` | 212-215 | REAL | — | Accessor (tests/diagnostics only per doc). |
| `tags` | 221-224 | REAL | — | Accessor; retained from `start`. |
| `attachments` | 230-233 | REAL | — | Accessor; bytes available but no consumer (doc admits issue #17 owns the font consumer). |
| `chapters` | 239-242 | REAL | — | Accessor. |
| `title` | 248-254 | REAL | — | `Info.Title` then global `TITLE` tag; correct fallback order. |
| `track_uid` | 261-264 | REAL | — | `entries.get(handle as usize)` — no panic; out-of-range → None. |
| `chapter_targets` | 275-313 | REAL (defect) | S2 | F-002: `/ scale` div-by-zero when scale=0 (latent; no shipped caller). Hidden-atom skip and i64 overflow guards correct. UNREACHABLE from shipped entry points today (tests only). |
| `damage_bytes` | 316-319 | REAL | — | Honesty counter accessor. |
| `pool_bytes` | 321-325 | REAL | — | Pool footprint accessor. |
| `open` | 340-466 | REAL (defects) | S2/S2 | F-003 (cue ceiling fatal at 417-421), F-004 (no walk budget). DocType policy: only `matroska`/`webm` accepted, else typed refusal — verified honest. Unknown-size Segment → clamps to length. Multiple Info/Tracks: last wins. |
| `collect_tags` | 471-475 | REAL | — | Merges in file order. |
| `collect_attachments` | 480-486 | REAL | — | Merges + folds damage. |
| `collect_chapters` | 489-495 | REAL | — | Merges + folds damage. |
| `load_next` | 499-549 | REAL | — | No underflow at `(window_end - el.data_pos)` (derived: hdr_len ≤ 12 < window). Unknown-size non-cluster masters skipped to segment end (streaming form). |
| `block_start_after_timecode` | 557-579 | REAL (defect) | S3 | F-005: `size as usize` unbounded before cast (line 573) — 32-bit wrap latent. Unknown-size child inside cluster → typed bad_structure. |
| `next_block_packets` | 590-693 | REAL (defect) | S1 | F-001: straddle gap between slide-at-599 and parse-at-623. Window restored on every exit (verified all break paths assign `self.window`). `size > MAX_BLOCK_BYTES` refusal at 644; `data_end` overflow impossible (u64, size ≤ 2^56). |
| `packets_from_element` | 697-760 | REAL (defect) | S3 | F-005: `csize as usize` (731-737) 32-bit truncation → slice panic latent. SimpleBlock path delegates to `cluster::parse_block_with`; BlockGroup child loop strictly advances (hdr_len ≥ 2) — no infinite loop on 64-bit. Multiple Blocks in one group: last wins (illegal input tolerated). |
| `emit_block` | 763-835 | REAL | S4 | Checked timestamp math (`absolute_ticks`, `checked_add` + `saturating_mul`) — no overflow panic. `entries[ordinal]` index safe by construction (map built from entries). Frame ceiling 16 MiB; unknown track → damage-skip, never a fabricated track. `payload` param unused (F-008c). |
| `landing` | 840-884 | REAL | S4 | Cue branch: `checked_add` + `< segment_end` filter — cluster pos out of range → typed refusal. Linear branch budgeted (F-004 contrast). Dead `save`/restore (F-008b). Ignores `relative_pos`. Cue target before first cue point → `seek_out_of_range` (VLC would land at first cluster; coarser but honest). |
| `target_ticks` | 888-896 | REAL | — | `Missing` start → 0 (seek to head); incompatible base → typed refusal. |
| `parse_header_mem` | 902-906 | REAL | — | In-memory twin of `parse_header_at`; no bounds beyond slice lengths — callers bound. |
| `Demuxer::start` | 909-915 | REAL | — | Idempotent; `tracks.clone()` per call (acceptable). |
| `Demuxer::next_event` | 917-1001 | REAL (defects) | S1/S3 | F-001 fatal propagation guard (979-987) correct for real truncation but makes the straddle error fatal; F-006 cumulative `skipped`; F-008a dead empty-check. Recovery skips to cluster end then continues — livelock fix (doc 584-589) holds: window always restored. |
| `Demuxer::seek` | 1003-1013 | REAL | — | Pure query per contract (plugin-api demux.rs:72-75) — verified correct, NOT a bug. |
| `Demuxer::seekability` | 1015-1024 | REAL | — | Fast{indexed} iff cues parsed; SlowLinear otherwise; None before start. |
| `Demuxer::reanchor` | 1026-1044 | REAL | — | Commits walk/window/pending/eos; pushes `Discontinuity`; D4 compute-then-commit holds (landing never moves walk). |

## 5. Test honesty (tests for this file)

This file has no `#[cfg(test)]` module of its own. Its behavior is exercised by `crates/demux/tests/mkv_fixtures.rs`, `mkv_corpus.rs`, `mkv_extras_wiring.rs`, `mkv_editions_tags.rs`, `mkv_attachments_chapters.rs` (this wave's B-tier files) and the W00 baseline (`cargo test --workspace`: 1751 pass / 2 fail (golden drift) / 25 ignored — run-dependent, cited from baseline, not re-run). Key gaps proven by grep: no test anywhere constructs a cluster larger than `MAX_CLUSTER_WINDOW` (so F-001's slide paths at lines 599/647 are **untested**), and no test drives `chapter_targets` with a zero `TimestampScale` (F-002 untested).

## 6. Doc claims touching this file

| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "VP8/VP9 and AV1 WebM video are demuxed (packets + timestamps + seek) but NOT decodable" | mod.rs doc comment, lines 17-23 | FALSE (AV1 part) | `apps/cli/src/pipeline_host.rs:376` lists `av1` among landed decoders; header.rs:807 maps `V_AV1 → "av1"`. VP8/VP9 part holds (not in landed list). F-007. |
| "huge or unknown-size clusters walk in consecutive windows" | mod.rs doc comment, lines 70-71 | SHALLOW | The consecutive-window walk exists but is broken at every window edge where a header straddles (F-001); no test covers a >4 MiB cluster. |
| "Cluster payloads are walked in bounded windows … zero-alloc per packet" | mod.rs doc comment, lines 25-37 | REAL-UNPROVEN | Pool-slot read path (`read_prefix_into_at` + `PayloadSlots::fill`) verified by reading riff.rs:588-671 and ebml.rs:361-370; the zero-alloc steady state is asserted by `tests/zero_alloc_read_chain.rs` (AVI-shaped; MKV lane UNVERIFIED — not run here). |
| "the registry decodes Vorbis, Opus (audio) and MJPEG/H.264-in-avcC" | mod.rs doc comment, lines 19-20 | REAL-UNPROVEN | Consistent with pipeline_host.rs:376 landed list (which is larger); decode execution not re-run in this lane. |
| chapters "seek targets derived" / "nothing downstream reads it today" | mod.rs doc comment, lines 266-274 | REAL | Grep confirms: only tests call `chapter_targets`; no engine/CLI consumer exists. The bracketed PROPOSAL admission is honest. |

## 7. Dead / duplicated / unreachable code

- F-008 items (dead empty-check, dead save/restore, unused `payload` param).
- `CuePoint.relative_pos`/`CuePoint.track` parsed and retained but unused by `landing` (mod.rs consumption seam) — coarse-but-correct seek, dead data.
- No dead modules; every `pub mod` child is referenced (`editions` via `chapters`, all others directly).
- `chapter_targets` itself is currently UNREACHABLE from shipped entry points (tests only) — kept as a documented seam, not fraud (the doc says so).

## 8. Dependencies & flags

- No feature flags gate this file: `crates/demux/Cargo.toml` builds `mkv` unconditionally (`pub mod mkv`, lib.rs:50) in every workspace member including `apps/cli` and `apps/web` (wasm32). No cfg-gating to check in the release workflow.
- Runtime dependency on `plugin_api::SourceStream` (seekable + length-known required — typed refusal otherwise); `media-types` time types; `PayloadSlots` borrowed from `crate::avi::riff` (cross-family reuse, not duplication).

## 9. Open questions for the next agent

- `cluster.rs` (this agent's next file): does `parse_block_with` bound lacing frame sizes against the payload (F-001's blast radius depends on frame reads being positional)? Does `absolute_ticks` check `cluster_timecode + relative` overflow?
- `cues.rs` is another agent's file: `parse_cue_point` line 131 `uint_from_be(&bytes).map(|v| v as i64)` — a CueTime > i64::MAX wraps negative, poisoning `landing_before`'s sorted order (mod.rs consumes it unguarded). Recommend they file it.
- Engine side: does any consumer sum `DemuxEvent::Recovered.skipped` into a budget (F-006's real impact)? Files: engine/pipeline crates, not in my assignment.
- Who consumes `MAX_SEEK_SCAN_BYTES`'s `TooSlow` error at the CLI surface — is the no-cues seek refusal user-visible and honest? (`apps/cli` seek paths.)
