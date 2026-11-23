# AUDIT: crates/demux/src/mp4/mod.rs
| Field | Value |
|---|---|
| Auditor | W05-A01 |
| Date | 2026-11-24 |
| Lines / functions | 2226 / 52 callables + 9 consts |
| File verdict | REAL (progressive demux + open path solid; in-session seek broken, several table-parser field bugs) |
| Reachable from shipped entry point? | YES — `lumen open` → apps/cli/src/containers/mp4.rs:100 `Mp4Demuxer::new` → `demuxer.start()`; engine `next_event` via pipeline.rs:2141; engine seek via pipeline.rs:1499 `demuxer_reanchor` (verified by execution, see F-001) |
| Findings | S0: 1 S1: 3 S2: 8 S3: 4 S4: 3 (+1 cross-file root cause) |
| Commands run | see §2 (all exit 0; no cargo available in sandbox — prebuilt binaries used) |

## 1. One-paragraph verdict

The progressive MP4 open path is real and works: ftyp/moov walk with moov-at-end (baseline
sample.mp4 moov@86752 opens, 2 tracks h264/aac, exit 0), stbl sample-table expansion
(stsz/stz2/stts/stsc/stco/co64/stss/ctts), file-offset interleave merge, PCM chunk collapse,
chapters, cover art, typed truncation/bad-box errors — 38 unit tests executed and passing.
But the file carries one shipped-broken core feature and a cluster of silent field-level
parser bugs: **in-session seek of any MP4 fails with `plugin.demux.reanchor.unsupported`**
(Mp4Demuxer never implements `reanchor`, the only seek entry the engine calls — executed
proof below, F-001, S0); `trex` fragment defaults are read with a +4 byte skew (F-002, S1);
`stts`/`ctts` entry counts drive unbounded loops (CPU DoS, F-003, S1); progressive backward
seek drops data because `order_cursor` is never reset (F-004, S1); the mdhd language unpack
is off by one letter — the root cause of the baseline `language: Some("voe")` (F-005, S2);
and the `container: None` in MediaInfo is NOT this file's fault —
crates/playback-engine/src/event.rs:929 hard-codes it (see §6 cross-ref).

## 2. What actually works (verified)

- `./target/debug/deps/demux-036dd705535b2fb3 mp4::tests` → `38 passed; 0 failed` (0.19s, exit 0).
- `./target/debug/deps/mp4_fixtures-40ff1b8beab65767` → `5 passed; 0 failed` (exit 0).
- `./target/debug/deps/corpus_mp4-324c0f2c426050ba` → `2 passed` — but corpus-v4 is ABSENT
  (`/tmp/my-project/corpus-v4/media: No such file`), both tests are SKIP no-ops (Wave-4
  systemic finding re-confirmed).
- Shipped binary, baseline sample: `lumen open samples/sample.mp4` walks Idle→…→Idle exit 0,
  2 tracks (audit/runs/baseline/mp4-smoke.md:14; re-executed this session).
- python3 box-walk of sample.mp4 (executed): both mdhd v0, language packed `0x55C4`,
  timescales 12800/44100, mvhd timescale 1000 — spec decode `und`, lumen formula `voe`.
- moov-at-end scan works (mp4-smoke.md:24 `moov@86752` + open exit 0).

## 3. Findings

F-001 [S0] In-session MP4 seek always fails; `seekability()` claims `Fast{indexed:true}` anyway
Where: mod.rs:2039-2048 (seekability) + 2050-2083 (`seek`, never called by the engine);
  apps/cli/src/containers/mp4.rs:173-178; crates/playback-engine/src/seek.rs:390
Code (mod.rs:2039-2048):
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
Code (apps/cli/src/containers/mp4.rs:173-178):
```rust
    fn reanchor(&mut self, target: TimeSpan) -> Result<TimeSpan, EngineError> {
        // The demuxer keeps the trait's default typed refusal (the
        // moving half of in-session seek is its honest gap) —
        // forwarded, never invented here.
        self.inner.reanchor(target)
    }
```
What is wrong: `Mp4Demuxer` implements `seek` but NOT `reanchor`; the trait default
(plugin-api/src/demux.rs:113-120) is a typed refusal. The engine's seek step 4 calls ONLY
`demuxer_reanchor` (pipeline.rs:1499 → seek.rs:390), with no `seek` fallback. Every other
demuxer in crates/demux (avi, mkv, wav, ogg, adts, ps, ivf, aiff) implements `reanchor` —
MP4, the priority format, is the lone outlier. Executed proof (shipped
/home/z/my-project/bin/lumen, `open samples/sample.mp4 --start-paused` then `seek 1s`):
`event: TimingSeekFailed { error: CommandError { stage: Demux, code: Unsupported, message:
"[demux] unsupported (plugin.demux.reanchor.unsupported)" } }` — while the same session's
MediaInfo reports `seekable: true`. The fully-implemented, unit-tested `seek()`
(stss walk-back, moof-scan) is reachable only from the CLI open-time `--start-time` clamp
(containers/mp4.rs:120) and tests. Why it matters: user-visible core broken — the seek
command on any MP4 fails in the shipped binary; the capability claim is false. What correct
looks like: `impl Demuxer for Mp4Demuxer { fn reanchor(...) = self.seek(target) }` (the
compute-then-commit invariant the code already honors).
Confidence: HIGH — Verification: ran (TimingSeekFailed output above).

F-002 [S1] `trex` defaults read with a +4 field skew: duration=size, size=flags, flags=past-end
Where: mod.rs:936-946
Code:
```rust
                        if t.kind == *b"trex" {
                            let body = &data[t.body.clone()];
                            let id = u32be(body, 4).unwrap_or(0);
                            trex.push((
                                id,
                                TrexDefaults {
                                    duration: u32be(body, 16).unwrap_or(0),
                                    size: u32be(body, 20).unwrap_or(0),
                                    flags: u32be(body, 24).unwrap_or(0),
                                },
                            ));
```
What is wrong: ISO 14496-12 §8.8.2 TrackExtendsBox body after version/flags is
track_ID@4, default_sample_description_index@8, default_sample_duration@12,
default_sample_size@16, default_sample_flags@20. The code reads duration@16 (= the SIZE
field), size@20 (= the FLAGS field), flags@24 (past a 24-byte body → 0). The crate's own
test builder writes the correct layout (test_support.rs:414-420: track_id, desc,
duration@12, size@16, flags@20), so the skew is provable in-repo. Why it matters: any fMP4
that relies on trex/tfhd defaults instead of explicit trun fields (common DASH/ISM shapes)
gets durations = size values and sizes = flag values — e.g. default_sample_flags
0x02000000 becomes a 33.5 MB "size", over the MAX_SAMPLE_BYTES cap → every sample
damage-noted and dropped; keyframe flags become 0 (all-sync). The tests pass only because
the builder's truns carry all per-sample fields explicitly (trun flags 0xF01). What correct
looks like: duration@12, size@16, flags@20.
Confidence: HIGH — Verification: read (spec layout + builder cross-check; no default-reliant
fMP4 fixture available — UNVERIFIED by execution).

F-003 [S1] `stts`/`ctts` entry-count loops are unbounded — CPU DoS from a 12-byte box
Where: mod.rs:1253-1266 (stts), 1314-1332 (ctts)
Code (stts):
```rust
                b"stts" => {
                    let count = u32be(body, 4).unwrap_or(0) as usize;
                    for i in 0..count {
                        let run = u32be(body, 8 + i * 8).unwrap_or(0) as u64;
                        let delta = u64::from(u32be(body, 12 + i * 8).unwrap_or(0));
```
What is wrong: every other table loop is capped (`count.min(MAX_SAMPLES)` — stsz:1230,
stz2:1241, stco:1270, co64:1280, stsc:1290, stss:1303, trun:1727) but stts/ctts iterate
over the raw header count (up to 4,294,967,295) with only the *pushes* capped by
`take = run.min(MAX_SAMPLES - len)`. After the box body is exhausted, each iteration does
two failing `data.get()`s (~few ns) — a crafted 12-byte stts with count=0xFFFFFFFF spins
~4.3e9 iterations (~15-20 s) per box, repeatable per trak (thousands of traks fit in a
16 MiB moov) → minutes-to-hours single-thread spin on open. ffmpeg validates entry_count
against remaining box bytes; this code does not. What correct looks like: cap the loop by
`min(count, (body.len() - 8) / 8)` or MIN(count, MAX_SAMPLES).
Confidence: HIGH — Verification: read (arithmetic; NOT executed — would hang the sandbox).

F-004 [S1] Progressive backward seek drops samples / returns EOS: `order_cursor` never reset
Where: mod.rs:2127-2142 (seek commit), 1571-1577 (emission loop)
Code (commit):
```rust
        // Commit: per-track cursors (compute-then-commit — every
        // landing above already succeeded).
        for (ti, t) in self.tracks.iter_mut().enumerate() {
            if ti == ref_idx {
                t.next = land_idx;
            } else {
```
Code (loop):
```rust
        while self.order_cursor < self.order.len() {
            let (_, ti, si) = self.order[self.order_cursor];
            self.order_cursor += 1;
            let track = &self.tracks[ti];
            if si < track.next {
                continue; // pre-seek sample — skipped by the cursor
```
What is wrong: `seek_progressive` resets per-track `next` cursors, `pending`, and `eos`,
but never `order_cursor` (grep: only 3 uses — decl 310, init 627, loop 1571-1573). The
emission order is the file-offset-sorted `order` vec marched by `order_cursor`. Backward
seek after partial play: cursor is past the landing → the `si < track.next` skip never
revisits earlier order entries → playback resumes at the OLD position, silently discarding
everything between the landing and the cursor. After a full drain (cursor == order.len()):
seek(t=0) succeeds, then `step_progressive` falls straight through the while-loop →
`eos = true` → EndOfStream. The seek returns a landing TimeSpan that is a lie. The
fragment path is correct (`frag_pos` is reset, 2216). Note: the shipped CLI cannot reach
this (its in-session seek dies earlier at F-001); it is reachable by any direct
`plugin_api::Demuxer` consumer and by the open-time clamp only in the forward-from-0
shape. What correct looks like: `self.order_cursor = 0;` in the commit (the skip loop
already makes re-scan correct) — or a binary search to the landing.
Confidence: HIGH — Verification: read (code-path proof; not executed — no shipped surface
survives F-001 long enough to hit it).

F-005 [S2] `unpack_language` is off by one letter — the baseline `Some("voe")` root cause
Where: mod.rs:535-539
Code:
```rust
fn unpack_language(packed: u16) -> Option<String> {
    let c = |shift: u32| -> char { (b'a' + ((packed >> shift) & 0x1F) as u8) as char };
    let tag: String = [c(10), c(5), c(0)].iter().collect();
    if tag == "und" { None } else { Some(tag) }
}
```
What is wrong: ISO 14496-12 §8.4.2 packs each letter as `value = letter - 'a' + 1`
(1-based; 0 is invalid). The decode must be `('a' - 1) + value`; this code adds `'a' + value`
— every decoded letter is one too high. Executed proof: python3 walk of the baseline
sample.mp4 shows both mdhd fields packed `0x55C4`; spec decode = `und`, this formula =
`v`+`o`+`e` = `"voe"` — exactly the `language: Some("voe")` in the baseline MediaInfoReady
(audit/runs/baseline/mp4-smoke.md:14) on an ffmpeg file that declares no language
("und"). Because "und" never matches, the None branch is dead: every track reports a
bogus tag, and real tags decode shifted ("eng" → "foh"). `LanguageTag::parse` then accepts
the garbage (2-8 ASCII letters, ids.rs:509-520). The crate's own builder documents the
correct packing — test_support.rs:124: `b.extend_from_slice(&u32b(0x55C4)); // "und"` —
but no test asserts a language, so it is invisible. What correct looks like:
`char = (b'a' - 1 + v)`, treat v==0 as invalid, and "und" → None.
Confidence: HIGH — Verification: ran (python3 decode + baseline smoke output).

F-006 [S2] `elst` empty-edit duration (movie timescale) added as media ticks
Where: mod.rs:1090-1093 (store), 1866-1869 (use), 1908-1912 (extent); tests.rs:196 enshrines
Code (store):
```rust
                            if media == -1 {
                                empty_edit_delay = seg_dur;
                            } else if media > 0 {
                                media_time = media as u64;
                            }
```
Code (use):
```rust
        let dts_ticks = (s.dts as i64).saturating_sub(t.media_time as i64);
        let pts_ticks = dts_ticks
            .saturating_add(s.cts)
            .saturating_add(t.empty_edit_delay as i64);
```
What is wrong: `segment_duration` is in units of the MOVIE timescale (mvhd); `media_time`
is in the MEDIA timescale (mdhd). The code stores seg_dur raw and adds it to ticks stamped
on the media `TimeBase`. On the baseline file (mvhd=1000, video=12800, audio=44100) an
empty edit of N movie ticks would misplace PTS by 12.8×/44.1×. The doc at mod.rs:224-225
("empty-edit delay (media ticks)") states the wrong unit as fact, and the test
`elst_empty_edit_delays_and_media_time_trims` (tests.rs:185-197, builder mvhd fixed at
1000, track 9000) asserts `pts == 1_000` — the buggy value; the spec-correct value is
9_000 media ticks. Golden generated from the code under test. Also `extent_nanos`
(1908-1912) mixes the two scales in the same expression. What correct looks like: convert
seg_dur via `movie_timescale` → media ticks at store time.
Confidence: HIGH — Verification: read (both timescales measured on sample.mp4 by python3).

F-007 [S2] `ctts` v1 reads an i64 spanning into the next entry
Where: mod.rs:1319-1324
Code:
```rust
                        let off = if version == 1 {
                            i64be(body, 12 + i * 8).unwrap_or(0)
                        } else {
                            // v0 composition offsets are signed i32.
                            i64::from(u32be(body, 12 + i * 8).unwrap_or(0) as i32)
                        };
```
What is wrong: ctts entries are 8 bytes in BOTH versions (§8.6.1.3: v1 only changes the
offset's signedness, i32). `i64be` reads 8 bytes at `12+i*8`, i.e. the 4-byte offset OR'd
with the next entry's sample_count in its high 32 bits → astronomically wrong composition
offsets for v1 ctts files (negative-CTS muxes, `-movflags negative_cts_offsets`). The
sample files carry no ctts at all (verified by python3 walk), so the corpus never catches
it. What correct looks like: signed i32 read like the v0 arm.
Confidence: HIGH — Verification: read (spec layout; no v1 ctts fixture — UNVERIFIED by run).

F-008 [S2] `stz2` field_size 4 (packed nibbles) read as one byte per sample
Where: mod.rs:1238-1248
Code:
```rust
                b"stz2" => {
                    let count = u32be(body, 8).unwrap_or(0) as usize;
                    let field = u32be(body, 4).unwrap_or(0) & 0xFF;
                    for i in 0..count.min(MAX_SAMPLES) {
                        let v = if field == 16 {
                            u64::from(u16be(body, 12 + i * 2).unwrap_or(0))
                        } else {
                            u64::from(body.get(12 + i).copied().unwrap_or(0))
                        };
```
What is wrong: §8.7.3.2 permits field_size 4, 8, or 16. For 4, two samples pack per byte
(high nibble first, ffmpeg mov.c does exactly that). The `else` arm treats every byte as
one sample: values 16× too large and only half the declared samples consumed — corrupt
sizes/offset ladder for any 4-bit stz2 file. What correct looks like:
`((byte >> 4) & 0xF)` / `(byte & 0xF)` for even/odd i.
Confidence: HIGH — Verification: read (spec; no fixture — UNVERIFIED by run).

F-009 [S2] mdhd v1 language read at offset 28 — inside the duration field
Where: mod.rs:1104-1118
Code:
```rust
                                let (ts_at, dur_at, lang_at) = if version == 1 {
                                    (20, 24, 28)
                                } else {
                                    (12, 16, 20)
                                };
```
What is wrong: mdhd v1 is v/f(4)+ctime(8)+mtime(8)+timescale(4)@20+duration(8)@24+
language(2)@32. The code reads language at 28 — bytes 28..30 are the 5th/6th bytes of
the 8-byte duration. v1 mdhd files (>13 h at 90 kHz / >26 h at 44.1 kHz) report a
"language" built from duration bits (usually `0x0000` → "aaa" after F-005's shift).
v0 (the common case) is correct at 20 — which is why the baseline is unaffected.
What correct looks like: lang_at = 32 for v1.
Confidence: HIGH — Verification: read (spec layout; no v1 mdhd fixture — UNVERIFIED by run).

F-010 [S2] Fragment path burst-loads a whole moof's payloads — contradicts its own zero-copy doc
Where: mod.rs:1802-1838 vs module doc 32-35
Code (doc):
```rust
//!   child>parent check on every level. Sample payloads are *never*
//!   read whole: `next_event` pulls exactly one sample's bytes
//!   through the pooled [`PayloadSlots`] arena (2 × 16 MiB ceiling,
//!   the house zero-copy scheme).
```
Code (step_fragment):
```rust
                    let mut frag_samples = self.parse_moof(&moof, pos)?;
                    // Emit in DTS order (stable by track ordinal).
                    frag_samples.sort_by(|a, b| a.1.dts.cmp(&b.1.dts).then(a.0.cmp(&b.0)));
                    // Queue everything; return the first.
```
What is wrong: `step_fragment` reads EVERY sample payload of a moof before returning the
first packet, queueing the rest in `pending`. The pool has 2 arenas (mod.rs:614); after
two live packets `PayloadSlots::fill` spills to a fresh exact-size `Vec` per sample
(avi/riff.rs:654-670 — read, verified) — the "pulled zero-copy" claim degrades to
per-sample allocation, and a single-moof-whole-movie file (legal: trun counts up to
MAX_SAMPLES each, offsets within stream) buffers the entire file in RAM before the first
packet. The doc claim "never read whole / exactly one sample" is FALSE on this path.
What correct looks like: one sample per next_event (the progressive path's shape).
Confidence: HIGH — Verification: read (pool spill semantics read from riff.rs:588-671).

F-011 [S2] Any `mvex/trex` forces the fragmented path — complete stbl tables silently dropped
Where: mod.rs:779-796
Code:
```rust
        let any_samples = self.tracks.iter().any(|t| !t.samples.is_empty());
        self.fragmented = self.tracks.iter().any(|t| t.trex.is_some())
            || (!any_samples && self.any_top_box(b"moof"));
```
What is wrong: `fragmented` is decided by trex presence alone, even when every track has
a complete progressive sample table. In that case `order` is never built (797-813 gated on
`!self.fragmented`), so the stbl samples are NEVER emitted; `step_fragment` scans for
moofs and, if the file has none (mvex present, fragments optional per §8.8.1), the stream
immediately ends with EndOfStream and ZERO packets — no error, no damage note (the
"mixed-progressive-fragment-tracks" note at 790-792 fires only when `!any_samples`).
What correct looks like: fragmented requires trex AND (no samples OR a moof actually
present), and mixed shapes should damage-note.
Confidence: MEDIUM-HIGH — Verification: read (rare-but-legal ISO shape; no such fixture).

F-012 [S2] `stsc` linear rescan per chunk — O(chunks × stsc entries), ~1e11 within caps
Where: mod.rs:1351-1359
Code:
```rust
            for (ci, chunk_offset) in offsets.iter().copied().enumerate() {
                // stsc run for this chunk (1-based).
                let chunk_1 = ci as u64 + 1;
                let mut per = stsc.last().map_or(0, |(_, p)| *p);
                for (first, p) in &stsc {
                    if *first <= chunk_1 {
                        per = *p;
                    }
                }
```
What is wrong: stsc entries are sorted by first-chunk; the correct lookup is a binary
search. With both counts allowed up to MAX_SAMPLES inside a 16 MiB moov (800k stsc
entries ≈ 9.6 MB + 900k chunk offsets ≈ 3.6 MB fits), the nested loop is ~7e11
iterations ≈ tens of minutes — a header-driven CPU DoS, and even legit large files
(100k chunks × 50 stsc runs = 5e6) pay a needless linear tax. The code even
binary-searches stss (1306-1311) — the same treatment was skipped here.
What correct looks like: `stsc.partition_point(|(first, _)| *first <= chunk_1)`.
Confidence: HIGH — Verification: read (arithmetic; not executed).

F-013 [S3] Present-but-empty `stss` means all-sync (contradicts the spec reading it cites)
Where: mod.rs:1368-1372
Code:
```rust
                    let sync = match &sync_set {
                        Some(set) => set.binary_search(&sample_no).is_ok() || set.is_empty(),
                        // ISO 14496-12 §8.6.2: absent stss ⇒ all sync.
                        None => true,
                    };
```
What is wrong: §8.6.2's all-sync rule applies to the ABSENT box (the `None` arm). A
present stss with entry_count 0 declares zero sync samples; `|| set.is_empty()` flips
that to every-sample-sync — seek walk-back (2119) would land on non-keyframes.
Confidence: MEDIUM — Verification: read.

F-014 [S3] Seek's `pts_of` omits ctts and clamps; comment claims it mirrors `mint_packet`
Where: mod.rs:2097-2103 vs 1866-1869
Code:
```rust
        let pts_of = |t: &TrackState, s: &Sample| {
            s.dts
                .saturating_sub(t.media_time)
                .saturating_add(t.empty_edit_delay)
        };
```
What is wrong: `mint_packet` computes pts = dts − media_time + **cts** + delay in i64
(negative-capable); `pts_of` (used for the landing choice and the returned span) omits
cts and saturates at 0 — the doc comment "exactly what `mint_packet` stamps" (2088-2091)
is false. Effect is bounded (landing ≤ target stays true; B-frame cts skews the "last
PTS ≤ target" pick by one frame), but a caller re-targeting the returned stamp can chase
a stamp the demuxer will not produce — exactly the failure the comment promises cannot
happen.
Confidence: HIGH — Verification: read.

F-015 [S3] `MAX_BOX_DEPTH` is dead; the doc claims a depth guard exists
Where: mod.rs:124-126 (const), 29-32 (doc claim)
Code (doc):
```rust
//! - **Structural boxes are read whole, ceiling-checked** — `moov` ≤
//!   [`MAX_MOOV_BYTES`], each `moof` ≤ [`MAX_MOOF_BYTES`] — then
//!   walked in memory with a depth guard ([`MAX_BOX_DEPTH`]) and a
//!   child>parent check on every level.
```
What is wrong: grep shows `MAX_BOX_DEPTH` is declared and doc-linked but never referenced
by any code (the only enforcement sites are the fixed-shape call chains). The doc asserts
a guard that does not exist. (No exploitable recursion exists today — nesting is
hard-coded per box family — so this is a false claim, not a hole.)
Confidence: HIGH — Verification: read + grep (3 sites: doc, const, nothing else).

F-016 [S3] Scan-budget off-by-one: a scan that completes exactly at the budget errors
Where: mod.rs:1854-1857 (step_fragment), 2209-2211 (seek_fragment)
Code:
```rust
        }
        if budget == 0 {
            return Err(self.fail(error::scan_budget_exhausted()));
        }
```
What is wrong: if the 65_536th box ends exactly at EOF (`pos >= len` and `budget == 0`
both true), the loop exits and budget==0 reports exhaustion for a completed scan —
a false typed error on a well-formed (if box-sprawl) file.
Confidence: HIGH — Verification: read.

F-017 [S4] Track titles / udta metadata never read; `stsd` entry_count ignored
Where: mod.rs:888-890 (`title: None` always), 1158-1165 (only first stsd entry)
Code:
```rust
        Ok(Track {
            id: TrackId::new(ordinal as u32),
            stream: StreamId::new(ordinal as u32),
            kind: t.kind,
            language,
            codec,
            title: None,
```
What is wrong: `TrackInfo.name: None` and `metadata: {}` in the baseline come from here —
udta ©nam/©ART etc. are never parsed (only chpl + covr are). Multi-entry stsd
(entry_count > 1) silently drops entries 2..n without a damage note.
Confidence: HIGH — Verification: read + baseline output (name: None).

F-018 [S4] Double-`start` error code is a misnomer
Where: mod.rs:2009-2012
Code:
```rust
    fn start(&mut self) -> Result<Vec<Track>, EngineError> {
        if self.started {
            return Err(error::not_started());
        }
```
What is wrong: returns "not-started" when the demuxer IS started (second-call refusal).
The CLI works around it with the StartedMp4 cache (containers/mp4.rs:145-158) and even
documents the confusion. Name lies about the condition.
Confidence: HIGH — Verification: read.

F-019 [S4] ftyp compatible brands truncated at 64 bytes; unknown-box "bytes" note counts boxes
Where: mod.rs:722-727, 763-765
Code:
```rust
                b"ftyp" => {
                    let body = self.reader.read_prefix_at(
                        (end - header.payload_start).min(64) as usize,
                        header.payload_start,
                    )?;
```
What is wrong: only the first ~13 compatible brands are kept (harmless today, probe only
needs one hit, but `compatible_brands()` observability silently truncates); the
`unknown-top` damage note reports a box COUNT through a field named/typed as bytes.
Confidence: HIGH — Verification: read.

Cross-file [S2, root cause outside this file] `container: None` in MediaInfo
Where: crates/playback-engine/src/event.rs:925-930
Code:
```rust
) -> commands::catalog::types::MediaInfo {
    commands::catalog::types::MediaInfo {
        source: source_spec_wire(source),
        container: None,
        title: None,
```
What is wrong: the engine's `media_info_wire` hard-codes `container: None` (comment at
916-918: "container/title/metadata/chapters are the v0 honest unknowns (the probe-side
enrichment lands with the hosts that own it)"). This demuxer exposes `major_brand()` /
`compatible_brands()` (mod.rs:633-643) which nothing consumes, and the CLI's own harvest
sets `container: "mp4"` only in `MediaFacts` (apps/cli/src/containers/mp4.rs:224) feeding
decode-bench text (mp4-smoke.md:37 "(container mp4)") — never the wire MediaInfo. So the
demuxer parses the container fine; the name is dropped by an engine-layer hard-code.
Confidence: HIGH — Verification: read + baseline `container: None` + decode-bench "(container mp4)".

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| probe | 154-174 | REAL | — | brand list + QT heads; verified by unit test |
| u32be/u16be/u64be/i64be | 318-340 | REAL | — | bounds-checked readers |
| iter_boxes | 345-384 | REAL | — | size==0/1, overrun, non-advancing checks all real |
| read_top | 390-433 | REAL | — | 16-byte prefix, largesize ≥ 16, clamped |
| codec_of | 438-470 | REAL | — | 30-entry fourcc table |
| pcm_codec_of_ipcm | 481-522 | REAL | — | pcmC child walk; unit-tested |
| kind_of | 525-532 | REAL | — | hdlr map |
| unpack_language | 535-539 | FALSE | S2 | F-005 off-by-one, "und"→"voe" |
| oti_of_esds | 542-570 | REAL | — | expandable-len walk |
| esds_oti | 576-599 | REAL | — | heuristic anchors 28/76/78 |
| StickyShape::of/mint | 252-263 | REAL | — | replayable error shape |
| Mp4Demuxer::new | 610-631 | REAL | — | 2×16 MiB pool |
| major_brand/compatible_brands | 633-643 | REAL-UNPROVEN | S4 | observability unused by wire (cross-ref container) |
| is_fragmented | 645-649 | REAL | — | |
| chapters | 651-656 | REAL | — | |
| pool_bytes | 658-662 | REAL | — | |
| sample_entry_bytes | 664-672 | REAL | — | avcC/esds blob source (CLI harvests at containers/mp4.rs:237) |
| note / surface_damage | 674-687 | REAL | — | damage→Recovered events |
| fail | 689-693 | REAL | — | sticky error |
| open | 697-842 | REAL | S2 | moov-at-end OK; F-011 fragmented decision; F-019 ftyp trunc |
| any_top_box | 845-866 | REAL | — | bounded 65 536 |
| track_record | 869-910 | REAL | S2/S4 | F-006 units via empty_edit_delay; F-017 title None |
| parse_moov | 914-999 | REAL | S1 | F-002 trex skew lives here |
| parse_udta_chpl | 1003-1041 | REAL-UNPROVEN | S3 | layout is a guess (comment admits); capped 64; no fixture |
| parse_trak | 1045-1426 | REAL | S1/S2 | F-003 stts/ctts loops, F-007 ctts v1, F-008 stz2, F-009 mdhd v1 lang, F-012 stsc scan, F-013 stss empty |
| push_cover_track | 1433-1480 | REAL | — | wkt 13/14 only, ceiling-checked |
| queue_cover_packets | 1485-1515 | REAL | — | header-resident, bounds-checked |
| read_chapter_titles | 1519-1565 | REAL | — | 4096-byte cap; `?` on read kills open (harsh but typed) |
| step_progressive | 1569-1614 | REAL | S1 | F-004 cursor skip; truncation typed (good) |
| parse_moof | 1621-1773 | REAL | S1 | tfhd/tfdt/trun correct; consumes F-002's skewed defaults; trun capped |
| step_fragment | 1775-1861 | REAL | S2/S3 | F-010 burst load, F-016 budget off-by-one |
| mint_packet | 1864-1891 | REAL | S2 | F-006 units; negative ticks legal (i64) |
| seek_reference | 1894-1899 | REAL | — | |
| extent_nanos | 1903-1915 | REAL | S2 | F-006 mixed scales |
| Demuxer::start | 2009-2017 | REAL | S4 | F-018 misnomer |
| Demuxer::next_event | 2019-2037 | REAL | — | pending-first, sticky replay |
| Demuxer::seekability | 2039-2048 | REAL | S0 | claims Fast while engine seek dies (F-001) |
| Demuxer::seek | 2050-2083 | REAL | S1 | correct API; engine never calls it; F-004 backward flaw |
| seek_progressive | 2092-2148 | REAL | S1/S3 | F-004, F-014 |
| seek_fragment | 2152-2224 | REAL | S3 | F-016; frag_pos reset correct |
| cover_in_meta | 1923-1959 | REAL | — | FullBox/plain meta both tried |
| cover_in_udta | 1963-1969 | REAL | — | |
| trak_chap_refs | 1972-1992 | REAL | — | capped 64 ids |
| ticks_to_nanos / nanos_to_ticks | 1995-2006 | REAL | — | u128, saturating |
| consts (9) | 106-146 | REAL | S3 | MAX_BOX_DEPTH dead (F-015); rest enforced |

## 5. Test honesty

| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| probe_accepts_known_brands… (38 unit tests, executed: 38 pass) | mostly YES | real byte fixtures, typed-error pins; good coverage of refusal paths |
| elst_empty_edit_delays_and_media_time_trims | NO (for the units bug) | asserts the buggy 1000-tick shift with mvhd=1000/track=9000 — golden from the code (F-006) |
| fragmented_demuxes_moofs_with_tfdt_ladder | NO (for trex skew) | builder writes truns with ALL per-sample fields (0xF01) so skewed trex defaults are never consumed (F-002) |
| progressive_seek_lands_on_keyframe | NO (for F-004) | seeks forward only, never drains before seeking backward |
| fuzz_lite_mutations_never_panic (×2) | weak | small fixtures; cannot reach 4e9-count stts loops (would hang, not panic) |
| corpus_mp4_family_matches_the_ffmpeg_truth | NO-OP | corpus-v4 absent → early return reported as "ok" (re-verified this session; Wave-4 finding #8) |
| corpus_mp4_seek_lands_and_resumes | NO-OP | same skip; even if run, seeks once forward then drains |
| mp4_fixtures (5, executed pass) | YES | real fixture bytes through the real open path |

No test anywhere asserts a track language, a trex default, stz2, ctts v1, mdhd v1, or a
backward seek — the exact surfaces where F-002..F-009 live.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "seek lands on the keyframe-safe position ≤ target" | mod.rs:289 | SHALLOW | API seek works (tests), but the engine's only seek call (reanchor) refuses — shipped seek fails (F-001, executed) |
| "walked in memory with a depth guard (MAX_BOX_DEPTH)" | mod.rs:31 | FALSE | const never referenced (F-015) |
| "Sample payloads are never read whole: next_event pulls exactly one sample's bytes" | mod.rs:32-35 | FALSE | fragment path burst-loads whole moof (F-010) |
| "a normal edit's media_time subtracts…, an empty edit's segment_duration delays the timeline" | mod.rs:38-41 | SHALLOW | applied but in the wrong timescale (F-006) |
| "cmov (compressed MOV) is refused typed (bad-box vocabulary)" | mod.rs:68 | SHALLOW | no cmov code; it falls to unknown-top skip then no_moov() — different vocabulary |
| "stss absence means all-samples-sync (ISO §8.6.2)" | mod.rs:48-49 | REAL | None arm; but present-empty inverted (F-013) |
| "avcC/hvcC/esds/alac parse belongs to the codec cards" | mod.rs:84-86 | REAL | CLI harvests avcC at containers/mp4.rs:237 (Wave-4 finding #10) — the blob handoff works |
| "2 × 16 MiB ceiling, the house zero-copy scheme" | mod.rs:34-35 | SHALLOW | spills per-sample in fragment bursts (F-010, riff.rs:654-670) |
| container name reported | baseline expectation | FALSE (engine layer) | event.rs:929 hard-codes None; demuxer's major_brand() unused (cross-file) |

## 7. Dead / duplicated / unreachable code

- `MAX_BOX_DEPTH` — dead (F-015).
- `Demuxer::seek`/`seek_progressive`/`seek_fragment` (mod.rs:2050-2224) — real but
  unreachable from the engine's seek flow (reanchor only); reachable solely via the CLI
  open-time clamp and direct API tests (F-001).
- `compatible_brands()`/`major_brand()` — reachable only by decode-bench text path;
  nothing feeds the wire MediaInfo.
- `DemuxEvent::Recovered` events for `unknown-top-box-skipped` count boxes as bytes
  (F-019).

## 8. Dependencies & flags

- std-only + media-types + plugin-api + crate::avi::riff (PayloadSlots/SourceReader shared
  with the AVI demuxer — no new deps; Cargo feature-gating: none on this path; the release
  lane builds it — baseline cargo-build-release-lumen-cli.txt exit 0).

## 9. Open questions for the next agent

- W05-A02 (whoever owns crates/demux/src/mp4/{error,fuzz,test_support,tests}.rs): confirm
  no test elsewhere covers language/trex/stz2/ctts-v1 (I found none); error.rs vocabulary
  itself unaudited here.
- Engine owner: does any host other than the CLI exercise reanchor for MP4 (desktop/web)?
  F-001's fix belongs in this file (implement reanchor = seek) but the seekability/Fast
  claim should be re-checked after.
- Who owns playback-engine/src/event.rs:929 — the `container: None` hard-code is the
  answer to the master prompt's second question; the fix is engine-side (consume
  MediaFacts/container or demuxer brand), not demux-side.
- The trex skew (F-002) should be re-tested against a real default-reliant fMP4 (DASH
  on-demand profile with sparse truns) once a fixture exists — corpus-v4 is absent in
  every environment so far.
