# AUDIT: crates/demux/src/mkv/mod.rs#part2 (lines 523–1045)
| Field | Value |
|---|---|
| Auditor | W05-A10 |
| Date | 2026-11-21 |
| Lines / functions | 523 (of 1046; part2 = 523–1045, part1 = 1–522; boundary note: `load_next`'s head (:499–522) sits in part1 — the function is audited whole here because its cluster-window body (:508–549) is this range's) / 13 units |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/host.rs` open_session → `apps/cli/src/demux_host.rs:open_source` → `apps/cli/src/containers/mod.rs:57` `"matroska" => mkv::open` → `apps/cli/src/containers/mkv.rs:147-148` `MkvDemuxer::new`+`start()` → engine drives `next_event` (`crates/playback-engine/src/pipeline.rs:2141`) and `reanchor` (pipeline.rs:1499 → this range :1026 — unlike MP4, MKV *implements* the acting half); `--start-time` reaches `seek()` via mkv.rs:167) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 4 S4: 4 |
| Commands run | `rg -n "\.keyframe" crates/ apps/` (only non-test consumer: `crates/transcode/src/mp4mux.rs:299` sync rows; playback-engine/video-engine/cli: none); `sed -n '215,218p' crates/demux/src/mkv/cluster.rs` (`keyframe: simple && …` — hard false for Block); `sed -n '55,60p' …cluster.rs` (doc delegates the derivation "to the caller"); `sed -n '2155,2175p' crates/playback-engine/src/pipeline.rs` (engine ADDS each `Recovered.skipped` into `report.damaged`/`damaged_demux`); `sed -n '290,310p' + '560,570p' crates/transcode/src/mp4mux.rs` (stss written from sync rows); `rg -n "transcode" apps/cli/Cargo.toml apps/desktop-tauri/Cargo.toml Cargo.toml` (transcode NOT wired into any shipped binary — lib-only consumer); `rg -n "^test mkv::" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (50 ok; NO mod-level tests — state machine covered only via `crates/demux/tests/mkv_corpus.rs` + `containers::mkv`); `sed -n '2090,2102p' …log.txt` (`corpus_matroska_seek_roundtrip ... ok`, `corpus_matroska_differential_vs_ffprobe ... ok`); `rg -n "reanchor" crates/demux/src/mkv/mod.rs` (:1026 — present, contrast MP4); `sed -n` on every quoted range |

## 1. One-paragraph verdict
This range is the MKV/WebM playback engine — the cluster window walk, block
parsing, packet emission, and the full seek surface (`seek`/`seekability`/
`reanchor`) — and it is the crate's best-shaped demuxer: unlike MP4 it
implements the acting seek half (`reanchor` :1026-1044) behind its
`seekability()` advertisement, the window/scratch/pool recycling is genuinely
bounded and allocation-free in the steady state, and it is executed evidence:
50 `mkv::` unit tests, `corpus_matroska_seek_roundtrip` +
`corpus_matroska_differential_vs_ffprobe`, and 11 `containers::mkv` CLI-lane
tests all pass in the baseline. The defects cluster in the damaged-input and
metadata seams: `landing()`'s no-cues scan has two early-return paths that
skip the `walk` restore, so a malformed cluster mid-scan corrupts the
playback position through what is supposed to be a pure seek query (and
falsifies reanchor's D4 guarantee); the documented BlockGroup
keyframe-from-ReferenceBlock-absence derivation **does not exist** (every
Block-in-BlockGroup packet ships `keyframe: false`, contradicting two doc
comments); `Recovered` events report the *cumulative* damage total which the
engine *sums*, double-counting damage quadratically; and one corrupt block in
an unknown-size (streaming-form) cluster discards the rest of the segment.

## 2. What actually works (verified)
- Demux + seek executed green in baseline: `corpus_matroska_seek_roundtrip ... ok`
  and `corpus_matroska_differential_vs_ffprobe ... ok`
  (`orchrun-test-workspace-nofailfast.log.txt:2098-2100`, suite
  `crates/demux/tests/mkv_corpus.rs`); 50 `mkv::` submodule tests ok
  (:1714-1751); 11 `containers::mkv` tests ok incl.
  `corpus_matroska_lanes_match_the_ffmpeg_truth`,
  `h264_aac_fixture_opens_with_expected_facts`,
  `start_time_skips_packets_and_reports_indexed_seek`,
  `vorbis_lane_decodes_the_full_stream_from_the_laced_private`,
  `webm_vp9_and_opus_decode_from_the_private_head` (:2803-2811, :2900).
- The seek architecture is the crate's correct shape: `seekability()` answers
  from real state (cues → `Fast { indexed: true }`, else `SlowLinear`,
  :1015-1024) and `reanchor` (:1026-1044) implements compute-then-commit with
  a `Discontinuity` marker — the engine's in-session seek flow
  (pipeline.rs:1499) has a real acting half here, unlike MP4 (see my
  mp4#part4 F-001).
- The 5.X2b memory claims hold by trace: window reads are bounded
  (`MAX_CLUSTER_WINDOW` 4 MiB, :517, :602, :651), block payloads are borrowed
  from the window (:668-672), frames are read straight into the pool slot
  (:810-813), `block_scratch`/`pkt_scratch` are recycled (:715, :772-773,
  :948-952), and the window is restored on every `next_block_packets` exit
  incl. errors (:689-692) — the documented mid-cluster-damage livelock fix.
- Bounded ceilings enforced: block ≤ `MAX_BLOCK_BYTES` 32 MiB (:644-646),
  frames > 16 MiB damage-skipped (:802-806), scan budget `MAX_SEEK_SCAN_BYTES`
  (:866-868), unknown-size elements inside a cluster refused typed (:632-635).
- The contained-damage recovery is real: a structurally failed block skips
  only the cluster remainder and continues at the next top-level element with
  a `Recovered` event (:973-997), and a genuine stream truncation is typed
  rather than recovered (:979-987).

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/mkv/mod.rs:862 and :877 (with :882, :854)
Code:
```rust
            let Some(el) = ebml::parse_header_at(&mut self.reader, pos, self.segment_end)? else {
                break Ok(best);
            };
```
```rust
                match cluster::parse_timecode(&mut self.reader, &el, self.segment_end) {
                    Ok(tc) if tc.ticks <= target_ticks => best = Some((tc.ticks, pos)),
                    Ok(_) => {}
                    Err(e) => return Err(e),
                }
```
What is wrong: The no-cues landing scan saves the walk (`let save =
self.walk;` :854) and restores it after the loop (:882 `self.walk = save;`) —
but two paths return **before** the restore: the `?` on `parse_header_at`
(:862 — a malformed element header mid-scan) and `return Err(e)` (:877 — a
cluster whose `Timecode` fails to parse). Both leave `self.walk` pointing at
the scan's current position, arbitrarily deep into the file. `landing()` is
called from the pure query `seek()` (:1008) and from `reanchor`'s compute
phase (:1032): in the first case, a *query* silently relocates playback; in
the second, reanchor returns `Err` while having already moved the walk —
falsifying invariant D4 ("on `Err` the demuxer is left at its pre-seek
position with a valid parsing state", plugin-api/src/demux.rs:100-103) and
the function's own comment "…then commit (D4: nothing above moved the walk)"
(:1031-1033).
Why it matters: On a cues-less file with one damaged cluster, any seek query
(or a failed reanchor) throws playback to the damaged position and continues
delivering from there — state corruption through the documented-pure path,
the worst interaction class for the engine's seek rollback.
What correct looks like: Route both early exits through the same restore
(e.g. `break Err(e)` instead of `return Err(e)`, and bind the header parse to
a `break Err`), or run the scan on a local reader position only.
Confidence: HIGH — Verification: read (control flow traced through both
exits; callers :1008/:1032 traced).

### F-002 [S3]
Where: crates/demux/src/mkv/mod.rs:751-754 (with :814-815; crates/demux/src/mkv/cluster.rs:58-60, :217)
Code:
```rust
                    // BlockDuration/ReferenceBlock: the keyframe
                    // derivation is ReferenceBlock-absence; duration
                    // rides DefaultDuration.
                    at = cend;
```
```rust
            // BlockGroup keyframes: no ReferenceBlock (derive at the
            // group level; SimpleBlock carries the flag bit).
```
What is wrong: Two doc comments claim the Matroska keyframe rule for BlockGroup
blocks — "keyframe ⇔ no `ReferenceBlock` in the group" — is derived somewhere:
mod.rs:814-815 says "derive at the group level", and cluster.rs:58-60 says
"derived from `ReferenceBlock` absence **by the caller**". No such derivation
exists. `packets_from_element` walks the BlockGroup children and simply skips
every non-Block child (:751-754 — the comment sits directly above a bare
`at = cend;`), never recording whether a `ReferenceBlock` was seen;
`cluster::parse_block_with` hard-codes `keyframe: simple && flags &
FLAG_KEYFRAME != 0` (cluster.rs:217) — i.e. `false` for **every** Block
inside a BlockGroup, keyframe or not.
Why it matters: Files muxed with BlockGroups (mkvtoolnix when
BlockDuration/BlockAdditions are needed; older muxers) report zero keyframes.
The packet flag is metadata-lies today (the playback engine reads
`packet.keyframe` nowhere — rg over playback-engine/video-engine/cli), but
the one real consumer found, `crates/transcode/src/mp4mux.rs:299` + :565-567,
builds the output MP4's `stss` from these flags: an MKV(BlockGroup)→MP4 remux
writes a degenerate `stss` (no sync samples) — a broken-seek output file.
(transcode is not wired into any shipped binary — lib-only — hence S3, not
S2.)
What correct looks like: In the BlockGroup walk, set `saw_reference =
saw_reference || cid == ebml::ID_REFERENCE_BLOCK`, then
`block.keyframe = !saw_reference` before emitting (or pass it into
`parse_block_with`).
Confidence: HIGH — Verification: read (both claimed-derivation sites +
hard-code traced; consumer traced mp4mux.rs:299,567).

### F-003 [S3]
Where: crates/demux/src/mkv/mod.rs:933-936, :966-969, :993-997 (with :988-993)
Code:
```rust
                    let mut skipped = 0u64;
                    if let Some(window) = self.window.take() {
                        skipped = window.end.saturating_sub(window.cursor);
                        self.walk = window.end;
                    }
                    self.damage_total += skipped;
                    return Ok(DemuxEvent::Recovered {
                        skipped: self.damage_total,
                        reason: "cluster-parse-recovery",
                    });
```
What is wrong: The per-recovery delta is computed into `skipped` (:990),
added to `damage_total`, and then the **cumulative total** is what rides the
event (`skipped: self.damage_total`). All three `Recovered` mint sites do
this (:933-936 EOS arm, :966-969 cluster-exhaust arm, :993-997 recovery
arm). The engine *sums* the field: `report.damaged_demux =
report.damaged_demux.saturating_add(skipped); report.damaged = …add(skipped)`
(crates/playback-engine/src/pipeline.rs:2165-2167), and the trait doc says
the engine "counts it against the demux recovery budget" (demux.rs:152-155).
With N recoveries of δ bytes each, the engine records δ+2δ+…+Nδ = O(N²·δ)
— the damage report inflates quadratically and every prior recovery is
re-counted by each new event.
Why it matters: Corrupted observability/metrics on exactly the damaged files
the counter exists for; if the recovery budget ever becomes a cap, it
prematurely trips.
What correct looks like: `skipped: skipped` (the delta) at all three sites;
`damage_total` stays the demuxer-side accessor truth.
Confidence: HIGH — Verification: read (all three sites + engine sum traced).

### F-004 [S3]
Where: crates/demux/src/mkv/mod.rs:873-879
Code:
```rust
            if el.id == ebml::ID_CLUSTER {
                match cluster::parse_timecode(&mut self.reader, &el, self.segment_end) {
                    Ok(tc) if tc.ticks <= target_ticks => best = Some((tc.ticks, pos)),
                    Ok(_) => {}
```
What is wrong: The no-cues scan keeps walking to the segment end even after
passing the target — `Ok(_) => {}` (timecode > target) does not break. Since
`best` already keeps the *last* cluster ≤ target, every cluster after the
target contributes nothing but cost: a seek to 10 % of a 2 h cues-less file
reads every cluster header to EOF (positional header reads, 12 counted bytes
each). The `best`-semantics ("last ≤ target") only beats an early break for
out-of-order clusters, which the Matroska spec already forbids ("clusters
… in chronological order").
Why it matters: Every seek on a cues-less file is O(whole-file) instead of
O(target) — a user-visible seek latency defect on the exact files
(`SlowLinear`) where seekability already warns.
What correct looks like: `Ok(tc) if tc.ticks > target_ticks => break` after
the first cluster past the target (keep the `best` fallback for safety).
Confidence: HIGH — Verification: read (loop traced; no break on the past-
target arm).

### F-005 [S3]
Where: crates/demux/src/mkv/mod.rs:510-513 (with :988-991)
Code:
```rust
                ebml::ID_CLUSTER => {
                    let effective_end = el
                        .declared_end()
                        .unwrap_or(self.segment_end)
                        .min(self.segment_end);
```
What is wrong: For an unknown-size Cluster (the streaming/live-capture form,
explicitly supported at :372-374 and :536-545), `effective_end` is the
**segment end**. The next_event recovery arm then "skips the damaged
remainder of the cluster" by jumping to `window.end` (:991
`self.walk = window.end;`) — which for the unknown-size form is the segment
end: one structurally failed block discards **every remaining cluster in the
file** as damage, with a single `Recovered` event. There is no forward scan
for the next `Cluster` element id (0x1F43B675) — the containment ffmpeg's
mov demuxer-family implements.
Why it matters: Live-capture MKV/WebM (unknown-size clusters are their
native shape) + one corrupt block = total loss of the tail; the module's
"bounded windows" memory doctrine is honored but the *damage* doctrine is
not bounded for this form.
What correct looks like: A bounded forward signature scan for the next
cluster id when the effective end is the segment end (mirrors the TS/AEI
resync ladders).
Confidence: HIGH on mechanism (paths traced), MEDIUM on real-world frequency
(unknown-size clusters are the streaming minority in file collections).
Note: `corpus_matroska_*` lanes pass — the corpus carries sized clusters, so
this shape is untested.

### F-006 [S4]
Where: crates/demux/src/mkv/mod.rs:552-556 (doc) vs :563-574 (code)
Code:
```rust
    /// Where blocks start inside a just-loaded window: the byte
    /// offset just past the `Timecode` element (blocks may legally
    /// precede it in malformed files; the spec mandates Timecode
    /// first — files that violate that lose those leading blocks to
    /// the skip, damage-counted).
```
What is wrong: "damage-counted" is false — the walk (:563-574) skips
pre-Timecode elements with `at += hdr_len + size as usize;` and touches no
counter. Malformed files silently lose leading blocks with zero observability.
Why it matters: A doc claim with no code behind it (the audit's trust-nothing
rule); the skip itself is spec-defensible.
What correct looks like: Count the skipped bytes into `cluster_damage`, or
delete the claim.
Confidence: HIGH — Verification: read (function traced, no counter use).

### F-007 [S4]
Where: crates/demux/src/mkv/mod.rs:819-828
Code:
```rust
            packets.push(Packet {
                track: TrackId::new(ordinal),
                pts: Stamp::Present(Timestamp::new(pts, self.base)),
                dts: Stamp::Missing,
```
What is wrong: Every MKV packet carries `dts: Stamp::Missing` (B-frame
streams get no decode timestamps — the MP4 demuxer stamps both), and
`generation: GenerationId::FIRST` (:827) is the dead stamp the engine
overwrites on intake (`packet.generation = self.generation` — "the demuxer's
own stamp is authoritative for nothing", pipeline.rs:2143-2147).
Why it matters: Missing DTS defers all reorder handling to the engine's PTS
policy (works for the current decode lanes, opaque for B-frame video); the
generation stamp is misleading dead weight.
Confidence: HIGH — Verification: read + engine overwrite cited.

### F-008 [S4]
Where: crates/demux/src/mkv/mod.rs:510-513 (with :363-365 [part1] for contrast)
Code:
```rust
                    let effective_end = el
                        .declared_end()
                        .unwrap_or(self.segment_end)
                        .min(self.segment_end);
```
What is wrong: A Cluster whose declared end exceeds the segment end is
silently clamped (`min(self.segment_end)`) with no damage note, while the
`open()` top-level walk types the same violation (`:363-365` — `if end >
length { return Err(error::truncated(...)) }`). Blocks lost to the clamp are
indistinguishable from a well-formed short cluster.
Why it matters: Inconsistent damage policy between the two walks of the same
file; silent truncation is the thing this crate's damage ledger exists to
prevent.
Confidence: HIGH — Verification: read (both sites).

### F-009 [S4]
Where: crates/demux/src/mkv/mod.rs:739-750
Code:
```rust
                        let emitted =
                            self.emit_block(&block, &payload[cstart..cend], cluster_timecode);
                        self.block_scratch.frames = std::mem::take(&mut block.frames);
                        packets = Some(emitted?);
```
What is wrong: A BlockGroup carrying multiple `Block` children (spec: exactly
one; wild files violate) has each Block's packets **replace** the previous
(`packets = Some(...)` instead of extending) — earlier Blocks' packets are
silently dropped, no damage note.
Why it matters: Silent data drop on spec-violating input; one-line fix
(extend, or damage-note the extras).
Confidence: HIGH — Verification: read (assignment traced).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `load_next` (head :499-522 is part1's) | 499-550 | REAL | S4 (F-008) | window ≤ 4 MiB; unknown-size cluster → segment end (F-005/F-008); non-cluster top-level elements skipped positionally; no panics; `parse_timecode` typed-refuses a missing timecode |
| `block_start_after_timecode` | 557-579 | REAL | S4 (F-006) | in-window header walk; unknown-size child → typed bad_structure (:567-568); doc's "damage-counted" false (F-006); `size as usize` cast safe on 64-bit targets |
| `next_block_packets` | 590-693 | REAL | — | window taken/restored on every exit incl. errors (the documented livelock fix — traced); slide logic correct; contained-lie truncation typed (:639-642); MAX_BLOCK_BYTES ceiling (:644); scratch recycling (:678-687); defensive dead branch :620-622 (cursor ≥ base invariant makes it unreachable) |
| `packets_from_element` | 697-760 | REAL | S3 (F-002) / S4 (F-009) | SimpleBlock + BlockGroup walks; ReferenceBlock never tracked (F-002); multi-Block replace (F-009); BlockGroup child bounds checked (:733-737); unknown child sizes typed (:728-729) |
| `emit_block` | 763-835 | REAL | S4 (F-007) | unknown-track damage-skip (:774-778); DefaultDuration step/duration with checked div (:786-796); frame ceiling 16 MiB (:802); pool read-in-place (:810-813); checked pts (:816-818); dts Missing + dead generation (F-007); `let _ = payload` dead param (:833) |
| `landing` | 840-884 | REAL | S2 (F-001) / S3 (F-004) | cues path: checked pos + filter < segment_end ✓; target before first cue → out_of_range (edge, table note); no-cues scan: budget-bounded, walk save/restore broken on two exits (F-001), no past-target break (F-004) |
| `target_ticks` | 888-896 | REAL | — | Present → base-tick convert, Missing → 0 (seek-to-start); typed out-of-range on conversion failure |
| `parse_header_mem` (free fn) | 902-906 | REAL | — | in-memory twin of `ebml::parse_header_at`; vint id+size; no panics |
| `Demuxer::start` | 909-915 | REAL | — | idempotent + retryable (no sticky field at all — the crate's third distinct policy after ts/sticky-cached and mp4/refusal); tracks cloned out |
| `Demuxer::next_event` | 917-1001 | REAL | S3 (F-003, F-005) | pending-first ✓; eos ✓; damage flush at EOS/cluster-end/recovery (cumulative reporting F-003); truncation-vs-damage classification (:979-987) sound; unknown-size-cluster recovery skips to segment end (F-005) |
| `Demuxer::seek` | 1003-1013 | REAL | S2 (F-001 via landing) | otherwise pure (no mutation); lands via cues/scan |
| `Demuxer::seekability` | 1015-1024 | REAL | — | cues → Fast{indexed:true}, else SlowLinear — and `reanchor` exists to back it (the MP4 contrast) |
| `Demuxer::reanchor` | 1026-1044 | REAL | S2 (F-001 via landing) | commit is clean (walk/window/pending/damage/eos + Discontinuity :1034-1039); D4 falsified only through landing's early-exit corruption |

## 5. Test honesty (demuxer state machine has NO unit tests in mod.rs — `rg "mod tests"` in mod.rs: none; coverage lives in submodules (W05 others) + `crates/demux/tests/mkv_corpus.rs` + `containers::mkv`)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `corpus_matroska_seek_roundtrip` (demux/tests/mkv_corpus.rs; baseline :2100 ok) | YES for the healthy seek lane | exercises seek/reanchor on corpus files end-to-end; would catch a broken cues landing — but corpus files carry cues and sized clusters, so F-001/F-004/F-005 (no-cues/damaged/unknown-size) are untested shapes |
| `corpus_matroska_differential_vs_ffprobe` (:2099 ok) | YES for packet truth | demux output diffed against ffmpeg on real files — would catch wrong payloads/timestamps; blind to keyframe flags (F-002) if ffmpeg lanes are SimpleBlock-based |
| `containers::mkv::tests::start_time_skips_packets_and_reports_indexed_seek` (:2809 ok) | YES for the open-time seek lane | drives the real CLI open with --start-time + seekability assertion |
| `containers::mkv::tests::h264_aac_fixture_opens_with_expected_facts` (:2806 ok) | YES for open facts | fixture-lane open assertions |
| No test for: no-cues scan damage (F-001), Recovered accounting (F-003), unknown-size cluster recovery (F-005), BlockGroup keyframe (F-002) | — | none exists in mod.rs (no test module) or the corpus; all four findings would survive mutation |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "BlockGroup keyframes: no ReferenceBlock (derive at the group level…)" | mod.rs:814-815 | FALSE | no derivation exists — `at = cend;` only (:751-754); cluster.rs:217 hard-codes false (F-002) |
| "the keyframe flag (…BlockGroup: derived from `ReferenceBlock` absence by the caller)" | cluster.rs:58-60 | FALSE | the caller (this range) never derives it (F-002) |
| "Cluster payloads are walked in bounded windows… zero-alloc per packet" (5.X2b) | mod.rs:27-37 | REAL | window/pool/scratch recycling traced (see §2); bounds enforced |
| "files that violate [Timecode-first] lose those leading blocks to the skip, damage-counted" | mod.rs:552-556 | SHALLOW | the skip is real, the counting is not (F-006) |
| "skip the rest of the cluster (the contained-lie recovery)" | mod.rs:974-976 | SHALLOW | true for sized clusters; for unknown-size clusters "the cluster" is the whole remaining segment (F-005) |
| "Demux Matroska and WebM: … cues-based seeking, block lacing" | docs/feature-matrix.toml:132 | REAL | cues + reanchor implemented and corpus-proven (:2098-2100); no-cues fallback has F-001/F-004 defects |
| "Mid-cluster-damage livelock fixed with the wiring (#34)" | matrix notes (:139) | REAL | window restore on every exit traced (:689-692) |
| "VP8/VP9 and AV1 WebM video are demuxed … but NOT decodable" | mod.rs:19-23 | REAL | `webm_vp9_and_opus_decode_from_the_private_head ... ok` (:2900 — decode lane) + registry truth; honest split claim |

## 7. Dead / duplicated / unreachable code
- `let _ = payload;` (:833) — dead parameter since the 5.X2b pool read.
- `generation: GenerationId::FIRST` (:827) — dead stamp (engine overwrites,
  pipeline.rs:2147).
- Defensive dead branch :620-622 (`offset >= window.data.len()` after the
  slide branch) — unreachable under the cursor ≥ base invariant.
- No sticky mechanism in this demuxer at all (contrast ts/mp4/wav StickyShape
  copies) — not dead code, but the crate's third distinct error-replay policy.
- `chapter_targets()` (:276-313, part1 boundary) — self-documented as
  unwired ("nothing downstream reads it today"); part1's to confirm.

## 8. Dependencies & flags
- No feature gates (`pub mod mkv;` unconditional, lib.rs:50); compiled into every
  shipped build; CLI dispatch containers/mod.rs:57.
- Reuses `crate::avi::riff::PayloadSlots` (:58) — the crate-wide
  AVI-primitives layering (S4 pattern on record since W04).
- Submodules (ebml/header/cluster/cues/chapters/tags/attachments/editions)
  are other W05 agents' files; this range owns only the state machine.
- No unsafe, no unwrap/expect in this range; clippy clean per baseline
  INDEX #5/#6.

## 9. Open questions for the next agent
- cluster.rs (W05-A02): `parse_block_with`'s `simple` flag conflates "is a
  SimpleBlock" with "has the keyframe bit" — the F-002 fix needs a
  ReferenceBlock-aware path from mod.rs's group walk; coordinate.
- mkv corpus owner: add a cues-less + damaged-cluster + unknown-size-cluster
  corpus lane — F-001/F-004/F-005 are all untested shapes today.
- transcode crate owner (wave TBD): mp4mux's stss degeneracy when all sync
  rows are false (F-002's downstream face) — worth its own row when that
  crate is audited.
- The engine's Recovered accounting (pipeline.rs:2155-2167) sums whatever the
  demuxer reports — the mp4/ts demuxers report per-event deltas, mkv reports
  cumulative (F-003): one engine, two reporting conventions; the engine side
  cannot tell.
