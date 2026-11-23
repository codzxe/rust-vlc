# AUDIT: crates/renderer-wgpu/src/negotiate.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Lines / functions | 433 / 10 prod items + 5 tests |
| File verdict | REAL chain logic, honestly-typed const hardware refusal — but UNREACHABLE from shipped entry points (test-only callers) |
| Reachable from shipped entry point? | NO — only caller of `negotiate()`/`fallback_to_software` is crates/renderer-wgpu/tests/backend_fallback.rs (rg across apps/ + crates/); the shipped CLI constructs the software compositor directly via `Renderer::new_headless` (apps/cli/src/pipeline_host.rs:1314 → renderer.rs:87-91 hard-codes `SoftwareCompositor::new()`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `./target/debug/deps/renderer_wgpu-cea255c0e8d5c422 negotiate` → 5 passed; 0 failed, exit 0; rg (callers, wgpu deps) |

## 1. One-paragraph verdict
Hostile-focus answer for "renderer negotiate": the fallback chain is real
logic with honest typing — but the hardware tier is a **hard-coded
constant refusal** (`TierStatus::Unavailable` with the Lead-gate reason
string), because the crate named "renderer-wgpu" contains **zero wgpu**
(crates/renderer-wgpu/Cargo.toml deps: media-types + video-engine only;
the wgpu 30.0.1 import is a recorded deferral). `select()` is a clean
first-Selected-wins walk, `negotiate()` deterministically lands on
software, and 5 tests ran live proving both, including simulated
hardware-available ordering. The problem is product wiring: no shipped
entry point ever calls `negotiate()` or `fallback_to_software` — the CLI's
renderer path constructs `SoftwareCompositor` directly — so the card 7.C1
"typed capability report surfaced to UI" exists only inside this crate's
own test binary. The `video.backend-changed` wire row this report is
shaped for can never originate from a shipped build.

## 2. What actually works (verified)
— verified by `./target/debug/deps/renderer_wgpu-cea255c0e8d5c422 negotiate`
→ `test result: ok. 5 passed; 0 failed` (exit 0): chain order, this
build's probe truth, first-Selected selection with simulated availability,
real software-compositor construction with the typed report naming the
skipped tier, and the relay-pair shape.

## 3. Findings

F-001 [S3]
Where: crates/renderer-wgpu/src/negotiate.rs:281-298 (whole `negotiate`)
+ renderer.rs:87-95
Code (the shipped path that bypasses it, renderer.rs:87-91):
```rust
    pub fn new_headless(size: Extent2D, config: SurfaceConfig) -> RendererResult<Renderer> {
        let surface = HeadlessSurface::new(size, config)?;
        Ok(Renderer::with_backend(
            Box::new(surface),
            Box::new(SoftwareCompositor::new()),
```
What is wrong: the negotiation chain, its typed tiers, the capability
report, and the mid-run `fallback_to_software` are called by nothing
outside crates/renderer-wgpu/tests/backend_fallback.rs. The shipped CLI
renderer path (apps/cli/src/pipeline_host.rs:1314) builds the software
compositor unconditionally. The module doc's claim "the renderer owns the
negotiation" describes a responsibility no shipped caller exercises: the
`video.backend-changed` relay (`BackendReport::relay_pair` →
`Engine::observe_renderer_backend`) is dead wiring in every product lane.
Why it matters: the wire event type exists — ui/src/generated/commands.ts:46
declars `{ backend: string; kind: "video.backend-changed"; reason: string }`
— so a UI can subscribe to a report no shipped producer can ever emit;
the fallback policy the docs promise ("degrade to software mid-run") has
no trigger in shipped code.
Confidence: HIGH — Verification: rg over apps/ and crates/ (only test
callers); read.

F-002 [S4]
Where: crates/renderer-wgpu/src/negotiate.rs:188-196
Code:
```rust
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
What is wrong: latent misreport. If a future build's hardware probe ever
answered `Selected` (the documented flip, :278-280), `select` would choose
Hardware and this arm would still stamp the report's backend id
`"software"` — the capability report would lie about the active backend.
The comment admits it ("must change with it") instead of using
`tier.id()`, which would be correct today and after the flip.
Confidence: HIGH — Verification: read; unreachable today (probe is const).

F-003 [S4]
Where: crates/renderer-wgpu/src/negotiate.rs:199-205
Code:
```rust
            None => (
                "none",
                String::from(
                    "auto-negotiation found no available backend tier (the software \
                     tier is unconditionally available — reaching here is a bug)",
```
What is wrong: the `chosen: None` report is self-described as "reaching
here is a bug", yet `Negotiated.compositor` then carries `None`
(:295) and the type offers no way to distinguish "bug" from a caller's
`None`-tolerant misuse — the bug path is typed identically to a legal
outcome instead of being an assertion/log line at the source.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| BackendTier::{id, probe} | 61-84 | REAL | - | hardware = const typed Unavailable (honest, documented) |
| TierStatus::is_selected | 100-102 | REAL | - | trivial |
| CHAIN | 107 | REAL | - | order pinned by test |
| select | 130-149 | REAL | - | first-Selected-wins; proven with simulated availability |
| BackendReport::negotiated | 169-212 | REAL | S4 | F-002 latent id misreport |
| BackendReport::fallback | 216-222 | REAL | - | honest fallback_from |
| BackendReport accessors/relay_pair | 226-249 | REAL | - | shape matches engine hook (test) |
| Negotiated + Debug | 254-271 | REAL | S4 | F-003 None-compositor "bug" path untyped |
| negotiate | 281-298 | REAL | - | deterministic; compositor None for Hardware (cannot happen) |
| Renderer::fallback_to_software | 311-315 | REAL | - | idempotent swap; test-only caller (F-001) |
| 5 tests | 324-432 | REAL | - | all ran live; would fail if order/probe/report broke |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| the_chain_is_hardware_then_software | YES | exact CHAIN equality |
| probes_answer_this_builds_truth | YES | exact reason string |
| select_walks_in_order_first_selected_wins | YES | 3 scenarios incl. simulated hardware |
| negotiate_lands_on_software_... | YES | constructs the real compositor; asserts report contents |
| the_relay_pair_is_the_engine_hooks_shape | PARTLY | shape equality only (does not call the engine hook) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "auto-negotiate the wgpu hardware backend → software present, and degrade ... mid-run" | negotiate.rs:1-4 | STUB-by-wiring (logic REAL, unwired) | F-001: no shipped caller |
| "capability report surfaced to UI" | negotiate.rs:24-25 | UNREACHABLE | report only read by this crate's tests |
| "Honest, never faked" (hardware tier) | negotiate.rs:16-17 | REAL | const typed Unavailable; no wgpu dep in Cargo.toml |
| crate description "RGBA8 upload ... sRGB present" | Cargo.toml | REAL-UNPROVEN | outside this file's scope (renderer.rs/staging.rs not assigned) |
## 7. Dead / duplicated / unreachable code
`negotiate`, `fallback_to_software`, `BackendReport` relay machinery:
production-quality dead code in every shipped lane (F-001).
## 8. Dependencies & flags
media-types + video-engine only; NO wgpu dependency at all (the crate
name overpromises — the deferral is documented in
docs/dependencies/records/wgpu-30.0.1.md per Cargo.toml header).
## 9. Open questions for the next agent
- backend.rs / renderer.rs (not assigned): is SoftwareCompositor a real
  rasterizer or a shape-tested stub? (`backend.rs:6` says "minimal CPU
  rasterizer" — verify.)
- Does any settings-schema key or UI row reference "hardware-wgpu" or the
  backend report? If yes, F-001 upgrades to S2 (UI wired to nothing).
