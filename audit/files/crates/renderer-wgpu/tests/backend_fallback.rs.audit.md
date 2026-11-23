# AUDIT: crates/renderer-wgpu/tests/backend_fallback.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 312 / 5 tests + 6 helpers |
| File verdict | REAL tests for the REAL fallback machinery — but the suite itself proves the card's headline ("auto-negotiated wgpu backend") is PAPER: the hardware tier is a permanent typed absence |
| Reachable from shipped entry point? | Test file (not shipped); code under test shipped: `Renderer::fallback_to_software`/`negotiate`/`CHAIN` in renderer-wgpu (dep of apps/cli + apps/desktop) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |

## 1. One-paragraph verdict
Five genuine tests of the renderer's degradation machinery, run live (5/5 pass): a scripted mid-run backend death surfaces a typed `SurfaceLost` error, `fallback_to_software` swaps the compositor while the same renderer/surface/params survive, presents resume with content verified through readback (final fill 192 asserted byte-wise), the capability report names the new backend + cause + replaced backend, and a software→software fallback is idempotent and honest about `fallback_from: "software"`. The suite is also honest about the elephant: `the_chain_is_hardware_first_software_terminal_with_typed_absence` asserts the hardware tier answers **Unavailable** with a "Lead-gated" reason citing `records/wgpu-30.0.1.md` — i.e., the "GPU/backend fallback chain" of card 7.C1 has no GPU tier in any shipped build; the "hardware backend" in the flagship test is the software compositor with scripted failures. The machinery is real; the card's headline capability (auto-negotiated wgpu backend) does not exist.

## 2. What actually works (verified)
- `timeout 100 ./target/debug/deps/backend_fallback-42547c38abb89860 --test-threads=2` → `5 passed; 0 failed`.
- The flagship test content-checks the whole loop: two healthy presents → readback, third present fails typed (message contains "surface"), fallback, two more presents, readback differs and equals the fill-192 frame. Not a smoke test.

## 3. Findings

F-001 [S3 — the card's headline (wgpu auto-negotiation, hotplug) is paper; only the software-degradation half exists]
Where: backend_fallback.rs:4-8 (the AC quote), 29-36 (the disclosure), 300-312 (the typed absence)
Code:
```
//! *"GPU/backend fallback chain (auto-negotiated wgpu backend →
//! software present) in renderer-wgpu; capability report surfaced;
//! hotplug tests with simulated device events"*:
```
```
//! No hardware backend exists in this build (the wgpu import is
//! Lead-gated — `records/wgpu-30.0.1.md`), so "a hardware backend
```
What is wrong: the AC's three legs — (a) auto-negotiated **wgpu** backend, (b) fallback chain, (c) **hotplug** tests with simulated device events — reduce to: (a) absent (Hardware.probe() is a permanent `Unavailable`, asserted by the suite itself), (b) real (software→software degradation of a scripted-failure compositor), (c) absent (no device-event/hotplug test exists in this file; the comment defers to "the engine's hotplug hooks" on their side — unverified here). The tests are honest about it, but any matrix row citing card 7.C1 as "GPU fallback landed" overstates: what landed is the fallback machinery plus a truthful absence.
Confidence: HIGH — Verification: ran the suite (the Unavailable arm executed); rg for wgpu import in renderer-wgpu Cargo (Lead-gated).

F-002 [S4 — `FlakyBackend` healthy-path delegation re-creates `SoftwareCompositor` per composite]
Where: backend_fallback.rs:125
Code:
```
        SoftwareCompositor::new().composite(_frame, _placement, _background, _target)
```
What is wrong: every healthy composite constructs a fresh `SoftwareCompositor` (test-only; if that type allocates per construction this is minor test churn). Cosmetic, no effect on what the tests prove.
Confidence: HIGH — Verification: read.

## 4. Per-function table (test file)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| frame / request / surface / renderer_over_flaky | 59-150 | REAL | — | real VideoFrame/PresentRequest/HeadlessSurface/Renderer wiring |
| FlakyBackend (Compositor impl) | 90-127 | REAL double | S4 | sanctioned test double; failure is the REAL typed SurfaceLost |
| backend_failure_mid_run_falls_back... | 161-208 | REAL | — | full degradation loop, content-checked via readback |
| fallback_preserves_params_and_surface_state | 212-235 | REAL | — | background survives; corner-pixel check |
| fallback_while_already_on_software... | 240-259 | REAL | — | idempotence + honest fallback_from |
| negotiated_renderer_presents_and_reports_software | 268-293 | REAL | — | startup report self-contained, relay_pair checked |
| the_chain_is_hardware_first..._typed_absence | 299-312 | REAL | S3 | proves the hardware tier absent (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| backend_failure_mid_run... | YES | error-text, backend names, report reason contents, readback content (192-fill) |
| fallback_preserves_params_and_surface_state | YES | extent equality + corner pixel = params background |
| fallback_while_already_on_software... | YES | fallback_from Some("software") + present works |
| negotiated_renderer_presents_and_reports_software | YES | compositor name, report fields, relay-pair text |
| the_chain_is_hardware_first... | YES | CHAIN equality + Unavailable reason text lock ("Lead-gated", record path) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "GPU/backend fallback chain (auto-negotiated wgpu backend → software present)" | card 7.C1 AC quoted at :4-8 | HALF-FALSE | the wgpu tier does not exist in any build (suite asserts its typed absence); only software terminal + machinery are real |
| "hotplug tests with simulated device events" | same AC | NOT-IN-THIS-FILE | no device-event test here; deferred to engine-side hooks (UNVERIFIED) |
| "the failure ... typed error — never a panic" | :21-23 | TRUE | present().err() path asserted |
| "Falling back while already on software is idempotent and honest" | :24-25 | TRUE | test asserts |

## 7. Dead / duplicated / unreachable code
None in-file. (The hardware tier of `CHAIN` is dead-in-build by design — recorded absence.)

## 8. Dependencies & flags
renderer-wgpu (shipped), video_engine Presenter, testkit. The wgpu dependency itself is Lead-gated off — see records/wgpu-30.0.1.md.

## 9. Open questions for the next agent
- Engine-side hotplug hooks (issue #42's device-event leg): which crate owns them and are they tested? This suite defers that leg.
- Matrix auditors: check whether any row claims "GPU rendering" or "hardware-accelerated present" — the shipped renderer is software-only with a typed-absence hardware tier.
