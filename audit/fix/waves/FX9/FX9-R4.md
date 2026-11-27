<!--FIXV1
agent: FX9-R4
title: Derive Clone on TappedFrame so the FrameTap snapshot compiles
verdict: DONE
changed:
- apps/cli/tests/fx9_a03_subtitle_composite.rs
tests:
- (EMPTY — remedial Clone fix; central run is the oracle)
evidence_read:
- audit/fix/RULES.md
-->

## Approach

The orchestrator's central compile flagged `frames.clone()` in `FrameTap::snapshot` (fx9_a03_subtitle_composite.rs, formerly :192) as failing because `TappedFrame: Clone` was not satisfied; rustc itself suggested deriving `Clone` on the struct (formerly :168). Per the assignment I first inspected the struct's fields before choosing between the derive and the sibling snapshot pattern: `TappedFrame { pts_ns: Option<u64>, rgba: Vec<u8> }` — both fields are `Clone`, and there is no manual `impl Clone` anywhere in the file, so the derive path is valid and is the smallest diff. The fallback (mirroring fx7_a01's snapshot of tapped frames) was not needed.

## The fix

Added `#[derive(Clone)]` directly above `struct TappedFrame` (now :168-169), exactly rustc's own suggestion. That makes `Vec<TappedFrame>` cloneable, so `frames.clone()` in `snapshot()` (auto-deref through the `MutexGuard`) compiles unchanged at the call site. Nothing else in the file was touched; no new deps, no API churn, no markers.

## Risks

Minimal: the derive only adds a concrete `Clone` impl for a test-local struct whose two fields are already `Clone`; behavior of the tap and its assertions is untouched. The only residual risk is environmental — rustfmt is not installed in this box (exit 127 from both `PATH` candidates), so the one added attribute line is unformatted by tooling; it is a single bare attribute matching file style, and the central run remains the sole compile oracle.
