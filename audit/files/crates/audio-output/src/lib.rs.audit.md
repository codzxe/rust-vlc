# AUDIT: crates/audio-output/src/lib.rs
| Field | Value |
|---|---|
| Auditor | W08-A02 |
| Date | 2026-11-15 |
| Lines / functions | 157 / 1 function (`builtin_backends`) + 9 module decls + re-exports + 1 doctest |
| File verdict | REAL (module wiring + re-exports real, doctest executed; its ONE function is dead code) |
| Reachable from shipped entry point? | YES — the crate is a CLI dependency (apps/cli/Cargo.toml:49); `NullBackend`/`NullStream` re-exports consumed at apps/cli/src/pipeline_host.rs:52-53 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `cargo test -p audio-output --doc` → 1 passed / 0 failed (fresh run); rg `builtin_backends` repo-wide (definition only, zero callers — independently confirmed; sibling audits W08-A03/A04/A06 concur); rg `unsafe` in src (comments only); rg `thread::spawn` in crate (0 hits); rg doc-03 quotes |

## 1. One-paragraph verdict
This is the crate root: honest, unusually candid documentation plus module wiring. The cpal story is told truthfully (not admitted, typed stub marker, records cited), the no-blocking contract is stated with its actual enforcement level, and the doctest is a real round-trip that I executed fresh (1 passed). The single wart is that the file's only executable code — `builtin_backends()`, presented as "The backend set v0 ships" — is dead: zero callers anywhere in the repo, so the "backend set that ships" is in fact nothing; shipped hosts hard-wire `NullBackend` directly. Every doc claim I could check against reality held up (no unsafe, no threads, borrows-only `render_into`, doc 03 quotes accurate).

## 2. What actually works (verified)
— `cargo test -p audio-output --doc` → `test crates/audio-output/src/lib.rs - (line 92) ... ok; 1 passed` (the null round-trip doctest, fresh run).
— Re-export surface verified: `pub use` lines 135-143 all resolve to real items (ring.rs, null.rs audited here; render/cpal/stub audited by siblings W08-A01/A03/A06).
— "Std-only; no `unsafe`" (line 86): TRUE — rg `unsafe` in crates/audio-output/src matches only comments (render.rs:29, cpal.rs:21, lib.rs:42/66).
— "No threads are spawned here" (line 74): TRUE — rg `thread::spawn|JoinHandle|scope(` in crates/audio-output → 0 hits.
— `render_into` "takes only borrows" (line 59-60): TRUE — signature `fn render_into(ring: &SpscRing, counter: &SampleCounter, underruns: &AtomicU64, out: &mut [f32]) -> RenderStats` (render.rs:89-93).

## 3. Findings
F-001 [S3]
Where: crates/audio-output/src/lib.rs:145-157
Code:
```
/// The backend set v0 ships: the null backend first (headless/CI),
/// then the typed not-supported stub for the current OS (the real
/// backends' placeholder until ADR-005 lands them in `platform-*`).
///
/// Selection policy (fallback order per device loss, doc 03 §4.9 d)
/// is deliberately the caller's in v0 — ADR-005 owns it when the real
/// backends exist `[PROPOSAL]`.
pub fn builtin_backends() -> Vec<Box<dyn AudioBackend>> {
```
What is wrong: dead code presented as the shipping surface. `rg builtin_backends` over the whole repo finds only this definition (plus audit files); no host, engine, or test calls it. The shipped CLI constructs `NullBackend` directly (apps/cli/src/pipeline_host.rs:1598-1599) and never sees `UnsupportedBackend`, so the "fallback order per device loss" and the stub's typed refusal are unreachable theater. Why it matters: the doc comment describes a selection mechanism that does not exist at any call site; a reader concludes device-loss fallback is wired when nothing is. What correct looks like: either wire `builtin_backends()` into the host's backend selection or delete it and state plainly that v0 hosts choose the null backend by construction. Confidence: HIGH — Verification: rg (independently) + sibling audits W08-A03 (stub.rs), W08-A04 (null.rs), W08-A06 (cpal.rs) all record the same zero-caller fact.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| builtin_backends | 152-157 | REAL code, UNREACHABLE | S3 | F-001; constructs NullBackend + UnsupportedBackend; correct in isolation |
| doctest (lib.rs:92-122) | 92-122 | REAL | — | executed fresh: 1 passed; asserts write→render→capture round-trip incl. exact bytes |
| module declarations | 124-133 | REAL | — | 9 modules, all exist |
| re-exports | 135-143 | REAL | — | all resolve |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| lib.rs doctest (line 92) | YES | writes 1 stereo frame via `write_bytes`, renders via `device.render`, asserts `frames_read==1`, `silence_filled==0`, exact sample values `vec![0.5, -0.5]`, and byte-exact capture — any break in ring push/read, render stats, or capture fails it |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cpal 0.18.2 is conditional-E2 and NOT admitted" | lib.rs:28-29 | TRUE | cpal.rs gate `AUDIO_ADR005_ADMITTED` audited by W08-A06; records exist (docs/dependencies/records/cpal-0.18.2.md, audio-output-cpal.md) |
| "the device sample counter is the only trusted progress source (doc 03 §4.7)" | lib.rs:12-13 | TRUE (quote accurate) | docs/03-engine-plan.md:553 verbatim |
| "never blocks... Forbidden: any lock, allocation, logging, syscall" | lib.rs:55-58 | REAL-UNPROVEN | structural argument + op-count ledger; panicking-allocator/lock-tracer enforcement marked [PROPOSAL: tooling] — doc 03:648 itself says "[PROPOSAL: tooling detail]; code review plus these tests are the gate"; zero-alloc assertions exist via testkit CountingGlobalAlloc (crates/testkit/src/alloc_count.rs) used by tests/zero_alloc_null_sink.rs |
| "The backend set v0 ships" (builtin_backends) | lib.rs:145-147 | FALSE in effect | F-001: zero callers |
| "Std-only; no `unsafe`" | lib.rs:86 | TRUE | rg: only comment mentions |
| "No threads are spawned here" | lib.rs:74 | TRUE | rg: 0 spawns in crate |

## 7. Dead / duplicated / unreachable code
`builtin_backends()` — see F-001 (also drags `UnsupportedBackend` into permanent unreachability, per W08-A03's stub.rs audit).

## 8. Dependencies & flags
No external deps in this file; crate deps: audio-engine, media-types, media-clock (per Cargo.toml, W08-A08 audits it). No feature gates. No cfg branches.

## 9. Open questions for the next agent
- If ADR-005 ever admits cpal, `builtin_backends()` is the intended flip point — until then every claim resting on "backend selection" (doc 03 §4.9 d) is unwired. Whoever audits the engine session wiring should confirm no other selection path exists.
