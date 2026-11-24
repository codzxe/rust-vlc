# Re-verification — W20-A01 (independent, retry)

| Field | Value |
|---|---|
| Auditor | W20-A01 (re-verification wave) |
| Method | Re-derived from scratch: source reads + own execution. Original audit files NOT read. |
| Binary | target/release/lumen (release lane) |
| Scratch | audit/scratch/W20-A01/ (ts187/188/189.bin + captured outputs) |
| Verdicts | CONFIRMED: 8 (of which 3 downgraded on shipped-reachability) |

---

## F1 — TS 188-byte probe OOB panic — CONFIRMED (S1, executed)

Verdict: **CONFIRMED — S1.** Panic on `lumen open` of a 188-byte file whose first
byte is 0x47; the code comment claims such a file is "accepted" — it is not, the
process aborts with exit 101.

Derivation: the open path is `lumen open` → `apps/cli/src/demux_host.rs:446`
`media_probe::probe_stream` → registry (with_builtins registers `TsSignature`,
`crates/media-probe/src/signature/mod.rs:139`) → `TsSignature::probe`. The probe
window is the whole leading `max_lookahead` bytes, stopping at EOS
(`crates/media-probe/src/lib.rs:216-228 fill_window`), so a 188-byte file yields a
188-byte prefix.

Code — `crates/media-probe/src/signature/phase5.rs:155-170` (verbatim, the
executed panic site is :161):

```rust
impl ProbePlugin for TsSignature {
    fn probe(&self, prefix: &[u8], _meta: &SourceMeta) -> Option<ProbeHit> {
        if prefix.is_empty() || prefix[0] != 0x47 {
            return None;
        }
        if prefix.len() >= TS_PACKET_BYTES {
            if prefix[TS_PACKET_BYTES] != 0x47 {
```

`TS_PACKET_BYTES == 188` (phase5.rs:43). len==188 → `prefix[188]` indexes one
past the end. Mirror twin with the same latent bug at
`crates/demux/src/ts/mod.rs:315-322`:

```rust
pub fn probe(header: &[u8]) -> bool {
    if header.is_empty() || header[0] != 0x47 {
        return false;
    }
    // A lone sync byte proves nothing; a second at the stride does
    // (a whole 188-byte file is also accepted — the walk will tell).
    header.len() < PACKET_BYTES || header[PACKET_BYTES] == 0x47
}
```

Both comments ("a whole 188-byte file is also accepted" / "a whole single-packet
file is the demuxer's own walk to judge") are false: the code panics instead.
The demux-crate twin is only exercised by `ts/tests.rs:249-255` with slices that
avoid len==188, so nothing catches it.

Executed (this session, own run):

```
$ python3 -c "open('ts188.bin','wb').write(b'\x47'+b'\x00'*187)"
$ ./target/release/lumen open ts188.bin
exit=101
thread 'main' (19738) panicked at crates/media-probe/src/signature/phase5.rs:161:16:
index out of bounds: the len is 188 but the index is 188
```

Contrast (typed refusals, no panic):

```
$ lumen open ts187.bin  -> exit=65  error: [demux.corrupt]: corrupt structure (demux.ts.no-pat)
$ lumen open ts189.bin  -> exit=65  error: [probe.unsupported] ... (probe.no-demuxer) ... size=189
```

Why S1: a single-packet TS file is valid input; opening it aborts the player
process. Severity unchanged from the original claim.

## F2 — MP4 trex +4 field skew — CONFIRMED (S2, static)

Verdict: **CONFIRMED — S2.** trex defaults are read from the wrong payload
offsets on every fragmented MP4; the unit fixtures cannot detect it.

Derivation: `RawBox.body` is the payload after the 8-byte box header
(`crates/demux/src/mp4/mod.rs:377-380`), so a trex payload starts with 4
version/flags bytes. ISO 14496-12 §8.8.2 payload layout: track_ID@4,
default_sample_description_index@8, **default_duration@12, default_size@16,
default_flags@20**.

Code — `crates/demux/src/mp4/mod.rs:936-946` (verbatim):

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

Reads are duration@16/size@20/flags@24 — each +4 vs spec 12/16/20. Effects:
duration←default_sample_size, size←default_sample_flags, flags←`u32be(body,24)`
which on a minimal 24-byte payload is out of range → `None` → 0
(`u32be` is bounds-checked, mod.rs:317-321, so no memory-unsafety — wrong values).

Cross-check with the fixture writer — `crates/demux/src/mp4/test_support.rs:414-420`
(verbatim) writes SPEC offsets:

```rust
let mut trex = Vec::new();
trex.extend_from_slice(&u32b(i as u32 + 1)); // track_id
trex.extend_from_slice(&u32b(1)); // default_sample_desc
trex.extend_from_slice(&u32b(1024)); // default_duration
trex.extend_from_slice(&u32b(0)); // default_size (variable)
trex.extend_from_slice(&u32b(0)); // default_flags (sync)
```

i.e. duration=1024 lands at payload offset 12 where the parser never looks. The
fixtures' trex values (size=0, flags=0) plus the trun carrying every field
explicitly (`fullbox_f(b"trun", 0, 0x000_0F01, &trun)`, test_support.rs:466)
mean the tests never consume trex defaults — the skew is invisible to the suite.

Impact: defaults are consumed at `mod.rs:1691-1706`
(`trex.map_or(0, |t| t.duration)` etc.) when a trun omits sample_duration/
size/flags (flags 0x100/0x200/0x400 unset) — the standard DASH segment shape.
A real file with trex{duration=1024, size=0, flags=0x02000000} yields per-sample
duration=default_size=0 and size=default_flags=33554432 (33 MiB "samples").
Wrong durations/sizes/sync flags → S2 (wrong behavior on realistic fragmented
input; no crash).

## F3 — MP4 stts/ctts unclamped entry-count loop — CONFIRMED (S2, static)

Verdict: **CONFIRMED — S2.** A 4-byte declared entry count spins up to ~4.29e9
loop iterations regardless of box size; a header-driven CPU stall on open.

Code — `crates/demux/src/mp4/mod.rs:1253-1257` (verbatim):

```rust
b"stts" => {
    let count = u32be(body, 4).unwrap_or(0) as usize;
    for i in 0..count {
        let run = u32be(body, 8 + i * 8).unwrap_or(0) as u64;
        let delta = u64::from(u32be(body, 12 + i * 8).unwrap_or(0));
```

`ctts` identical (`mod.rs:1316-1317`, `for i in 0..count`). The loop bound is the
attacker-declared u32 with NO clamp — contrast every sibling: stsz
`count.min(MAX_SAMPLES)` (:1230), stco (:1270), co64 (:1280), stsc (:1290),
stss (:1303). Only the inner materialization is clamped
(`take`/`MAX_SAMPLES=1048576` at :122), so memory is capped (no OOM) but the
outer loop runs the declared count: past the payload `u32be`→None→run=0, each
iteration is two bounds-checked reads + ALU. entry_count=0xFFFFFFFF in a 12-byte
payload ≈ 4.3e9 iterations ≈ tens of seconds of CPU per box; the stbl walk
processes every stts/ctts box, so stacking N boxes multiplies the stall.
Reachable on `lumen open` of a crafted MP4 (probe → containers/mp4 → moov parse).
Executed: not (static trace per orders); magnitude ~4.3e9 iterations is exact
arithmetic (u32 max).

## F4 — mkv window-slide livelock, block size in (4MiB,32MiB] — CONFIRMED (S1, static; not executed per orders)

Verdict: **CONFIRMED — S1 (open-path hang).**

Constants: `MAX_CLUSTER_WINDOW = 4*1024*1024` (mkv/mod.rs:72),
`MAX_BLOCK_BYTES = 32*1024*1024` (:80).

Code — `crates/demux/src/mkv/mod.rs:644-663` (verbatim):

```rust
if size > MAX_BLOCK_BYTES {
    break Err(error::bad_structure(data_start));
}
if data_end > window.base + window.data.len() as u64 {
    // Spans past the window but inside the cluster: slide
    // so the block's head is at the window start.
    let from = window.cursor;
    let to = window.end.min(from + MAX_CLUSTER_WINDOW);
    let data = match self.reader.read_prefix_at((to - from) as usize, from, to) {
```

and the slide rebuilds `window = ClusterWindow { data, base: from, cursor: from,
.. }` then `continue`.

Trace for a block header of h bytes, declared size S with **h+S > 4MiB and
S ≤ 32MiB**, inside a cluster whose end covers data_start+S:

1. window = {base:B, data ≤4MiB, cursor:B, end:E}; the line-599 slide check
   (cursor ≥ base+data.len()) is false (cursor==base).
2. `parse_header_mem` at offset 0 → S; `data_end = B+h+S > base+data.len()`.
3. Line 647 fires: slide reloads `[cursor, cursor+4MiB)` — `read_prefix_at`
   seeks and reads **every iteration** (`crates/demux/src/mkv/ebml.rs:336-348`);
   new window has base==cursor==B.
4. Goto 1 — identical state. `window.cursor` only advances at :677 *after* a
   block parses, which requires h+S ≤ window size — impossible while h+S > 4MiB.

No exit condition ever changes: data_end ≤ end stays, S ≤ 32MiB stays, the
window can never exceed 4MiB. Infinite loop of 4MiB disk reads + header
re-parses: 100% CPU + I/O, no EOS, no error, no progress. Attacker-craftable
(declared block size in (4MiB−h, 32MiB]) and legal in the wild (Matroska puts no
4MiB cap on blocks; lossless 4K frames exceed 4MiB). NOT executed per orders.

## F5 — asf/header.rs:558 `off as u64 + size` overflow — CONFIRMED (defect), DOWNGRADED (unreachable from shipped entry)

Verdict: **CONFIRMED as a code defect; DOWNGRADED to latent** — the ASF demuxer
has no production caller in the shipped binary (S1-if-wired).

Code — `crates/demux/src/asf/header.rs:553-562` (verbatim):

```rust
let child = Guid::read(&bytes, off);
let size = guid::u64le(&bytes, off + 16);
if size < 24 {
    return Err(error::bad_object_size(header_offset + off as u64));
}
let end = off as u64 + size;
if end > header_size {
    return Err(error::bad_object_size(header_offset + off as u64));
}
let payload = &bytes[off + 24..end as usize];
```

`size` is a raw attacker u64 (only `size < 24` refused). With off=30,
size=2^64−8: `end = (30 + 2^64 − 8) mod 2^64 = 22` — release profile wraps, the
guard `end > header_size` sees 22 ≤ header_size and passes, then
`&bytes[54..22]` panics (slice begin > end). Debug builds panic at :558 with
'attempt to add with overflow' instead. Same wrap pattern again at :996
(`off as u64 + size > end as u64` in walk_header_extension).

Reachability (the downgrade): `AsfDemuxer`/`asf::probe` have no production
caller — the CLI container dispatch `apps/cli/src/containers/mod.rs:52-74` has
15 arms, none "asf", and `media-probe` `with_builtins`
(`crates/media-probe/src/signature/mod.rs:130-147`) registers no ASF signature.
Only `asf/tests.rs` and the `asf/fuzz.rs` harness (whose module docs promise
"Never panics") reach it.

## F6 — xing.rs parse_vbri rest[14..21] after len<14 — CONFIRMED (defect), DOWNGRADED (fuzz/test-reachable only)

Verdict: **CONFIRMED as a code defect; DOWNGRADED to latent** — parse_vbri's
only caller (`decode_stream`) is not on any shipped lane.

Code — `crates/codecs/src/audio/mp3/xing.rs:339-367` (key lines verbatim):

```rust
let rest = frame.get(pos + 4..).unwrap_or(&[]);
if rest.len() < 14 {
    return Err(corrupt(
        "mp3.vbri.truncated",
        ...
```

then (:364-367):

```rust
let toc_entries = u16::from_be_bytes([rest[14], rest[15]]);
let toc_scale = u16::from_be_bytes([rest[16], rest[17]]);
let toc_entry_size = u16::from_be_bytes([rest[18], rest[19]]);
let toc_frames_per_entry = u16::from_be_bytes([rest[20], rest[21]]);
```

Rustc-free arithmetic: the guard admits len ∈ [14, ∞). `rest[14]` needs len ≥ 15;
`rest[20]` needs len ≥ 21. len==14 (with frames/bytes at rest[6..14] nonzero to
pass the :356 zero-totals gate) → panic "index out of bounds: the len is 14 but
the index is 14" at :364; len ∈ [16,21) panics at :365-:367. No memory-safety
issue beyond the index panic; it is a plain panic on crafted input.

Reachability: parse_vbri's sole caller is `decode_stream`
(`crates/codecs/src/audio/mp3/decoder.rs:215`); decode_stream is called only by
in-crate tests (`mod.rs:110`, decoder tests) and the fuzz harness
(`mp3/fuzz.rs:131`) whose documented contract is "absence-of-panic/hang/
unbounded-allocation" — violated by this input. The shipped CLI's mp3 lane is
the trait `Mp3Decoder` (`apps/cli/src/pipeline_host.rs:261-265`, symphonia
`MpaDecoder`), which never calls parse_vbri.

## F7 — ass/parser.rs:722 `num + den/2` add-overflow — CONFIRMED (arithmetic), DOWNGRADED (unreachable)

Verdict: **CONFIRMED as an unchecked-arithmetic defect; DOWNGRADED to latent**
— the subtitles crate is not a dependency of any shipped app.

Code — `crates/subtitles/src/ass/parser.rs:716-724` (verbatim):

```rust
let to_percent = |milli: i64, extent: i64| -> Option<i32> {
    let num = milli.checked_mul(100)?;
    let den = extent.checked_mul(1000)?;
    if den == 0 {
        return None;
    }
    let q = (num + den / 2) / den;
    i32::try_from(q).ok().filter(|v| (0..=100).contains(v))
};
```

Bounds: `\pos` coordinates come from `parse_decimal_milli` (parser.rs:898-928),
accepting whole up to i64::MAX/1000 → milli ≤ ~9.22e18; `PlayResX` is
`parse::<i64>()` filtered `> 0` (play_res, parser.rs:692-699) → extent ≤
i64::MAX. The two checked muls cap num ≤ i64::MAX and den ≤ i64::MAX — and then
:722 adds them UNCHECKED.

Overflow example: `\pos` x = 92233720368547000 milli → num = 9223372036854700000;
PlayResX = 9223372036854775 → den = 9223372036854775000, den/2 = 4611686017925387500;
sum = 13835058054780087500 > i64::MAX (9223372036854775807). Debug: panic
'attempt to add with overflow' at :722. Release: wraps negative → q < 0 → the
(0..=100) filter rejects → the cue is counted unmappable (benign output; the
stats counter is wrong). No user-visible corruption in release.

Reachability: `crates/subtitles` is referenced only by `fuzz/Cargo.toml` —
no app Cargo.toml lists it. Latent; debug-build panic / release-defined wrap.

## F8 — dash/resolve.rs division by zero @duration="0" — CONFIRMED (S1-if-wired; executed evidence: W10-A08 exit 101)

Verdict: **CONFIRMED — division-by-zero panic, unconditional in every build
profile.** Reachability from a shipped entry point: none yet (DASH session is
library/test surface); prior executed evidence (W10-A08, exit 101) was reported
in my task brief (original files not read, per orders).

Code — `crates/streaming/src/dash/resolve.rs:731-737` (verbatim):

```rust
fn fixed_segment_count(mpd: &Mpd, period: &Period, timescale: u64, duration: u64) -> Option<u64> {
    let end = period_duration_ticks(mpd, period, timescale)?;
    let count = end / duration + u64::from(end % duration != 0);
    if count == 0 { None } else { Some(count) }
}
```

Arithmetic chain re-derived:
1. `crates/playlists/src/mpd.rs:1819`:
   `template.duration = Some(fatal_u64("SegmentTemplate@duration", value)?)` —
   `fatal_u64` (:1664-1669) is `value.trim().parse::<u64>()`; `"0"` parses fine,
   no zero rejection anywhere in the parser.
2. `resolve.rs:578-582`:
   `let duration = template.duration.ok_or(DashResolveError::MissingFact {...})?`
   = 0 → `fixed_segment_count(mpd, period, timescale, 0)`.
3. `:734` requires a known Period end (`Period@duration` or
   `MPD@mediaPresentationDuration − Period@start`, `period_duration_ticks`
   :718-729 — e.g. PT10S, timescale 1 → end = 10).
4. `:735` `end / 0` → Rust integer division by zero — panics in BOTH debug and
   release (unlike overflow, division by zero is an unconditional panic).
   The `count == 0 → None` guard at :736 is unreachable dead code for this input.

Sibling instances of the identical pattern: `expand_timeline` at :410/:416
(`span / entry.d` with an `S@d="0"` entry and a negative `@r`).

Trigger input: an MPD whose Period carries a known duration and whose
SegmentTemplate has `duration="0"`, `media` set, and no SegmentTimeline.

---

## Severity / verdict summary

| # | Finding | Verdict | Severity | Execution |
|---|---|---|---|---|
| F1 | TS 188-byte probe OOB | CONFIRMED | S1 | Own run: exit 101, panic phase5.rs:161 |
| F2 | MP4 trex +4 skew | CONFIRMED | S2 | Static (fixtures hide it) |
| F3 | MP4 stts/ctts unclamped loop | CONFIRMED | S2 | Static (~4.3e9 iters) |
| F4 | mkv window-slide livelock | CONFIRMED | S1 | Static per orders (no hang run) |
| F5 | asf off+size overflow → slice panic | CONFIRMED, DOWNGRADED (no shipped caller) | latent S1 | Static |
| F6 | xing parse_vbri rest[14..21] | CONFIRMED, DOWNGRADED (fuzz/test only) | latent S1 | Static + arithmetic |
| F7 | ass \pos num+den/2 overflow | CONFIRMED, DOWNGRADED (crate unwired) | latent S3/S4 | Static + arithmetic |
| F8 | dash fixed_segment_count /0 | CONFIRMED (S1-if-wired) | latent S1 | Prior W10-A08 exit 101 (cited), arithmetic re-derived |

No REFUTED findings; no UPGRADES. Three downgrades are reachability facts, not
code-defect refutations: asf (F5), mp3 decode_stream (F6), subtitles (F7),
dash-session (F8) are library/fuzz surfaces the shipped `lumen` open path never
drives.
