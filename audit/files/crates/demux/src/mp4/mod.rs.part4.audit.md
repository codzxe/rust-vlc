# AUDIT: crates/demux/src/mp4/mod.rs#part4 (lines 1669–2226)
| Field | Value |
|---|---|
| Auditor | W05-A10 |
| Date | 2026-11-21 |
| Lines / functions | 558 (of 2226; part4 = 1669–2226, starting inside `parse_moof`'s `tfdt` arm — fn head at :1621 is part3's; parts: 1–556/557–1112/1113–1668/1669–2226) / 17 units |
| File verdict | REAL (with a dead seek lane: the demuxer half works, the engine-facing seek half does not exist) |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/host.rs` open_session → `apps/cli/src/demux_host.rs:open_source` → `apps/cli/src/containers/mod.rs:67` `"mp4" => mp4::open` → `apps/cli/src/containers/mp4.rs:100-101` `Mp4Demuxer::new`+`start()` → `StartedMp4` wrapper → `apps/cli/src/demux_host.rs` `SelectDemuxer` → `crates/playback-engine/src/pipeline.rs:2141` `self.demuxer.next_event()` → `step_progressive`/`step_fragment`/`mint_packet` (this range); `--start-time` reaches `seek()` via mp4.rs:120; in-session seek reaches the *default* `reanchor` via pipeline.rs:1499 — see F-001) |
| Findings | S0: 0 S1: 0 S2: 4 S3: 5 S4: 3 |
| Commands run | `rg -n "fn reanchor" crates/demux/src/` (ONLY avi/mod.rs:971 + mkv/mod.rs:1026 — mp4 and ts have none); `rg -n "seekability|demuxer_reanchor" crates/playback-engine/src/` (session.rs:2021,2784 gate `!= SeekSupport::None`; pipeline.rs:1499 reanchor); `sed -n '360,412p' crates/playback-engine/src/seek.rs` (step-3 flush THEN step-4 reanchor; Err → `SeekFailure::Demux` :409-412); `sed -n '2140,2150p' crates/playback-engine/src/pipeline.rs` (`packet.generation = self.generation` — demuxer stamp overwritten); `rg -n "order_cursor" crates/demux/src/mp4/mod.rs` (:310 decl, :627 init, :1571-1573 consume — NO reset in seek); `rg -n "struct PayloadSlots" -A 160 crates/demux/src/avi/riff.rs` (fill/recycle/spill :588-670: pinned arenas → fresh `vec![0u8; want]` spill); `rg -n "fn progressive_seek_lands_on_keyframe|fragmented_seek_scans_moofs|seek_failure_leaves_state_usable_d4" -A 30 crates/demux/src/mp4/tests.rs`; `rg -n "^test mp4::" audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` (38 ok, :1772-1790); `rg -n "containers::mp4::" …` (11 ok, :2813-2823); `rg -n "reanchor" …log.txt` (8 avi reanchor tests, ZERO mp4); baseline evidence reads (mp4-smoke-open-session-420.txt, mp4-smoke-open-session.txt, mp4-smoke-bench-420.txt, lumen-engine-info.txt, orchrun-test-journey-seek.log.txt); `git log --all --oneline | rg 262a99cd` (W00 correction present); `sed -n` on every quoted range |

## 1. One-paragraph verdict
This range is the sample-delivery half of the MP4 demuxer — the moof/trun
fragment parser, both walk steps, packet minting, and the entire seek
surface — and the delivery core is real and executed: the baseline
4:2:0 H.264+AAC MP4 plays its full lifecycle through exactly this code
(`audit/runs/baseline/mp4-smoke-open-session-420.txt`: tracks 2 ready →
TimingDurationKnown 3 s → Buffering → Playing → Stopped → Idle, EXIT=0),
38 `mp4::` and 11 `containers::mp4` tests pass, and the Wave-0 "MP4 cannot
be opened" headline is confirmed by the W00 correction (commit 262a99cd) to
be a decode-side refusal (`h264.sw.unsupported` for the yuv444p sample),
not a demux failure — this range delivered the packets that played. But the
seek story is hollow: `seekability()` advertises `Fast { indexed: true }`
while the demuxer implements **no `reanchor`** — the trait default refuses —
so every in-session MP4 seek in the shipped engine fails *after* the seek
flow already flushed the pipeline, and the `seekable: true` the user sees in
MediaInfo is false advertising. The committing `seek()` itself violates the
trait's pure-query contract and is broken in the backward direction in both
modes (`order_cursor` never reset; `frag_pos` never rewinds); `step_fragment`
materializes an entire moof's payload in one call, defeating the 2×16 MiB
pool and the "bounded per call" promise; and the `elst` empty-edit delay is
consumed in the wrong timescale. One cross-range trace also resolves a
standing baseline defect: the garbage `language: Some("voe")` is this file's
`unpack_language` off-by-one (part1's range, :536).

## 2. What actually works (verified)
- End-to-end MP4 playback through this range: `mp4-smoke-open-session-420.txt`
  (sample-420.mp4, h264 Constrained Baseline 4:2:0 + AAC): `tracks 2 ready`,
  `TimingDurationKnown { duration: Timestamp(3000000) }`, `MediaInfoReady …
  tracks: [Video h264, Audio aac]`, `Buffering → Playing → Stopped → Idle`,
  `EXIT=0` — `start`/`next_event`/`step_progressive`/`mint_packet` all executed
  in the shipped binary.
- Demux exonerated in the Wave-0 headline: the yuv444p original shows the
  same demux success (2 tracks, 3 s duration) and then
  `error [decode.unsupported] unsupported in h264 (h264.sw.unsupported)`
  (`mp4-smoke-open-session.txt:14-16`; INDEX CORRECTION v2 + commit 262a99cd:
  the sw decoder exists, refuses 4:4:4 by design — sw.rs:54,367-371,527).
- Unit + container suites: 38 `mp4::` tests ok incl.
  `fragmented_demuxes_moofs_with_tfdt_ladder`, `fragmented_two_tracks_per_moof`,
  `progressive_seek_lands_on_keyframe`, `seek_failure_leaves_state_usable_d4`
  (`orchrun-test-workspace-nofailfast.log.txt:1772-1790`); 11
  `containers::mp4` tests ok incl. `corpus_iso_bmff_lanes_match_the_ffmpeg_truth`
  and `h264_aac_fixture_opens_with_expected_facts` (:2813-2823).
- `--start-time` seek at open works end-to-end:
  `containers::mp4::tests::start_time_skips_packets_and_reports_indexed_seek ... ok`
  (:2823) — the forward-seek-from-fresh-open shape (order_cursor == 0) is
  exactly the case `seek_progressive` handles correctly.
- default-base-is-moof is genuinely implemented: `base_offset = moof_start`
  (:1631), tfhd `base-data-offset` override (:1643-1646), carry-forward
  damage-noted (:1661-1666) — traced against module doc :50-54.
- Bounded ceilings enforced in this range: samples ≤ `MAX_SAMPLE_BYTES`
  (:1584, :1809), moof header ≤ `MAX_MOOF_BYTES` (:1790), fragment scan
  ≤ `MAX_FRAGMENT_SCAN_BOXES` (:1778, :2166), trun counts ≤ `MAX_SAMPLES`
  (:1727, :1767-1769), sample extents range-checked against the stream
  (:1588, :1813).

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/mp4/mod.rs:2039-2048 (and the absence of `reanchor` file-wide)
Code:
```rust
    fn seekability(&self) -> SeekSupport {
        if !self.started {
            return SeekSupport::None;
        }
        if self.fragmented {
            SeekSupport::SlowLinear
        } else {
            SeekSupport::Fast { indexed: true }
        }
    }
```
What is wrong: `Mp4Demuxer` answers `Fast { indexed: true }` (progressive) or
`SlowLinear` (fragmented) but implements **no `reanchor`** — `rg "fn reanchor"
crates/demux/src/` finds only avi/mod.rs:971 and mkv/mod.rs:1026 — so the
trait default applies: the typed `plugin.demux.reanchor.unsupported` refusal
(crates/plugin-api/src/demux.rs:113-120). The engine's seek flow gates entry
on exactly this answer (`crates/playback-engine/src/session.rs:2021,2784`:
`seekability() != SeekSupport::None`), then step 3 flushes the pipeline and
step 4 calls `pipeline.demuxer_reanchor` (seek.rs:377-390) → `Err` →
`SeekFailure::Demux` (seek.rs:409-412). Net effect in the shipped binary:
**every in-session MP4 seek fails, after the seek already flushed frames**,
while `lumen open` reports the file as `seekable: true`
(`mp4-smoke-open-session-420.txt:12`). AVI ships 8 passing reanchor tests
(no-failfast log :1646-1653); MP4 has zero — the owner's priority format is
the demuxer without the acting seek half. The CLI wrapper even documents the
gap as "honest" (`apps/cli/src/containers/mp4.rs:173-179`: "The demuxer keeps
the trait's default typed refusal (the moving half of in-session seek is its
honest gap)") — honest in a comment, dishonest at the `seekability()` API
the engine consults.
Why it matters: A user dragging the position bar on any MP4 gets SeekFailed
plus a gratuitous flush; the advertised capability
(module doc :55-60: "Seek: progressive = `stss` keyframe walk-back …
`Fast { indexed: true }`") is not what the shipped engine can act on.
What correct looks like: Implement `reanchor` (the commit logic already
exists in `seek_progressive`/`seek_fragment` — move it there per the AVI
precedent), or answer `SeekSupport::None` until it exists.
Confidence: HIGH — Verification: read (trait default + engine call chain
traced); baseline evidence cited (seekable: true; 8-vs-0 reanchor test
asymmetry).

### F-002 [S2]
Where: crates/demux/src/mp4/mod.rs:2127-2142 (with :1575-1577, :2164)
Code:
```rust
        // Commit: per-track cursors (compute-then-commit — every
        // landing above already succeeded).
        for (ti, t) in self.tracks.iter_mut().enumerate() {
            if ti == ref_idx {
                t.next = land_idx;
            } else {
```
```rust
            if si < track.next {
                continue; // pre-seek sample — skipped by the cursor
            }
```
What is wrong: `seek_progressive` sets per-track `t.next` cursors but never
resets `self.order_cursor` (`rg order_cursor`: declared :310, initialized
:627, consumed :1571-1573 — no other writer). `step_progressive` walks the
global file-offset-sorted `order` list from the CURRENT cursor and only
skips samples with `si < track.next` — a forward filter. After a **backward**
seek (play to 90 %, seek to 10 %): `t.next` drops to the landing, but
`order_cursor` sits near the end; the walk resumes from the old position and
delivers late-file samples while the demuxer just *reported* the 10 %
keyframe as the landing — reported position and delivered data disagree.
The fragment sibling has the same blindness: `seek_fragment` scans from
`self.frag_pos` (:2164: `let mut pos = self.frag_pos.unwrap_or(len);`) which
only ever moves forward (:1840, :2216) — a backward fragment seek finds no
moof ≤ target behind the cursor and returns `seek_out_of_range` (:2212-2213).
Why it matters: Backward seek is a bog-standard user operation; the
progressive mode silently delivers the wrong samples, the fragment mode
refuses. Currently masked in the shipped lane only because the engine's
in-session path dies earlier at `reanchor` (F-001); the committing `seek()`
is reachable via the trait API and is the module's documented capability.
What correct looks like: `self.order_cursor = 0;` in the seek_progressive
commit (the skip filter then lands correctly), and `seek_fragment` scanning
from `moov_end` (or min(frag_pos, first-moof-after-moov)) when the target is
behind the current position.
Confidence: HIGH — Verification: read (cursor writers enumerated; skip
filter traced); both seek tests seek forward from a fresh open
(tests.rs:416-445, :448-470) — the backward case is untested and would not
be caught.
Note: `seek_failure_leaves_state_usable_d4` (tests.rs:742-752) consumes 2
packets then only a *failing* seek — the successful-backward-seek mutation
("reset order_cursor" deleted) passes the whole suite.

### F-003 [S2]
Where: crates/demux/src/mp4/mod.rs:1805-1838 (with :1813, :1817-1821)
Code:
```rust
                    // Queue everything; return the first.
                    let mut first = true;
                    let mut first_packet: Option<Packet> = None;
                    for (ti, sample) in frag_samples {
```
```rust
                        let packet = self.mint_packet(ti, &sample, payload, false);
                        if first {
                            first = false;
                            first_packet = Some(packet);
                        } else {
                            self.pending.push_back(DemuxEvent::Packet(packet));
                        }
```
What is wrong: One `next_event` call materializes the **entire** payload
described by one moof: every sample is range-checked (:1813), read through
the pool (:1819-1821), minted, and queued into `pending` before the first
packet is returned. `MAX_MOOF_BYTES` (1 MiB) caps the moof *header*, not the
mdat bytes it describes — the retention bound is the file length. The
2×16 MiB `PayloadSlots` pool is pinned by the first two live views; from the
third sample on, `fill` spills to a fresh exact-size `vec![0u8; want]`
(crates/demux/src/avi/riff.rs:656-670, the `fresh` at :660), so in the fragment lane the pooled
zero-copy scheme is structurally defeated (one heap allocation per sample,
all retained until the engine drains the queue). A single-moof file
describing a 1 GiB mdat retains 1 GiB in one call — the trait's
"reads through its access stream as needed (**bounded per call**)"
(crates/plugin-api/src/demux.rs:33-35) is violated.
Why it matters: fMP4/DASH segments realistically describe 2–6 s of media
per moof (tens of MiB) — every fragment boundary is a full-materialize
spike; a malicious single-moof file is an OOM vector.
What correct looks like: Queue sample *descriptors*, not payloads: return
the first packet and re-enter `step_fragment` (or a per-moof cursor) so each
`next_event` reads exactly one sample — the progressive path's shape.
Confidence: HIGH — Verification: read (fill/spill traced riff.rs:588-670;
queue loop traced); fMP4 demux itself is exercised green in baseline
(`fragmented_two_tracks_per_moof`, `fragmented_demuxes_moofs_with_tfdt_ladder`
:1772-1774) — the defect is the retention shape, not correctness.

### F-004 [S2]
Where: crates/demux/src/mp4/mod.rs:1866-1869 (with :1088-1092 [part3], :2099-2103, :1908-1911)
Code:
```rust
        let t = &self.tracks[ti];
        let dts_ticks = (s.dts as i64).saturating_sub(t.media_time as i64);
        let pts_ticks = dts_ticks
            .saturating_add(s.cts)
            .saturating_add(t.empty_edit_delay as i64);
```
```rust
                            if media == -1 {
                                empty_edit_delay = seg_dur;
```
What is wrong: `elst`'s `segment_duration` is expressed in **movie-timescale
units** (ISO/IEC 14496-12 §8.6.6), but :1091 (part3's parse) stores it raw
into `empty_edit_delay`, and every consumer in this range — `mint_packet`
(:1869), `seek_progressive`'s `pts_of` (:2099-2103), `extent_nanos`
(:1908-1911) — adds it to timestamps in **media ticks** (`t.base` =
1/mdhd-timescale). No movie→media conversion exists anywhere (rg for
`movie_timescale` uses: :620 default, :922-923 read, :1446/:1460 cover track
only). For the canonical gapless M4A (movie_timescale 1000, audio 44 100 Hz)
an empty edit's delay is applied ~44× too small; A/V start alignment, seek
landings and the track span are all shifted on files with empty edits.
(Secondary asymmetry: the delay is added to PTS only, never DTS, so even a
correctly-scaled delay leaves decode times unshifted.)
Why it matters: Empty edits are the standard gapless/lead-in encoding shape;
this is a unit-confusion defect on a realistic input class — exactly the
checklist's "unit confusion (ms/µs/timescale)".
What correct looks like: `empty_edit_delay_ticks =
nanos→media-ticks(movie→nanos(seg_dur, movie_timescale))` at parse (or at
each consumer), and DTS shifted with the same delay.
Confidence: HIGH on the mechanism (scales traced end-to-end), MEDIUM on
real-world frequency (empty edits are common in M4A, rarer in video MP4).
Cross-boundary note: the storing site :1081-1094 is part3 (W05-A06)'s
range; all consuming sites are this range.

### F-005 [S3]
Where: crates/demux/src/mp4/mod.rs:2050-2083 (with :2129-2142, :2216-2218)
Code:
```rust
    fn seek(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        if !self.started {
            return Err(error::not_started());
        }
```
What is wrong: The trait defines `seek` as the **pure query** — "Does
**not** move the read position: the engine decides whether to act"
(crates/plugin-api/src/demux.rs:36-38, 70-75) — with `reanchor` as the
acting half. `Mp4Demuxer::seek` commits: it rewrites every track cursor
(:2129-2140), clears `pending`, resets `eos` (:2141-2142), and moves
`frag_pos` (:2216-2218). The shipped `--start-time` path *depends* on the
violation: `apps/cli/src/containers/mp4.rs:115-120` calls it "the pure seek
query" while relying on its side effects to position the walk (plus its own
`skip_until_ns` watermark as belt-and-braces). Any host that uses the
documented query shape (validate a target, keep playing) gets its playback
state silently moved.
Why it matters: Contract violation with a shipped caller built on the
wrong half; the same shape is what makes F-002 reachable at all.
What correct looks like: `seek` computes and returns the landing without
mutation (per AVI's `locate`), `reanchor` commits (per AVI's :971-979).
Confidence: HIGH — Verification: read (mutation sites enumerated; caller
comments read).

### F-006 [S3]
Where: crates/demux/src/mp4/mod.rs:1692-1706
Code:
```rust
            let dur_default = if default_dur != 0 {
                default_dur
            } else {
                trex.map_or(0, |t| t.duration)
            };
```
What is wrong: tfhd presence flags (0x8/0x10/0x20, read at :1650-1659) are
discarded and replaced by a `!= 0` value test. Per ISO/IEC 14496-12 §8.8.7,
an *explicitly present* `default_sample_duration/size/flags` of 0 is the
value; the code overrides it with the trex default. A traf whose tfhd
declares `default_sample_duration = 0` (legal — e.g. streams that carry all
durations per-sample and zero the default) gets every duration-less sample
stamped with the trex duration instead.
Why it matters: Wrong durations on a spec-legal encoding; the correct
information (the flags) was already in hand at :1643-1660.
What correct looks like: Track presence booleans (`has_dur = flags & 0x8 !=
0`) and fall back to trex only when absent.
Confidence: HIGH on mechanism (spec + code traced), MEDIUM on real-world
frequency.
(Parallel defect in the same merge: the flags_default fallback means a tfhd
default_flags of 0 falls back to trex flags — the same class.)

### F-007 [S3]
Where: crates/demux/src/mp4/mod.rs:1751-1756
Code:
```rust
                    if size > 0 {
                        out.push((
                            ti,
                            Sample {
                                offset: sample_offset.unsigned_abs().min(u64::MAX / 2),
```
What is wrong: `sample_offset` is `base_offset + data_offset` with a signed
`data_offset` (:1716-1718, :1726). When the sum is negative, `.unsigned_abs()`
**mirrors** it into a large positive offset (then clamps to u64::MAX/2) —
fabricating a sample location near the start of the file instead of
damage-noting the impossible offset. The fabricated offset passes the
`:1813` stream-range check for any small magnitude, and wrong bytes are
delivered as a media packet.
Why it matters: Malformed (or pathologically-authored) fMP4 input silently
produces fabricated sample locations — the opposite of the module's
"declared sizes are hints; the stream is authority" damage-noting doctrine.
What correct looks like: `if sample_offset < 0 { damage-note and skip the
sample }` — mirrors the `frag-sample-past-stream` handling at :1813-1815.
Confidence: HIGH — Verification: read (sign path traced; range check at
:1813 does not reject small mirrored offsets).

### F-008 [S3]
Where: crates/demux/src/mp4/mod.rs:1781, 1795-1798, 1802 (contrast :1787, :1800, :1824, :1827, :1856)
Code:
```rust
            let Some(header) = read_top(&mut self.reader, pos)? else {
                break;
            };
```
```rust
                    let got = self
                        .reader
                        .read_prefix_into(&mut moof, header.payload_start)?;
```
What is wrong: `step_fragment`'s stickiness is inconsistent: the truncation
and pool failures go through `self.fail` (sticky — :1787, :1800, :1824,
:1827, :1856), but a bad top-box header (`read_top` Err at :1781), a source
read error (:1796-1798), and a structurally unparseable moof
(`parse_moof` Err at :1802) propagate **non-sticky** with `frag_pos`
unmoved — the next `next_event` re-reads the same position and returns the
same error forever (no EOS, no advance). `step_progressive` wraps *all* its
errors (:1593, :1602, :1605). Same class as the TS demuxer's F-004b.
Why it matters: An engine retry loop after a mid-scan error spins without
progress; the crate's own "unrecoverable ⇒ sticky" doctrine (WAV/Ogg
 StickyShape precedent, :689-693) is applied arbitrarily.
What correct looks like: Route every step_fragment failure through
`self.fail` (or make the walk advance past the damaged box with a damage
note, as the moof-over-ceiling path already does :1790-1794).
Confidence: HIGH — Verification: read (all error sites enumerated and
classified).

### F-009 [S3] — CROSS-RANGE (code lives at :534-539, part1/W05-A08's range; traced here because it resolves a standing baseline defect)
Where: crates/demux/src/mp4/mod.rs:534-539
Code:
```rust
fn unpack_language(packed: u16) -> Option<String> {
    let c = |shift: u32| -> char { (b'a' + ((packed >> shift) & 0x1F) as u8) as char };
    let tag: String = [c(10), c(5), c(0)].iter().collect();
    if tag == "und" { None } else { Some(tag) }
}
```
What is wrong: ISO/IEC 14496-12 §12.4.2 packs each mdhd language character
as its ASCII value **minus 0x60** ('a'=1). Unpacking must add 0x60 back;
this adds `b'a'` (0x61) — one too far. Packed "und" = (21<<10)|(14<<5)|4 =
0x55C4 decodes to "v"+"o"+"e" = **"voe"**, not "und" — so the `None`-for-
"und" filter never fires, and *every* MP4 track carries a garbage language
shifted by +1 letter ("eng"→"foh"). This is exactly the standing baseline
defect the W00 correction flagged: "TrackInfo `language: Some("voe")` on
tracks with no language metadata — garbage default (demux wave to trace)"
(INDEX.md CORRECTION v2), visible in both MP4 smoke sessions
(`mp4-smoke-open-session-420.txt` / `mp4-smoke-open-session.txt`:
`language: Some("voe")` on both tracks). "voe" appears nowhere as a literal
in the repo (rg) — it is computed by this function.
Why it matters: Wrong metadata on every MP4/MOV track surfaced to users;
also defeats the intended "undeclared → None" semantics.
What correct looks like: `(0x60u8 + ((packed >> shift) & 0x1F) as u8) as char`.
Confidence: HIGH — Verification: read + arithmetic reproduced from the
packed "und" constant (0x55C4 → v/o/e with +0x61, u/n/d with +0x60);
baseline output cited.

### F-010 [S4]
Where: crates/demux/src/mp4/mod.rs:2010-2012
Code:
```rust
        if self.started {
            return Err(error::not_started());
        }
```
What is wrong: A second `start()` answers `demux.mp4.not-started`
(error.rs:142-145) — the code *says* "not started" about a demuxer that IS
started; TS answers the cached track list (ts/mod.rs:1036-1038) and AVI
likewise. The CLI works around it with the `StartedMp4` adapter
(mp4.rs:141-158, comment: "the demuxer's deliberate second-call refusal
(`demux.mp4.not-started` — unlike ADTS/AVI, which answer the cached list)").
Why it matters: A misleading error code plus a per-demuxer contract
divergence that already forced a host-side adapter; S4 because it is
documented, deliberate, and worked around in the shipped lane.
What correct looks like: Return the cached list (crate precedent) or mint
an accurate code (`already-started`).
Confidence: HIGH — Verification: read + adapter comment cited.

### F-011 [S4]
Where: crates/demux/src/mp4/mod.rs:1889
Code:
```rust
            generation: GenerationId::FIRST,
```
What is wrong: Every minted packet is stamped `GenerationId::FIRST` — the
pipeline overwrites it on intake: `packet.generation = self.generation` with
the comment "the demuxer's own stamp is authoritative for nothing"
(crates/playback-engine/src/pipeline.rs:2143-2147). A dead field in every
packet, misleading about who owns generation identity.
Why it matters: Hygiene/misleading-code; zero behavioral effect today.
What correct looks like: Document the field as engine-owned in the Packet
type, or stop requiring demuxers to fill it.
Confidence: HIGH — Verification: read + overwrite site cited.

### F-012 [S4]
Where: crates/demux/src/mp4/mod.rs:1751 (contrast :1580-1582)
Code:
```rust
                    if size > 0 {
                        out.push((
```
What is wrong: Zero-size fragment samples are dropped silently inside
`parse_moof`, while the progressive walk damage-notes the same shape
("zero-size-sample", :1580-1582). Inconsistent damage accounting between the
two lanes for the identical condition.
Why it matters: Observability drift; a fragment stream of zero-size samples
demuxes to nothing with no note.
What correct looks like: `self.note(0, "frag-zero-size-sample")` alongside
the skip.
Confidence: HIGH — Verification: read (both sites).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes (panic paths, complexity, ignored params, reachability) |
|---|---|---|---|---|
| `parse_moof` — `tfdt` arm tail | 1669-1676 | REAL | — | v1 u64 / v0 u32 baseMediaDecodeTime per §8.8.12; boundary note: arm head :1668 is part3's |
| `parse_moof` — track resolution + trex merge | 1680-1706 | REAL | S3 (F-006) | unknown-track and chapter-track trafs damage-noted and skipped (:1681, :1688); defaults merge ignores tfhd presence flags (F-006) |
| `parse_moof` — trun loop | 1708-1770 | REAL | S3 (F-007) / S4 (F-012) | flags 0x1/0x4/0x100/0x200/0x400/0x800 per §8.8.8; signed data_offset correctly reinterpreted (u32→i32→i64 :1718); first_flags applies to each trun's first sample ✓; dts/offset carry across truns ✓; negative offset mirroring (F-007); zero-size silent drop (F-012); count ≤ MAX_SAMPLES noted ✓ |
| `step_fragment` | 1775-1861 | REAL | S2 (F-003) / S3 (F-008) | budget-bounded ✓; moof-over-ceiling skipped-with-note ✓; full-materialize queue (F-003); mixed stickiness (F-008); no panics; `header.end > pos` invariant holds (read_top rejects non-advancing) |
| `mint_packet` | 1864-1891 | REAL | S2 (F-004) / S4 (F-011) | elst application (media_time subtract ✓, cts add ✓, empty_edit_delay wrong-scale + PTS-only F-004); duration via saturating nanos conversion, None on error; exposed-ordinal TrackId consistent with start() list (chapter guard upstream); dead generation stamp (F-011) |
| `seek_reference` | 1894-1899 | REAL | — | first non-chapter video, else first non-chapter |
| `extent_nanos` | 1903-1915 | REAL | S2 (F-004) | empty_edit_delay/media_time/media_duration mix inherits the unit bug; u128-safe tick math |
| `cover_in_meta` | 1923-1959 | REAL | — | FullBox(4)/plain(0) dual anchor ✓ documented; returns offset within moov payload — absolute conversion at :988-990 (part3) verified correct |
| `cover_in_udta` | 1963-1969 | REAL | — | thin compositor over cover_in_meta |
| `trak_chap_refs` | 1972-1992 | REAL | — | 64-id cap; bounds-checked u32 walk |
| `ticks_to_nanos` | 1995-2001 | REAL | — | u128 intermediate, saturating; timescale-0 guarded |
| `nanos_to_ticks` | 2004-2006 | REAL | — | u128, floor, saturating |
| `Demuxer::start` | 2009-2017 | REAL | S4 (F-010) | second-call refusal (inverted code); errors routed through fail() → sticky ✓ |
| `Demuxer::next_event` | 2019-2037 | REAL | — | ordering correct: sticky → not-started → eos → pending → step (pending-first before EOS ✓); dispatches fragmented/progressive |
| `Demuxer::seekability` | 2039-2048 | REAL | S2 (F-001) | Fast{indexed:true}/SlowLinear advertised with no reanchor behind it |
| `Demuxer::seek` | 2050-2083 | REAL | S3 (F-005) | sticky replay ✓; target validation (missing stamp, negative ticks, nanos conversion) ✓; extent range check gated on Some(>0) (fragmented files declare none → unchecked, honest); impure commit (F-005) |
| `seek_progressive` | 2092-2148 | REAL | S2 (F-002) / S3 (F-005) | monotonic pts_of walk + break ✓; keyframe walk-back ✓ (sample 0 floor); per-track landing positions ✓; missing order_cursor reset (F-002); computes-then-commits but in the query (F-005); O(n) scan per seek acceptable |
| `seek_fragment` | 2152-2224 | REAL | S2 (F-002) / S3 (F-005) | moof scan budget-bounded ✓; first-sync-≤-target landing ✓; stop-after-target ✓; backward blindness via frag_pos (F-002); commits in the query (F-005); scan_budget_exhausted non-sticky (:2210) — F-008 class |

## 5. Test honesty (tests live in mp4/tests.rs — A04/A05's assignment; assessed only against this range)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `progressive_seek_lands_on_keyframe` (tests.rs:416-445) | YES for forward landing / NO for F-002 | asserts concrete landings (0, 4000 ticks) + drain resumes at keyframe + typed out-of-range — but both seeks are forward from a fresh open (order_cursor 0); deleting an order_cursor reset cannot fail it |
| `fragmented_seek_scans_moofs` (tests.rs:448-470) | YES for forward frag landing / NO for F-002 | same fresh-open forward-only shape; backward fragment seek (out_of_range today) untested |
| `seek_failure_leaves_state_usable_d4` (tests.rs:742-752) | YES for D4-on-failure | consumes 2 packets, failing seek, walk continues — but never a successful backward seek; blind to F-002 |
| `fragmented_demuxes_moofs_with_tfdt_ladder` / `fragmented_two_tracks_per_moof` | YES for fragment demux correctness | assert packet counts/ladder on constructed moofs (baseline :1772-1774 ok); blind to F-003 (retention) — nothing asserts memory shape |
| reanchor (absent) | — | zero MP4 reanchor tests vs AVI's 8 (no-failfast :1646-1653) — F-001 is structurally untested |
| `containers::mp4::tests::start_time_skips_packets_and_reports_indexed_seek` | YES for the open-time path | drives the real CLI open with --start-time; would catch a broken forward seek/landing report (baseline :2823 ok) |
| `containers::mp4::tests::corpus_iso_bmff_lanes_match_the_ffmpeg_truth` | YES for demux truth | 30-lane corpus vs ffmpeg oracle (baseline :2814 ok) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Seek: progressive = `stss` keyframe walk-back … `Fast { indexed: true }`; fragmented = bounded moof-scan (`SlowLinear`)… Both are compute-then-commit (D4)" | mod.rs:55-60 | SHALLOW | the compute/commit exists — in `seek()`, the half the engine never calls; `reanchor` (the acting half, demux.rs:93-120) is the default refusal → in-session seek always fails (F-001); D4's "on Err the demuxer is left at its pre-seek position" is untestable through the engine path |
| "Declared sizes are hints; the stream is authority… never a silent short parse" | mod.rs:22-25 | REAL | sample extents range-checked (:1588, :1813); truncations typed (:1593, :1787, :1800); except the mirrored negative offset (F-007) |
| "fMP4 base-offset: `base-data-offset` when present, else the enclosing moof's file offset… carry-forward… damage-noted" | mod.rs:50-54 | REAL | :1631, :1643-1646, :1661-1666 traced |
| "next_event yields samples in file order (progressive merge or moof-scan)" | mod.rs:288 / :45-47 | REAL | order merge sorted at :811 (part3), fragment sort :1804; baseline corpus lane green |
| "Sample payloads are *never* read whole: `next_event` pulls exactly one sample's bytes through the pooled arena" | mod.rs:32-35 | FALSE (fragment lane) | step_fragment reads and queues **every** sample of a moof per call (:1808-1838) — F-003; progressive lane honors it |
| trait: "seek … Does **not** move the read position" | plugin-api/src/demux.rs:36-38, 70-75 | VIOLATED | seek() commits cursors/pending/eos/frag_pos (:2129-2142, :2216-2218) — F-005 |
| matrix `container-mp4-mov-3gp` status implemented, "stco/stsc sample tables, edts/elst, … fragmented MP4" | docs/feature-matrix.toml:142-151 | REAL (demux) with stale note | demux proven (smoke 420 + corpus lanes); the row's own notes still carry the pre-5.C1 text "No MP4 demuxer code exists" alongside "2026-11-15: end-to-end VIDEO PLAYBACK proven" — internal contradiction to clean up; no seek claim in the row (seek gap F-001 is a module-doc/API issue, not a matrix FALSE) |
| "An MP4 cannot be opened… the decoder layer refuses H.264" | baseline INDEX CORRECTION v2 / 262a99cd | REAL | this range delivered packets in both smoke sessions; 4:4:4 refused at decode (mp4-smoke-open-session.txt:14-16); decode-bench's "no decoder in this build" is the hard-coded bench lane (mp4-smoke-bench-420.txt: 0 frames for a file that plays), engine-info's `decoders: mjpeg, pcm_s16le` the hard-coded literal (lumen-engine-info.txt) — both standing defects already on record, demux side exonerated |

## 7. Dead / duplicated / unreachable code
- `GenerationId::FIRST` stamp (:1889): dead by overwrite (pipeline.rs:2147).
- The entire committing half of `seek_progressive`/`seek_fragment`
  (cursor writes :2129-2142, :2216-2218) is unreachable from the shipped
  engine (which stops at the default reanchor refusal, F-001); its only
  shipped caller is open-time `--start-time` (mp4.rs:120) — REACHABLE but
  only in the fresh-open shape where F-002 cannot trigger.
- `StickyShape` (:243-263) is the crate-wide copy (WAV/Ogg/ADTS/TS/AVI
  precedent — copy #6+).
- `SAMPLE_IS_NON_SYNC` bit-15 + bit-16 double test (:1760): bit 16 is a
  padding field per ISO (§8.8.3.1) — ffmpeg-quirk compatibility, commented
  (:142-145); conservative direction (sync→false), kept.

## 8. Dependencies & flags
- No feature gates (`pub mod mp4;` unconditional; compiled into every
  shipped build; CLI dispatch containers/mod.rs:67).
- Reuses `crate::avi::riff::{PayloadSlots, SourceReader}` (:97) — the
  crate-wide AVI-primitives layering (S4 pattern on record since W04).
- No unsafe, no unwrap/expect in this range; clippy clean per baseline
  INDEX #5/#6; `#[must_use]` accessors present.

## 9. Open questions for the next agent
- W05-A08 (mp4 part1): `unpack_language` :534-539 off-by-one (F-009) is in
  your range — the "voe" trace resolves the INDEX CORRECTION v2 standing
  defect; one-character fix (`b'a'`→`0x60`).
- W05-A06 (mp4 part3): the `elst` `segment_duration` movie-timescale store
  (:1081-1094) is F-004's root; also `track_record`'s `start` (:870)
  inherits it.
- A04/A05 (mp4 tests): the suite has no backward-seek and no reanchor
  coverage — F-001/F-002 would survive any current test mutation.
- Seek-fix sequencing note for the orchestrator: F-001 and F-002 interact —
  implementing `reanchor` by delegating to today's `seek()` bodies would
  ship the backward-seek wrong-data bug into the engine path; fix F-002's
  cursor reset first.
- The first-vertical `journey_seek` integration test FAILED in the
  orchestrator run (`orchrun-test-journey-seek.log.txt:30-33`: "keeps the
  pre-seek deliveries — observed 90, expected 154") on the AVI lane —
  cross-check with W03-A07's root-cause; if the AVI seek machinery loses
  pre-seek deliveries, the MP4 lane will inherit it once reanchor lands.
