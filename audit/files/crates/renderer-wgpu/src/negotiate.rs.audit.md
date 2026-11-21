# AUDIT: crates/renderer-wgpu/src/negotiate.rs
| Field | Value |
|---|---|
| Auditor | W03-A06 |
| Date | 2026-11-21 |
| Lines / functions | 433 / 18 rows |
| File verdict | REAL (as library) / UNREACHABLE (from shipped entry points) |
| Reachable from shipped entry point? | NO for `negotiate()` and `Renderer::fallback_to_software`: complete caller map (rg over repo) = `tests/backend_fallback.rs` only. The shipped CLI builds its renderer via `Renderer::new_headless` (apps/cli/src/pipeline_host.rs:1314), which hardwires `SoftwareCompositor` (renderer.rs:88–93) without walking any chain. `BackendReport::relay_pair`'s destination (`Engine::observe_renderer_backend`, hotplug.rs:311) is likewise called only by `tests/hotplug.rs:579,616`. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:2 |
| Commands run | read Cargo.toml (no wgpu dep — media-types + video-engine only); read `docs/dependencies/records/wgpu-30.0.1.md` (the Lead-gate record exists and is substantive); `rg -n "negotiate\(\)|fallback_to_software|relay_pair|observe_renderer_backend"` (caller map); `rg -n "negotiate::tests::"` in nofailfast baseline (5/5 green :4782–4786); read `tests/backend_fallback.rs` mid-run fallback integration tests; `sed -n` on all quoted ranges. |

## 1. One-paragraph verdict
The file is honest about its own hollowness — the "auto-negotiate wgpu → software present" chain is a two-element walk in which tier 1 is a compile-time constant refusal and tier 2 is unconditional, the crate ships zero wgpu (Cargo.toml deps: media-types, video-engine), and every doc comment names the Lead-gate record (`docs/dependencies/records/wgpu-30.0.1.md` — verified to exist and to contain the empirical no-GPU probes). The walk, selection, typed report, and mid-run fallback are correctly implemented and well tested (5/5 unit + the backend_fallback integration suite, all green in the no-fail-fast baseline). What is NOT true is any implication this machinery runs: the shipped CLI never negotiates — it calls `Renderer::new_headless`, which links `SoftwareCompositor` directly — so the "auto-negotiated" capability report and the `video.backend-changed` relay are test-only theater in the shipped binary. One latent trap: if a future build flips the hardware probe to `Selected`, `negotiate()` returns `compositor: None` while the report claims `software`.

## 2. What actually works (verified)
— Chain order, first-Selected-wins selection, skip audit trail: `select_walks_in_order_first_selected_wins` proves the order with simulated availability (green, nofailfast :4784); pure function, no side effects.
— This build's probes: hardware = typed `Unavailable` with the recorded reason string (exact-equality pinned, :4782); software = `Selected`.
— Startup negotiation outcome + report content: `negotiate_lands_on_software_with_the_typed_hardware_absence` (:4783) asserts compositor name, backend id, no fallback, and all three reason fragments.
— Mid-run fallback through a simulated dying backend: `backend_failure_mid_run_falls_back_to_software_and_presents_resume` (backend_fallback.rs:161–213) — typed failure, fallback swap, presents resume on the SAME surface, content-checked through readback (fill byte 192 asserted); `fallback_preserves_params_and_surface_state` and `fallback_while_already_on_software_is_idempotent_and_honest` cover the parameter/idempotence contracts. File ran green in baseline (:4832).
— Relay-pair shape matches the engine hook's `(backend, reason)` signature: `the_relay_pair_is_the_engine_hooks_shape` (:4786).

## 3. Findings
### F-001 [S3]
Where: crates/renderer-wgpu/src/negotiate.rs:281-298
Code:
```
pub fn negotiate() -> Negotiated {
    let probed: [(BackendTier, TierStatus); CHAIN.len()] =
        [(CHAIN[0], CHAIN[0].probe()), (CHAIN[1], CHAIN[1].probe())];
    let selection = select(&probed);
```
What is wrong: `negotiate()` — the module's headline "auto-negotiate the backend chain (card 7.C1)" — has no shipped caller. The CLI's video path constructs `Renderer::new_headless` (pipeline_host.rs:1314) which links `SoftwareCompositor` unconditionally (renderer.rs:91); no tier walk, no probe, no `BackendReport`, no relay to `Engine::observe_renderer_backend` ever happens in the shipped binary. The "capability report surfaced to UI" (module docs L24–33) surfaces only in tests.
Why it matters: Card 7.C1's fallback chain is described as product behavior; in the shipped product the chain is bypassed by a constructor that hard-codes its answer. The chain cannot degrade "mid-run when a hardware backend fails" either — there is no hardware backend to fail and no call site. Same unreachable-policy family as recovery.rs's guards (my W03-A06 recovery.rs#part1 F-002).
What correct looks like: The CLI's video limb should construct through `negotiate()` and relay the report (one call each), or the module docs should state plainly that v0's only constructor bypasses negotiation.
Confidence: HIGH — Verification: rg caller map over the whole repo (negotiate/fallback_to_software/observe_renderer_backend: src definitions + tests only).
### F-002 [S4]
Where: crates/renderer-wgpu/src/negotiate.rs:188-196
Code:
```
                let id = if tier == BackendTier::Software {
                    tier.id()
                } else {
                    // The hardware tier's probe is a const
                    // `Unavailable` in this build, so selection cannot
                    // choose it; if that ever changes, this report
                    // must change with it (stated, not guessed).
                    "software"
                };
```
What is wrong: If the hardware tier ever answers `Selected` (the exact event the module says will happen when the Lead ratifies the import, L278–280), `BackendReport::negotiated` stamps the report's backend id as `"software"` — reporting the wrong active backend — and `negotiate()`'s compositor arm (L295) returns `None` for `Some(BackendTier::Hardware)`. The future build gets: no compositor + a report claiming software is active. The guard is a comment, not a type.
Why it matters: The one migration this module exists to enable (flipping the probe) lands on a path that misreports and returns no compositor; the "must change with it" note is buried in an else-branch.
What correct looks like: Make the hardware arm `unreachable!`-free but typed: return `None`-compositor AND a report that names `hardware-wgpu` as chosen-with-no-constructible-backend (or make `probe()`'s const status a build-time cfg so the arm cannot compile silently wrong).
Confidence: HIGH — Verification: read (L169–212, L286–296).
### F-003 [S4]
Where: crates/renderer-wgpu/Cargo.toml:27-29
Code:
```
[dependencies]
media-types = { path = "../media-types" }
video-engine = { path = "../video-engine" }
```
What is wrong: The crate is named `renderer-wgpu` and its own description says "GPU video presentation" (Cargo.toml:21), but it has no wgpu dependency at all — it is a pure-CPU software renderer. The name lies to every reader of a dependency graph.
Why it matters: Naming that lies is a checklist item; anyone grepping for "wgpu" finds this crate and assumes GPU presentation exists. Mitigations are real (Cargo.toml header comment L5–15 documents the Lead-gated deferral; the record file exists; `lumen engine-info` truthfully reports `video.software-compositor`, baseline INDEX) — which is why this is S4 and not higher.
What correct looks like: Rename on the wgpu admission (the record already implies the rename is owed), or add a `[package] description` that leads with "software-present renderer; wgpu deferred".
Confidence: HIGH — Verification: read Cargo.toml; record file read; engine-info string cited from baseline INDEX §MP4 SMOKE.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1–40 | REAL | S3 | Honest about the Lead gate; presents the chain as card 7.C1 behavior without saying no shipped caller runs it (F-001). |
| `BackendTier` (enum) | 45–54 | REAL | — | Two tiers. |
| `BackendTier::id` | 60–66 | REAL | — | "hardware-wgpu"/"software"; software id matches `Compositor::name` (asserted by test). |
| `BackendTier::probe` | 74–84 | REAL | — | Hardware: const typed `Unavailable` with the recorded reason; Software: `Selected`. Deterministic. |
| `TierStatus` (enum) + `is_selected` | 88–103 | REAL | — | Typed refusal; trivial classifier. |
| `CHAIN` (const) | 107 | REAL | — | Order pinned by test. |
| `Selection` (struct) | 111–118 | REAL | — | chosen + skipped audit trail. |
| `select` | 130–149 | REAL | — | Pure first-Selected-wins; order proven with simulated availability; one Vec alloc (event rate). |
| `BackendReport` (struct) | 155–164 | REAL | — | backend/reason/fallback_from strings. |
| `BackendReport::negotiated` | 169–212 | REAL | S4 | F-002 hardware-arm misreport landmine; reason assembly verified by fragment asserts. |
| `BackendReport::fallback` | 216–222 | REAL | — | Names replaced backend + cause. |
| `BackendReport::backend`/`reason`/`fallback_from` | 225–241 | REAL | — | Trivial accessors. |
| `BackendReport::relay_pair` | 247–249 | REAL | — | Shape pinned to the engine hook by test. |
| `Negotiated` (struct) + Debug | 254–271 | REAL | — | `compositor: Option<Box<dyn Compositor>>` — None only in the impossible/latent arms. |
| `negotiate` | 281–298 | REAL / UNREACHABLE | S3 | F-001: no shipped caller; deterministic software win in this build. |
| `impl Renderer::fallback_to_software` | 300–316 | REAL / UNREACHABLE | — | Swap + report; idempotence documented and test-pinned; test-only callers. |
| `tests::the_chain_is_hardware_then_software` | 324–327 | REAL | — | Order pin. |
| `tests::probes_answer_this_builds_truth` | 331–342 | REAL | — | Exact reason-string equality — pins the honest refusal. |
| `tests::select_walks_in_order_first_selected_wins` | 349–389 | REAL | — | Simulated availability both directions + total-absence typed case. |
| `tests::negotiate_lands_on_software_with_the_typed_hardware_absence` | 394–422 | REAL | — | Full report content asserted. |
| `tests::the_relay_pair_is_the_engine_hooks_shape` | 426–432 | REAL | — | Shape pin (weak — only asserts the pair equals its own accessors; the real cross-check lives in backend_fallback.rs:287 + hotplug tests). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| the_chain_is_hardware_then_software | YES | Const array equality. |
| probes_answer_this_builds_truth | YES | Exact string equality on the refusal reason. |
| select_walks_in_order_first_selected_wins | YES | Reordering or last-wins breaks chosen/skipped asserts. |
| negotiate_lands_on_software_with_the_typed_hardware_absence | YES | Compositor presence + all report fragments asserted. |
| the_relay_pair_is_the_engine_hooks_shape | WEAK | Tautology-adjacent (pair vs its own accessors); the hook-side cross-check is `hotplug.rs` tests (`renderer_backend_report_relays_as_video_backend_changed`, green in baseline :3905). |
| backend_fallback.rs integration (3 tests) | YES | Real Renderer + real surface + typed simulated-hardware failure; content-checked readback (fill bytes 192/200/9,8,7 asserted). |

All green in nofailfast baseline (:4782–4786, :4832).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "auto-negotiate the wgpu hardware backend → software present, and degrade to software mid-run" | negotiate.rs L1–4 | REAL as library / UNREACHABLE as product | F-001: no shipped caller; CLI bypasses the chain. |
| "the import is Lead-gated … deny.toml's libloading ban, the ungranted dependency envelope" | negotiate.rs L13–16 | REAL | `docs/dependencies/records/wgpu-30.0.1.md` exists and documents exactly this; Cargo.toml has no wgpu. |
| "capability report surfaced to UI" (card 7.C1) | negotiate.rs L24–33 | UNREACHABLE in product | Report produced only under tests; no host relays it. |
| "When the Lead ratifies the wgpu import, that build's probe flips and this same walk selects hardware first" | negotiate.rs L278–280 | REAL-UNPROVEN (future) + latent bug | F-002: the flipped-probe future lands on a report that says "software". |
| engine-info `video.software-compositor` | baseline INDEX | REAL | Consistent with the shipped software-only renderer. |

## 7. Dead / duplicated / unreachable code
- `negotiate()`, `Renderer::fallback_to_software`, `BackendReport::relay_pair` (as a live relay): unreachable from shipped entry points (F-001) — exported, test-driven.
- No duplication within the crate.

## 8. Dependencies & flags
Cargo.toml: media-types + video-engine only; NO wgpu (F-003). testkit + media-clock dev-only. No cfg gates in negotiate.rs. Crate compiled clean in baseline build #1.

## 9. Open questions for the next agent
- To the wave that audits `crates/playback-engine/src/hotplug.rs` (W03-A10): `Engine::observe_renderer_backend`'s wire relay (`video.backend-changed`) is the destination of `relay_pair` — the pair of unreachable ends (renderer reports, engine relays) means the entire backend-change wire row is test-only in v0. Cross-check the feature-matrix row for `video.backend-changed` for a FALSE status.
- To the Lead/evidence wave: the wgpu deferral record is unusually rigorous (empirical /dev/dri, ICD, loader probes) — a good model for other deferral records; nothing to fix here.
