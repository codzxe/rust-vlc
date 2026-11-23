# AUDIT: crates/subtitles/src/compositing/bitmap.rs
| Field | Value |
|---|---|
| Auditor | W09-A02 |
| Date | 2026-11-23 |
| Lines / functions | 313 / 7 (+3 tests) |
| File verdict | REAL (crate-level UNREACHABLE from shipped binaries — see dvb/mod.rs F-008) |
| Reachable from shipped entry point? | NO — renderer-wgpu/playback-engine do not depend on the subtitles crate |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `./target/debug/deps/subtitles-85f02b91c21cc80b compositing::bitmap` -> 3 passed, 0 failed (exit 0); Grep line verification |

## 1. One-paragraph verdict
Clean, small, honest layer: `from_rgba` validates zero-dims/stride/baseline with checked
arithmetic and typed errors, span→ns conversion happens exactly once with the half-open rule
pinned by tests that pass. The three defects are all latent-hazard hygiene: a pub `pixel()`
that indexes unchecked in release behind a debug_assert, a pub `solid()` with unchecked usize
arithmetic and no size cap, and a `u64 as i64` narrowing that can silently negate absurd
timestamps. All current callers of the unsafe-ish two are test code.

## 2. What actually works (verified)
— verified by `./target/debug/deps/subtitles-85f02b91c21cc80b compositing::bitmap` -> 3
passed (stride/baseline/zero-dim error shapes; 90kHz→ns exact values; half-open boundary
checks at all four edges; open-ended stamps).

## 3. Findings
F-001 [S4]
Where: crates/subtitles/src/compositing/bitmap.rs:173-177
Code:
```
    #[must_use = "reading a pixel you will not look at is a bug in the test"]
    pub fn pixel(&self, x: u32, y: u32) -> Rgba {
        debug_assert!(x < self.width && y < self.height);
        let i = self.offset(x, y);
```
What is wrong: In release builds there is NO bounds check — `self.rgba[i]` panics on any
out-of-range coordinate a future non-test caller passes. The doc says "callers pass in-range
coordinates"; every current caller is `#[cfg(test)]` (rg over the crate: ass/render.rs,
cc/render.rs tests only). Public API that only debug-asserts its safety contract.
What correct looks like: `rgba.get(i..i+4)` with a documented fallback or get() -> Option.
Confidence: HIGH — Verification: read + rg caller scan.

F-002 [S4]
Where: crates/subtitles/src/compositing/bitmap.rs:151
Code:
```
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
```
What is wrong: Unchecked usize multiply: width=height≈2.15e9 overflows (debug panic, release
wrap → "capacity overflow" abort in Vec); any large-but-valid dims allocate w*h*4 bytes with
no cap. Inconsistent with `from_rgba`'s `checked_mul` path (L116-122). All current callers
are tests (rg: bitmap.rs/layout.rs tests, tests/*.rs). What correct looks like: same
checked_mul gate as from_rgba before allocating.
Confidence: HIGH — Verification: read + rg caller scan (no production caller).

F-003 [S4]
Where: crates/subtitles/src/compositing/bitmap.rs:222
Code:
```
                    Ok(Some(ns.0 as i64))
```
What is wrong: `Nanoseconds(pub u64)` (media-types/src/time.rs:781) narrowed with `as i64`:
a timestamp whose nanosecond value lands in (i64::MAX, u64::MAX] (constructible with an
exotic timebase, e.g. num=2/den=1 and large ticks) silently becomes a negative cue start/end
instead of a typed `CueError::Time`. Real 90kHz/µs-scale inputs cannot reach it.
Confidence: MEDIUM — Verification: read (media-types Nanos type confirmed; overflow window
confirmed by arithmetic; not executed).

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CueError + Display + Error + From | 20-72 | REAL | — | exhaustive, honest messages; usize::MAX sentinel "expected" on overflow path is a white lie (L120) |
| CueBitmap::from_rgba | 105-138 | REAL | — | zero/stride/baseline validated, checked_mul; executed by tests |
| CueBitmap::solid | 145-161 | REAL | S4 | unchecked capacity math (F-002); test-only callers |
| CueBitmap::offset | 163-165 | REAL | — | only used by pixel() |
| CueBitmap::pixel | 171-184 | REAL | S4 | release-unchecked index (F-001); test-only callers |
| CompositedCue::new | 239-260 | REAL | S4 | ns conversion once; u64→i64 narrowing (F-003); TimeError typed |
| CompositedCue::active_at | 236-241 | REAL | — | delegates to timing::active_at, boundary-tested |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| stride_is_validated | YES | asserts all three error variants with exact payloads |
| span_converts_to_ns_once | YES | exact ns values + all four half-open boundary probes |
| missing_stamps_are_open_ended | YES | None start/end semantics + active_at(i64::MIN/0/end) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Timestamp conversion happens ONCE, at build — per-frame timing is a plain i64 compare" | mod.rs:5-6 | REAL | CompositedCue::new stores ns; active_at is a compare (timing.rs) |
| "# Panics / Never in release; debug-asserts bounds" (pixel) | L170-172 | REAL | the doc admits the release hazard (F-001) |
| "MP §8: data, never a panic" (CueError) | L20 | SHALLOW | true for the constructor paths; pixel()/solid() can panic/abort (F-001/F-002) |

## 7. Dead / duplicated / unreachable code
Unreachable from shipped binaries (crate-level). `pixel`/`solid` are effectively test-only
vocabulary (documented as such) — dead code in any production sense.

## 8. Dependencies & flags
Depends on compositing::layout/target, media_types::time, testkit (dev). No feature gates.

## 9. Open questions for the next agent
- W09 compositing/compose.rs + target.rs owner: does the compositor index `bitmap.rgba`
  directly (bypassing offset()) and are those indexes bounds-checked per pixel? Not in my
  file list — verify the blend loop's clipping.
