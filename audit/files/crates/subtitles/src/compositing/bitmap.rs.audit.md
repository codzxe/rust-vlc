# AUDIT: crates/subtitles/src/compositing/bitmap.rs
| Field | Value |
|---|---|
| Auditor | W09-A08 |
| Date | 2026-11-21 |
| Lines / functions | 313 / 12 units (CueError+Display+From, CueBitmap×4, CompositedCue×2, 3 tests) |
| File verdict | REAL-UNPROVEN → in shipped terms UNREACHABLE (correct, in-crate-executed contract layer; no shipped caller) |
| Reachable from shipped entry point? | NO — `CueBitmap`/`CompositedCue` producers are the six in-crate format modules (pgs/mod.rs:631, vobsub/mod.rs:389, srt/render.rs:297, cc/render.rs:160, dvb/mod.rs:906, ass/render.rs:425,556); every one of those is itself unreachable (crate unlinked from all apps). Consumers: compose.rs (A05), in-crate tests. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | `rg "CueBitmap::solid"` (callers: bitmap/layout test code + tests/compositing_ac.rs only — no production caller); `rg "from_rgba("` (6 production producers, listed above); `rg "\.pixel("` in compositing/ (0 production callers — target.rs has its own `Rgba8Surface::pixel`); media-types time.rs `as_nanos` traced (:192-197, :461-480 — returns `Nanoseconds(u64)`, typed overflow, negative refused); baseline: `orchrun-test-workspace-nofailfast.log.txt:5347-5349` (3/3 bitmap unit tests ok), `:5569-5581` (compositing_ac 8/8 ok). `sed -n` quote verifies. |

## 1. One-paragraph verdict
The shared payload type of the whole subtitle stack is small, careful, and correctly validated on the hot path: `from_rgba` refuses zero dims, checks `w·h·4` in u64 with `checked_mul` (no overflow), and validates the baseline; span→ns conversion happens once at build and is pinned by exact tests; the half-open `active_at` rule is test-pinned at both boundaries. The defects are all in the penumbra: `solid` (test vocabulary) skips the very overflow check `from_rgba` does 30 lines earlier, `pixel`'s rustdoc promises "Never [panics] in release" while indexing a Vec that absolutely panics in release when out of range, the `Nanoseconds(u64) as i64` narrowing silently wraps for exotic time bases the sibling `as_nanos` deliberately permits, and `CompositedCue::new` enforces no span ordering — leaving each of the six producers to remember it (ASS does; VobSub does not — my vobsub F-ref).

## 2. What actually works (verified)
— Baseline `orchrun-test-workspace-nofailfast.log.txt:5347-5349`: `compositing::bitmap::tests::{missing_stamps_are_open_ended, span_converts_to_ns_once, stride_is_validated}` all ok; `:5569-5581`: `tests/compositing_ac.rs` 8/8 ok (pixel-exact seams, z-order stacking, 1080p/4K↔CIF scaling, half-transparent rounding).
— Trace: `from_rgba` (:105-138) is the crate's allocation gate — every producer's bitmap passes through it; u64 checked math means no `capacity-overflow` abort path for any u32×u32 input (overflow → typed `StrideMismatch{expected: usize::MAX}`).
— `CompositedCue::new` (:212-234): single ns conversion at build (the per-frame path is an i64 compare — `active_at`), `TimeError` typed; verified against media-types `as_nanos` (fast path + i128 reference agree bit-exactly per its own tests, time.rs:1370-1388).

## 3. Findings

### F-001 [S4]
Where: crates/subtitles/src/compositing/bitmap.rs:145-154
Code:
```
    pub fn solid(width: u32, height: u32, color: Rgba) -> Result<CueBitmap, CueError> {
        if width == 0 || height == 0 {
            return Err(CueError::ZeroDimension {
                size: (width, height),
            });
        }
        let mut rgba = Vec::with_capacity(width as usize * height as usize * 4);
```
What is wrong: `width as usize * height as usize * 4` is unchecked — for width=height=u32::MAX the product overflows usize: panic in dev-profile builds, wrapped (absurd) capacity in release. `from_rgba` does the identical computation with `checked_mul` and a typed refusal (:116-122); `solid` omits it.
Why it matters: an inconsistency in the one type whose job is bounded allocation; any future production caller of `solid` (it is `pub`, documented for "degenerate boxes") inherits the unguarded path. Current callers are test-only (verified), hence S4 not S2.
What correct looks like: the same `checked_mul` chain as `from_rgba`, mapping overflow to `CueError::TooLarge`-equivalent.
Confidence: HIGH — Verification: read (arithmetic: (2^32−1)²·4 ≈ 7.4e19 > u64::MAX).

### F-002 [S4]
Where: crates/subtitles/src/compositing/bitmap.rs:168-183
Code:
```
    /// One pixel (synthetic-test and debug vocabulary).
    ///
    /// # Panics
    /// Never in release; debug-asserts bounds (the constructor
    /// validated the stride, callers pass in-range coordinates).
```
What is wrong: the rustdoc "Panics" contract is false. `self.rgba[i]` (:178-181) is ordinary safe indexing — bounds-checked and panicking in EVERY profile, release included, when `(x,y)` is out of range. The `debug_assert` adds a check in debug only; it removes nothing in release.
Why it matters: rustdoc `# Panics` sections are API contracts; this one promises an absence of panics the code cannot deliver. (No in-tree production caller — 0 hits in compositing/, verified — so no runtime impact today.)
What correct looks like: `# Panics / Panics if (x, y) is outside the bitmap; callers in release must keep coordinates in range.` or return `Option<Rgba>`.
Confidence: HIGH — Verification: read (Vec indexing semantics).

### F-003 [S4]
Where: crates/subtitles/src/compositing/bitmap.rs:218-226
Code:
```
        let stamp_ns = |stamp: &Stamp| -> Result<Option<i64>, CueError> {
            match stamp {
                Stamp::Present(ts) => {
                    let ns = ts.as_nanos()?;
                    Ok(Some(ns.0 as i64))
```
What is wrong: `as_nanos()` guarantees the value fits **u64** (`Nanoseconds(u64)`, media-types time.rs:475-480 — values > u64::MAX are a typed `TimeError::Overflow`); `ns.0 as i64` then narrows without a check, so any timestamp whose ns value lies in (i64::MAX, u64::MAX] silently becomes a negative start/end. Unreachable through in-tree producers (ASS µs-bounded by H:MM:SS.CC; PGS/DVB by 33-bit 90 kHz PTS; VobSub by the ms math), but a `CompositedCue::new` caller with a custom `TimeBase` (e.g. num=2, den=1e9, large ticks) hits it — exactly the "any time base" freedom the doc comment (:206-207 "a media-types span (any time base …)") advertises.
Why it matters: the crate's discipline is typed refusal everywhere; this seam wraps silently instead. (compose.rs:73 has the same cast — A05's file, cross-ref.)
What correct looks like: `i64::try_from(ns.0).map_err(|_| CueError::Time(TimeError::Overflow { .. }))`.
Confidence: HIGH — Verification: read (as_nanos contract traced in media-types time.rs).

### F-004 [S4]
Where: crates/subtitles/src/compositing/bitmap.rs:212-234
Code:
```
    pub fn new(
        span: TimeSpan,
        bitmap: CueBitmap,
        layout: CueLayout,
        z: u16,
    ) -> Result<CompositedCue, CueError> {
```
What is wrong: no span-ordering validation — `end < start` is accepted and produces a cue that `active_at` can never light (silent never-active). Enforcement is left to each producer and is inconsistent: the ASS path refuses (`EndBeforeStart`, ass/render.rs:360-365,468-473) but the VobSub path does not (an SPU stop date before its start date yields `end_ns < start_ns` straight through — my vobsub audit; CcCueBuilder likewise, my cc F-005).
Why it matters: the contract type could enforce the crate-wide rule once; instead the rule is per-producer folklore and two producers already miss it.
What correct looks like: a `CueError::EndBeforeStart` variant checked in `new`.
Confidence: HIGH — Verification: read (all six producers' guard presence compared).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `CueError` + `Display` + `From<TimeError>` | 15-82 | REAL | — | 8 variants, precise fields; the usize::MAX sentinel for u64-overflow (:120) is odd but typed; `expected as usize` (:125) truncates on 32-bit targets in the REPORTED field only (decision math stays u64) |
| `CueBitmap` (struct) | 84-97 | REAL | — | invariants documented per field |
| `CueBitmap::from_rgba` | 99-138 | REAL | — | the gate: zero-dim, u64 checked w·h·4, baseline ≤ height; correct |
| `CueBitmap::solid` | 140-161 | REAL | S4 | F-001 unchecked mul; test-only callers (verified census) |
| `CueBitmap::offset` | 163-166 | REAL | — | `(y·w+x)·4` — max index w·h·4−1 consistent with rgba.len() (from_rgba-validated), no overflow for realizable bitmaps |
| `CueBitmap::pixel` | 168-183 | REAL | S4 | F-002 doc contract false; `#[must_use]` correct |
| `CompositedCue` (struct) | 186-203 | REAL | — | pre-converted ns Option fields; doc "never per frame" true (active_at is an i64 compare) |
| `CompositedCue::new` | 205-234 | REAL | S4 | F-003 narrowing; F-004 no ordering check; else correct single-conversion |
| `CompositedCue::active_at` | 236-241 | REAL | — | delegates to timing::active_at (A07); half-open pinned by tests at both ends |
| tests `stride_is_validated` | 250-270 | REAL | — | pins all three from_rgba refusals with exact fields |
| tests `span_converts_to_ns_once` | 272-293 | REAL | — | 90 kHz → exact ns + all four boundary actives |
| tests `missing_stamps_are_open_ended` | 295-312 | REAL | — | Missing start ⇒ always active until end |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| stride_is_validated | YES | exact variant+field asserts on 63-vs-64, zero dim, baseline 5>4 |
| span_converts_to_ns_once | YES | exact ns values (1e9/2e9) and all four half-open boundary actives |
| missing_stamps_are_open_ended | YES | active_at(i64::MIN) true, end boundary false |
| (integration) compositing_ac 8 tests | YES | pixel-exact seams/z-order/scaling/rounding — baseline :5569-5581 all ok |
| NOT covered | — | F-001 (solid never called with huge dims), F-003 (no exotic-base caller), F-004 (no producer feeds a reversed span to `new` in tests) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Timestamp conversion happens ONCE, at build — per-frame timing is a plain i64 compare" | bitmap.rs:1-4 module doc | REAL | CompositedCue::new + active_at trace; compositing_ac timing tests |
| lib.rs "the blend math lives HERE, once" (contract framing) | lib.rs:19-25 | REAL (in-crate) / UNREACHABLE (shipped) | blend_row in target.rs, driven by compose.rs — crate unlinked from apps |
| "# Panics Never in release" (pixel) | bitmap.rs:169-172 | FALSE | F-002 |
| "no panics on any input — failure is typed" (crate-level) | lib.rs:70-72 | SHALLOW | true of the traced hot paths; false for `solid`'s dev-profile overflow (F-001) and the silent `as i64` wrap (F-003) — "no silent wrap" is the implied promise the cast breaks |

## 7. Dead / duplicated / unreachable code
`solid` and `pixel` are test-vocabulary `pub` fns with zero production callers (census in header) — fine as scaffolding, but they carry the two unguarded paths (F-001/F-002). Nothing else dead; no duplication.

## 8. Dependencies & flags
`media-types::time` (Stamp/TimeError/TimeSpan), in-crate `layout`/`target` types. Matches boundaries.toml (`internal_deps = ["media-types"]`). No unsafe, no features, std-only.

## 9. Open questions for the next agent
- A05 (compose.rs): your `now.as_nanos()?.0 as i64` (:73) shares F-003 — record it on your side; also does compose re-validate span ordering or rely on producers (F-004)?
- A07 (timing.rs): confirm `active_at`'s treatment of `start = None` (live-from-beginning) is what every producer intends — bitmap tests pin it, but no producer other than tests ever emits a Missing start.
