# AUDIT: crates/audio-output/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W08-A08 |
| Date | 2026-11-23 |
| Lines / functions | 41 / manifest (no functions; 1 package, 3 deps, 1 dev-dep, 0 features) |
| File verdict | REAL (every header claim verified against the tree) |
| Reachable from shipped entry point? | YES — audio-output is a dependency of apps/cli (lumen-cli, the release lane) and apps/web |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg "unsafe" over src (8 hits, ALL in doc comments — zero unsafe blocks); rg boundaries.toml:408-413; read stub.rs, cpal.rs references; sed quotes |

## 1. One-paragraph verdict
A clean manifest whose unusually long header makes verifiable claims, every
one of which I checked and none of which is false: zero external dependencies
(three internal path deps + internal dev-dep only; boundaries.toml
external_allowlist = [] agrees), std-only with no unsafe (rg over src/: all
eight "unsafe" hits are doc-comment prose, zero blocks), cpal NOT admitted
(the typed BackendNotAdmitted marker in cpal.rs + a compile-time tripwire
test pin the gate shut), null backend fully implemented (read: null.rs —
enumerate/open/pause/resume/capture all real), and real-device backends
stubbed with typed refusals (stub.rs — every method answers typed
Unsupported, never fake success). The one registry deviation (media-clock is
a dependency though doc 01 §2.6's row lists media-types + audio-engine only)
is self-documented in the header AND recorded in boundaries.toml:410 — the
deviation is governed, not hidden.

## 2. What actually works (verified)
- Claim "zero external dependencies" -> rg the [dependencies] table: 3 path
  deps; boundaries.toml:412 `external_allowlist = []` (arch gate agrees).
- Claim "no unsafe (the wait-free ring is atomics-only)" -> `rg -n "unsafe"
  crates/audio-output/src/` = 8 hits, all in `//!`/`//` doc lines (render.rs
  :29, pattern.rs:3, pump.rs:3, cpal.rs:21, lib.rs:42,66,86); zero unsafe
  blocks or functions.
- Claim "cpal is conditional-E2, NOT admitted" -> crates/audio-output/src/
  cpal.rs carries AUDIO_ADR005_ADMITTED=false; tests/pcm_pipeline.rs:238-243
  compile-time asserts it; docs/feature-matrix.toml:2362 agrees.
- Claim "null backend (fully implemented, CI-safe)" -> null.rs read (files
  2/3 of this audit): open_stream/pause/resume/take_device/capture real;
  baseline 33 lib + 13 integration tests green.
- Claim "typed not-supported stubs for real devices" -> stub.rs: enumerates
  no devices, every op returns typed Unsupported naming the platform.

## 3. Findings
F-001 [S4]
Where: crates/audio-output/Cargo.toml:26
Code:
```
description = "Lumen audio output: the AudioBackend adapter trait, wait-free SPSC ring + real-time render callback (no-blocking contract, op-count budget), underrun accounting, the null backend and typed OS-backend stubs (doc 03 §4.9)."
```
What is wrong: "op-count budget" as a package-level feature phrase invites
reading it as a proven real-time guarantee. As established in the
callback_budget audit: the op-count ledger is proven for the pure core only;
allocation-freedom is structural (untrapped); WCET is unmeasured (Phase-3
debt). The description names mechanisms, not proofs, so it is not false —
but "budget" without the null-adapter scoping overstates to a crates.io-style
reader. publish=false limits the blast radius to repo-internal readers.
What correct looks like: "op-count budget (null adapter; WCET Phase-3)".
Confidence: HIGH — Verification: read + companion audit
(audit/files/crates/audio-output/tests/callback_budget.rs.audit.md).

## 4. Per-function table
Not applicable (manifest, 41 lines, no code).

## 5. Test honesty
Not applicable (no tests in manifest). The manifest's dev-dep claim
("testkit is dev-only by contract, MP §3.7") is honored: testkit appears
only in [dev-dependencies]; src/ never imports it (rg-verified in files 2/3
reads: only tests/ files use testkit::Must).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cpal is conditional-E2, NOT admitted... this sandbox cannot build it (alsa-sys/pkg-config probe fails, exit 101)" | :11-15 | REAL | cpal.rs marker; docs/dependencies/records/cpal-0.18.2.md; matrix :2362 notes the audited spike incl. "sandbox build failure exit 101" |
| "trait-based adapter layer + null backend (fully implemented, CI-safe) + typed not-supported stubs" | :15-17 | REAL | null.rs, stub.rs, device.rs read; baseline green |
| "Std-only by C5: zero external dependencies, no unsafe" | :18-20 | REAL | dep table + boundaries.toml; rg unsafe = comments only |
| "This card adds the media-clock edge the registry under-specifies" | :5-6 | REAL (governed deviation) | boundaries.toml:410 internal_deps includes media-clock |
| "Real backends land in platform-* (L5) per doc 01 §2.7" | :18-19 | PAPER-ONLY (future) | platform-* audio streaming halves do not exist yet (only enumeration seams) |

## 7. Dead / duplicated / unreachable code
Not applicable. (The crate itself is reachable and used by shipped binaries
via the null path — see files 2/3 audits.)

## 8. Dependencies & flags
[dependencies]: audio-engine, media-clock, media-types (all path).
[dev-dependencies]: testkit (path). No [features] section — no feature flags
to gate code out of the release lane; everything in src/ ships. Edition
2024, publish=false, workspace lints inherited.

## 9. Open questions for the next agent
- When ADR-005 opens (cpal or direct adapters), this manifest gains its
  first external dependency and the header's "zero external dependencies"
  becomes historical — ensure the header is rewritten with the admission
  record, and that stub.rs/cpal marker tests flip in the same commit (the
  compile-time tripwire will force the test edit).
