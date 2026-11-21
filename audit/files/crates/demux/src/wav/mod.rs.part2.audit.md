# AUDIT: crates/demux/src/wav/mod.rs#part2 (chunk: lines 555-1109 of 1109; the file's final chunk)
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-21 |
| Lines / functions | 555 audited / 11 units (tail of `parse_container` 555-694, `parse_fmt_chunk`, `parse_ds64`, `parse_info_list`, `step`, `locate`, `landing_span`, and the full `impl Demuxer`: `start`/`next_event`/`seek`/`seekability`/`reanchor`) |
| File verdict (chunk) | REAL |
| Reachable from shipped entry point? | YES — `apps/cli/src/containers/wav.rs:50` `WavDemuxer::new(stream)` (lumen CLI WAV lane); also `apps/cli/src/demux_host.rs:742`. Executed in baseline: 51 `wav::*` tests green (orchrun-test-workspace-nofailfast.log.txt:1910-1952, 5856-5863), zero wav failures in the 4534/7/49 suite. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 5 |
| Commands run | `Read` 395-1109 (chunk + parse_container head for context, boundary at 554 = part1/A07's last line); `rg -n "WavDemuxer" apps/` (callers); `rg -n "test wav::" orchrun-…nofailfast.log.txt` (51 green, cited lines); `sed -n` on every quoted range; `rg -n "MAX_PACKET_BYTES|MAX_INFO"` + `sed -n 103,113p` (64 KiB packet cap, 64 KiB/1 KiB INFO caps); `rg -n "fn unit_size" -A 8 format.rs` + `sed -n 330,420p format.rs` (unit_size ≥ 1 proven: channels==0/rate==0/ADPCM block_align==0 all refused, MAX_CHANNELS=64 → PCM unit ≤ 512 B < 64 KiB → `MAX_PACKET_BYTES / unit ≥ 1`, no empty-packet livelock, no div-by-zero at :856); `rg -n "fn frames|fn emit|fn fail" mod.rs` (part1 helper contracts traced). |

## 1. One-paragraph verdict
Real, carefully bounded demuxer tail: the packetizer delivers whole
packetization units only, ladders PTS in sample frames, refuses partial
units with damage, and the seek trio (`locate`/`seek`/`reanchor`) is a
correct compute-then-commit pair — `seek` provably mutates nothing and
`reanchor` commits only after `locate` validates (pinned by baseline tests
:1946 `seek_errors_stay_sticky_free_and_demuxer_unmoved`, :1947
`seek_maps_frames_and_reanchors`). ADPCM seek floors to block boundaries;
PCM lands on exact frames; FLAC honestly refuses (no frame grammar). I
traced the arithmetic for overflow, division-by-zero, and empty-packet
livelock and found none reachable: `unit_size() ≥ 1` is enforced upstream
in `format::parse_fmt` and every unit size is below the packet cap. The
five findings are hygiene: a dead-defensive FLAC arm, sticky-before-pending
ordering that can strand queued damage, discarded error detail in two
`map_err(|_| …)`s, silent duplicate-`ds64` overwrite (parse gate lives in
part1's caller), and the truncation-as-terminal-error design choice (here
internally consistent and test-pinned, unlike AVI's split behavior).

## 2. What actually works (verified)
— 51 wav tests green in baseline (log lines 1910-1952, 5856-5863); the
chunk's behaviors pinned by name: `packets_are_bounded_and_laddered` :1940,
`pts_ladder_counts_sample_frames` :1943, `trailing_partial_frame_is_damaged_and_dropped`
:1948, `data_sliver_below_one_frame_is_no_data` :1929, `adpcm_seek_lands_on_block_boundaries`
:1928, `rf64_ds64_sizes_are_the_truth` :1944, `plain_riff_data_sentinel_means_to_stream_end`
:1941, `flac_packets_carry_no_stamps` :1935, `oversized_info_value_is_damage_not_fatal`
:1939, `declared_size_past_actual_is_typed_truncation_after_packets` :1930.
— Mutation reasoning: (a) if `want_units` dropped the `.min()` the
`packets_are_bounded` bound assert fails; (b) if the ADPCM landing did not
floor to block boundaries :1928 fails; (c) if `reanchor` committed before
validation :1946 fails; (d) if the sliver were rounded up instead of
dropped :1948/:1929 fail; (e) if `frames_emitted` skipped the short-read
whole-unit credit, `pts_ladder_counts_sample_frames` :1943 fails.
— Overflow/livelock tracing (this chunk): `available = effective_end -
walk` guarded by the :844 check; `whole_bytes` multiple of `unit`;
`want_units ≥ 1` whenever `whole_bytes ≥ unit` because `unit ≤ 65535 <
65536 = MAX_PACKET_BYTES`; `frames_emitted` clamped via
`.min(i64::MAX as u64)` at :903; `landing_span` clamps at :1013/:1016.

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/wav/mod.rs:996-999
Code:
```rust
            // Unreachable: the `spu` binding above already refused
            // the FLAC payload; the match keeps the exhaustiveness
            // honest for future codecs.
            format::WavCodec::Flac { .. } => return Err(error::seek_out_of_range()),
```
What is wrong: Dead-defensive arm. `locate` already returned at :957-962
(`let Some(spu) = fmt.samples_per_unit() else { return Err(...) }`), and
`samples_per_unit()` is `None` exactly for FLAC (format.rs:228-235) — the
match arm cannot be reached.
Why it matters: Unlike AVI's silent-drop dead branch (my mod.rs part1
F-001), this one fails loud (typed refusal), so the failure mode is safe —
but it is still unreachable code whose comment must be trusted rather than
checked. If a future codec with `samples_per_unit() == None` ≠ FLAC is
added, the arm silently mislabels its refusal reason as out-of-range.
What correct looks like: keep the arm but make it `unreachable!()`-free
typed distinct error, or restructure so exhaustiveness is over a type that
cannot encode the impossible case.
Confidence: HIGH — Verification: read of locate + format.rs
samples_per_unit.

### F-002 [S4]
Where: crates/demux/src/wav/mod.rs:1044-1050
Code:
```rust
    fn next_event(&mut self) -> Result<DemuxEvent, EngineError> {
        if let Some(shape) = &self.sticky {
            // Fatal errors are sticky (the walk frozen at the damage
            // site); the first return carried the full error, replays
            // carry stage/kind/context.
            return Err(shape.mint());
        }
```
What is wrong: sticky is checked BEFORE `pending` — a fatal error strands
any queued-but-unsurfaced `Recovered` events. Sequence: `start` queues
damage (:1034) → first `next_event` calls drain it → fine in practice; but
if the FIRST `step` fails fatally (source read error on the first packet)
while `pending` still holds start-time damage, `next_event` returns Err and
the queued damage events are never delivered — the engine sees the error
without the container's own account of what it already knew.
Why it matters: contradicts the damage-record-first contract stated at
:1063-1065 ("Damage queued by the final step surfaces BEFORE the terminal
EndOfStream") — the same courtesy is not extended to the terminal Err.
What correct looks like: drain `pending` before the sticky check (an event
already queued is a fact the caller deserves before the error replay), or
fold queued damage into the error's context.
Confidence: MEDIUM — Verification: read of next_event + start; the
stranding sequence requires a first-packet source error after start-time
damage — not covered by any baseline test (no failing wav test).
Blast radius: one lost observability event in an already-fatal path.

### F-003 [S4]
Where: crates/demux/src/wav/mod.rs:655-658
Code:
```rust
        let codec = CodecDescriptor::new(fmt.codec_descriptor())
            .map_err(|_| error::bad_format_params(0))?;
        let base =
            TimeBase::new(1, fmt.samples_per_sec).map_err(|_| error::bad_format_params(0))?;
```
What is wrong: both `map_err(|_| …)` discard the underlying error's
detail (which descriptor string failed to construct; why the TimeBase was
rejected). Position is also hardcoded `0`.
Why it matters: `parse_fmt` (format.rs:344-346) rejects zero rate and
over-ceiling channels at the real offset with the same error kind — a
`CodecDescriptor::new` failure here is structurally near-unreachable, so
if it ever fires, the operator gets a wrong position and no cause.
What correct looks like: propagate or log the inner error; use the fmt
chunk's real offset (available at this call site).
Confidence: HIGH — Verification: read; the near-unreachability is why this
is S4 not S3.
Note: the same pattern recurs at :842 and :964 (`step`/`locate`) — the
:842 comment even says "Structurally unreachable".

### F-004 [S4]
Where: crates/demux/src/wav/mod.rs:731-766 (gate missing; the caller side is :475-495 in part1/A07's range)
Code:
```rust
    fn parse_ds64(
        &mut self,
        payload_start: u64,
        size: u64,
        extent_end: u64,
    ) -> Result<Option<Ds64>, EngineError> {
```
What is wrong: `parse_ds64` has no duplicate guard, and its caller
(part1's walk) unconditionally overwrites `self.ds64` — a second `ds64`
chunk replaces the first with zero damage accounting. Contrast: duplicate
`fmt ` (":704-707 duplicate-fmt") and duplicate `data` (":497-499
duplicate-data") are both damage-noted, first-wins.
Why it matters: inconsistent damage doctrine for the same class of
duplication; a file with two `ds64` chunks silently takes the second's
riff/data sizes (which also re-drive `extent_end` growth at :481-493) with
no observable signal.
What correct looks like: `if self.ds64.is_some() { self.damage.note(size,
"duplicate-ds64"); return Ok(None); }` at the top of `parse_ds64` (or in
the caller).
Confidence: HIGH — Verification: read of parse_ds64 (no `self.ds64`
read) + part1 caller :475-495 (unconditional `self.ds64 = Some(ds64)` at
:494). Cross-ref to A07 (part1 owns the caller line).

### F-005 [S4]
Where: crates/demux/src/wav/mod.rs:844-850
Code:
```rust
        if self.walk >= region.effective_end {
            if region.effective_end < region.declared_end {
                // The data was clamped (RIFF extent ∩ stream) but
                // declared more: the container is truncated, and the
                // walk says so instead of a silent empty EOF.
                return Err(self.fail(error::truncated(region.declared_end, region.effective_end)));
```
What is wrong: design choice, not a defect: a truncated-but-playable WAV
plays its whole available prefix, then lands the engine in a typed Error
(sticky) rather than EndOfStream. Internally CONSISTENT here (WAV never
delivers partial units — the final sliver is damage-dropped at :857-866 —
so there is no AVI-style split behavior), and explicitly test-pinned
(baseline :1930 `declared_size_past_actual_is_typed_truncation_after_packets
... ok`).
Why it matters: cross-format UX inconsistency with AVI (my avi/mod.rs
part1 F-004): AVI's mid-chunk cut delivers a partial final packet and ends
EOS; WAV's equivalent cut drops the partial unit and ends Err when the
data was over-declared (but EOS when merely sliver-terminated). A user
playing a 99%-downloaded file of each format sees different final states.
What correct looks like: one engine-wide policy for "truncated after
playable prefix" — either both end Error, or both end EOS-with-Recovered.
Confidence: HIGH — Verification: read + baseline test name :1930 + AVI
part1 F-004 cross-ref.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `parse_container` (tail) | 555-694 | REAL | — | RF64-without-ds64 → typed bad_structure (:561-562); declared_end_final sentinel resolution; truncation-vs-no_fmt/no_data precedence honest; fact-frames disagreement counted both directions with byte total (:625-646); FLAC span falls back to fact (:659-669); track carries INAM title (:670-674) |
| `parse_fmt_chunk` | 698-725 | REAL | — | duplicate damage-noted first-wins; bounded by MAX_FMT_BYTES ∩ extent; short read → typed truncation with real positions; stores verbatim `fmt_bytes` for the decoder seam |
| `parse_ds64` | 731-766 | REAL | S4 | F-004 (no duplicate gate); ≥28-byte floor with damage for smaller; table skipped positionally per doc; `le_u64` indexing safe (bytes ≥ 28) |
| `parse_info_list` | 773-827 | REAL | — | bounded 64 KiB list / 1 KiB value; over-ceiling value = damage not fatal; saturation + `next <= pos` spin guard (:821-823); lossy UTF-8 with NUL trim (advisory) |
| `step` | 831-937 | REAL | — | whole-units-only packetization; `MAX_PACKET_BYTES / unit ≥ 1` proven (unit ≤ 65535) — no empty-packet livelock; short-read path counts the loss and re-reads (converges, one extra call — noted, not a defect); PTS ladder = frames_emitted, DTS mirrors, all-keyframe; FLAC unstamped by design (:914-916) |
| `locate` | 944-1002 | REAL | S4 | pure query (no `&mut self` writes); FLAC refused twice (belt+suspenders, F-001); negative target refused; past-end refused, at-end legal; ADPCM floors to block; PCM exact frame; total_frames=0 defensive branch unreachable post-start |
| `landing_span` | 1006-1019 | REAL | — | clamped ticks; end from `frames()` (consistent with locate's total_units) |
| `Demuxer::start` | 1023-1042 | REAL | — | idempotent via tracks cache; sticky replay; damage surfaced after parse (:1034) |
| `Demuxer::next_event` | 1044-1073 | REAL | S4 | F-002 (sticky before pending); pending → eos → step order otherwise correct; final-step damage surfaces before EOS (:1062-1069) |
| `Demuxer::seek` | 1075-1079 | REAL | — | provably pure: two `&self`-only helpers — D4 holds by construction |
| `Demuxer::seekability` | 1081-1093 | REAL | — | honest None for FLAC/unstarted/broken; Fast{indexed:false} for CBR — matches the arithmetic seek |
| `Demuxer::reanchor` | 1095-1108 | REAL | — | commit-only-after-locate; clears pending (stale pre-seek damage dies — documented); revives eos; landing reported |

## 5. Test honesty
| Test (baseline line) | Would it fail if the code were broken? | Reason |
|---|---|---|
| `declared_size_past_actual_is_typed_truncation_after_packets` :1930 | YES | pins F-005's exact behavior (packets then typed truncation) — mutation to silent-EOS fails it |
| `seek_errors_stay_sticky_free_and_demuxer_unmoved` :1946 | YES | D4: failed seek leaves no sticky, no cursor move |
| `adpcm_seek_lands_on_block_boundaries` :1928 | YES | floors-to-block mutation lands mid-block → fails |
| `trailing_partial_frame_is_damaged_and_dropped` :1948 + `data_sliver_below_one_frame_is_no_data` :1929 | YES | round-up-instead-of-drop mutations fail both |
| `packets_are_bounded_and_laddered` :1940 + `pts_ladder_counts_sample_frames` :1943 | YES | bound/`min()` removal, ladder skip, or short-read credit loss fail them |
| `rf64_ds64_sizes_are_the_truth` :1944 + `plain_riff_data_sentinel_means_to_stream_end` :1941 | YES | sentinel/ds64 resolution mutations fail |
| `flac_packets_carry_no_stamps` :1935 | YES | stamping FLAC would fail the "honestly absent" assert |
No tautologies found in the chunk-relevant set; none of the 51 wav tests failed in baseline.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "one packet per call, ≤ MAX_PACKET_BYTES, whole packetization units only" | mod.rs:829-830 doc | REAL | step :855-897 traced; test :1940 green |
| "Never commits — reanchor mutates only after this answers Ok (D4)" | mod.rs:940-943 doc | REAL | locate takes `&self`; reanchor :1097-1107; test :1946 green |
| "RF64/BW64 carry their size truth in a mandatory ds64" | mod.rs:558-560 doc | REAL | :561-562 refusal; tests :1944/:1945 green |
| "ADPCM floors to the containing block boundary" | mod.rs:985-988 doc | REAL | :990-994; test :1928 green |
| "FLAC-in-WAV … the stamps stay honestly absent" | mod.rs:899-901 doc | REAL | :914-916; tests :1934/:1935 green |
| pool claim "bounded by 2 × MAX_PACKET_BYTES" (doc :343-345, part1) | part1 doc | REAL-UNPROVEN | `PayloadSlots::new(2, MAX_PACKET_BYTES)` :297 traced; no alloc-counting run in baseline (same gap as AVI's walk_bench) |

## 7. Dead / duplicated / unreachable code
- F-001 (Flac arm in locate) — unreachable, loud.
- `step`'s base construction comment admits "Structurally unreachable"
(:840-842) — defensive, loud (`bad_structure`), acceptable.
- No duplication within the chunk; the AVI-parallel structure
(fail/surface_damage/emit, StickyShape) is deliberate cross-format pattern
reuse, noted as such in comments.

## 8. Dependencies & flags
- `format` module (A06's chunks) supplies validated facts — the chunk's
safety proofs (unit ≥ 1, channels ≤ 64, rate ≥ 1) live there and were
traced, not assumed.
- `plugin_api::{Demuxer, SeekSupport, DemuxEvent}`; `media_types` time
types; riff primitives shared with AVI (`pad_for`, FourCC). No feature
gates; nothing cfg'd out of shipped builds.

## 9. Open questions for the next agent
- A07 (wav/mod.rs#part1): the duplicate-ds64 gate belongs at your caller
(:475-495) — my F-004; also verify `read_prefix_at`/`read_exact_or_end_at`
short-read contracts used at :714/:742/:790 (part1/riff.rs).
- A06 (wav/format.rs): confirm `ima_samples_per_block`/`ms_samples_per_block`
cannot return 0 (I traced `None` for degenerate geometry, but the ≥1 bound
of the Some-case is yours to pin).
- Wave-3 demux wave: WAV lane exercised end-to-end anywhere? The CLI has
  containers/wav.rs; baseline smoke evidence is AVI/MP4 only — WAV playback
  is REAL-UNPROVEN at the engine level (unit tests prove the demuxer, not
  the lane).
