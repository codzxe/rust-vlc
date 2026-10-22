# Engineering Directives — Universal Low-Memory, High-Performance Standards

Status: **BINDING** (user directive, 2026-10-22). This file binds every agent
(human or AI) writing or reviewing code in this repository, alongside
AGENTS.md §3 hard rules. Where a directive conflicts with an older recorded
decision, the directive wins unless the Lead records an explicit, justified
exception in the decision register. Audit compliance is mandatory before any
card can claim COMPLETE.

Origin: user-issued engineering standard ("Autonomous Systems Engineering:
Universal Low-Memory, High-Performance Directives"). Text adapted verbatim in
substance; section numbers D-1..D-5 are citation handles.

---

## D-1. Memory & Allocation Discipline (Zero-Allocation Hot Paths)

- **Eliminate heap churn.** NEVER allocate memory (`malloc`, `new`, `vec![]`,
  `Box`, cloning, string formatting, dynamic resizing) inside hot paths, inner
  loops, per-frame renders, or real-time callbacks. All buffers, scratch spaces,
  and working memory MUST be pre-allocated, reused, and managed via object
  pools, ring buffers, or fixed-capacity arenas.
- **Zero-copy architecture.** Pass data by borrowing, slices, or shared
  immutable reference counters. Never copy or re-allocate buffers across layer
  boundaries unless ownership transfer is strictly required.
- **Strict memory boundedness.** Every queue, buffer, cache, and collection
  MUST have a hard, unbreachable maximum capacity. Unbounded collections are
  strictly prohibited. If a memory cap is reached, handle it via backpressure,
  eviction, or explicit rejection — never by unbounded growth or silent memory
  exhaustion.

Project anchors: doc 03 §8 (queue invariants, bounded memory under burst);
packet-queue `BoundedQueue`; audio-output SPSC ring. These are the floor, not
the ceiling.

## D-2. Algorithmic Rigor & Hardware Sympathy

- **No textbook-naive math or brute force.** NEVER implement naive O(N²) or
  O(N³) textbook definitions, nested linear scans, or bit-by-bit scalar loops
  where optimized, logarithmic, or O(1) alternatives exist. Where mathematical
  transforms, parsing, or searches occur, ALWAYS use mathematically factored
  fast algorithms, lookup tables (LUTs), or hierarchical indexing.
- **Mechanical sympathy.** Design data layouts for cache locality: prefer flat,
  contiguous arrays and Struct-of-Arrays (SoA) over scattered pointers and
  deeply nested references. Avoid expensive CPU operations in tight loops:
  avoid scalar hardware integer divisions (use bit-shifts/masks for power-of-two
  arithmetic), avoid branch-heavy loops, and avoid mutex acquisition inside
  time-critical sections. Structure algorithms so the compiler can
  auto-vectorize inner loops with SIMD instructions.

## D-3. Defensive Ingestion & The "Dirty World" Principle

- **The spec is an ideal; real inputs are corrupted.** NEVER assume real-world
  inputs (files, network packets, user payloads, hardware streams) strictly
  obey formal specifications. Real-world data is created by buggy third-party
  encoders, legacy software, and interrupted transmissions. Parsers must be
  robust, forgiving, and battle-hardened.
- **Anticipate all edge-case permutations.**
  - *Type mismatches & signedness:* anticipate signed values packed into
    unsigned containers, negative dimensions, and overflow values.
  - *Zero & missing fields:* never crash when optional, advisory, or legacy
    fields are zero, empty, or absent. Implement intelligent fallback heuristics
    using secondary headers.
  - *Offsets & locality:* never hardcode single-point assumptions about
    offsets, pointer anchors, or alignments. Probe boundaries dynamically to
    support relative, absolute, and legacy variations.
  - *Casing & variants:* handle identifiers, tags, and magic bytes
    case-insensitively and support standard extensible wrapper variants.
- **Graceful recovery over rigid rejection.** Do not abort an entire pipeline
  because of localized corruption. Contain and isolate corrupt units, conceal
  damage gracefully, and continue processing valid subsequent data.

Project anchors: codec-api damage model (ConcealedFrame/CorruptInput, "never
fabricated silence"); D4 no-fake-success; 3.C2 malformed-corpus card.

## D-4. Concurrency, Pipeline & Lifecycle Integrity

- **No destructive architectural shortcuts.** NEVER simulate complex state
  transitions (such as seeking, reconnecting, or re-synchronizing) by
  destroying subsystems, dropping hardware handles, or re-initializing sessions
  from scratch. All operational flows MUST be handled in-pipeline: flush
  queues, reset generation counters, re-anchor synchronization clocks, and
  seek data streams natively.
- **Strict state isolation & leak prevention.** NEVER leak data, sound, or
  frames across state boundaries. If an operation starts in a paused,
  buffering, or idle state, intermediate frames must be held without
  presentation or playback leakage.
- **Decoupled asynchronous processing.** NEVER couple I/O, demuxing, decoding,
  DSP, and presentation onto a single synchronous blocking thread. Separate
  pipeline stages onto independent worker loops communicating through bounded,
  non-blocking, lock-free ring buffers (SPSC/MPMC).

Project note: the Phase-2 engine is a deliberately synchronous virtual-time
pump for determinism (doc 03); the audio render callback and the desktop
mailbox transport are the real-time surfaces today. The decoupled-worker
requirement applies to real-time surfaces immediately and to the engine at the
phase where wall-clock pacing lands (3.B3 queue backpressure + later). Any
known deviation must be recorded, not silent.

## D-5. Escaping the "Synthetic Echo Chamber" in Testing

- **Do not test only what you generated.** NEVER evaluate code only against
  self-authored, perfectly formatted test fixtures. If the same entity authors
  both the generator and the parser, they share the exact same blind spots.
- **Adversarial & chaos testing.** Systematically test against malicious,
  malformed, and truncated inputs: zero-length buffers, off-by-one boundaries,
  corrupt headers, out-of-order packets, extreme values, and simulated I/O
  starvation. Cross-verify outcomes against established external industry
  oracles (reference tools and production engines), never relying solely on
  internal assertions.
- **Never weaken an invariant to make a test pass.** Never silence a test
  failure by widening tolerances, commenting out assertions, or adding
  artificial delays. Fix the underlying architectural flaw.

Project anchors: AGENTS.md §3.5; fixtures corpus with external pinned sources
(tests/fixtures + corpus plan doc 10 §5); differential-vs-VLC harness (3.E3).
Fixture provenance must always be recorded (generator-authored vs external).

Adversarial escalation is corpus-backed (user directive, 2026-10-23): every
new engine surface gets corpus classes whose explicit purpose is breaking
it. The v3 corpus suite (`tools/corpus/`) ships the escalation vocabulary —
VFR/timestamp chaos (`vfr-av`), dynamic-resolution switching
(`dynres-video`), metadata bombs (`meta-bomb-av`), dirty cross-file splices
(`splice-other` corruption op), and 23 garbage archetypes
(`GARBAGE_ARCHETYPES` in `tools/corpus/corpus_vocab.py`, builders in
`tools/corpus/corpus_builders.py`) — and ffmpeg speed parity is a tracked,
measured metric (`tools/corpus/speed_bench.py`: ffmpeg decode-to-null
baseline, best-of-N timing, ratio-vs-ffmpeg columns, JSON sidecar), not an
aspiration.

---

## Universal Pre-Submission Audit (mandatory before claiming any card COMPLETE)

1. **Memory:** does any part of this execution allocate heap memory in a loop
   or per-tick routine?
2. **Complexity:** is this algorithm the fastest known production method, or a
   textbook naive formula?
3. **Resilience:** what happens if an input field is negative, zero,
   corrupted, or formatted by an outdated encoder?
4. **Lifecycle:** does this feature work natively in-pipeline without tearing
   down or leaking state?
5. **Budgets:** does this execute well within real-time latency and compute
   budgets without spiking the CPU?

Every implementation card's final report must include a filled-in copy of this
checklist. Verdicts are backed by evidence (file:line, benchmark, test name) —
"looks fine" is not a verdict.

## Compliance audit protocol

- Audits are adversarial: their job is to find violations, not to bless code.
- Findings carry severity CRITICAL / MAJOR / MINOR, each with file:line
  evidence and a concrete fix proposal.
- Violations do not block merging honest progress, but every CRITICAL/MAJOR
  violation gets an owning fix card before the next phase gate.
- Re-audit at every phase gate (folded into the gate card's checklist).

---

# Reality-Grounding Layer (D-6..D-10) — issued 2026-10-22, extended 2026-10-23, same authority

These supplement D-1..D-5 with the "battle-hardened systems engineer"
posture. Citation handles D-6..D-10. The 2026-10-23 user session added five
directives — functional reality over ritual, no deferring the hardest
problem, tribal-knowledge capture, hardware-clock realism & binary hot
paths, and the four-question pre-submission reality check — merged below
into the thematically-matching sections and marked (2026-10-23).
Numbering note: that session's working labels "D-8 tribal knowledge" and
"D-9 clock realism" land under this document's established **D-9** and
**D-8** handles respectively; those numbers are already cited throughout
the tree (docs/audits/2026-10-22-directives-audit.md, crate doc-comments)
and are not renumbered.

## D-6. Functional Reality Over "Looking Plausible" — and Over Ritual

- **No superficial masterpieces.** Never write elegant types, complex trait
  ladders, and comprehensive comments that merely wrap a hollow or naive
  algorithm. If a 100-line function can be a fast, optimal 20-line vectorized
  routine, write the 20 lines. Zero architectural ceremony for its own sake.
- **Syntactic green ≠ runtime green.** Compiling clean and passing unit tests
  does NOT mean the code works. Evaluate against runtime latency, memory
  bandwidth, cache misses, and CPU cycle consumption.
- **Shipping ceremony is not capability (2026-10-23).** Status docs,
  dashboards, and milestone theater never substitute for a working
  capability. Every card's acceptance is measured end-to-end behavior on
  real media, not artifact existence. If a ritual artifact stops informing
  decisions, delete it.

## D-7. No Deferring the Hardest Problem (Outlaw the "Path of Least Resistance")

- **No destructive architectural cheats** (see D-4; restated because it is the
  single most common AI failure): seeking must NEVER destroy the session, drop
  hardware devices, or re-open files from disk. It must seek the byte stream,
  flush queues, and re-anchor clocks in-pipeline.
- **No deferred implementation debt — deferral-by-default is a process
  violation (2026-10-23).** NEVER leave the hardest engineering problem as
  a `// TODO`, `[PROPOSAL]`, or "follow-up card" while implementing only
  the easy surrounding scaffolding. None of those may park the single
  hardest sub-problem of the current card: either solve it in-card, or
  escalate to the Lead with a concrete blocker — a failing repro, a
  measured number, exhausted alternatives. Tackle the core algorithmic
  hurdle first.
- **No leaking state across transitions** (see D-4): a paused system must
  never blurt audio samples or render frames before the pause takes effect.

## D-8. Hardware-Clock Realism & Binary Hot Paths

- **Clocks drift — model it, never assume it away.** Never assume hardware
  timers run synchronously — audio DAC clocks and monitor refresh rates
  drift apart over time. The clock chain MUST model DAC drift and crystal
  ppm error, fed by measurement, not hand-waving: the corpus `drift-av`
  class exists to measure long-duration DAC/clock drift (`DRIFT_DUR_RANGE`
  45–110 s draws, `tools/corpus/corpus_vocab.py`; builder `build_drift` in
  `tools/corpus/corpus_builders.py`). Always implement drift-compensation
  loops and master-clock re-anchoring (project anchor:
  `crates/media-clock/src/drift.rs`, `DriftCorrection`).
- **Zero-allocation on hot paths** (see D-1): per-frame, per-tick, and
  real-time audio callback code MUST be zero-allocation with pre-allocated
  pools/arenas.
- **No heavy serialization on fast streams — FORBIDDEN, not merely
  discouraged (2026-10-23).** JSON or text serialization on 60 fps hot
  paths (frame and audio timing streams) is forbidden between threads or
  processes: binary structs with measured nanosecond budgets only — shared
  memory, raw byte rings, compact binary protocols. Text formats live in
  cold paths: config, and telemetry at ≥ 1 s intervals.

## D-9. The Tribal Knowledge Mandate (Capture It — and Dirty Reality)

- **Capture hard-won knowledge or lose it (2026-10-23).** Any debugging
  insight that took more than ~30 minutes to find MUST land in a `docs/`
  file within the same card: root cause, repro recipe, prevention.
  Undocumented hard-won knowledge is lost knowledge; the next agent pays
  the cost again.
- **Dirty reality > clean specs (restates and hardens D-3).** Expect
  signed integers in unsigned fields (negative dimensions), advisory
  headers that lie (declared durations ≠ data), legacy variants
  (lowercase identifiers, omitted optional chunks). Always implement
  graceful degradation and damage recovery rather than aborting on the
  first non-standard byte.

## D-10. The Pre-Submission Reality Check (mandatory, per card, in addition to the D-0..D-5 checklist)

**Four questions, answered in every final report before merge** (user
directive, 2026-10-23). A report without all four answers is not accepted:

- **What did I MEASURE?** — numbers and commands, not adjectives.
- **What did I NOT test?** — the honest gaps, named.
- **What would break this?** — adversarial input, load, ordering.
- **Who hits this first?** — the user scenario that meets the flaw before
  anyone else does.

The code-facing form of the same check (2026-10-22 issuance; numbering
preserved — cited in-tree as D-10 q1..q4, e.g. crates/codecs/tests/benches.rs):

1. Did I take any shortcut that tears down and rebuilds state instead of
   handling it in-pipeline?
2. Will this code freeze or drop frames if fed a 4K 60fps stream on physical
   hardware?
3. Does this parser fail if a third-party encoder wrote a negative number or
   a 0 in an optional field?
4. Am I testing only what I generated, or did I test against corrupt and
   adversarial inputs?

## D-11. The Randomized Real-Media Corpus Mandate (user-issued 2026-10-22)

> "For testing make all resolution and type video and audio and test with
> them so we just don't depend on code — real audio and video generated from
> script, many random formats, NOT deterministic, too many random samples,
> which tell us real numbers."

Testing against a hand-picked fixture list is still an echo chamber with
extra steps. The binding rules:

- **Real media, script-generated.** The project maintains a corpus
  generator (`tools/corpus/gen_corpus.py`) that drives **ffmpeg/ffprobe**
  (real production encoders) — the media under test is authored by an
  external engine, never by Lumen's own fixture writer.
- **RANDOM, not deterministic.** Every corpus run draws its parameters
  (codec, container, resolution, fps, pix_fmt, sample rate, channels,
  duration, quality) from OS entropy — two runs never match. A fixed,
  hand-curated "known-good matrix" is forbidden as the primary evidence
  base. Reproducibility is preserved honestly: the manifest records the
  seed + the exact encoder command per file, and `--replay` re-executes
  them for defect triage.
- **All resolutions and all types.** Coverage-guided randomness: every
  codec-family × container-family and every resolution class (16×16 →
  3840×2160, plus odd-dimension draws) is guaranteed at least one slot per
  corpus, with random draws filling the rest.
- **Dirty world included.** Every corpus carries corrupted siblings
  (truncation, byte flips, header smash, tail garbage, lying extensions)
  and garbage archetypes; nothing may panic or hang on them (D-3).
- **Real numbers, not green checkmarks.** Conformance is judged against
  **ffprobe/ffmpeg as the external oracle** and reported as measured
  numbers: identification accuracy, packet-count and duration deltas vs
  the oracle, PSNR/max-diff fidelity vs the reference decode, and decode
  throughput (MB/s, fps) per resolution class. A suite that cannot quote
  its numbers has not run.

Project anchors: `tools/corpus/gen_corpus.py` (generator),
`tools/corpus/run_harness.py` (oracle harness + report),
`crates/{media-probe,demux,codecs}/tests/corpus.rs` (env-gated suites,
activated only by the harness driver — the local gate never depends on a
generated corpus existing), reports under `docs/corpus/`.
