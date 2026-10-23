# Memory audit — 2026-10-23 (card W5-15)

The counting-allocator memory audit of the first-vertical decode path
(input access → media-probe → demux AVI → codecs MJPEG/PCM behind
codec-api's `Decoder` contract), measured with the new standalone
instrument **`tools/mem-audit`** (package `lumen-mem-audit`) on the
real engine crates. Every number in this document was measured on this
machine, this date, with the commands in §1; none is argued, rounded
in our favor, or carried over from another run. Raw tool output lines
(`MEMNOOP`/`MEMAUDIT`/`MEMSUM`/`MEMOVERHEAD`/`ATTRIB`/`STREAM`) are
quoted verbatim.

## 1. Methodology

**Instrument.** `lumen-mem-audit` installs a counting **global
allocator** (wrapping `std::alloc::System`) that maintains four
monotonic atomic counters — `alloc_bytes`, `alloc_calls`,
`dealloc_bytes`, `dealloc_calls` — plus a running max of live requested
bytes (`peak_live`). Sizes are the *requested* `Layout::size()`, not
system-internal block sizes. `realloc` is implemented explicitly so
growth/shrink count as the single real call they are.

**What is measured.** Per file: a *open window* (access → probe →
demuxer start → strf harvest → decoder configure) and a *walk window*
(packet loop → drains → teardown drops of demuxer, limbs, sink).
Windows are sequential snapshot deltas — never counter resets — and all
formatting/printing happens outside every window, so every counted
byte inside a window belongs to the audited code. The driver performs
no allocation of its own inside a window. `peak_B` = max(open peak,
walk peak) (a max over concatenated intervals is the max of the
per-interval maxima). A window's peak includes the process baseline
live at window open; `growth` = peak − baseline isolates the audited
code's own footprint. Both are reported.

**Per-file line** (the D-1 detector is the last two ratios over packets
*fed to decoders*):

```text
MEMAUDIT file=... class=... rep=N verdict=decoded|refused|fatal size=B container=... tracks=N limbs=v+a|v|a|none v_pkts=N a_pkts=N v_dec=N a_dec=N v_conc=N a_conc=N dmx_rec=N open_alloc_B=N open_calls=N open_peak_B=N open_base_B=N walk_alloc_B=N walk_calls=N walk_peak_B=N walk_base_B=N peak_B=N B_per_pkt=F calls_per_pkt=F [stage=...] [error=...]
```

**Verdicts.** `decoded` = EndOfStream reached, limbs drained;
`refused` = typed open-phase refusal (probe miss, non-AVI container,
demuxer start refusal — an honest bounded answer, numbers still
reported for the open phase); `fatal` = mid-walk typed error
(demux sticky fatal, concealment budget, non-per-frame decode error).
Per-frame damage is concealed with the engine's rule
(`Corrupt` / `Decoder{Bitstream}`, 1000-consecutive budget), mirroring
`apps/cli`'s bench.

**Runs.** All runs release profile (`cargo build --release -p
lumen-mem-audit`), shared target dir, single-threaded walk. Reproduce:

```sh
cargo run --release -p lumen-mem-audit -- --self-check
cargo run --release -p lumen-mem-audit -- <files...>           # CLASS=PATH overrides
cargo run --release -p lumen-mem-audit -- --repeat 2 <files...> # stability proof
cargo run --release -p lumen-mem-audit -- --demux-only <files...> # demux-only drilldown
cargo run --release -p lumen-mem-audit --example mjpeg-frame-attribution -- <file.avi>
```

**Corpus.** 545 files in `/home/z/my-project/corpus-big` (159 AVI; the
first vertical decodes AVI+MJPEG+PCM only — non-AVI files are honest
`refused` rows). Sweep set: 52 files across 7 classes (§4). Fixtures:
all 30 files under `tests/fixtures/generated/{valid,malformed}`
(§3).

## 2. Instrument overhead honesty (the no-op run + the counter cost)

A zero-file run prints only the floor line — the process cost before
any audit work:

```text
MEMNOOP alloc_bytes=1645 alloc_calls=5 live=1572
```

`--self-check` measures the counters' own per-call cost — the same
alloc+dealloc pair through the installed counting global allocator vs.
through `System` directly (the un-instrumented path), 5 size classes:

```text
MEMNOOP alloc_bytes=1777 alloc_calls=7 live=1668
MEMOVERHEAD size=16      iters=200000 counting_ns_per_pair=29.12 system_ns_per_pair=11.34 delta_ns_per_pair=17.78
MEMOVERHEAD size=256     iters=200000 counting_ns_per_pair=29.05 system_ns_per_pair=11.34 delta_ns_per_pair=17.72
MEMOVERHEAD size=4096    iters=200000 counting_ns_per_pair=38.24 system_ns_per_pair=17.71 delta_ns_per_pair=20.53
MEMOVERHEAD size=65536   iters=50000  counting_ns_per_pair=37.57 system_ns_per_pair=17.71 delta_ns_per_pair=19.86
MEMOVERHEAD size=1048576 iters=10000  counting_ns_per_pair=38.99 system_ns_per_pair=16.57 delta_ns_per_pair=22.41
```

**Honest reading:** the counting wrapper adds **~18–22 ns per
alloc+dealloc pair** (four relaxed atomic ops + one `fetch_max` per
alloc, two per dealloc). This biases *timing* benchmarks, not the
allocation *counts and bytes* this audit reports — the audit's numbers
are exact by construction (counter arithmetic, `tests::counted_allocation_moves_counters_by_exact_size`).
The no-op floor (1.6–2.3 KB live) rides under every window's
`baseline_live` and is subtracted out by the `growth` column.

## 3. Committed fixtures (30 files, all of valid/ + malformed/)

Run: all 30 files, one repeat each. 10 decoded (8 AVI damaged-but-decodable + 2
valid AVI), 19 refused (mp4 container, JSON/sha256 sidecars, flipped
magic), 1 fatal (trunc2, mid-walk demux truncation — partial counts
reported).

```text
MEMSUM class=fixture-malformed files=22 runs=22 decoded=8 refused=13 fatal=1 med_calls_per_pkt=2.08 max_calls_per_pkt=2.31 med_B_per_pkt=117321.58 max_B_per_pkt=120249.97 max_peak_B=443201 max_growth_B=353216 max_size_B=429612
MEMSUM class=fixture-valid files=8 runs=8 decoded=2 refused=6 fatal=0 med_calls_per_pkt=2.82 max_calls_per_pkt=3.55 med_B_per_pkt=120983.64 max_B_per_pkt=124645.70 max_peak_B=459914 max_growth_B=366536 max_size_B=497160
```

Reference decoded row (class-A fixture, 320×240 mjpeg + pcm_s16le,
60+60 packets, every packet decoded):

```text
MEMAUDIT file=tests/fixtures/generated/valid/avi_mjpeg_pcm.avi class=fixture-valid rep=1 verdict=decoded size=425516 container=avi tracks=2 limbs=v+a v_pkts=60 a_pkts=60 v_dec=60 a_dec=60 v_conc=0 a_conc=0 dmx_rec=0 open_alloc_B=218253 open_calls=53 open_peak_B=214758 open_base_B=17829 walk_alloc_B=14078590 walk_calls=250 walk_peak_B=449485 walk_base_B=96269 peak_B=449485 B_per_pkt=117321.58 calls_per_pkt=2.08
```

Decomposition (verified by the drilldowns in §5): 250 calls ≈ 60 video
packets × 4 (2 demux spill + 2 decoder handoff) + 60 audio packets × 0
(pooled uniform payload + zero-copy PCM) + teardown. The fixture's
encoder writes **static DHT tables**, so no per-frame table rebuilds.

## 4. corpus-big sweep (52 files, 7 classes)

Selection: every AVI-bearing class family plus refusal-path classes.
Non-AVI clean files, garbage `.bin`s and dirty non-AVI files are
*refused* rows by design (the first vertical has one demuxer) — their
open-phase peaks are still measured and bounded.

```text
MEMSUM class=avi-video files=10 runs=10 decoded=7 refused=3 fatal=0 med_calls_per_pkt=11.98 max_calls_per_pkt=12.50 med_B_per_pkt=2908407.00 max_B_per_pkt=24977678.84 max_peak_B=37472418 max_growth_B=37386020 max_size_B=217748608
MEMSUM class=avi-av files=7 runs=7 decoded=7 refused=0 fatal=0 med_calls_per_pkt=5.24 max_calls_per_pkt=9.29 med_B_per_pkt=1174195.27 max_B_per_pkt=14510174.84 max_peak_B=24625351 max_growth_B=24533028 max_size_B=110488954
MEMSUM class=multivideo-av files=3 runs=3 decoded=3 refused=0 fatal=0 med_calls_per_pkt=3.42 max_calls_per_pkt=5.91 med_B_per_pkt=45178.95 max_B_per_pkt=198595.22 max_peak_B=1169992 max_growth_B=1073294 max_size_B=1012574
MEMSUM class=dirty-avi files=12 runs=12 decoded=7 refused=4 fatal=1 med_calls_per_pkt=3.45 max_calls_per_pkt=12.71 med_B_per_pkt=199076.14 max_B_per_pkt=6428637.57 max_peak_B=8020770 max_growth_B=7928982 max_size_B=28290376
MEMSUM class=dirty-other files=2 runs=2 decoded=0 refused=2 fatal=0 med_calls_per_pkt=- max_calls_per_pkt=- med_B_per_pkt=- max_B_per_pkt=- max_peak_B=218465 max_growth_B=196935 max_size_B=960204
MEMSUM class=garbage files=6 runs=6 decoded=0 refused=6 fatal=0 med_calls_per_pkt=- max_calls_per_pkt=- med_B_per_pkt=- max_B_per_pkt=- max_peak_B=186517 max_growth_B=163618 max_size_B=32239
MEMSUM class=nonavi-clean files=12 runs=12 decoded=0 refused=12 fatal=0 med_calls_per_pkt=- max_calls_per_pkt=- med_B_per_pkt=- max_B_per_pkt=- max_peak_B=227788 max_growth_B=196919 max_size_B=1234328
```

All 24 decoded rows (raw `MEMAUDIT` fields; `growth` = walk peak over
walk baseline unless open-phase dominated; dims from ffprobe):

| file | size B | dims | limbs | v/a pkts | peak B | calls/pkt | B/pkt |
|---|---|---|---|---|---|---|---|
| 0001-avi-video | 8,089,236 | 3840×2160 | v | 133/0 | 37,472,418 | 2.17 | 24,977,678.84 |
| 0012-avi-video | 151,110 | 32×32 | v | 186/0 | 205,325 | 3.55 | 8,448.95 |
| 0026-avi-video | 1,493,842 | 720×480 | v | 28/0 | 3,995,061 | 12.50 | 2,908,407.00 |
| 0127-avi-video | 217,748,608 | 960×540 | v | 1338/0 | 5,646,668 | 11.98 | 4,473,551.91 |
| 0158-avi-video | 41,172,624 | 720×1280 | v | 616/0 | 4,450,781 | 12.02 | 2,901,008.13 |
| 0284-avi-video | 87,943,680 | 720×480 | v | 1502/0 | 4,137,933 | 12.01 | 2,882,697.28 |
| 0017-avi-video | 69,679,148 | 1280×720 | v | 95/0 | 10,960,211 | 11.96 | 8,868,778.27 |
| 0021-avi-av | 2,491,732 | 360×640 | v+a | 16/41 | 2,552,745 | 3.61 | 606,983.37 |
| 0022-avi-av | 779,896 | 320×240 | v+a | 48/137 | 870,657 | 0.66 | 160,344.38 |
| 0035-avi-av | 11,772,226 | 1280×720 | v+a | 203/107 | 10,300,701 | 7.92 | 4,908,762.43 |
| 0104-avi-av | 32,089,106 | 640×480 | v+a | 221/289 | 3,640,311 | 5.24 | 1,174,195.27 |
| 0191-avi-av | 110,488,954 | 1920×1080 | v+a | 94/29 | 24,625,351 | 9.29 | 14,510,174.84 |
| 0197-avi-av | 14,445,966 | 719×1280 | v+a | 171/986 | 10,281,626 | 1.17 | 1,094,559.77 |
| 0330-avi-av | 12,311,226 | 1600×900 | v+a | 256/101 | 16,050,907 | 8.64 | 8,339,330.70 |
| 0096-multivideo-av | 205,792 | 16×16(+mpeg4) | v+a | 54/23 | 211,795 | 2.35 | 1,642.78 |
| 0307-multivideo-av | 1,012,574 | 640×360(+mpeg4) | v+a | 42/110 | 1,169,992 | 3.42 | 198,595.22 |
| 0335-multivideo-av | 284,586 | 120×90(+mpeg4) | v+a | 45/48 | 215,913 | 5.91 | 45,178.95 |
| flip-bytes-041-0012 | 151,110 | 32×32 | v | 186/0 | 213,283 | 3.88 | 8,322.92 |
| head-size-lie-007-0280 | 1,754,290 | 64×48 | v+a | 519/115 | 214,078 | 9.77 | 22,712.55 |
| cut-mid-chunk-002-0106 | 643,916 | 1024×768 | v | 21/0 | 8,020,770 | 12.71 | 6,428,637.57 |
| dup-range-063-0307 | 1,022,966 | 640×360(+mpeg4) | v+a | 42/110 | 1,165,447 | 3.45 | 199,076.14 |
| bit-drift-region-045-0111 | 1,625,042 | 720×1280 | v | 120/0 | 4,257,596 | 2.18 | 2,776,565.48 |
| tail-garbage-012-0244 | 286,415 | pcm_s24le only | a | 0/38 | 217,672 | 0.13 | 511.95 |
| truncate-049-0297 | 2,791,266 | 640×480 | v+a | 157/57 | 3,514,129 | 1.59 | 1,796,204.35 |

Coverage notes (honest): 3 `avi-video` refusals are huffyuv streams
(the demuxer's `no-supported-streams` typed refusal — correct, first
vertical is mjpeg+pcm); `dirty-avi` refuses are probe/demux-start
refusals on smashed headers; the 1 fatal is `splice-other-017-0408`
(demux sticky error mid-walk, 501/503 frames decoded first).
`tail-garbage-012` decoded audio-only (its video track is mpeg4 —
outside the first vertical — so only the PCM limb ran). Peak on every
refusal path ≤ 228 KB regardless of file
size — the probe reads a bounded sniff window.

## 5. D-1 violation hunt (allocs-per-frame > 0 → site + cause)

The instrument's `calls_per_pkt` column is the D-1 detector. Three
drilldowns attribute it (all measured, §5.1–5.3); the register (§5.4)
lists every site. **Nothing in the engine crates was modified — these
are handoff notes, per the card.**

### 5.1 Demux-only drilldown (isolates demux + pool from decoders)

```text
MEMAUDIT file=media/0127-avi-video.avi class=demuxonly ... v_pkts=1338 ... walk_alloc_B=435446314 walk_calls=2676 walk_peak_B=767705 ... B_per_pkt=325445.68 calls_per_pkt=2.00
MEMAUDIT file=media/0026-avi-video.avi class=demuxonly ... v_pkts=28 ... walk_alloc_B=2975384 walk_calls=57 ... B_per_pkt=106263.71 calls_per_pkt=2.04
MEMAUDIT file=media/0021-avi-av.avi  class=demuxonly ... v_pkts=16 a_pkts=41 ... walk_alloc_B=4633392 walk_calls=35 ... B_per_pkt=81287.58 calls_per_pkt=0.61
```

0127: exactly **2 calls and 2×payload bytes per video packet** on the
demux side alone. 0021: 2 calls per *video* packet, ~0 per audio
packet (35 ≈ 16×2+3 over 57 fed).

### 5.2 Decoder-in-isolation (the attribution example)

`mjpeg-frame-attribution` re-feeds ONE real packet through the real
`MjpegDecoder` 200×, then streams the rest of the file once, per-packet
deltas histogrammed:

```text
# 0026 (720×480): frame 50113 B
ATTRIB calls=2 bytes=2764816 iterations=199      # steady state
ATTRIB calls=21 bytes=3802304 iterations=1       # first decode (context init)
STREAM calls=12 bytes=~2.87e6 packets=1 (×~27)   # real pipeline interleaved

# 0127 (960×540): frame 328259 B
ATTRIB calls=2 bytes=4147216 iterations=199
ATTRIB calls=21 bytes=5192132 iterations=1
STREAM: 1316 of 1322 packets calls=12; 5 packets calls=4; 1 packet calls=13
```

Steady-state decoder cost for a *static-table* frame: **2 calls =
output buffer + `SharedBytes` Arc wrap** (2,764,816 B = 2×(720×480×4)
+ 16 for 0026's RGBA output; 4,147,216 B = 2×(960×540×4)+16 for
0127). The in-pipeline STREAM norm of 12 = demux 2 + decoder 2 +
**~8 table-rebuild calls** (see 5.3).

### 5.3 Root cause of the ~8-call spread: per-frame DHT re-emission

The corpus generator's encoder re-emits a **different DHT segment on
every frame** (verified by extracting frames and hashing the DHT
payloads): 0127 DHT lengths run 109→119→123→122→124→122→126→124→126→125
B, all distinct hashes; 0026: 198/215/207/206. The decoder's
`DhtKey::matches` (allocation-free compare) correctly detects the
change and rebuilds — semantically required, since the tables *are*
different. Files whose encoder writes static tables measure clean:
0001 (DHT 84 B every frame) `calls_per_pkt=2.17`; 0022 (171 B static)
`0.66`; the committed fixtures (static) `2.08`.

### 5.4 Violation register (file:line + cause; NO fixes applied)

| # | site | per-event cost | cause |
|---|---|---|---|
| V-1 | `crates/demux/src/avi/riff.rs:527` — `let mut fresh = vec![0u8; want]` in `PayloadSlots::fill` | 1 alloc + zero-fill of the full payload | **Every video packet spills**: MJPEG payload sizes vary per frame and commonly exceed the pool's `slot_cap` (default **2 slots × 64 KiB**, `riff.rs:562`), so the exact-size 2-slot pool never hits for video (it does for uniform audio — measured 0 allocs/pkt). Measured: 2 calls + 2×payload bytes per video packet (§5.1). Handoff: pool sizing for frame-class payloads (the `slot_cap` `[PROPOSAL]` figure, design D4). |
| V-2 | `crates/media-types/src/packet.rs:16` — `SharedBytes::from_vec` (`Vec` → `Arc<[u8]>`) | 1 alloc + full buffer copy | The `Arc` conversion cannot reuse the `Vec` allocation (header must be inline). Doubles the spill cost (V-1) and wraps every decoded frame (V-4). This is the registered **Lead-owned 3.B3 pooled-`Arc` follow-up**; the "no copy" doc comment on `from_vec` is optimistic at the byte level (handoff note). |
| V-3 | `crates/codecs/src/video/mjpeg/huffman.rs:115` (`values.to_vec()`), `decoder.rs:248` (`DhtKey::of` → `vals.clone()`), `decoder.rs:613`, `parser.rs:473` (vals growth) | ~6–8 calls per packet on DHT-varying streams | Huffman table rebuild on streams whose encoder re-emits DHT per frame (corpus generator does; ffmpeg-default and fixture streams do not). The rebuild itself is *required for correctness*; the sites could reuse in-place buffers instead of fresh `Vec`s. Handoff. |
| V-4 | `crates/codecs/src/video/mjpeg/decoder.rs:466-467` (`out.resize` after `take_output`) + `mod.rs:325` (`FrameStorage::Cpu(SharedBytes::from_vec(..))`) | 2 allocs + 1 copy per decoded frame | The documented one-buffer-per-frame ownership handoff (decoder.rs module docs; pinned by `codecs`' `zero_alloc_decode.rs` at exactly 2 events). Frame-buffer pooling / pooled-`Arc` is the registered 3.B3 follow-up. |

**Clean paths (measured):** PCM audio limb — 0 allocs/packet (uniform
payloads pool; zero-copy native path). Refusal paths — 14–19 calls per
*file*, open phase only, peak ≤ 228 KB. Open phase — 197–235 KB
alloc / ≤ 65 calls per file, independent of file size (0127's 217 MB
open peak: 206,123 B).

## 6. Bounded-memory proof (largest files, streamed twice)

`--repeat 2` walks each file twice consecutively in one process — the
second run's peak must be dimension-stable, and the peak must not
scale with file size. Measured pairs (peak_B rep1 → rep2; walk totals
byte-identical across reps):

| file | size B | dims | peak rep1 | peak rep2 | Δ |
|---|---|---|---|---|---|
| 0127-avi-video | 217,748,608 | 960×540 | 5,639,512 | 5,640,014 | +502 |
| 0191-avi-av | 110,488,954 | 1920×1080 | 24,614,893 | 24,614,896 | +3 |
| 0001-avi-video | 8,089,236 | 3840×2160 | 37,467,863 | 37,468,922 | +1,059 |

Totals across reps are *identical* (0127: 5,985,612,452 B / 16,028
calls both reps; 0191: 1,784,751,505 / 1,143; 0001: 3,322,031,286 /
288) — allocation behavior is fully deterministic; the ≤ 1,059 B peak
deltas are baseline/argv noise, i.e. **peak is dimension-stable**.

**Peak does not scale with file size — it scales with frame
dimensions:**

| file | size B | dims | peak B | peak/size |
|---|---|---|---|---|
| fixture avi_mjpeg_pcm | 425,516 | 320×240 | 449,485 | 1.06 |
| 0127-avi-video | 217,748,608 | 960×540 | 5,646,668 | **0.026** |
| 0191-avi-av | 110,488,954 | 1920×1080 | 24,625,351 | 0.22 |
| 0001-avi-video | 8,089,236 | 3840×2160 | 37,472,418 | **4.63** |

The 217 MB file peaks at 5.6 MB (2.6% of size); the 8 MB 4K file
peaks at 37.5 MB (4.6× size). Peak ≈ output buffer (w×h×4 for RGBA
streams) + padded decode planes + live payload — a function of
*dims*, not duration: a 1338-frame file and a 1502-frame file of
similar dims peak identically (0127 vs 0284: 5.6 MB vs 4.1 MB at
960×540/720×480). The tool's own regression test
(`fixture_walks_twice_with_stable_peak`) pins the property (≤ 64 KiB
tolerance) for the committed fixture.

## 7. Limitations (named, not hidden)

- First vertical only: AVI + MJPEG + PCM. Every other container class
  appears as refusal rows (open-phase memory only). When demuxers are
  added, re-run the sweep — the tool needs zero changes.
- Counted sizes are requested sizes; the system allocator's real
  overhead per block (headers, page rounding) is not captured — counts
  and byte sums are exact, RSS is not measured (a `/proc` RSS sampler
  would be a separate instrument).
- The counting allocator's ~18–22 ns/pair overhead (§2) makes the tool
  unsuitable for timing claims; the W4-R2 speed-parity numbers remain
  the timing source of truth.
- Single-threaded walk: the engine's threaded pipelines (queues,
  playback-engine) are not exercised — this audit measures open+demux+
  decode in isolation, the same scope as `apps/cli`'s bench.
- `calls_per_pkt` blends the video and audio limbs (denominator = all
  fed packets); the §5 drilldowns are the limb-split view.

## 8. Handoffs to the Lead

1. **Root `Cargo.toml`** members array: `tools/mem-audit` appended
   (this card's one sanctioned line) — union with other waves' edits
   at merge.
2. **`boundaries.toml`**: needs a `[crates.lumen-mem-audit]` entry
   (arch-gate ownership map — Lead file, not edited here).
3. **Unsafe posture ratification** [PROPOSAL]: the instrument carries
   minimal, safety-commented `unsafe impl GlobalAlloc` +
   `#[global_allocator]` (+ raw alloc calls in the self-check/example)
   — no safe Rust surface exists for a counting global allocator. It
   is a measurement tool, not an engine/codec crate; the wave's
   `std::arch` exception for codecs/mjpeg SIMD is the adjacent
   precedent. Suggested ADR: "measurement instruments may carry unsafe
   impl GlobalAlloc".
4. **Fix follow-ups from the register** (engine crates, Lead-owned
   assignment): V-1 pool `slot_cap` sizing; V-2/V-4 the 3.B3
   pooled-`Arc`/frame-pool follow-up; V-3 in-place table rebuild.
   `media-types` `from_vec` doc comment ("no copy") contradicts the
   measured byte-level copy — one-line doc fix in a media-types-owned
   card.

— W5-15, 2026-10-23. Tool: `tools/mem-audit` (branch `wip/w5-15`).
