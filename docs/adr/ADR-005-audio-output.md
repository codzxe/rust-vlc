# ADR-005: Audio Output Backends — the adapter seam in audio-output (L4), OS implementations in platform-* (L5), backend admission gated

- **Status:** **accepted for the adapter-seam architecture** (2026-10-22,
  formalized by card 3.DOC-1 from the 2.C3 draft; the seam is implemented,
  merged, and carried the 2.E2-V gate). **Real-backend admission (cpal or the
  direct-API alternative): PENDING** — no OS audio library is imported, on any
  target; the admission trigger and its obligations are recorded under
  "Revisit triggers" below. The wave-14 admission spike (issue #1,
  `docs/dependencies/records/audio-output-cpal.md`) refreshed the evidence:
  the verdict remains **conditional-E2, not admitted** — the recommended
  path is recorded under "Admission-spike outcome" below.
- **Deciders:** Lead (draft authored by the 2.C3 continuation agent, worklog
  entry "2.C3" §"ADR-005 DRAFT"; formalization dispatched by 3.HARD-AUDIT
  decision D-c)
- **Evidence:** worklog entry 2.C3 (the draft this file formalizes);
  `docs/dependencies/records/cpal-0.18.2.md` (the P0 verification record —
  cpal **not admitted**; four independent grounds); `crates/audio-output/**`
  (the implemented seam: `lib.rs`, `device.rs`, `null.rs`, `stub.rs`,
  `ring.rs`, `render.rs`, `counter.rs`, `error.rs` + the AC suites
  `tests/{sine_sink_hash,callback_budget,clock_truth}.rs`);
  `docs/gates/phase2-exit.md` §1 rows "audio on screen" and §3 dependency
  posture; `apps/cli` + `tests/integration/first-vertical/**` (the null-backend
  vertical that carried the gate, mock-labeled)
- **Supersedes:** open question in doc 01 §7 / doc 04 §3 candidates ("cpal vs
  per-platform direct"); the D-008 promise "ADR-005 at Phase 1 wave 1.C" is
  now discharged — late, honestly — by this file
- **Consequence for risks:** R-29 re-scored (file exists; backend admission
  pending); R-20's audit finding "ADR-005 draft exists only at worklog 2.C3"
  closed by this file

## Context

MP §9 makes the engine Rust-only; doc 11 §1.2's **E2 exception class** covers
audited Rust bindings to **OS-provided audio services** (not vendored C), and
reserves audio output's exception slot for this ADR. Doc 01 §2.6/§2.7 place
the OS backends in `platform-*` (L5): "audio backend (API per ADR-005)".

The cpal P0 verification (card 2.C3, 2026-10-21; `records/cpal-0.18.2.md`)
settled the candidate facts: cpal 0.18.2 is a pure-Rust crate (Apache-2.0)
whose **Linux default graph links the system ALSA** via `alsa`/`alsa-sys` +
pkg-config — conditional-E2, not a `pass`; its admission vehicle (this ADR)
was unwritten at the time; the dev sandbox can neither build it (no `alsa.pc`
— empirical exit 101 recorded) nor exercise a device (none exists). The
record's verdict: **conditional (E2), NOT admitted by 2.C3 — the card's
platform-adapter fallback is executed instead.** The fallback became the
implemented architecture this ADR now accepts.

## Decision

1. **`audio-output` (L4) stays std-only and owns the contract, not the
   devices**: the `AudioBackend` adapter trait (extends audio-engine's
   `AudioSink` with enumerate/open/pause/resume + the
   `Arc<dyn SampleCounterSource>` clock truth + underruns), the wait-free SPSC
   ring, the `render_into` real-time callback (borrows only; ≤1 ring read,
   ≤1 silence fill, 1 counter advance, ≤1 underrun count per call — doc 03
   §4.9 c's no-blocking contract, structurally enforced by the signature and
   measured by the `RenderStats` ledger), the offset-carried `SampleCounter`,
   the fully-implemented `NullBackend`, and typed
   `UnsupportedBackend` stubs (stable `audio.os_backend_pending` error naming
   the platform and this ADR; enumerates nothing, never fakes success).
   **L4 never links an OS audio library directly.**
2. **Real backends are implemented in `platform-linux`/`platform-macos`/
   `platform-windows` (L5)** as `AudioBackend` impls within this ADR's
   ratified scope, with documented unsafe invariants per adapter (MP §9) —
   the same template as the ADR-001 hardware-decode adapters.
3. **cpal 0.18.2 MAY be the platform-* implementation substrate per-target**
   (it is conditional-E2, not pure-Rust-pass): each admission needs the
   `[VERIFY-P1]` refresh the record lists (lockfile graph, RustSec at
   admission date, publisher pins, `dasp_sample` license re-check) **plus a
   CI image carrying the target's audio dev packages** — the sandbox's
   empirical build failure is the recorded reason this cannot be waved
   through.
4. **The direct-API alternative stays open**: ALSA/CoreAudio/WASAPI via
   metadata-generated bindings (the ADR-001 windows-rs/objc2 pattern) may win
   on any target where cpal's abstraction costs latency or callback-contract
   fidelity. The decision input is doc 03 §4.9's no-blocking contract, which
   the trait seam enforces regardless of substrate — so the choice is
   per-target engineering, not architecture.
5. **Device-loss fallback order (doc 03 §4.9 d) is configured policy**, not
   hardcoded (the `[PROPOSAL]` recorded in `audio-output/src/lib.rs`).

## What is already implemented and verified (the accepted half)

- `crates/audio-output` — 3,097 LOC, std-only, zero dependencies; 32 tests:
  23 lib + 4 callback-budget + 1 clock-truth + 3 sine-sink-hash + 1 doctest
  (2.C3, gate 1133/0/3 at merge).
- AC evidence: the sine fixture sink-buffer hash (byte-identical to the
  pinned f32 conversion of the corpus PCM; writer/consumer SHA-256
  cross-pins), the 180-callback operation ledger with zero storage growth,
  the starved-path data-then-silence ordering (one underrun event per short
  callback, zeros never stale audio), the geometry sweep (segments ≤ 2
  everywhere), and clock truth through a real `MediaClock::audio_master`
  (30 ticks = exactly 1.000 s; pause freezes; resume lands exactly on 2.000 s).
- The 2.E1 first vertical runs end-to-end on the null backend through this
  seam, mock-labeled (`tests/integration/first-vertical/evidence/ledger.json`;
  2.E2-V clause "audio on screen" = MET (mock-labeled)).

## Consequences

- The v0 vertical (2.E1 and successors) runs headless on the null backend in
  CI; **real-device evidence is Phase 3+ hardware-labeled work** (MP §13) —
  no real-backend claim exists anywhere today, and the typed stubs say so on
  every platform.
- Honest limits of the current evidence (the 2.C3 caveats, carried): callback
  timing under a real OS scheduler, device loss mid-stream, and the cpal
  graph's build are all unexercised `[VERIFY-P1]`; the fault-injecting
  allocator + lock-tracer (doc 03 §4.9 c tooling) remain `[PROPOSAL:`
  `tooling]` owed to testkit.
- The registry delta stands: audio-output's `media-clock` edge (L4→L2, the
  device sample counter is the clock's truth) is recorded in
  `boundaries.toml`; doc 01 §2.6's row is reconciled by 3.DOC-1.

## Revisit triggers (the admission gate)

- **Trigger**: the first real-backend card (the platform-* audio cards,
  Phase 3+/7.C per doc 13) **or** any CI image with target audio dev
  packages, whichever comes first. At admission: re-run the cpal record's
  `[VERIFY-P1]` set, add the allowlist row with a pinned range, and amend
  this ADR with the per-target substrate choice (cpal vs direct-API) and the
  safety review per doc 11 §1.2 E2.
- If cpal's abstraction fails the no-blocking contract or latency budget on
  any target (doc 03 §4.9 e), that target flips to the direct-API path — a
  per-target amendment, not a new ADR.
- Device-loss behavior (doc 03 §4.9 d) is re-scored when the policy
   configuration lands (decision 5).

## Admission-spike outcome (wave 14, issue #1 — 2026-11-09)

The wave-14 admission spike produced a second, independent evidence
base (`docs/dependencies/records/audio-output-cpal.md`): real `cargo
tree` output for all three desktop targets from a throwaway /tmp project
(cpal 0.15.3 per the assignment pin; latest 0.18.2 stays the admission
target), plus license/MSRV/unsafe counts from the extracted sources and a
reproduced sandbox build failure (alsa-sys pkg-config probe, exit 101).

- **Graph (real):** Linux `cpal → alsa 0.9.1 → alsa-sys 0.3.1
  [links="alsa"] → pkg-config build dep` (+ dasp_sample, libc, bitflags,
  cfg-if); macOS `cpal → coreaudio-rs 0.11.3 → coreaudio-sys 0.2.18`
  with an **unconditional bindgen build chain** (clang-sys, libloading,
  regex, nom…); Windows `cpal → windows 0.54.0` (metadata-based, no
  bindgen). Licenses all inside the allow set; cpal MSRV 1.70.
- **C-FFI exposure (E-class evidence):** unsafe is structural in every
  host (cpal wasapi/device.rs 28 occurrences, coreaudio 11+, alsa 8;
  alsa pcm.rs 49; alsa-sys generated.rs = **1,301 `extern "C"`
  functions**). Confirms: cpal may only ever sit behind an approved L5
  platform adapter — never in an L0–L4 crate (AGENTS §3.3).
- **Alternatives (surveyed, real crate data):** `alsa`/alsa-rs is the
  same E2 class and cpal's own substrate (the direct-API candidate);
  `libpulse-binding` binds a userspace sound *server's* C client (weaker
  fit, daemon dependency); `rodio` is not a backend — it depends on cpal
  itself. No pure-Rust path to OS audio exists on any desktop target;
  the E2 exception is inherent to audio output, and the L4 seam stays
  the only pure-Rust layer.

**Recommended path `[PROPOSAL — Lead sign-off]` (Linux-first):** admit
cpal at the latest version as the `platform-linux` (L5) substrate
implementing the existing `AudioBackend` trait, default features only
(ALSA host; no jack/pulse/pipewire features), unsafe confined to the
adapter with documented invariants (the ADR-003/004 exception pattern).
Preconditions: a CI image carrying `libasound2-dev` (the recorded
sandbox blocker) and the 0.18.2 record's `[VERIFY-P1]` refresh
(lockfile graph, RustSec, publisher pins). macOS/Windows follow as
separate per-target admissions (decision 4): macOS weighs cpal's
coreaudio-sys bindgen build chain against metadata-generated bindings;
Windows is the cleanest (windows-crate path, no bindgen).

**Conformance plan for post-admission** (what the platform adapter must
prove against this ADR before any real-device claim):

1. **PCM format matrix**: interleaved f32 is the seam's v0 contract
   (`error::format_unsupported` otherwise); the adapter's device-open
   negotiation must be swept across the corpus rates (8/16/22.05/32/
   44.1/48/96 kHz), channel counts (1/2/6), and the s16→f32 pinned
   conversion, asserting the ring receives byte-identical samples for
   every accepted shape and a typed refusal for every rejected one —
   the sine-hash suite's oracle generalized to device-negotiated
   formats.
2. **Latency / callback discipline**: the adapter installs
   `render_into` as the cpal callback unchanged (the no-blocking
   contract is structural: borrows only, ≤1 ring read + ≤1 silence
   fill + 1 counter advance per call); conformance = the
   `RenderStats`-ledger budget measured on the real device (segments
   ≤ 2, zero allocation/growth across a soak), the `SampleCounter`
   remaining the clock's only truth source (cpal exposes no position
   API on all hosts — recorded), underruns counted per short callback,
   and pause/resume freezing the counter without a rewind. The
   fault-injecting allocator hook stays `[PROPOSAL: tooling]` testkit
   work.
3. **Bounded queue / drop policy**: the `PcmPump` pacing proof
   (wave 14: `audio_output::pump` + `audio_output::pattern`) must hold
   on-device — in-flight frames ≤ high-water at every step, the
   configured drop/backpressure policy's accounting exact, and the
   whole-session capture hash equal to the decoded input prefix.
