# ADR-005: Audio Output Backends — the adapter seam in audio-output (L4), OS implementations in platform-* (L5), backend admission gated

- **Status:** **accepted for the adapter-seam architecture** (2026-10-22,
  formalized by card 3.DOC-1 from the 2.C3 draft; the seam is implemented,
  merged, and carried the 2.E2-V gate). **Real-backend admission (cpal or the
  direct-API alternative): PENDING** — no OS audio library is imported, on any
  target; the admission trigger and its obligations are recorded under
  "Revisit triggers" below.
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
