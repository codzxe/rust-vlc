# AUDIT: crates/audio-output/src/pump.rs
| Field | Value |
|---|---|
| Auditor | W08-A02 |
| Date | 2026-11-21 |
| Lines / functions | 338 / 16 (+4 unit tests, 2 test helpers) |
| File verdict | UNREACHABLE (real, tested logic; no shipped entry point constructs `PcmPump`) |
| Reachable from shipped entry point? | NO — `rg -n "PcmPump"` over the whole repo: only `audio-output/src/lib.rs:140` (re-export), `audio-output/src/error.rs:55` (doc comment), this file, and `audio-output/tests/pcm_pipeline.rs`. The shipped `lumen` audio path is `apps/cli/src/pipeline_host.rs` `open_audio_session` → `NullBackend::open_stream` → `AudioLimb::stage` → `NullSink::write_bytes` — the pump is absent from the chain. |
| Findings | S0: 0 S1: 0 S2: 3 S3: 2 S4: 2 |
| Commands run | `rg -n "PcmPump"` (callers: none in production); `rg -l "audio_output|audio-output"` in `**/Cargo.toml` (dependents: cli, web, platform-{linux,macos,windows}); `rg -n "NullBackend|builtin_backends"` (production open sites: `apps/cli/src/pipeline_host.rs:1599`, `apps/web/src/host.rs:1732`); `sed -n` on pump.rs, null.rs, counter.rs, cpal.rs, pipeline_host.rs (quote verification); `rg -n "pump|pcm_pipeline"` in `audit/runs/baseline/test-workspace.log.txt` (unit tests lines 176–179 and integration tests lines 211–217: executed, passed) |

## 1. One-paragraph verdict

The pump's arithmetic is real and its tests execute and pass in the
baseline run — but the component is **dead code in every shipped
build**: nothing outside its own crate ever constructs a `PcmPump`, and
the shipped CLI pushes decoded PCM straight into the null sink's ring
(`AudioLimb::stage`, pipeline_host.rs:663), reimplementing pacing with
its own quantum/park-fill gates. The module doc's framing ("The PCM
pump: decoded frames → the bounded device queue → the sink"; "the pump
is the producer half of the SPSC discipline") describes the pipeline
the repo *wants*, not the pipeline that ships. Beyond unreachability,
the pacing design has two genuine holes for its intended future role: a
buffer larger than `high_water` can never be accepted (permanent
Backpressure livelock / 100% Drop loss), and the documented
device-switch offset-carry (`SampleCounter::with_base`) makes every
push of a fresh pump fail `audio.pump_clock_mismatch` forever. For the
wave question "how does a user hear sound": **they cannot** — the only
shipped sink discards every sample (`NullConfig { capture: false }`,
pipeline_host.rs:1599) and the only real-device constructor is a typed
refusal (`cpal_backend()` → `BackendNotAdmitted::CPAL`,
cpal.rs:98-107, gate `AUDIO_ADR005_ADMITTED = false`, cpal.rs:43).
`lumen engine-info` advertises exactly that: `audio.null-device`
(baseline `lumen-engine-info.txt:4`).

## 2. What actually works (verified)

- The pump's frame accounting, high-water gating, drop ledger, and typed
  errors — verified by baseline execution:
  `audit/runs/baseline/test-workspace.log.txt:176-179`
  (`pump::tests::{zero_high_water_is_refused_at_construction,
  partial_frames_are_refused_before_the_sink_is_touched,
  a_foreign_clock_is_a_typed_mismatch,
  drop_policy_counts_and_backpressure_is_typed} ... ok`) and
  `:211-217` (`tests/pcm_pipeline.rs` — 4 passed, including the
  bit-exact sine oracle `clock_synced_pump_delivers_the_exact_sine_stream`).
- `fits()` (pump.rs:152-154) is exactly the negation of `push`'s
  over-water test (pump.rs:203): drive-loop ready test and push accept
  condition cannot disagree — verified by reading both sites.
- `push` updates `written`/`dropped` only after the sink accepted
  (pump.rs:218-219): a sink refusal never corrupts the ledger —
  verified by reading; exercised by
  `drop_policy_counts_and_backpressure_is_typed` (`assert_eq!(pump.written_frames(), 4, "the refused buffer never counted")`, pump.rs:336).

## 3. Findings

### F-001 [S2]
Where: crates/audio-output/src/pump.rs:1-21 (module docs), with the contradicting production path at apps/cli/src/pipeline_host.rs:657-664
Code:
```
//! The pump is the producer half of the SPSC discipline: exactly one
//! producer (this), exactly one consumer (the device callback).
```
and the actual shipped producer (pipeline_host.rs:657-664):
```
    pub fn stage(&mut self, frame: &AudioFrame) -> Result<(), EngineError> {
        self.pipeline.process(0, frame)?;
        self.pipeline.mix()?;
        if let Some(buffer) = self.pipeline.take_output() {
            self.sink.write_bytes(buffer.data.as_slice())?;
        }
```
What is wrong: `PcmPump` has no production caller anywhere in the
workspace (`rg -n "PcmPump"`: lib.rs re-export, error.rs doc-comment,
self, tests only). The shipped `lumen` binary writes decoded PCM to the
null sink via `NullSink::write_bytes` and paces with its own
AUDIO_PARK_FILL / quantum gates; every bounded-queue, drop-ledger and
clock-mismatch mechanism in this file is bypassed. `lib.rs:76-80`
repeats the false-in-production claim ("exactly one producer (the
pump, calling [`AudioSink::write`])").
Why it matters: the one component that enforces MP §8 bounded-queue
discipline is not in the shipped path; the shipped path's pacing is a
second, parallel implementation. Docs (module header, lib.rs) describe
the pipeline as if this type were load-bearing. A reader auditing
"where is audio paced" is pointed at dead code.
What correct looks like: either `AudioLimb::stage` routes through
`PcmPump` (the module's stated purpose), or the module docs state
plainly "not wired; the shipped producer is `AudioLimb`" — today the
docs claim integration that does not exist outside tests.
Confidence: HIGH — Verification: read + `rg` call-chain (no run needed;
absence of a call site is static).

### F-002 [S2]
Where: crates/audio-output/src/pump.rs:196-217 (with doc advice at 180-182)
Code:
```
        let before = self.written - consumed;
        if before + frames > self.high_water {
            return match self.policy {
                DropPolicy::Backpressure => {
                    Err(error::pump_backpressure(self.high_water, before + frames))
                }
```
What is wrong: a buffer whose `frames > high_water` can NEVER be
accepted — `before ≥ 0`, so `before + frames > high_water` holds even
on an empty queue. Under `Backpressure` this is a permanent livelock:
the attached clock only advances when the device renders, the device
only renders what the sink holds, and the sink only receives data via
an accepted push — which is always refused. The doc's remediation
(pump.rs:180-182: "retry after the clock advances") is impossible by
construction. Under `Drop` the same wiring silently discards 100% of
the stream (only the `dropped` ledger says so). Neither `new` (which
checks only `high_water == 0`) nor `push` distinguishes "permanently
unsatisfiable buffer" from "temporarily over budget" — same error code,
opposite prognosis.
Why it matters: decoded PCM chunks are codec-fixed (AAC ≈ 1024 frames,
MP3 1152, the repo's own quantum 1470); any `high_water` configured
below one chunk size bricks the session — stall under Backpressure,
total silence under Drop. ADR-005:189-193 plans to rely on this proof
"on-device".
What correct looks like: a typed distinct error (e.g.
`audio.pump_buffer_exceeds_budget`) when `frames > high_water` outright,
or constructor-time validation against the negotiated chunk size, or
internal buffer splitting so an in-budget prefix is always deliverable.
Confidence: HIGH — Verification: read (arithmetic traced; no run —
baseline has no test exercising `frames > high_water` with a clock
attached).

### F-003 [S2]
Where: crates/audio-output/src/pump.rs:196-201, against crates/audio-output/src/counter.rs:52-63
Code:
```
        if consumed > self.written {
            // The clock advanced past this pump's writes: it is not
            // this session's truth source.
            return Err(error::unsupported("audio.pump_clock_mismatch"));
        }
```
and the documented device-switch discipline (counter.rs:59-63):
```
    pub fn with_base(base: u64, rate_hz: u32) -> Result<SampleCounter, EngineError> {
        let counter = SampleCounter::new(rate_hz)?;
        counter.count.store(base, Ordering::Relaxed);
        Ok(counter)
    }
```
What is wrong: `SampleCounter::with_base` exists precisely to carry the
count across device switches ("the old device's final count is the new
device's origin", counter.rs:53-55, doc 03 §4.9 e). But a device switch
also means a new sink, hence a new `PcmPump` with `written == 0`.
Attaching the offset-carried clock (base = old device's count > 0)
makes `consumed > self.written` true on the very first push — and on
every push forever. There is no re-basing API (`written` is private; no
constructor takes an origin). The guard cannot distinguish "clock from
another session" (wiring bug — correct to refuse) from "own session's
clock after the documented device-loss fallback" (doc 03 §4.9 d/e —
correct operation). After a device switch the pump has NO working
configuration: carried clock → permanent `pump_clock_mismatch`; no
clock → `consumed() == 0` forever, so `in_flight` only grows and every
push past `high_water` hits the policy (stall or drop everything).
Why it matters: device loss + re-open is the exact scenario the crate
is designed around (null.rs doc "the 3.D2 device-loss simulator";
`with_base` tested as "the offset-carry invariant, structural and
tested", counter.rs:18-19). The pacing proof breaks in that scenario by
construction.
What correct looks like: `PcmPump::new` (or `attach_clock`) accepting
an initial `written` origin, or the mismatch check tolerating a
documented base offset re-anchored once at attach time.
Confidence: HIGH — Verification: read (both files traced; unit test
`a_foreign_clock_is_a_typed_mismatch` uses exactly the carried-base
shape, base=100, and enshrines the refusal).

### F-004 [S3]
Where: crates/audio-output/src/pump.rs:203-216 (doc claim at 27-31)
Code:
```
                DropPolicy::Drop => {
                    self.dropped += frames;
                    Ok(PushOutcome {
                        accepted: 0,
                        dropped: frames,
                        in_flight: before,
                    })
```
What is wrong: the whole buffer is dropped even when a prefix would
fit: with `in_flight = 6`, `high_water = 8`, a 4-frame buffer drops all
4 frames though 2 were within budget. Under `Drop` this is avoidable
loss up to `frames − 1` per event; under `Backpressure` the same
coarseness stalls delivery of the in-budget prefix.
Why it matters: on a live path this is audible glitching beyond what
the budget forces; the doc oversells it ("bounded memory, honest loss
ledger; the sink still receives an exact in-order prefix" — true, but
the prefix is needlessly shorter than the budget allows).
What correct looks like: accept the fitting prefix (split the buffer or
push a sub-slice), drop/refuse only the remainder — or document the
whole-buffer granularity as deliberate with its worst-case loss.
Confidence: HIGH — Verification: read (branch traced;
`drop_policy_bounds_memory_and_keeps_the_prefix` confirms whole-buffer
drops at the boundary but never exercises a partial fit).
Note: today's blast radius is zero because nothing ships this code
(F-001); rated S3, not S2, on that basis.

### F-005 [S3]
Where: crates/audio-output/src/pump.rs:1 (name), vs apps/cli/src/pipeline_host.rs:35-37 and 107
Code:
```
//! The PCM pump: decoded frames → the bounded device queue → the
//! sink, paced by the playback clock
```
vs the CLI's unrelated drive loop (pipeline_host.rs:35-37):
```
//! device half of the null audio session and the scheduler's
//! tick/present driving live in [`crate::host`]'s pump — the consumer
//! only takes delivery; the pump advances time (audio renders move
```
What is wrong: two different mechanisms are called "the pump": this
crate's `PcmPump` (producer-side pacing) and the CLI host's drive loop
(consumer-side render/schedule driver, pipeline_host.rs:35-44, 107-112,
"the pump's render passes"). Neither uses the other. Naming that lies
about architecture.
Why it matters: cross-referencing "the pump" in logs, docs, or a
profiling session is ambiguous; a maintainer wiring ADR-005's "pump
must hold on-device" against the CLI "pump" audits the wrong component.
What correct looks like: rename one (e.g. the CLI loop is a "drive
loop"/"render driver"; or this type is a `PcmFeeder`), or make the
module docs name and distinguish the other.
Confidence: HIGH — Verification: read both sites.

### F-006 [S4]
Where: crates/audio-output/src/pump.rs:187
Code:
```
        let frame_bytes = buf.format.frame_bytes().max(1) as usize;
```
What is wrong: a degenerate format (`channels == 0` →
`frame_bytes() == 0`, media-types/src/frame.rs:70-72) is silently
clamped to 1 byte/frame, so any byte length counts as "whole frames"
and the pump's frame arithmetic runs on garbage instead of refusing.
`AudioFormat` has no constructor validation (plain struct), so the
invalid value is constructible.
Why it matters: swallowed invalid input; the honest behavior elsewhere
in this crate is a typed refusal (`audio.partial_frame`,
`audio.render_buffer_unaligned`).
What correct looks like: refuse `frame_bytes() == 0` with a typed
`Unsupported` error (the crate's own `error::unsupported` pattern).
Confidence: HIGH — Verification: read (no execution needed; path is
static).
Blast radius: effectively nil today (NullBackend refuses to open
0-channel streams before a pump would exist) — hence S4.

### F-007 [S4]
Where: crates/audio-output/src/pump.rs:313-314
Code:
```
        assert!(pump.push(&frames_buf(format, 2)).must().accepted == 2);
        assert!(pump.push(&frames_buf(format, 2)).must().accepted == 2);
```
What is wrong: `assert!(x == y)` instead of `assert_eq!` — on failure
it prints only "assertion failed" without the operands, hiding which
push and what value broke.
Why it matters: failure diagnostics in the one test that proves the
Drop-policy accept path.
What correct looks like: `let o = pump.push(...).must(); assert_eq!(o.accepted, 2);`
Confidence: HIGH — Verification: read.
(Curiously, the same file uses proper `assert_eq!` everywhere else —
this is the only `assert!(… == …)` pair.)

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `enum DropPolicy` (Backpressure/Drop) | 57-69 | REAL | — | plain data, no logic |
| `struct PushOutcome` (+`Default` derive) | 73-82 | REAL | — | plain data; doc "in_flight always ≤ high_water" holds on both arms (verified both return paths) |
| `struct PcmPump` | 85-92 | REAL | — | private fields; owns sink by value (forces new pump per device switch → F-003) |
| `impl Debug for PcmPump::fmt` | 94-106 | REAL | — | records `clock_attached` presence, not identity — honest |
| `PcmPump::new` | 116-128 | REAL | — | only `high_water == 0` refused; no buffer-size/chunk validation (F-002), no origin (F-003) |
| `PcmPump::attach_clock` | 135-137 | REAL | — | no re-base; first push after attach is the only mismatch detection point |
| `PcmPump::consumed` (private) | 139-141 | REAL | — | no clock → 0 (documented); single word read, non-blocking per source.rs:12-17 contract |
| `PcmPump::in_flight` | 146-148 | REAL | — | `saturating_sub`; saturates only when the mismatch guard would already have fired in `push` — consistent |
| `PcmPump::fits` | 152-154 | REAL | — | exact complement of push's accept test; advisory only (state can change between fits and push — fine for its stated drive-loop use) |
| `PcmPump::written_frames` | 157-159 | REAL | — | accessor |
| `PcmPump::dropped_frames` | 162-165 | REAL | — | accessor; monotonic |
| `PcmPump::high_water` | 168-170 | REAL | — | accessor |
| `PcmPump::push` | 186-225 | REAL (executed in baseline) | S2×2, S3 | core: F-002 (oversized livelock), F-003 (base-carry mismatch), F-004 (whole-buffer drop); ledger updated only post-accept (correct); no panics, no unwraps, no overflow risk (u64 frames bounded by buffer bytes) |
| `PcmPump::sink` | 228-230 | REAL | — | borrow accessor |
| `PcmPump::sink_mut` | 233-235 | REAL | — | mutable borrow — lets a caller write behind the pump's back, breaking the `written` ledger; acceptable only because the type is crate-private discipline — worth a doc warning (not counted separately) |
| `PcmPump::into_sink` | 238-240 | REAL | — | teardown accessor |

## 5. Test honesty (if tests exist in/for this file)

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `zero_high_water_is_refused_at_construction` (273-277) | YES | removing the guard makes `new` return `Ok` → `.err().must()` panics (testkit Must asserts Some). Executed: baseline test-workspace.log.txt:179. |
| `partial_frames_are_refused_before_the_sink_is_touched` (280-292) | PARTIAL — NO under the natural mutation | deleting the pump's own partial-frame check (187-194) still passes: `NullSink::write` refuses the identical shape with the identical code `audio.partial_frame` (null.rs:251-256). The test proves the outcome, not the layer; the "before the sink is touched" claim is untested (no spy distinguishes which layer refused). |
| `a_foreign_clock_is_a_typed_mismatch` (295-305) | YES | removing the guard makes push succeed → `.err().must()` panics. Note: the fixture (base=100) is byte-identical in shape to a legitimate `with_base` device-switch carry (F-003) — the test enshrines the flaw as a feature. |
| `drop_policy_counts_and_backpressure_is_typed` (308-337) | YES | asserts the full outcome tuple (0,2,4), the `dropped_frames` ledger, `written_frames` under refusal, and `fits`; mutating any ledger/gate breaks an assert. Style defect F-007 at 313-314. |
| Integration `clock_synced_pump_delivers_the_exact_sine_stream` (pcm_pipeline.rs:75-140) | YES | bit-exact per-sample oracle against `SinePattern::sample_at`, media clock must land on exactly 1.000 s, underrun/silence-free asserts; any pacing or ledger mutation breaks it. Executed: baseline test-workspace.log.txt:217. Null-backend-only (no real device exists in the repo — that is the domain verdict, not a test defect). |
| Integration `drop_policy_bounds_memory_and_keeps_the_prefix` (pcm_pipeline.rs:143-191) | YES | 6-tick starved scenario asserts accept/drop boundaries, both invariants (`in_flight`, `buffered_frames`) per tick, and exact-prefix capture. |
| Integration `backpressure_is_typed_and_clears_as_the_clock_advances` (pcm_pipeline.rs:194-234) | YES | same buffer refused then accepted after exactly one render — proves the clock-clears semantics. |

No `#[ignore]`, no cfg-gated tests, no mocks of the subject (the null
sink is a real sink implementation, not a mock of `PcmPump`).

## 6. Doc claims touching this file

| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The pump is the producer half of the SPSC discipline: exactly one producer (this)" | pump.rs:14-16; repeated lib.rs:76-80 | FALSE (in shipped builds) | shipped producer is `AudioLimb::stage` → `NullSink::write_bytes` (pipeline_host.rs:663); `rg PcmPump` = zero production callers |
| "Nothing is ever silently discarded or repeated" | pump.rs:31 | SHALLOW | true within the type (drops counted), but F-002: a `frames > high_water` stream under `Drop` is 100% discarded with only a counter, and under `Backpressure` the doc's own retry advice is unsatisfiable |
| "bounded memory, honest loss ledger; the sink still receives an exact in-order prefix" | pump.rs:28-30 | REAL | verified by `drop_policy_bounds_memory_and_keeps_the_prefix` (baseline log:212-215 pass) — with the F-004 caveat that the prefix is coarser than the budget allows |
| "the `PcmPump` pacing proof … must hold on-device" | docs/adr/ADR-005-audio-output.md:189-193 | PAPER-ONLY (as on-device evidence) | all executed proof is null-backend (baseline log:211-217); no on-device run exists or can exist (cpal gate closed, cpal.rs:43) — the ADR words it as a future obligation, honestly |
| row `platform-aout-cpal` evidence cites `crates/audio-output/src/{pump,pattern}.rs … (wave-14 PCM pipeline proof: clock-synced bounded-queue pump …)`, status partial, notes "no real-device evidence exists" | docs/feature-matrix.toml:2351-2362 | REAL (as "implemented and tested") / honest about partial | implemented: this file; tested: baseline log:176-179, 211-217; not wired into any shipped path (F-001) — the row does not claim wiring, so no FALSE |
| `features: … audio.null-device …` | `lumen engine-info` (baseline lumen-engine-info.txt:4) | REAL (the label is accurate) | the shipped output device IS the null device (pipeline_host.rs:1598-1599 `NullConfig { capture: false }`) — the feature string advertises the discard sink; a user hears nothing |

## 7. Dead / duplicated / unreachable code

- The entire module is unreachable from shipped entry points (`lumen`
  CLI, `lumen-tauri` — the tauri shell's audio path is the same
  null-device seam per ADR-004; desktop build itself UNVERIFIED-BUILD
  in baseline INDEX.md #2). Call-chain proof: `rg -n "PcmPump"` →
  lib.rs re-export only; `rg "audio_output::pump"` → pcm_pipeline.rs
  (test) only.
- Duplicated discipline: bounded-queue pacing exists twice — this type
  (producer-side high-water) and the CLI's own gates
  (`AUDIO_PARK_FILL`, `buffered_frames()` checks,
  pipeline_host.rs:571-580) — independent implementations of the same
  MP §8 rule, only one of which ships.

## 8. Dependencies & flags

- No feature flags gate this module: `pub mod pump;` is unconditional
  (lib.rs:130) and `audio-output` has zero optional dependencies
  (Cargo.toml:32-35: audio-engine, media-clock, media-types — all
  path deps). It compiles into the shipped binary as dead weight
  (small: monomorphized only where used — nowhere in production, so
  likely fully DCE'd by the linker; UNVERIFIED, no build runs allowed).
- `testkit` is dev-only (Cargo.toml:37-41) — correct.
- The cpal seam this module's docs lean on ("the ADR-005 seam") is a
  closed compile-time gate: `AUDIO_ADR005_ADMITTED = false` with
  `const _: () = assert!(!…)` (cpal.rs:43-52) — flipping it without the
  adapter breaks the build by design.

## 9. Open questions for the next agent

- W08 agents auditing `null.rs`/`render.rs`/`ring.rs`: does
  `AudioLimb`'s direct `write_bytes` path uphold the same bounded
  guarantees the pump would (ring capacity 131 072 vs decoded-burst
  23 520 — pipeline_host.rs:105-112 claims a margin analysis; worth an
  independent check, it is the pacing that actually ships)?
- Whoever owns ADR-005 execution: F-002/F-003 must be resolved before
  the "must hold on-device" clause can be tested; a device-loss
  (doc 03 §4.9 d) integration test with a pump in the loop does not
  exist and would fail today.
- `sink_mut()` (pump.rs:233-235) permits bypassing the ledger — is any
  intended caller (pause-side integration per its doc) actually
  specified anywhere? None found.
