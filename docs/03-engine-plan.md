# 03 — Media Engine Plan (subsystem-by-subsystem specification)

Status: planning draft for Lead review. Task ID 1.A3.
Scope: the Rust media engine described in MASTER-PROMPT (MP) §8, structured by the
pipeline of MP §3 and the crate layout of MP §4. Companion documents: doc 01
(architecture overview — not yet present at time of writing; this document is
derived directly from the MASTER-PROMPT and notes where it assumes 01's
decisions), doc 04 (codec/format roadmap and external-crate candidates), doc 05
(command layer). External crates are never chosen here; where a capability needs
a third-party library this document says only "candidate: see doc 04".

Traceability: §2 ← MP §3; §3 ← MP §8 bullet 1; §4.x ← MP §4 crate names and
MP §8; §5 ← MP §8 bullet 2; §6 ← MP §8 bullet 4 ("a failed open or seek is
never reported as success"); §7 ← MP §16 Phase 2; §8 ← MP §13; §9 ← MP §8
bullet 3 ("no locks across blocking work, no blocking in real-time audio
callbacks").

Everything marked [PROPOSAL] is an invention beyond the brief and needs Lead
acceptance or an ADR. Facts not yet verified are marked [VERIFY].

---

## 1. Table of contents

1. Engine overview: pipeline, dataflow, ownership rules
2. Shared type foundations (`media-types`)
3. Per-subsystem specifications
   3.1 input · 3.2 media-probe · 3.3 demux · 3.4 packet-queue · 3.5 codec-api ·
   3.6 playback-engine · 3.7 media-clock · 3.8 audio-engine · 3.9 audio-output ·
   3.10 video-engine · 3.11 renderer-wgpu · 3.12 subtitles · 3.13 playlists
4. Cancellation and generation semantics
5. Seek specification
6. First vertical slice (Phase 2)
7. Engine test strategy
8. Threading summary
9. Open questions and decisions requested from the Lead

(§1 is this table of contents; §2–§4 and §5–§10 follow in order.)

## 2. Engine overview

### 2.1 Pipeline end-to-end

The engine is a staged pipeline (MP §3): **input** turns a `SourceSpec` (file,
URL, synthetic test source) into a byte stream with deadlines; **probe** scores
formats over a bounded lookahead; a **demuxer** is instantiated and emits
timestamped packets per track; packets flow through **bounded per-medium
queues** into **decoders** behind the `codec-api` traits; decoded audio goes
through the **audio-engine** (conversion, filters, mixing) into an
**audio-output** device whose real-time callback is fed from a ring buffer;
decoded video frames go through the **video-engine** presentation scheduler to
the **renderer-wgpu** surface. One **media-clock**, audio-master by default,
drives timing. Subtitles, chapters and metadata ride the same pipeline as timed
events. The **playback-engine** session object owns the state machine, mints
generation IDs, and orchestrates every transition. Playlists live outside the
pipeline and interact with it only by issuing source/queue commands.

### 2.2 Dataflow diagram

```
              command layer (doc 05): CLI · Tauri IPC · wasm binding · tests
                      │  typed commands (validated at boundary)
                      ▼                    ▲
        ┌───────────────────────────────────────────────────────┐
        │   playback-engine  (one session object per playback)  │
        │   state machine · generation minting · orchestration  │
        └──┬──────────────────┬──────────────────────┬──────────┘
           ▼                  ▼                      ▼
     ┌──────────┐    ┌──────────────────┐     ┌──────────────┐
     │  input   │───▶│ media-probe      │     │  media-clock │
     │ (source) │    │  └▶ demux        │     │ (audio-      │
     └──────────┘    │ packets + gen id │     │  master)     │
                     └───┬─────────┬────┘     └──────┬───────┘
                 A pkts  │         │  V pkts         │ clock reads
             ┌───────────▼──┐  ┌───▼────────────┐    │
             │ audio queue  │  │ video queue    │◀───┘ watermark
             └──────┬───────┘  └───┬────────────┘   signals
                    │ pull         │ pull
          ┌─────────▼────────┐  ┌──▼──────────────────────┐
          │ codec-api (A)    │  │ codec-api (V) + workers │
          └─────────┬────────┘  └──┬──────────────────────┘
                   │ frames        │ frames
          ┌────────▼─────────┐  ┌──▼──────────────┐
          │ audio-engine     │  │ video-engine    │
          │ convert·filter·  │  │ presentation    │
          │ mix              │  │ scheduler       │
          └────────┬─────────┘  └──┬──────────────┘
                   │ SPSC ring      │ present requests
          ┌────────▼─────────┐  ┌──▼──────────────┐
          │ audio-output     │  │ renderer-wgpu   │
          │ (RT callback)    │  │ (GPU surface)   │
          └──────────────────┘  └─────────────────┘
     subtitles / chapters / metadata ── timed events ──▶ renderer + event bus
     playlists ──▶ engine via SetSource / queue commands only
```

### 2.3 Ownership rules

1. The engine owns playback state and every media decision (MP §2): which
   tracks are default-selected (rules from `product.toml`, MP §5), buffering
   watermarks, seek target snapping, frame-drop policy, device fallback order.
   The UI owns presentation and component state, nothing else.
2. The playback session object is the **single writer** for mutable playback
   state (position, rate, selected tracks, state-machine node). Everything
   else holds read snapshots delivered as events.
3. The renderer is isolated from decode and demux (MP §3.4): `renderer-wgpu`
   depends only on media-types frame handles and its own surface types. It
   never sees packets, demuxers or codec types.
4. Crates depend downward only (MP §3.1–3.2, 3.8): media-types is the leaf;
   input/demux depend on media-types + shared interfaces; codec-api depends on
   media-types; audio/video engines depend on codec-api + media-clock; the
   playback-engine depends on all of the above; nothing engine-side depends on
   UI, Tauri, wasm-bindgen, or platform crates except through the adapter
   traits in §3.9. CI enforces this with the boundary check (MP §3.8).
5. No cyclic dependencies, no global mutable state; cross-thread communication
   is via explicit channels, ring buffers and generation-checked queues only.
6. Test utilities may depend on production crates, never the reverse (MP §3.7).

## 3. Shared type foundations (`media-types`)

(a) **Responsibility.** The leaf crate that defines the explicit vocabulary
types every other crate speaks (MP §8 bullet 1). No I/O, no threads, no
platform.

(b) **Type sketch.**

```rust
pub struct TimeBase { pub num: u32, pub den: u32 }        // tick = num/den s
pub enum Ts { Present(u64), Missing, Invalid }            // raw ticks, no sentinels
pub struct PacketTs { pub dts: Ts, pub pts: Ts, pub dur: Ts }

/// Normalized media time in nanoseconds. Constructed only via checked
/// conversions from (Ts, TimeBase); conversion never wraps silently.
pub struct MediaTime(i64);
impl MediaTime {
    pub fn from_ts(t: Ts, base: TimeBase) -> Option<MediaTime>; // None on overflow
    pub fn saturating_add(self, rhs: MediaTime) -> MediaTime;
}

pub type TrackId = u32;
pub enum TrackKind { Audio, Video, Subtitle, Data, Unknown }
pub struct TrackInfo {
    pub id: TrackId, pub kind: TrackKind, pub codec: CodecId,
    pub language: Option<LangTag>, pub title: Option<String>,
    pub default_selected: bool, pub bit_rate: Option<u64>,
}

/// Reference-counted, non-UTF8 byte buffer (candidate: see doc 04).
pub type Bytes = /* alias chosen in doc 04 */;

pub struct Packet {
    pub track: TrackId, pub ts: PacketTs, pub keyframe: bool,
    pub data: Bytes,                       // may be empty (container-level event)
    pub discontinuity: bool,               // explicit, set by demuxer
}
pub type Generation = u64;                 // minted only by playback-engine

pub enum PixelFormat { /* Rgba8, Nv12, I420(=Yuv420P), ... enumerated */ }
pub enum SampleFormat { I16, F32, PlanarF32, /* ... */ }
```

(c) **Threading.** None; all types are `Send + Sync` plain data.
(d) **Errors.** Conversions return `Option`/`Result`, never wrap. `Ts::Invalid`
is carried, not corrected: repairing timestamps is a demux-stage policy, and it
must be observable (counted, event-emitted).
(e) **Invariants.** `MediaTime` is only produced by checked conversion; no
sentinel values exist (`u64::MAX` is a legal tick count, not a marker);
`TimeBase.den != 0` enforced by constructor.
(f) **Tests.** Property tests (candidate: see doc 04) over random
`(Ts, TimeBase)` pairs: overflow never wraps, missing/invalid propagate,
round-trip within precision; unit tests for documented saturation semantics.

## 4. Per-subsystem specifications

### 4.1 input — byte-stream access abstraction

(a) **Responsibility.** Turn a `SourceSpec` into a `ByteStream`: positioned
reads with explicit seekability, size, deadlines, and cancellation. Implements
file, HTTP(S) (Phase 6), and test synthetic sources. Knows nothing of formats.

(b) **Trait sketch.**

```rust
pub enum Seekability { Seekable { size: u64 }, Unseekable { total: Option<u64> } }
pub enum ReadErr { Eof, Timeout, Interrupted, Unreachable, Denied, Corrupt, Gone }

pub trait ByteStream: Send {
    /// Reads up to buf.len() bytes; Ok(0) == Eof is an error variant, not a trap.
    fn read(&mut self, buf: &mut [u8], deadline: Instant) -> Result<usize, ReadErr>;
    /// Non-consuming lookahead used by probe. Returns bytes actually available.
    fn peek(&mut self, out: &mut Vec<u8>, want: usize, deadline: Instant)
        -> Result<usize, ReadErr>;
    fn seek(&mut self, to: u64) -> Result<u64, ReadErr>;      // Err if unseekable
    fn position(&self) -> u64;
    fn seekability(&self) -> Seekability;
    /// Cooperative cancellation checked between internal read chunks.
    fn set_cancel(&self, token: CancelToken);
    fn source_meta(&self) -> SourceMeta { /* mime hint, uri, content headers */ }
}
pub trait SourcePlugin { /* manifest-selected factory: SourceSpec -> ByteStream */ }
```

(c) **Threading.** One reader thread per open source (§9). The thread performs
blocking reads and honors `deadline` and the cancel token; adapters for sockets
must use timeout-capable APIs (candidate: see doc 04) so deadlines are real,
not advisory.

(d) **Errors.** Typed `ReadErr`; `Timeout` is not fatal: the session decides
(persistent timeout on a file → `Error(ResourceLimit)`; live/network source →
stay in `Buffering` and keep retrying within policy). Secrets (URLs with
credentials, headers) are redacted in logs and errors (MP §12).

(e) **Invariants.** Reads never exceed the deadline except by a documented
adapter margin; cancellation is observed within one internal chunk (≤ 64 KiB
of transferred data) [PROPOSAL: bound]; `peek` never consumes — the probe
prefix is handed to the demuxer, so unseekable sources are never re-read.
(f) **Tests.** Unit: synthetic stream with scripted faults (delay, truncate,
reset, garbage). Property: read/seek positions always consistent with
`Seekability`. Integration: missing file, unreadable file, file that shrinks
mid-read, interrupted network (scripted socket source), source that hangs
forever (deadline fires). Fuzz: malformed URI parsing.

### 4.2 media-probe — bounded format detection

(a) **Responsibility.** Identify the container format from a bounded lookahead
window, with confidence scores, and select the demuxer plugin. Never probe
unbounded data (MP §12: bound probing depth).

(b) **Type sketch.**

```rust
pub struct ProbeHit { pub format: FormatId, pub score: u8 /* 0..=100 */ }
pub trait ProbePlugin {
    fn probe(&self, prefix: &[u8], meta: &SourceMeta) -> Option<ProbeHit>;
}
pub struct ProbeConfig { pub max_lookahead: usize, pub min_score: u8 }
```

Defaults [PROPOSAL, overridable from `product.toml` resource limits]:
`max_lookahead = 64 KiB` hard-capped at 1 MiB; `min_score = 60`. Probing reads
*only* through `ByteStream::peek` up to `max_lookahead` bytes; there is no code
path that probes more.

(c) **Threading.** Runs on the session's open path (the input reader thread
fills the peek buffer; scoring itself is trivial CPU work on the session
thread).

(d) **Errors.** No hit ≥ `min_score` → `Error(Unsupported)` carrying whatever
is known: magic bytes inspected, mime hint, size. Never a guess presented as
fact (MP §8 bullet 6).

(e) **Invariants.** Scoring is a pure function of `(prefix, meta)`; ties break
by plugin priority from the manifest (MP §6), then by shorter prefix consumed
[PROPOSAL]. The probe never consumes stream bytes; the selected demuxer
receives `prefix` as already-read data. One-shot probing in v1: a format that
only becomes identifiable deep in the stream is reported unsupported, not
silently re-probed [PROPOSAL — VLC's "format growing" behavior is out of scope
v1, revisit in Phase 5].
(f) **Tests.** Unit: known-prefix corpus per format plugin. Property: for any
input, lookahead bytes read ≤ `max_lookahead` (instrumented stream asserts
this). Fuzz: random prefixes must never panic any probe plugin. Integration:
files with valid content but wrong extension (extension is a hint, not truth),
empty files, 1-byte files.

### 4.3 demux — container parsing, track discovery, packet emission, seeking

(a) **Responsibility.** Parse the container, expose tracks, duration and
seekability; emit packets with explicit timestamps; implement the seek
contract including keyframe indexes.

(b) **Trait sketch.**

```rust
pub enum SeekSupport { Fast { indexed: bool }, SlowLinear, None }
pub struct DemuxCtx<'a> {
    pub cancel: &'a CancelToken,       // checked between packets
    pub on_tracks: &'a dyn FnMut(&[TrackInfo]),  // discovery may be incremental
}
pub trait Demuxer: Send {
    fn open(prefix: Bytes, stream: Box<dyn ByteStream>, cfg: &DemuxConfig)
        -> Result<Self, DemuxError> where Self: Sized;
    fn tracks(&self) -> Vec<TrackInfo>;              // snapshot, may grow
    fn duration(&self) -> Option<MediaTime>;         // None = unknown, never guessed
    fn seekability(&self) -> SeekSupport;
    /// Returns the *actual* positioned time (≤ target normally; formats may
    /// only approximate). On Err the demuxer MUST be left in a defined state
    /// (see invariant D4) — this is what makes failed seeks recoverable.
    fn seek(&mut self, target: MediaTime) -> Result<MediaTime, DemuxError>;
    /// Next packet; Err(Eof) at end of stream. Interrupted if cancelled.
    fn read_packet(&mut self, ctx: &mut DemuxCtx) -> Result<Packet, DemuxError>;
    fn keyframe_index(&self, track: TrackId) -> Option<Vec<MediaTime>>; // sparse ok
}
```

(c) **Threading.** One demux thread per active source, owns the `Demuxer`
instance exclusively. No locks: packets flow into the bounded queues (§4.4).

(d) **Errors.** `DemuxError::{ Unsupported, Corrupt { offset, detail }, Eof,
Interrupted, ResourceLimit }`. Corrupt-but-skippable data: drop the damaged
packet/run, count it, emit a `DamagedData` event, continue — bounded recovery
(MP §8 bullet 4). Structural corruption that desynchronizes parsing: attempt
resynchronization once within a bounded window [PROPOSAL: 1 MiB or 1000
packets], else fatal `Corrupt`.
(e) **Invariants.**
- D1: every emitted packet carries `PacketTs` with explicit variants; repairs
  (e.g., interpolating from durations) are counted and flagged via
  `discontinuity = true` or a repair counter, never silent.
- D2: keyframe flags are format-truth: `keyframe=true` only when the container
  or codec grammar guarantees a random-access point.
- D3: track discovery may be incremental; `TracksChanged` events are emitted
  whenever the snapshot grows.
- D4: a failed `seek` leaves the demuxer at a defined position (either the
  pre-seek position or EOF) with a valid parsing state; this is the foundation
  of §6's failed-seek behavior.
- D5: `read_packet` observes cancellation between packets (returns
  `Interrupted`), so a stuck demuxer cannot outlive its generation by more
  than one packet.
(f) **Tests.** Per-format conformance kit (MP §6): valid corpus with recorded
expected track lists and packet counts; truncated-at-every-offset corpus;
corrupt-byte-injection corpus; property tests that emitted timestamps are
monotonic within a discontinuity-free run; seek tests: seek(N random targets)
must land within the format's documented tolerance of a keyframe ≥ target-or-
below per policy. Every demuxer plugin has a registered fuzz target (MP §6).

### 4.4 packet-queue — bounded queues, backpressure, generation stamps

(a) **Responsibility.** Bounded packet and frame buffers between demux,
decode, and the engines; enforce backpressure; drop stale generations at every
boundary (MP §8 bullet 2).

(b) **Type sketch.**

```rust
pub struct BoundedQueue<T> { /* condvar + Mutex<VecDeque<Stamped<T>>>>, or
    lock-free MPMC from doc 04 behind this type [PROPOSAL: start with the
    simple version, swap internals later without changing the API] */ }
pub struct Stamped<T> { pub gen: Generation, pub value: T }
pub struct QueueLimits { pub max_packets: usize, pub max_bytes: usize,
                         pub max_duration: MediaTime }   // whichever hits first

impl<T> BoundedQueue<T> {
    /// Parks the producer while full. Wakes on: space, generation bump, or
    /// deadline. Producer rechecks its own generation after every wake.
    fn push(&self, item: Stamped<T>, gen_check: &CancelToken)
        -> Result<(), PushAborted>;
    /// Consumer-side; Err(Empty) means "would block", callers poll or await.
    fn pop(&self) -> Result<Stamped<T>, QueueEmpty>;
    fn flush(&self, new_gen: Generation);  // O(len): drop all, publish gen
    fn watermark(&self) -> Watermark;     // {low, high} hit flags, bytes, span
}
```

Defaults [PROPOSAL, config in `product.toml` cache limits]: audio queue 1.0 s /
200 packets, video queue 2.0 s / 300 packets, both hard-capped in bytes by the
memory budget; subtitle/event queue small and duration-only.

(c) **Threading.** Producers: demux thread (packets), decode workers (frames).
Consumers: audio pump, video scheduler. The internal lock is held only for
deque pushes/pops — never across I/O or decode (MP §8 bullet 3).
(d) **Errors.** No fallible behavior of its own; `PushAborted` means "your
generation is stale or the queue closed" — the producer must stop, not retry.
(e) **Invariants.**
- Q1: at most `max_packets`/`max_bytes`/`max_duration` items resident, ever
  (property-tested).
- Q2: after `flush(new_gen)` returns, no `Stamped` with `gen < new_gen` can be
  popped, and a later `push` of an old-gen item is dropped at the boundary.
- Q3: backpressure is the only flow control: the demux thread parks when full;
  it never spins, never drops fresh data, and never blocks the session thread.
- Q4: watermark state is the single input to `Buffering` state transitions.
(f) **Tests.** Property: bounds hold under adversarial interleavings
(model-check a small state machine: push/pop/flush concurrent). Race tests:
producer parked on a full queue when flush happens must wake and observe
`PushAborted` within a bounded number of wakeups. Unit: duration accounting
with missing timestamps (unknown-duration packets must not wedge the
`max_duration` computation — they count as 0 toward duration but still count
in packets/bytes [PROPOSAL]).

### 4.5 codec-api — decoder traits, negotiation, output declaration

(a) **Responsibility.** Define the stable traits all decoders implement
(built-in plugins today; OS hardware decoders behind the same traits after the
Phase 0 ADR, MP §2). Own the push/pull decode contract, capability
negotiation, and the decoder error model.

(b) **Trait sketch.**

```rust
pub enum DecStatus { Ok, NeedMoreInput, Drained }

pub struct DecoderConfig { pub codec: CodecId, pub extradata: Option<Bytes>,
                           pub max_width: u32, pub max_height: u32, /* limits */ }
/// Output declaration: the engine builds its conversion path from this.
pub struct VideoOutDesc { pub pix_fmt: PixelFormat, pub w: u32, pub h: u32,
    pub aspect: AspectRatio, pub color: ColorDesc /* primaries, range */ }
pub struct AudioOutDesc { pub rate: u32, pub layout: ChannelLayout,
                          pub sample_fmt: SampleFormat }

pub trait VideoDecoder: Send {
    fn negotiate(&mut self, cfg: &DecoderConfig) -> Result<VideoOutDesc, CodecError>;
    fn send(&mut self, pkt: Packet) -> Result<(), CodecError>;      // non-blocking
    fn receive(&mut self, out: &mut Vec<VideoFrame>) -> Result<DecStatus, CodecError>;
    fn flush(&mut self);   // drop reorder buffers; next output restarts at a keyframe
    fn drain(&mut self);   // emit everything decodable, then DecStatus::Drained
}
pub trait AudioDecoder: Send {
    fn negotiate(&mut self, cfg: &DecoderConfig) -> Result<AudioOutDesc, CodecError>;
    fn send(&mut self, pkt: Packet) -> Result<(), CodecError>;
    fn receive(&mut self, out: &mut Vec<AudioFrame>) -> Result<DecStatus, CodecError>;
    fn flush(&mut self);
    fn drain(&mut self);
}
```

Capability negotiation is manifest-driven (MP §6): each decoder plugin's
manifest lists `formats` (codec IDs), priority, and limits; the engine registry
selects by codec ID then priority; config may disable/reorder, never invent
(MP §5). If no decoder exists for a track's codec: the track is reported
`Unsupported` in `TrackInfo`-derived selection state — playback continues with
the other tracks, and the UI shows a truthful unsupported state (never
fabricated silence or frames, MP §8 bullet 6).

(c) **Threading.** One `AudioDecoder` instance owned by the audio pump thread;
`VideoDecoder` instances owned by decode workers in a pool sized to cores
(persistent per track so internal state survives). `send`/`receive` are
non-blocking; the worker parks on packet availability, not inside the decoder.

(d) **Errors.** `CodecError::{ Unsupported, CorruptPacket { detail }, Fatal,
ResourceLimit }`. Contract: `CorruptPacket` affects only the current packet —
the decoder must return to a state where it can accept the next packet
(decoders for formats where a damaged packet poisons following ones must
document that and emit `Fatal` instead). Decoders never panic (workspace lints
deny `panic`, MP §13). `Fatal` from a video decoder ends only that track's
decode with a `TrackFailed` event; audio `Fatal` is session-fatal because the
clock master is gone.
(e) **Invariants.** Frames leave `receive` only with `Ts` resolved to
`MediaTime` by the caller (engine applies the track's time base and
discontinuity policy — decoders deal in raw ticks or packet-relative timing,
per their capability); `flush` makes the next `receive` return nothing until a
`send` after the flush; output after `drain` is `Drained` forever until
`flush`. Output format may change mid-stream (stream switching); `receive`
then returns a `FormatChanged(VideoOutDesc)` marker event [PROPOSAL] and the
engine rebuilds the conversion path.
(f) **Tests.** Contract test suite shipped in `testkit`, run against every
decoder plugin (plugin conformance, MP §6): flush/drain state machine
exercised exhaustively; corrupt-packet corpus per codec; decode-to-EOF; output
determinism (same input → identical frame hashes — required for differential
tests against external oracles, MP §13). Fuzz: bitstream parsers get dedicated
fuzz targets with memory and time bounds.

### 4.6 playback-engine — session model and state machine

(a) **Responsibility.** The orchestrator: one `Session` object per playback;
owns the state machine, mints generations, applies commands at defined points,
wires the pipeline, and emits the event stream the command layer (doc 05)
forwards to every client.

(b) **Types and state machine.**

```rust
pub enum SessionState { Idle, Opening, Probing, Demuxing, Buffering,
                        Playing, Paused, Ended, Stopped, Error }
pub struct SessionHandle { /* Send commands, receive events */ }
// Events (full list is doc 05's contract; engine-side subset):
// StateChanged, TracksChanged, Buffering{range}, SeekAccepted, Seeked{req,act},
// SeekFailed{reason, position}, FirstFrame{ts}, Ended, Error(stage,e),
// DamagedData{count}, DeviceLost, ClockMasterChanged, TrackFailed
```

Full transition table (trigger → target, with side effects):

| From | Trigger | To | Side effects |
|---|---|---|---|
| Idle | `Open(source)` | Opening | mint session epoch; spawn input thread with open deadline; emit `StateChanged` |
| Opening | stream ready | Probing | run probe (§4.2) |
| Opening | input error / deadline | Error | typed `Error(Inaccessible…)`; teardown |
| Opening | `Stop` | Stopped | cancel token; join threads |
| Probing | format ≥ min_score | Demuxing | construct demuxer with probe prefix; emit initial `TracksChanged` |
| Probing | no format | Error | `Error(Unsupported, known magic/mime)` |
| Demuxing | first playable packet queued | Buffering | watermark accounting begins |
| Demuxing | EOF before watermark (short media) | Playing | immediate start, then quick drain to `Ended` |
| Buffering | high watermark | Playing | clock un-paused; audio output started; first frame presented → `FirstFrame` |
| Buffering | EOF before high watermark | Playing | short media: start with what exists |
| Buffering | `Stop` | Stopped | flush, join |
| Playing | `Pause` | Paused | clock frozen; audio sink paused (device kept); demux fills to high watermark then parks |
| Paused | `Play` | Playing | clock resumes |
| Playing | low-watermark underrun | Buffering | clock frozen; `Buffering{…}` event |
| Playing | EOF + queues drained | Ended | clock stops at end; session retained (seek back allowed) [PROPOSAL] |
| Ended | `Seek` | Buffering | seek flow §6, then Playing |
| any active | `Stop` | Stopped | new generation; flush all; join; emit `SessionClosed`; engine → Idle |
| any active | fatal error | Error | typed error; teardown as Stopped |
| Playing/Paused/Buffering | `SwitchTrack` | Buffering | pipeline generation bump; flush affected limbs; demux rewinds/advances as needed; re-prime |
| any active | `SetSource` (replacement) | Opening | new session epoch; old pipeline torn down concurrently under its stale generation; fresh open begins |

**Command application points.** Commands are validated twice (MP §3.9): shape
at the command layer boundary, then semantics at application time on the
session thread. The session thread is the single writer and applies commands
only between pipeline ticks — it never runs I/O or decode itself, so command
latency is bounded by one tick (target: < 5 ms [PROPOSAL: measured in Phase 3
benches, MP §11]). Commands invalid for the current state (e.g. `Pause` in
`Idle`) return `CommandRejected{reason}` — a typed result, never an error
crash. Accepted ≠ completed: e.g. `Seek` returns `SeekAccepted` immediately;
truth of completion is the later `Seeked` event (MP §2 truthful UI).

(c) **Threading.** One session thread per `Session`. Owns: state, generation
counters, watermark policy, clock anchor decisions. Communicates: command
inbox (mpsc), event bus (broadcast), and control atomics to demux/decode/
schedulers.
(d) **Errors.** Every failure names its stage (MP §8 bullet 4):
`Error{stage: Open|Probe|Demux|Decode|AudioOut|VideoOut|Resource, kind}`.
Open failure never yields a Playing-looking session (MP §8 bullet 4).
(e) **Invariants.** S1: state transitions occur only on the session thread.
S2: a session in `Error`/`Stopped` is inert: no thread it spawned is joinable
later than a documented teardown deadline [PROPOSAL: 2 s]. S3: the session
thread performs no blocking call with unbounded duration. S4: source
replacement never reuses the old epoch's demuxer, queues, or decoders.
(f) **Tests.** Model-based property tests: the transition table above is the
model; random command sequences (open/play/pause/seek/switch/stop interleaved)
must never (i) reach an illegal state, (ii) emit events out of the documented
order per command, (iii) hang (all with virtual time). Integration: the §8
scenario list maps here 1:1. Soak: open/play/stop 1000× watching for leaks
(queue resident bytes, thread count, fd count — via testkit instruments).

### 4.7 media-clock — audio-master timing

(a) **Responsibility.** Provide `now(): MediaTime` for the whole pipeline,
anchor it to the audio output position when audio exists, fall back explicitly
when it does not, correct drift, and re-anchor on discontinuities.

(b) **Type sketch.**

```rust
pub enum ClockMaster { AudioOutput, SystemMonotonic }
pub struct ClockAnchor { pub media: MediaTime, pub at_sample: Option<AudioSampleCount>,
                         pub at_instant: Option<Instant> }
pub trait PlaybackClock: Send + Sync {
    fn now(&self) -> MediaTime;                 // media time, monotonic per anchor
    fn master(&self) -> ClockMaster;
    fn reanchor(&self, media: MediaTime);       // used at open, seek, discontinuity
    fn set_rate(&self, rate: f64);              // 1.0 default; range-checked
}
```

(c) **Threading.** No dedicated thread. The audio pump updates the anchor from
device sample counters (read atomically); readers (`now()`) are wait-free
(seqlock-style or two-word atomic — no locks on the read path; the internal
short lock, if any, is never held across I/O).
(d) **Errors.** The clock has no failure modes of its own; it reports master
changes (`ClockMasterChanged`) as events. Missing audio track or unsupported
audio → master is `SystemMonotonic`, and the UI is told (truthful state, not a
silent approximation).
(e) **Invariants and semantics.**
- Audio-master: `now() = anchor.media + (device_samples - anchor.at_sample) /
  rate` when audio is running; the device sample counter is the only trusted
  progress source. Video never advances the clock.
- Monotonic: within one anchor epoch, `now()` never goes backwards; on
  `reanchor` (seek, discontinuity) a jump is expected and reported.
- Discontinuity: demuxer flags (packet `discontinuity`, PTS jumps beyond a
  threshold [PROPOSAL: half the queue duration]) cause the session to
  `reanchor` at the next audio frame actually output, and video re-syncs
  against the new anchor.
- Drift correction: video presentation error is absorbed by the video
  scheduler (§4.10). Audio-side drift (device clock vs media rate) is absorbed
  by a bounded micro-resampler in audio-engine: correction rate limited to
  ±1000 ppm [PROPOSAL — final number from Phase 3 measurements; the limiter
  itself is the invariant, not the value].
- Pause: `now()` freezes (audio device paused, sample counter frozen); resume
  continues from the same media time.
(f) **Tests.** Unit: anchor arithmetic incl. rate ≠ 1. Property: monotonicity
under concurrent readers during reanchors; virtual-clock determinism. Tests
for the fallback: audio-less media keeps playing on system time and emits
`ClockMasterChanged`. Overflow: extreme sample counts and rates.

### 4.8 audio-engine — filters, conversion, mixing, gapless

(a) **Responsibility.** Take decoded `AudioFrame`s from every active audio
track, convert to the mixer format, run the user filter chain, mix, convert to
the device format, and feed the output ring. Also owns gapless transitions.

(b) **Type sketch.**

```rust
pub struct AudioPipeline { /* built per track: decoder-out -> filters -> mixer in */ }
pub trait AudioFilter: Send {
    fn input_desc(&self) -> AudioOutDesc; fn output_desc(&self) -> AudioOutDesc;
    fn process(&mut self, inp: &[AudioFrame], out: &mut Vec<AudioFrame>);
}
/// Insertion points (fixed by design, contents are config-driven, MP §5):
///  A: decoder output (per track) — e.g. format normalization
///  B: user filters (EQ, compressor, spatializer — profiles/, MP §4)
///  C: mixer stage — per-track volume/pan, summing
///  D: device adaptation — rate/channel/format conversion, micro-resample
pub struct MixPoint { pub track: TrackId, pub gain_db: f32, pub pan: PanLaw }
```

(c) **Threading.** The audio pump thread (§9): pull frames → filter chain →
mix → push into the SPSC ring feeding the device. The pump is deadline-driven:
it must keep the ring above a safety level computed as device latency + jitter
margin [PROPOSAL: 20 ms default]. The pump never allocates per frame (buffers
preallocated at format negotiation).
(d) **Errors.** Filter graph construction failures are `Error(Resource)` at
setup; runtime filter faults isolate to a bypass of that filter plus an event
— audio continues (bounded recovery, MP §8 bullet 4). Underrun (pump too slow)
is measured and reported as `AudioUnderrun{count}` (MP §11 metric), audible
glitch policy: output silence, never repeat old audio.
(e) **Invariants.** One mixer format chosen at first track negotiation
[PROPOSAL: floating-point planar at the track's rate, converted at stage D];
later tracks convert into it. Sample counts leaving the pipeline map 1:1 to
device samples advanced (clock correctness depends on this). Per-track volume
never clips the sum before stage D limiting [PROPOSAL: soft limiter at stage D
end, on by default, configurable].
(f) **Tests.** Unit: conversion correctness (rates, layouts) against
generated sine/sweep signals; mixer sums; gain math. Property: for any filter
chain, `out_samples == in_samples` unless the filter declares a rate change
(model-checked). Gapless: two synthetic tracks spliced at a sample boundary —
no gap, no overlap, no clock jump. Fuzz: filter parameter ranges.

### 4.9 audio-output — device abstraction, real-time rules

(a) **Responsibility.** Own the OS audio device behind an adapter trait; run
the real-time callback; handle device loss and device switching (platform
adapters implement the trait; OS types never leak, MP §3.6).

(b) **Trait sketch.**

```rust
pub struct DeviceDesc { pub name: String, pub layout: ChannelLayout,
                        pub rate: u32, /* pref format */ }
pub enum SinkEvent { DeviceLost, DeviceChanged, Underrun(u64) }
pub trait AudioSink: Send {
    fn open(&mut self, want: AudioOutDesc) -> Result<DeviceSession, SinkError>;
    fn pause(&mut self) -> Result<(), SinkError>;
    fn resume(&mut self) -> Result<(), SinkError>;
    fn latency(&self) -> MediaTime;
    fn sample_counter(&self) -> u64;   // the clock's truth source
}
/// The callback contract (RT rules below):
pub trait RenderCallback {
    /// Copy interleaved frames from the ring into `out`. Runs on the device's
    /// high-priority thread.
    fn render(&mut self, out: &mut [f32]);
}
```

(c) **Threading + real-time rules (binding, MP §8 bullet 3).** The callback
executes on the OS device thread. Inside `render` it is **forbidden**: any
lock except the SPSC ring's wait-free index swap; allocation; logging; syscalls;
unbounded loops. Enforcement: `testkit` provides a fault-injecting allocator
and a lock-tracer that run the callback under an allocator hook which panics
(or flags in tests) on allocation [PROPOSAL: tooling detail]; code review plus
these tests are the gate. The callback only: reads ring indices, `memcpy`s
frames, fills silence on underrun, increments the sample counter.
(d) **Errors.** `DeviceLost` (unplug, server restart) → event; session policy:
switch to the next device from the configured fallback order, or enter an
explicit `Buffering`-like suspended state if none — playback position is
preserved, never silently restarted [PROPOSAL: suspend-and-notify default].
Open failure at start is a typed `Error(Device)`.
(e) **Invariants.** The sample counter is monotonic and never rewinds across
device switches (offset-carried). Callback worst-case execution time is bounded
and measured (testkit asserts under synthetic CPU load) [VERIFY: per-platform
numbers to be recorded in Phase 3].
(f) **Tests.** `NullSink` and `CapturingSink` in testkit for determinism;
`VanishingSink` that drops the device mid-stream; `StallingSink` that stops
pulling. Platform integration runs against real devices labeled as
real-hardware evidence (MP §13).

### 4.10 video-engine — presentation scheduling

(a) **Responsibility.** Decide *which* frame is on screen *when*: consume
decoded frames, compare against the clock, schedule presents to the renderer,
apply the late-frame/drop policy.

(b) **Type sketch.**

```rust
pub struct PresentRequest { pub frame: VideoFrameHandle, pub when: MediaTime,
                            pub gen: Generation }
pub enum DropPolicy { Never, Late { threshold: MediaTime }, Auto }
pub struct SchedulerCfg { pub policy: DropPolicy, pub max_repeat: usize,
                          pub target_lead: MediaTime /* vsync lead */ }
```

(c) **Threading.** One scheduler thread (vsync-paced via the renderer's frame
callback or a timed wait). Pulls frames from the video frame queue, keeps a
small reorder window, emits `PresentRequest`s to the render thread.
(d) **Errors.** None fatal: a missing frame means repeat the previous; a
missing *track* means the renderer shows background + subtitles if any.
(e) **Invariants and policy.**
- Frame timing: present the frame with the largest `pts ≤ clock + target_lead`
  [PROPOSAL: lead = half a frame duration]. If none is due, repeat the last
  frame (up to `max_repeat`, then background) — video is subordinate to the
  audio clock, always.
- Late frames: `pts < clock − threshold` → drop, count, advance (`DropPolicy::
  Late`, threshold default one frame duration [PROPOSAL]; configurable, MP §5).
  Audio is never dropped for video's sake.
- Inter-frame reorder (B-frames) is the decoder's job (codec-api); the
  scheduler only handles display order.
- Deinterlace: out of scope here; a later video filter plugin (MP §6) inserted
  between decode and schedule — the insertion seam exists in the pipeline
  (frame filter point), and this document reserves it.
(f) **Tests.** Deterministic harness: virtual clock + synthetic frames with
known PTS → assert exact present/drop/repeat decisions for every policy
parameter combination (exhaustive over small grids). Property: no frame is
presented out of order; drops are monotone in lateness. Integration: A/V sync
scenario (§8) measures `|audio_pts − video_pts_presented|` at each present and
asserts it stays within a stated tolerance [PROPOSAL: ±10 ms after first
frame; tightened with evidence].

### 4.11 renderer-wgpu — GPU surface, color, scaling, compositing

(a) **Responsibility.** Turn `PresentRequest`s into pixels on a surface:
color conversion (YUV→RGB), scaling and aspect handling, background, and
subtitle compositing. Isolated: it consumes frame handles and render
parameters, nothing else (MP §3.4). On desktop it must also work behind the
Tauri webview (native surface behind a transparent webview, or frame streaming
to a canvas — the Phase 2 prototype decides, MP §7; web build uses
WebGPU/WebGL, MP §10).

(b) **Type sketch.**

```rust
pub trait RenderSurface {          // implemented by: native wgpu surface,
    fn resize(&mut self, size: Extent2D);   // canvas/WebGL bridge (web), test headless
    fn present(&mut self, frame: PresentRequest) -> Result<(), RenderError>;
}
pub enum AspectMode { Fit, Fill, Stretch, Crop16x9 /* config-driven set, MP §5 */ }
pub struct RenderParams { pub aspect: AspectMode, pub crop: Option<Rect>,
                          pub background: ColorDesc, pub subtitle_layer: Option<TexId> }
```

(c) **Threading.** One render thread owns the surface, the wgpu device and all
GPU resources. Receives `PresentRequest`s via a small mailbox (latest-wins:
a new request replaces an unsent one — rendering never back-pressures decode
[PROPOSAL: mailbox depth 1]).
(d) **Errors.** Surface loss (window closed, GPU reset) → `RenderError::
SurfaceLost`; the session degrades to audio-only with an explicit event, or
reattaches when a new surface is provided (resize is non-fatal by
construction).
(e) **Invariants.** Color conversion happens in shaders on the GPU (MP §11:
GPU color conversion); the CPU→GPU upload path is the only allowed frame copy
and it is zero-copy for compatible buffer layouts when the decoder's output
supports import [VERIFY: per-backend capability, recorded per platform].
Scaling/aspect math is pure and unit-tested against reference rectangles.
(f) **Tests.** Headless wgpu surface in testkit renders frames to an offscreen
buffer and compares against CPU reference conversion within tolerance.
Property: aspect/crop math never produces empty or inverted rects. Screenshot
tests of the compositing result (video + subtitle layer) with synthetic
textures.

**Subtitle compositing [PROPOSAL].** Subtitle events are rasterized on the CPU
(text shaping and glyph rasterization — candidate: see doc 04) into an RGBA
atlas uploaded as a texture per event; the video shader composites: video →
color convert → tone/scale → then alpha-blend the subtitle layer with the
event's position. Rationale: keeps text rendering out of the real-time path
(events change at ~Hz, not per frame), keeps the renderer generic (it
composites *a layer*, not "subtitles"), and gives one compositing code path
for all subtitle formats. Chapters/metadata never composite — they are events
only.

### 4.12 subtitles, chapters, metadata — one timed-event model

(a) **Responsibility.** Parse subtitle formats (embedded tracks from demux and
external files), expose chapters and metadata, all as timed events on the
shared model (MP §3 last paragraph). Provide delay and encoding controls.

(b) **Type sketch.**

```rust
pub struct TimeSpan { pub start: MediaTime, pub end: Option<MediaTime> }
pub struct TimedEvent<T> { pub span: TimeSpan, pub data: T }
pub enum TimedStream {
    Subtitle(Vec<TimedEvent<SubtitleEvent>>),
    Chapters(Vec<TimedEvent<Chapter>>),
    Static(Metadata),                       // tags: untimed, still the same model
}
pub struct SubtitleEvent { pub cues: Vec<Cue> } // positioned styled text runs
pub struct SubtitleTrackState { pub delay: MediaTime /* signed, applies at
    consumption time, never rewrites parsed timestamps */, pub encoding: EncodingOverride }
```

External files load through `input` (file or URL — same deadlines and
cancellation); discovery of sidecar files (`movie.srt` next to `movie.avi`) is
a playlist/layer concern that issues an engine `AddExternalSubtitle` command.
Encoding detection: BOM, UTF-8 validation, then configured fallback charset
with conversion (candidate: see doc 04); the chosen encoding is reported, and
misdecoding never corrupts timing.
(c) **Threading.** A light parser worker per external file (embedded subtitle
tracks flow through the normal packet/decode path — a subtitle "decoder" is
just another `codec-api` plugin whose output is `TimedEvent`s [PROPOSAL:
unify embedded subs as codec plugins, external files as standalone parsers —
both produce `TimedStream`]).
(d) **Errors.** Unparseable file → typed `Error(Corrupt)` for that track only;
never session-fatal. Unknown encoding → error naming the attempted encodings.
(e) **Invariants.** Delay applies at consumption (renderer/event query time),
so changing it is O(1) and never mutates parsed data; events are immutable
once parsed; event overlap is permitted (rendering decides stacking).
(f) **Tests.** Per-format conformance (SRT first, Phase 5); property tests:
delay in [-60 s, +60 s] commutes with any event query; fuzz every parser
(MP §6, §12); encoding corpus (UTF-8, UTF-16 BOM, Latin-1). Chapters: from
container metadata only — never invented (MP §8 bullet 6).

### 4.13 playlists — data model and deterministic order semantics

(a) **Responsibility.** Own the playlist data model, repeat/shuffle/order
semantics as pure data transforms, and the queue concept. It contains no
playback logic — it drives the engine exclusively through commands
(`SetSource`, on `Ended` → next item).

(b) **Type sketch.**

```rust
pub struct PlaylistItem { pub id: ItemId, pub source: SourceSpec,
    pub title: Option<String>, pub duration: Option<MediaTime>, pub meta: Metadata }
pub struct PlaylistModel { items: Vec<PlaylistItem>, /* stable ids */ }
pub enum Repeat { Off, One, All }
/// Deterministic shuffle: the play order is a pure function
/// (seed, repeat, item ids, queue) -> sequence of ItemId.
pub struct PlayOrder { pub repeat: Repeat, pub shuffle_seed: Option<u64> }
pub struct QueueOp { /* PlayNext(itemId), PlayNow(itemId), Remove(id) ... */ }
```

Shuffle is a seeded permutation computed as a data transform (Fisher–Yates
over item ids with the recorded seed) — same seed + same items ⇒ same order,
property-tested. "Play next" is an insertion into the remaining order, not a
mode. Repeat does not loop the iterator implicitly: it is expanded into the
computed sequence in bounded chunks [PROPOSAL: expand one cycle ahead, so an
infinite All-repeat never materializes an infinite list].
(c) **Threading.** None of its own; the model is owned by whoever loads it
(the app shell via command layer). The engine's only interaction is receiving
`SetSource`.
(d) **Errors.** Load errors are per-item (unreachable file stays in the list
marked unavailable — from engine open results); a broken item never blocks the
sequence.
(e) **Invariants.** Item identity is stable across reorders (ids, not
indices); the computed order is a permutation of exactly the current item set
(property); repeat+shuffle keeps the permutation property per cycle [PROPOSAL:
reshuffle per cycle with a derived seed, documented].
(f) **Tests.** Property: order is a permutation; deterministic for a seed;
repeat-expansion terminates per chunk; queue insert/remove semantics against a
model. Fuzz: playlist format parsers (M3U/PLS first, Phase 5). Integration:
gapless handoff to the engine (§4.8) at item boundaries.

## 5. Cancellation and generation semantics

MP §8 bullet 2 requires that seek, stop, source replacement and track switch
cancel old work via generation IDs so stale packets and frames cannot
reappear. This section is the exact lifecycle.

**Two-level generations [PROPOSAL].**
- **Epoch** (session generation): minted on `Open` and on `SetSource`
  (replacement). Everything owned by an epoch — input thread, demuxer, queues,
  decoders, device session — is destroyed with it.
- **Pipeline generation** (sub-generation, resets each epoch): minted on
  `Seek`, `SwitchTrack`, `Stop` (final), and discontinuity-driven re-anchor.
  Epoch = (epoch, pipeline) pair compared for equality; comparisons are
  "different epoch OR different pipeline ⇒ stale".

**Minting.** Only the session thread mints, atomically publishing the pair to
a `CancelToken` (`Arc`-shared snapshot: epoch u64, pipeline u64, plus an
interrupt flag). Mint order is program order on one thread — no races by
construction.

**Checking points (every stage, every hop).**
1. Input reads: between chunks (deadline + token).
2. Demux loop: between `read_packet` calls (D5); parked producers wake on
   token change.
3. Queue push: the demux driver stamps the *current* token generation onto
   each `Stamped` at push; `push` itself compares against the queue's
   published generation and drops mismatches (Q2) — this is the boundary that
   makes in-flight producers harmless.
4. Queue pop/flush: `flush(new)` is called by the session before any new
   producer starts; consumers then only ever observe current-generation items.
5. Decode workers: check the token before `send` and before enqueuing each
   output frame; a worker holding a parked decoder aborts via queue
   `PushAborted`.
6. Audio pump: on token change, drops pending frames, calls `flush()` on the
   decoder, drains the ring to silence, then waits for the new generation's
   first packet.
7. Video scheduler: drops every queued/late frame whose stamp differs;
   renderer mailbox is replaced wholesale.
8. Events: any event in flight when a generation is minted is delivered but
   tagged `superseded: true` [PROPOSAL] so the UI can discard rather than
   flash stale state.

**Guarantee (testable).** After the session publishes generation G and the
flush of each queue has returned, no packet or frame with stamp ≠ G can be
popped, decoded-into-output, presented, or mixed into audible output. This is
a property test in testkit (§8): a fuzz of interleaved
seek/switch/stop/replacement commands against instrumented sinks that assert
every observed stamp equals the live generation.

**Interaction with bounded queues.** Queues never grow to absorb stale
producers: the push-boundary check (3) drops stale items *counted* (metric
`stale_dropped`), and parked producers receive `PushAborted` on flush, so
memory stays within bounds during cancellation storms (e.g. rapid repeated
seeking). Teardown order for `Stop`: mint → cancel token interrupt → flush
queues (producers abort) → join threads with deadline (S2) → destroy device
session.

## 6. Seek specification

Step-by-step (numbers = ordering constraints):

1. **Accept.** `Seek(target, flags)` arrives on the session thread. If the
   state is `Idle/Opening/Probing` → `CommandRejected`. If demuxer
   `seekability == None` → `SeekFailed{NotSeekable}` (never reported as
   success, MP §8 bullet 4). Validated seek → emit `SeekAccepted` (≠
   completed) and return to the client.
2. **Bump.** Mint pipeline generation G+1; publish token.
3. **Flush pipeline.** Flush packet queues and frame queues (producers abort
   per §5), flush audio decoder and ring (to silence), flush video decoder
   (drop reorder buffers), clear renderer mailbox and subtitle active set.
   Clock is frozen at the *old* position (UI shows a seeking state, not a
   fabricated new one).
4. **Demuxer seek.** Call `Demuxer::seek(target)`.
   - With keyframe index: position directly to the keyframe at or before
     target (within format tolerance).
   - `SlowLinear`: bounded linear scan [PROPOSAL: cap at 1000 packets or 2 MiB
     of parse work; exceeding → `SeekFailed{TooSlow}` with the session still
     at its old position and playable].
   - Returns actual position A (containers that can only seek approximately
     report the truth).
5. **Re-prime.** Demux emits packets from A; decode until the first keyframe
     packet per enabled track has produced ≥ 1 output frame (video) and the
     first audio packet is queued.
6. **Clock re-anchor.** `clock.reanchor(A_adjusted)` where `A_adjusted` is the
   first *audio* frame's own timestamp if audio exists (anchor on truth, not
   request); otherwise A. Video scheduler snaps to the new anchor with no
   drop-count penalty for the first frame after a seek (documented exemption
   in the drop policy).
7. **First-frame report.** Renderer presents the first frame → `FirstFrame{ts}`
   event; clock unfreezes; audio output resumes. Emit `Seeked{requested,
   actual}` — completion truth.

**Failed seeks.** If step 4 errors: emit `SeekFailed{reason, position: <old>}`.
By invariant D4 the demuxer is at a defined, parseable position, so the
session *explicitly* chooses: (i) resume demux/decode from the old position
(state preserved — the default), or (ii) if the demuxer reports itself broken
post-seek, enter `Error` (degraded explicitly, never a false success). Steps
5–7 failing (e.g. corrupt data at the landing point) degrade identically:
`SeekFailed` or `Error`, plus `DamagedData` events.

**Re-anchoring coherence.** During steps 2–6 the state is `Buffering`
(seeking) — no `Playing`-looking output. A second `Seek` arriving mid-seek
simply re-runs from step 1 against the in-flight generation (repeated-seek
storm safety, MP §13 scenario "repeated seeks").

## 7. First vertical slice (Phase 2, MP §16)

Goal (MP §16 Phase 2): one real file from open to decode to audio and video on
screen, through the command layer, from CLI (`lumen play file`) and from the
UI. The slice exercises: input, probe, demux, packet-queue, codec-api,
playback-engine, media-clock, audio-engine, audio-output (real device),
video-engine, renderer-wgpu (native surface or canvas streaming per the
webview prototype), command layer, and a minimal event set.

**Candidate container/codec pairs evaluated [PROPOSAL]:**

1. **WAV + PCM only.** Simplest possible (RIFF parsing is trivial, PCM is
   passthrough). Rejected as *the* slice: it exercises no video path, no
   presentation scheduler, no A/V sync — Phase 2's point is the full vertical.
   Retained as the first integration milestone within the slice (audio-only
   playback lands days earlier and de-risks audio-output early).
2. **AVI + MJPEG (video) + PCM (audio).** AVI is a simple RIFF container with
   one 'movi' chunk list; per-frame keyframe semantics (every MJPEG frame is a
   keyframe) make seek trivially correct; MJPEG decoding is baseline JPEG
   decoding, for which well-maintained pure-Rust candidates exist (candidate:
   see doc 04; audit gate applies); PCM passthrough requires no real decoder
   state machine. Costs: JPEG decode is real work (DCT, huffman) and AVI's
   index ('idx1') handling has quirks.
3. **Ogg + Theora + Vorbis.** Clean formats, but two inter-frame-adjacent
   codecs with real entropy coding and granulepos timestamp semantics — the
   codec cost dwarfs the container cost and delays the vertical.

**Recommendation [PROPOSAL]: candidate 2, AVI + MJPEG + PCM**, with candidate
1 as the in-slice audio milestone. Rationale: the container is simple enough
to finish, the video path is genuinely exercised end-to-end (real decoder
plugin, real frame timing, real GPU conversion), every frame being a keyframe
makes the seek contract honest and testable, and no patent-encumbered video
bitstream is required (JPEG baseline patent situation is clear for decode;
recording in doc 04 and the risk register, MP §14).

**Real vs stubbed in the slice:**
- Real: input (file), probe (WAV + AVI scorers), demux (WAV + AVI), queues,
  codec-api (PCM "decoder" + MJPEG decoder), session state machine (full),
  clock (audio-master + system fallback), audio-engine (conversion only, one
  track, no user filters), audio-output (one platform first, then the other
  two), video scheduler, renderer (YUV→RGB or RGB path, fit aspect),
  command layer, events.
- Stubbed/differed: playlists (single file), subtitles (none), filters (empty
  chain), network input, track switching (single audio + single video track),
  device-loss policy (event only), hardware decode (Phase 0 ADR).

**Acceptance criteria (must be objectively checkable):**
1. `lumen play fixture.avi` opens the file, shows video on screen, plays
   audio, and both stop at `Ended`; exit code 0; a second run is identical.
2. The same through the UI via the command layer: Play/Pause/Stop reflect
   engine state truthfully (accepted vs completed distinguished in the UI).
3. Seek (to 3 positions incl. 0 and near-end) lands within ±1 frame of a
   keyframe and audio resumes within 300 ms [PROPOSAL: number revised with
   Phase 3 measurements].
4. Ctrl-C / window close during playback exits within the teardown deadline
   with no audio device left open.
5. `cargo test` suite for the slice passes: unit + property + the integration
   scenarios of §8 applicable to the slice (open/play/pause/seek/repeated
   seeks/short media/reopen/exit-during-playback).
6. No `unsafe`, no C toolchain in the build (local gate, MP §13).

**Fixtures.** Generated, legally clean (MP §13 corpus rules): a `testkit`
fixture generator produces (a) WAV files (sine/noise, rates, bit depths,
truncated variants) and (b) AVI+MJPEG+PCM files by encoding synthetic frames
with a tiny in-tree baseline JPEG *encoder* written for testkit only
[PROPOSAL: a minimal deterministic baseline-JPEG writer, ~hundreds of lines,
keeps fixtures license-clean and reproducible; alternatively a pure-Rust JPEG
encoder candidate — see doc 04]. Recorded expectations: frame count, frame
hashes, duration, track list. Malformed variants: truncation at every 256-byte
boundary, bit flips, missing 'idx1'.

## 8. Engine test strategy

**testkit crate** (MP §4; depends on production crates, never the reverse,
MP §3.7):
- `VirtualClock`: implements `PlaybackClock`; manual time advance; scripted
  drift; used by every deterministic test.
- `ScriptSource`: `ByteStream` from a byte buffer plus a fault script
  (delay-before-read, hang-forever, truncate, reset, corrupt, shrink). This is
  the "interrupted network" scenario driver.
- Sinks: `NullAudioSink`, `CapturingAudioSink` (records sample hashes),
  `VanishingSink` (device loss), `StallingSink`; `FrameRecorder` (video
  presents with stamps, hashes, drop counts).
- `SessionHarness`: builds the whole pipeline on virtual devices, steps it
  deterministically (single-threaded mode where workers run inline — the same
  code paths, scheduling only differs), and asserts on the event log.
- Instruments: resident-bytes counters for queues, thread/fd counters for
  leak tests, allocation/lock fault-injection for the audio callback (§4.9).

**Property tests** (candidate framework: see doc 04):
- Time-base conversions (MP §8 bullet 1): overflow, discontinuity, missing
  timestamps (§3 f).
- Queue bounds under adversarial interleavings (§4.4 f).
- Generation races: the §5 guarantee as a machine-checked property over
  random command storms.
- State-machine model checking against the §4.6 table.
- Scheduler decisions over parameter grids (§4.10 f); shuffle permutation
  properties (§4.13 f).

**Integration scenarios** mapped 1:1 to MP §13's list:

| MP §13 scenario | Driver in testkit | Pass condition |
|---|---|---|
| open | `ScriptSource` valid + variants | correct tracks/duration or typed error |
| play | `SessionHarness` + sinks | `FirstFrame` + audio samples flow; A/V stamps within tolerance |
| pause / resume | virtual clock freeze | no samples while paused; resumes at same media time |
| seek | §6 flow on 3 targets | `Seeked{actual}` within tolerance; no stale frames |
| repeated seeks | 100 random targets, no waits between | no stale output ever; bounded memory; all complete or typed-fail |
| A/V sync | instrumented sinks over full playback | drift within §4.10 tolerance |
| track switching | multi-track synthetic container | new track's first frame follows switch generation |
| corrupt / missing files | fault scripts + malformed corpus | typed `Corrupt`/`Inaccessible`, bounded recovery counts |
| interrupted network | `ScriptSource` hang/reset scripts | `Buffering` then resume or typed error; no hang |
| device loss | `VanishingSink` | `DeviceLost` event, position preserved or explicit suspension |
| long / short media | synthetic 4 h stream; 0.1 s clip | Ended reached; short-media edge cases per §4.6 |
| reopen cycles | 1000× open/play/stop | no leaks (threads/fds/bytes) |
| exit during playback | drop session mid-decode | teardown within deadline, device closed |

Differential tests (MP §13): slice fixtures decoded and compared against VLC
3.0.23 run *externally* as an oracle (frame hashes, durations) — never
vendored (MP §2). Fuzz targets registered per parser: AVI, WAV, MJPEG/JPEG,
SRT, M3U, URI parsing (MP §6, §12). Mock evidence (virtual devices) is labeled
separately from real-hardware evidence (MP §13).

**WASM variant note [PROPOSAL].** The same engine compiles to wasm32 (MP §2);
threads there need COOP/COEP (MP §10), so the harness runs the single-threaded
cooperative mode; multi-threaded WASM is a doc 09 concern.

## 9. Threading summary

| Thread | Owns | Communicates via | May block? | Real-time? |
|---|---|---|---|---|
| Host/UI thread (app, not engine) | UI/webview, command client | command layer (doc 05): mpsc in, event bus out | yes | no |
| Session thread (1 per playback) | state machine, generations, clock anchor decisions, watermark policy | command inbox, event bus, control atomics | bounded waits only (S3) | no |
| Input reader (1 per source) | `ByteStream` instance, open deadline | byte buffer + condvar to demux | yes — I/O, deadline-bounded | no |
| Demux thread (1 per source) | `Demuxer` instance | bounded packet queues (§4.4) | parks on full queue; wakes on generation | no |
| Video decode workers (pool, N = cores-capped) | `VideoDecoder` instances | packet queue in, frame queue out | CPU-bound; no I/O, no locks held | no |
| Audio pump (1) | audio decoder(s), filter chain, mixer | frame queue in, SPSC ring out | bounded by ring safety level | near-RT |
| Audio device callback (OS thread) | nothing — stateless reader of the SPSC ring | SPSC ring indices | **never** | **yes** |
| Video scheduler (1) | reorder window, drop policy state | frame queue in, present mailbox out, clock reads | no (vsync-paced waits) | no |
| Render thread (1) | wgpu device, surface, pipelines | present mailbox (latest-wins), resize events | GPU present may block briefly | no |
| Subtitle/parse worker (1, light) | external-file parsers | timed-event queue to engines/events | file I/O bounded by input deadlines | no |
| Playlist prefetch worker (app-side) | next-item open | issues `SetSource` via command layer | yes | no |

Rules recap (MP §8 bullet 3): no async runtime in decode paths; no lock is
held across blocking work (the only locks are short queue/ring critical
sections); the audio callback obeys §4.9's forbidden-operations list.

## 10. Open questions and decisions requested from the Lead

1. **State machine**: is modeling seek/track-switch as re-entry into
   `Buffering` (rather than dedicated states) acceptable? (§4.6) — affects the
   doc 05 event contract.
2. **First-slice format**: confirm AVI+MJPEG+PCM (§7) so doc 04 can pin the
   JPEG-decoder candidate audit as Phase-0-critical.
3. **`Ts` repair policy**: demux-stage timestamp repair with counters (D1) vs
   strictly-pass-through with engine-side repair — interface impact on the
   `Demuxer` trait.
4. **Queue internals**: simple mutex+deque now, lock-free later behind the
   same API (§4.4) — needs Lead sign-off because it touches a shared
   interface's implementation contract.
5. **Ended-state seek-back** and **suspend-on-device-loss** defaults (§4.6,
   §4.9) are [PROPOSAL]s awaiting acceptance.
6. doc 01 (architecture overview) did not exist when this was written; if its
   crate-boundary decisions differ, this document's §2.3 defers to it and
   needs a reconciliation pass by the Lead.

— End of 03-engine-plan.md —
