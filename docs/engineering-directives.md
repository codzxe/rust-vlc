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
