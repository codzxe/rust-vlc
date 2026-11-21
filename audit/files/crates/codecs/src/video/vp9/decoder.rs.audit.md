# AUDIT: crates/codecs/src/video/vp9/decoder.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-21 |
| Lines / functions | 577 / 17 (1 struct, 5 methods, 3 trait impls, 1 free fn, 7 test helpers/tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open x.ivf` → apps/cli/src/containers/ivf.rs → demux ivf "vp9" descriptor → apps/cli/src/pipeline_host.rs:313-317 `"vp9" => Vp9Decoder::new()` → `Decoder::decode`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 3 S4: 2 |
| Commands run | rg vp9 in baseline test logs (both runs cite execution); rg "VP90" producers; read codec-api decode.rs/contract.rs for DecodeOutcome semantics; sed -n quote verification |

## 1. One-paragraph verdict
The rusty_vp9 wrapper is real, reachable from the shipped CLI, and executed:
five 8-bit 4:2:0 differential lanes pinned to ffmpeg MD5s pass in baseline
(both `test-workspace.log.txt:1286-1292` and the no-fail-fast repair
`orchrun-test-workspace-nofailfast.log.txt:1245-1251`), and the CLI-lane tests
decode full VP9 streams end-to-end through the real demux+pipeline
(nofailfast log :2802 `vp9_lane_decodes_the_full_stream`, :2900
`webm_vp9_and_opus_decode_from_the_private_head`, :2945 `ivf_vp9_family_lane`).
Geometry is bounds-checked before any copy (the stride arithmetic cannot
panic). The defects are contained: the frame-slot pool silently degrades to
per-frame allocation after any resolution change, every frame is stamped
BT.709-limited although VP9's unspecified-color default is BT.601, and a
mid-stream 10-bit/profile switch turns into a per-packet fatal error by
design of the `?` at the emit seam. Engine-side semantics (push/Again/Eof)
are UNVERIFIABLE in this sandbox (no vendored rusty_vp9 source) but are
behaviorally pinned by the executed tests.

## 2. What actually works (verified)
— `differential_lanes_vs_ffmpeg_libvpx_vp9 ... ok` —
audit/runs/baseline/test-workspace.log.txt:1286-1292 and
orchrun-test-workspace-nofailfast.log.txt:1245-1251: five lanes (320x240
125f, 640x480 g30 90f, odd-size, …) whole-stream MD5 == ffmpeg libvpx-vp9
dump, through THIS wrapper + demux::ivf (crates/codecs/tests/vp9_differential.rs).
— CLI-lane VP9 decode executed: nofailfast log :2802, :2900, :2945 (apps/cli
tests over committed ffmpeg fixtures, incl. WebM V_VP9 + seek-recovery row).
— Unit tests executed (test-workspace.log.txt:776-779): typed bad-state,
garbage/zero-length packets → `Recovered` not fatal, wrong descriptor typed
refusal.
— Plane-bounds closure (emit_frame 224-241) verified by read: `checked_mul`
+ `checked_add` and `stride < pw` reject before any copy; `needed` ≤ plane
len guarantees every later slice index is in-range; the pre-copy check also
proves `len = w*h + 2*cw*ch` cannot overflow (the engine already allocated
≥ that many bytes).

## 3. Findings

### F-001 [S3] Frame-slot pool degrades to per-frame allocation after any resolution change
Where: decoder.rs:404-437 (quote 404-411, 433-436)
Code:
```rust
    let count = slots.len();
    for i in 0..count {
        let idx = (*next + i) % count;
        if slots[idx].len() != len {
            continue;
        }
```
```rust
    if slots.len() < FRAME_SLOTS {
        slots.push(fresh.clone());
    }
```
What is wrong: slots of a stale size are skipped but never evicted, and the
pool stops growing at FRAME_SLOTS=4. After a mid-stream resolution change
(VP9 does this on keyframes — adaptive content is the norm), the pool holds
four wrong-size slots, `slots.len() < FRAME_SLOTS` is false forever, and
every subsequent `emit_frame` allocates a fresh `vec![0; len]`.
Why it matters: the module's own posture ("without per-frame allocation in
steady state", 76-79) silently breaks for exactly the streams that change
size; per-frame heap churn in the decode hot path for the rest of the
session.
What correct looks like: evict/replace a non-matching slot (e.g. track the
current size and rebuild the pool on change), or key slots by size.
Confidence: HIGH — Verification: read; no test exercises a size change
through the pool (the differential lanes are constant-size).

### F-002 [S3] Every frame stamped BT.709-limited; VP9's unspecified-color default is BT.601
Where: decoder.rs:265-273 (quote 270)
Code:
```rust
        let frame_out = Frame::Video(VideoFrame {
            format: PixelFormat::I420,
            size: (frame.width, frame.height),
            data: FrameStorage::Cpu(data),
            pts,
            color: ColorDescription::BT709_LIMITED,
```
What is wrong: the wrapper hard-codes BT.709 limited for all output. The
doc (56-60) admits the engine does not surface color config and cites "the
av1 module's documented assumption" — but VP9 is not AV1: per the VP9
spec/libvpx, a stream that does not signal color space defaults to BT.601.
So for typical unsignaled VP9 (the majority of WebM/VP9 content) the
renderer receives BT.709 metadata for BT.601 pixels — a systematic color
shift on every frame.
Why it matters: silent visual drift on the default case; the "conservative
default" chosen is the wrong conservative default for this codec.
What correct looks like: BT.601-limited as the VP9 unsignaled default (and
surface real color config when the engine grows it).
Confidence: MEDIUM-HIGH — Verification: read + VP9 spec color-config
default; renderer-side effect not executed (no baseline run asserts color
output for VP9).
Severity: S3 (visual drift, no crash; documented as an assumption).

### F-003 [S3] engine.push rejection is fatal, contradicting the module's damage philosophy
Where: decoder.rs:337-346
Code:
```rust
        if let Some(engine) = &mut self.engine {
            engine.push(packet.data.as_slice(), ticks).map_err(|err| {
                corrupt(
                    "vp9.packet.push",
```
What is wrong: the module doc (34-40) states the damage model is
"skipped-and-counted recovery … a `Recovered` outcome" for frames the engine
rejects. Frames the engine rejects in `next_frame` indeed count as damage
(drain, 157). But a packet the engine rejects at `push` (nothing consumed)
propagates as a fatal `Err` from `decode` — the pipeline counts it toward
the concealment budget and eventually kills the session
(playback-engine/src/pipeline.rs:2385-2395).
Why it matters: doc-vs-behavior drift on the corruption path; a class of
corrupt input the wrapper promises to recover from can instead terminate
playback. Whether rusty_vp9's `push` can actually reject is UNVERIFIED (no
vendored source in sandbox; the executed garbage/zero-length tests show
those two classes pass push and error in drain instead).
What correct looks like: count a push rejection as `Recovered { skipped: 1
}` (the frame is lost either way) or fix the doc to name the split honestly.
Confidence: MEDIUM — Verification: read; engine behavior UNVERIFIED.

### F-004 [S4] Dead "VP90" accept arm
Where: decoder.rs:116-118
Code:
```rust
    fn accepts(descriptor: &str) -> bool {
        matches!(descriptor, "vp9" | "VP90")
    }
```
What is wrong: no producer mints the descriptor string "VP90": demux ivf
maps the fourcc to `"vp9"` (crates/demux/src/ivf/mod.rs: `b"n" => Some("vp9")`
family), MKV V_VP9 maps to "vp9" (crates/demux/src/mkv/header.rs), and
pipeline_host dispatches only on "vp9". The arm is unreachable dead code
that implies a second descriptor spelling exists.
Why it matters: misleading surface; a future demuxer emitting "VP90" would
quietly work here but not match the CLI's dispatch table, hiding a wiring
gap.
What correct looks like: accept "vp9" only, or make the CLI table and this
fn share one constant.
Confidence: HIGH — Verification: rg "VP90" over crates/ apps/ (only this
file and prose docs).

### F-005 [S4] Mixed-base timestamp construction assumes a per-stream constant timebase
Where: decoder.rs:261-264
Code:
```rust
        let pts = match (frame.pts, self.base) {
            (Some(ticks), Some(base)) => Stamp::Present(Timestamp::new(ticks, base)),
            _ => Stamp::Missing,
        };
```
What is wrong: `frame.pts` ticks can originate from an EARLIER packet
(alt-ref lag) while `self.base` is the MOST RECENT stamped packet's base
(328-330). If a container ever changes timebase mid-stream, ticks and base
are from different eras → wrong timestamps. IVF fixes the timebase and MKV
per-track scales are constant, so this is theoretical today — the comment
itself says "in practice".
Why it matters: latent assumption documented only in a comment, no assert.
What correct looks like: carry the base with the ticks into the engine, or
assert base stability.
Confidence: HIGH that the code does this; LOW practical impact —
Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Vp9Decoder::new | 110-112 | REAL | — | Default::default delegate |
| Vp9Decoder::accepts | 116-118 | REAL | S4 | dead "VP90" arm (F-004) |
| Vp9Decoder::take_damage | 122-133 | REAL | — | folds damage into outcome; Emitted→Recovered conversion is contract-legal (contract.rs:281-284 "Recovered is unconstrained"); frame count loss is by design |
| Vp9Decoder::drain | 145-161 | REAL | — | Again/Eof terminate; other errors count damage; relies on engine consuming on error — UNVERIFIED engine contract, but pinned by executed flush/garbage tests |
| Vp9Decoder::emit_frame | 166-275 | REAL | S3 | bounds-checked copies (verified); typed refusals vp9.bitdepth/vp9.layout (fatal per call via `?` at 350 — mid-stream profile change kills session by design); BT.709 stamp (F-002) |
| Decoder::configure | 279-301 | REAL | — | descriptor gate, fresh engine per generation, pool reset |
| Decoder::decode | 310-362 | REAL | S3 | stale drop, base latch, push fatality (F-003) |
| Decoder::flush | 364-391 | REAL | — | end-of-input latch, sticky Drained verified by contract suite + tests |
| pooled_frame_slot | 398-438 | REAL | S3 | pool degradation (F-001); fresh-slot exclusivity loud failure |
| tests: Sink / configured / packet | 447-487 | REAL | — | helpers |
| test wrong_descriptor_is_a_typed_refusal | 489-508 | REAL | — | executed (baseline :779) |
| test decode_before_configure_is_typed_bad_state | 510-527 | REAL | — | executed (:776) |
| test garbage_packet_is_recovered_not_fatal | 529-557 | REAL | — | executed (:777); would fail if push made garbage fatal |
| test zero_length_packet_is_recovered_not_fatal | 559-577 | REAL | — | executed (:778); pins that empty data passes push |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| vp9_differential differential_lanes_vs_ffmpeg_libvpx_vp9 | Yes, strongly | whole-stream MD5 equality vs external ffmpeg oracle + per-frame counts/sizes/pts ladder through the REAL demuxer and wrapper; goldens are ffmpeg's output, not the code's own |
| vp9_lane_decodes_the_full_stream (CLI) | Yes | full stream frame count through lumen_cli's open pipeline (nofailfast :2802) |
| garbage/zero-length tests | Yes for the Recovered contract | mutation: making push errors fatal or outcome Emitted would fail the matchers |
| No test covers | — | resolution change through the pool (F-001), BT.601 content color (F-002), a 10-bit lane plays-then-fails mid-stream (only the first-frame refusal lane F is pinned) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bit-exact against ffmpeg libvpx-vp9 on six lanes (431/431 frames)" | decoder.rs:10-13 (module doc) | REAL | vp9_differential executed in both baseline runs (5 MD5 lanes + typed-refusal lane F); "431/431" count itself not re-summed — UNVERIFIED arithmetic |
| "zero unsafe" | decoder.rs:22 | REAL | read: no unsafe tokens in file |
| "8-bit 4:2:0 → I420 … typed refusals" | decoder.rs:48-52 | REAL | emit_frame 185-207 + lane F refusal pinned in differential |
| "damage is counted … never fatal" | decoder.rs:34-38 | SHALLOW | true for drain-side rejections only; push rejections are fatal (F-003) |
| "without per-frame allocation in steady state" | decoder.rs:76-79 | SHALLOW | false after any resolution change (F-001) |
| "conservative BT.709-limited default" | decoder.rs:56-60 | FALSE-as-conservative | VP9's unsignaled default is BT.601 (F-002) |

## 7. Dead / duplicated / unreachable code
- F-004 "VP90" arm (no producer).
- `if let Some(engine) = &mut self.engine` at 337 re-checks what line 319
  already guaranteed — harmless duplication.

## 8. Dependencies & flags
- `codec-vp9 = ["dep:rusty_vp9"]` (crates/codecs/Cargo.toml:81) is in
  `default` (line 58) — the shipped `lumen` build (apps/cli/Cargo.toml:51,
  no feature override; release.yml:193 `cargo build --release --locked -p
  lumen-cli -p desktop`) compiles this file in. No cfg gates inside.
- rusty_vp9 "=0.1.3" pinned, default features (empty). Source not present
  in sandbox → engine internals UNVERIFIED (only behavior via tests).

## 9. Open questions for the next agent
- vp9/error.rs (other agent): are `corrupt`/`unsupported` mapping each
  variant to a public ErrorKind consistently with the taxonomy test?
- If a wave adds VP9 color-config surfacing, F-002 becomes a user-visible
  fix; until then the renderer eats the wrong matrix.
- Engine internals (push rejection classes, post-flush Again semantics)
  need a vendored-source audit or an orchestrator run:
  REQUEST-ORCH-RUN: cargo tree -p codecs -i rusty_vp9 (confirm single consumer, lockfile consistency only).
