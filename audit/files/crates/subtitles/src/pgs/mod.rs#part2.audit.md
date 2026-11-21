# AUDIT: crates/subtitles/src/pgs/mod.rs#part2
| Field | Value |
|---|---|
| Auditor | W09-A05 |
| Date | 2026-11-21 |
| Chunk boundary | **part2 = lines 574-1092** (PgsCueBuilder doc+impl, `TRANSPARENT_BLACK`, the whole `#[cfg(test)] mod tests`). Part1 = lines 1-573 (module docs, PgsDamage, PgsDisplaySet/ObjectState/PgsDecoder, decode_framed, segment handlers, compose) — A06's assignment. Boundary chosen at the `PgsCueBuilder` doc block; `compose()` (521-571) is part1 but its output feeds this part — cross-referenced, not re-audited. |
| Lines / functions | 519 audited / 5 production items + 21 test fns/builders |
| File verdict | UNREACHABLE (real, test-executed library code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — `PgsCueBuilder` callers: crates/subtitles/src/pgs/fuzz.rs:75 and crates/subtitles/tests/pgs_fuzz_survival.rs:232,265 (crate-internal + tests). The `subtitles` crate's only repo-wide dependent is fuzz/Cargo.toml:50 (lumen-fuzz, separate workspace, not shipped); apps/cli and apps/desktop-tauri have no subtitles dependency. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | rg "PgsCueBuilder\|layout_for\|TRANSPARENT_BLACK" (callers above); sed -n verifications of every quote; Read of compositing/layout.rs place_cue (clamp semantics), compositing/bitmap.rs from_rgba (3rd arg is `baseline`, not stride); rg media-types time.rs Timestamp/TimeBase constructors; baseline: orchrun-test-workspace-nofailfast.log.txt:5436-5467 (32 pgs tests ok, incl. all 11 mod tests). |

## 1. One-paragraph verdict
Part2 is the 5.L1 contract adapter (`PgsCueBuilder`) plus the module's 11-test suite, and it is genuinely written and green: `cues()` produces half-open `[t0, next-1ns)` spans with rational exact-position anchors, and `cue_builder_spans_and_layout` pins the span edges, anchor fractions (960/1920, 540/1080), reference extent, open-ended tail, and z — all passing in the baseline run (log:5457). Hostile reading breaks the "exact" promise twice: `layout_for` stamps every PGS cue with `SafeMargin::TitleSafe`, so `place_cue` (compositing/layout.rs:310-335) clamps legally-placed edge subtitles into the 5% title-safe box — the module doc's "exact PCS position" claim is false for any object in the outer 5% band; and out-of-extent coordinates silently fall back to the broadcast bottom-center anchor instead of surfacing typed damage. A non-monotonic `next_pts_ns` silently produces an open-ended cue. The test block is honest about lacking a differential oracle but leans on an unverifiable "fetched ffmpeg semantics" provenance claim. None of this is reachable from the shipped binary.

## 2. What actually works (verified)
— `cues()` span/anchor/z/layout semantics: `cue_builder_spans_and_layout` asserts `[1_000_000_000, 2_000_000_000-1)`, `active_at` boundary exclusivity, `Rational::new(960,1920)`/`(540,1080)` anchors, `reference == Some((1920,1080))`, open-ended tail (`end == None`), `z == 3` — passes in baseline (orchrun-test-workspace-nofailfast.log.txt:5457).
— Epoch-machine behaviors exercised by the part2 tests all pass in baseline (log:5458-5467): damage cap (`DamageOverflow{dropped:16}` for 80>64), empty-composition clear, epoch-start flush → `PaletteMissing`, object persistence across Normal sets with live palette replace, multi-fragment ODS assembly (2+2+4 into declared 8), palette-missing drops set / object-missing skips, truncated/unknown segments typed, RLE overrun clipped+typed, invalid object size typed, `.sup` segment PTS filling a silent container PTS.
— `CueBitmap::from_rgba(width, height, baseline, rgba)` third-argument semantics verified in compositing/bitmap.rs:105-135: it is `baseline` (must be ≤ height), NOT a stride — PGS passing `object.height` is the max-valid baseline choice, no width/height transposition.
— `Timestamp::new(ticks, base)` (media-types time.rs:147) is infallible for any i64 — the `ns` closure cannot panic; `TimeBase { num: 1, den: 1_000_000_000 }` satisfies TimeBase::new's invariants (non-zero, coprime).

## 3. Findings

### F-001 [S2] `layout_for` clamps PGS "exact" positions into the 5% title-safe box — the doc's "exact PCS position" promise is false at the edges
Where: crates/subtitles/src/pgs/mod.rs:659-666 (and doc 574-579)
Code:
```
        CueLayout {
            anchor: Anchor::new(u, v).unwrap_or(Anchor::BOTTOM_CENTER),
            alignment: CueAlignment {
                horizontal: HorizontalAlign::Start,
                vertical: VerticalAlign::Top,
            },
            margin: SafeMargin::TitleSafe,
            reference: Some(video),
```
Doc (574-577): "each composed object becomes a [`CompositedCue`] with its exact PCS position as a rational anchor over the video extent".
What is wrong: the anchor is exact, but the placement is not: `CueLayout.margin = SafeMargin::TitleSafe` sends every PGS cue through `place_cue`'s `clamp_axis` (compositing/layout.rs:321-335), which forces the origin into `[5% inset, extent-5% inset-size]`. A PGS object legally placed in the outer 5% band (forced subs pinned to screen edges, sign translations at frame corners — routine Blu-ray authoring; ffmpeg/VLC render pgssub at the raw x/y) is silently repositioned inward. On 1920×1080 that is up to a 96 px horizontal / 54 px vertical displacement.
Why it matters: bitmap subtitle formats are absolute-position formats; this is wrong rendering on valid input, silent (no damage event, no error), and directly contradicts the module's own "exact position" claim.
What correct looks like: `SafeMargin` needs a `None`/`Exact` variant (or the PGS builder uses it), so absolute-position formats bypass the broadcast safe-area clamp; text formats keep TitleSafe.
Confidence: HIGH — Verification: read of both sides (pgs layout_for + layout.rs clamp_axis/inset_px) — clamp math re-derived.

### F-002 [S3] Out-of-extent PCS coordinates silently relocate to the broadcast bottom-center anchor instead of typed damage
Where: crates/subtitles/src/pgs/mod.rs:657-660
Code:
```
        let (w, h) = (video.0.max(1), video.1.max(1));
        let u = Rational::new(u32::from(x), w).unwrap_or(Rational::ZERO);
        let v = Rational::new(u32::from(y), h).unwrap_or(Rational::ZERO);
        CueLayout {
            anchor: Anchor::new(u, v).unwrap_or(Anchor::BOTTOM_CENTER),
```
What is wrong: PCS x/y are u16 and can exceed the PCS-declared video extent (malformed but perfectly decodable). Then `u = x/w > 1` and `Anchor::new` errors (`num > den`, layout.rs:98-109) — and the `.unwrap_or(Anchor::BOTTOM_CENTER)` swallows it: the object teleports to the broadcast subtitle anchor (u=1/2, v=9/10) with Start/Top alignment. No `PgsDamage`, no `CueError`, no log. The module's contract (mod.rs doc 50-53) says "every malformation … is typed data". Additionally the two `Rational::new(..).unwrap_or(Rational::ZERO)` are dead: `w`/`h` are `max(1)`, so the denominator is never 0 and `Rational::new` never errs.
Why it matters: silent wrong output on corrupt input is the exact failure class this crate's damage ledger exists to prevent; the same pattern is duplicated in dvb/mod.rs:933 (A06/A07 territory).
What correct looks like: return the `CueError` from `cues()` (it already returns `Result`) or record a damage event and skip the object; delete the dead `unwrap_or(ZERO)` arms or make them `must()`.
Confidence: HIGH — Verification: read of `Anchor::new` bounds in layout.rs:98-109.

### F-003 [S3] Non-monotonic `next_pts_ns` silently yields an open-ended (never-ending) cue
Where: crates/subtitles/src/pgs/mod.rs:609-612
Code:
```
        let end = match (next_pts_ns, start) {
            (Some(next), Some(start)) if next > start => Some(next - 1),
            _ => None,
        };
```
What is wrong: when `next <= start` (seek, non-monotonic PTS, or a caller unit mix-up) the span's end becomes `None` — the cue stays active forever instead of being empty/dropped/errored. The function returns `Result<Vec<CompositedCue>, CueError>` yet swallows this condition. This is not hypothetical: the crate's own survival suite hits the path — tests/pgs_fuzz_survival.rs:233 calls `builder.cues(&set, Some(90_000 * 2), 0)` against a set decoded with `Some(90_000)` 90 kHz ticks (= 1_000_000_000 ns), i.e. the test passes ticks where nanoseconds are expected, `next < start`, open-ended — and asserts nothing about the span, so the unit confusion is invisible.
Why it matters: overlapping/stuck cues on timestamp disorder; also an API-contract trap (ns vs 90 kHz ticks) proven by the crate's own test mis-calling it.
What correct looks like: `next <= start` → empty span (drop the cue) or a typed error; and the fuzz-survival caller should pass converted nanoseconds.
Confidence: HIGH — Verification: read + trace of the cited test call.

### F-004 [S4] Dead zero-dimension guard in `cues()`
Where: crates/subtitles/src/pgs/mod.rs:628-630
Code:
```
        for (i, object) in set.objects.iter().enumerate() {
            if object.width == 0 || object.height == 0 {
                continue;
            }
```
What is wrong: `compose()` (part1, 468-478 + 560-568) only ever inserts objects whose `w > 0 && h > 0` (the ObjectSizeInvalid path removes them), so a composed `PgsObjectImage` can never carry zero dimensions; the guard is unreachable through the real pipeline and would silently skip (no damage) if it ever fired on a hand-built set.
Why it matters: dead defensive code that hides the invariant instead of asserting it.
What correct looks like: delete the guard (from_rgba already returns `CueError::ZeroDimension` for 0×0) or assert the invariant.
Confidence: HIGH — Verification: read of compose() insertion paths in part1.

### F-005 [S4] z saturation can collapse distinct objects onto one z; TimeBase struct-literal bypass
Where: crates/subtitles/src/pgs/mod.rs:645 and 617-620
Code:
```
                z_base.saturating_add(i as u16),
```
What is wrong (minor, two hygiene items): (a) `z_base` near `u16::MAX` with several objects saturates — later objects share one z, losing PCS order silently; a wrapping or clamping-with-damage policy is unstated. (b) `let ns_base = TimeBase { num: 1, den: 1_000_000_000 };` constructs the type by literal, bypassing `TimeBase::new`'s validation — documented inline as intentionally equal to the `Ok` value, but it will silently diverge if `TimeBase` ever gains an invariant.
Why it matters: both are correctness-by-coincidence spots a future edit can break without a compiler or test signal.
What correct looks like: (a) u32 z or explicit clamp+damage; (b) a `const NANOS_BASE: TimeBase` near the type or a `TimeBase::NANOS` constructor.
Confidence: HIGH — Verification: read.

## 4. Per-function table (part2 items; part1 fns listed only where called from here)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct PgsCueBuilder` | 580-581 | REAL | — | unit struct |
| `impl Default for PgsCueBuilder` | 583-587 | REAL | — | delegates to new() |
| `PgsCueBuilder::new` | 591-593 | REAL | — | trivial |
| `PgsCueBuilder::cues` | 601-649 | REAL (flawed) | S3 | F-003 (open-ended on next<=start), F-004 dead guard, F-005a z saturation; per-object `rgba.clone()` (fine at subtitle rates); `?` propagation of CueError never exercised by tests |
| `PgsCueBuilder::layout_for` | 655-668 | REAL (flawed) | S2 | F-001 TitleSafe clamp contradicts "exact position"; F-002 anchor fallback + dead unwrap_or(ZERO) |
| `TRANSPARENT_BLACK` const | 671-674 | REAL | — | consumed by pgs/palette.rs:112 — not orphaned |
| `tests::seg` | 689-694 | REAL | — | stripped-framing segment builder |
| `tests::pcs` | 696-718 | REAL | — | full PCS grammar incl. object refs |
| `tests::pds` | 720-726 | REAL | — | palette entries (id,y,cr,cb,a) |
| `tests::ods_first` / `ods_first_declared` / `ods_cont` | 728-754 | REAL | — | fragment flags 0x80/0x40 verified against segment parser tests (baseline 5449) |
| `tests::END` / `WHITE_ENTRY` / `BLACK_ENTRY` | 756-761 | REAL | — | constants |
| `tests::epoch_lifecycle_full_display_set` | 763-794 | REAL | — | strong: clean decode, extent, fps id, composition number, geometry, all-white pixels, framing detection |
| `tests::epoch_normal_reuses_objects_palette_update` | 796-830 | REAL | — | pins live-palette replace semantics + pts_ns conversion |
| `tests::empty_composition_clears_and_epoch_start_flushes` | 832-860 | REAL | — | clear + flush → PaletteMissing typed |
| `tests::multi_fragment_ods_assembles` | 862-880 | REAL | — | 3-fragment assembly, clean |
| `tests::palette_missing_drops_set_object_missing_skips` | 882-915 | REAL | — | both damage variants pinned |
| `tests::truncated_and_unknown_segments_are_typed` | 917-952 | REAL | — | empty payload / unknown 0x77 / truncated PCS / LengthOverrun |
| `tests::rle_overrun_and_short_fragment_are_typed` | 954-986 | REAL | — | clipped-still-renders + FragmentWithoutHeader |
| `tests::invalid_object_size_is_typed` | 988-1006 | REAL | — | 0x0 object |
| `tests::sup_framing_uses_segment_pts_when_container_silent` | 1008-1040 | REAL | — | PTS-prefixed framing, pts None → segment PTS fills |
| `tests::cue_builder_spans_and_layout` | 1042-1074 | REAL | — | the part2 core test; square 2x2 object (see §5) |
| `tests::damage_ledger_is_capped` | 1076-1091 | REAL | — | 80 events → 64 + dropped:16 |

## 5. Test honesty (part2 tests)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| epoch_lifecycle_full_display_set | YES | asserts concrete rgba pixels, geometry, fps id, framing |
| epoch_normal_reuses_objects_palette_update | YES | pins the palette-replace semantics (entry 1 → transparent, entry 2 → black) |
| empty_composition_clears_and_epoch_start_flushes | YES | multi-step, damage variant pinned |
| multi_fragment_ods_assembles | PARTIALLY | asserts clean + rgba len; would not catch a fragment mis-concatenation that still totals 8 bytes |
| palette_missing_drops_set_object_missing_skips | YES | both variants |
| truncated_and_unknown_segments_are_typed | YES | four damage variants |
| rle_overrun_and_short_fragment_are_typed | YES | overrun + orphan fragment |
| invalid_object_size_is_typed | YES | 0x0 size |
| sup_framing_uses_segment_pts_when_container_silent | YES | pts_ns == Some(1_000_000_000) |
| cue_builder_spans_and_layout | YES | span edges, active_at, anchors, reference, open-ended, z — but see note below |
| damage_ledger_is_capped | YES | dropped:16 exact |
| Note (mutation blind spot) | — | every object in every cues() test is SQUARE (2x2, 4x1 is non-square but only checks rgba.len); a width/height transposition inside the bitmap path would be invisible — verified the 3rd `from_rgba` arg is `baseline` so no actual transposition, but the suite would not catch one |
| Un-tested paths in part2 | — | out-of-order next_pts (F-003 — and pgs_fuzz_survival.rs:233 mis-exercises it with a ticks-vs-ns mix-up), out-of-extent anchor fallback (F-002), TitleSafe clamp effect on PGS positions (F-001), `cues()` error paths (all call sites use `.must()`), z saturation |

## 6. Doc claims touching this part
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "each composed object becomes a CompositedCue with its exact PCS position" | mod.rs:574-577 | FALSE at the placement level | F-001: `margin: SafeMargin::TitleSafe` + layout.rs `clamp_axis` moves any object in the outer 5% band; anchor is exact, position is not |
| "the half-open span closes one nanosecond before the next set's PTS" | mod.rs:577-579 | REAL-UNPROVEN for disorder | true for monotonic PTS (test pins it); F-003 shows next<=start silently yields open-ended |
| "32 unit tests + fuzz_sub_pgs survival harness, all green in this tree 2026-10-26" | docs/feature-matrix.toml:992 | REAL | baseline orchrun-test-workspace-nofailfast.log.txt:5436-5467 = exactly 32 pgs tests (4 palette + 8 rle + 9 segment + 11 mod), all ok |
| "fuzz_target = fuzz_sub_pgs (in-crate survival harness)" | docs/formats/sub-pgs.md:16-18 | PAPER-ONLY (registry) | in-crate harness + integration suite exist, but no `fuzz_sub_pgs` cargo-fuzz target is registered: fuzz/fuzz_targets/ has 35 targets, sub-targets only srt/ass/vtt |
| "The renderer overlay contract is still missing (R-27)" | docs/formats/sub-pgs.md:35-39 | REAL (honest) | matches the UNREACHABLE verdict: reduce-to-cue exists, nothing renders it in a shipped path |
| "Reduce to compositing::CompositedCue — the 5.L1 compositing contract" | docs/formats/sub-pgs.md:35 | REAL | PgsCueBuilder::cues implemented + tested (baseline 5457) |
| tests "HONEST LABEL … against the ffmpeg pgssubdec.c semantics fetched 2026-10-25" | mod.rs:683-687 | UNVERIFIED | no ffmpeg source or fetch artifact in the tree; no network in sandbox; the honest part (no reference encoder exists) is credible, the "fetched/cross-checked" part is not reproducible |

## 7. Dead / duplicated / unreachable code
- Dead `Rational::new(..).unwrap_or(Rational::ZERO)` arms in `layout_for` (den never 0 after `max(1)`) — F-002.
- Dead zero-dimension guard in `cues()` — F-004.
- `Anchor::BOTTOM_CENTER` fallback duplicates the broadcast default for a format that should never use it — F-002; same pattern in dvb/mod.rs:933.
- Whole part (like the whole crate): unreachable from shipped entry points.

## 8. Dependencies & flags
- Uses crate-internal `compositing::{bitmap,layout,target}` + `media-types::time` — consistent with the crate's single runtime dep.
- `testkit::Must` dev-only in tests ✓. No feature flags. No unsafe.

## 9. Open questions for the next agent
- A06 (part1): `compose()` line 557 indexes `palette.rgba[index as usize]` — confirm `rle::decode_object` bounds indices to ≤255 (a 16-bit long-run color byte could exceed 255 if unmasked → panic path; rle.rs is yours).
- A06 (part1): the "palette-update PCS" comment (408-410) says the reference does not implement palette updates — verify the chosen semantics (ignore the flag) is what ffmpeg does.
- A09 (pgs/fuzz.rs): does the harness assert anything about cues()' spans? pgs_fuzz_survival.rs:233 has the ticks-vs-ns mix-up (F-003) — same check needed in the fuzz harness call sites.
- Whoever owns compositing/layout.rs after me (file is W09-A05 too — F-001 stands): consider `SafeMargin::Exact` before any renderer wiring lands (R-27).
- W18/W19: matrix row subtitle-pgs says vlc_modules = ["avcodec"] — ffmpeg's pgssub decoder lives in libavcodec, fine, but the row's "implemented" remains library-only; record sub-pgs.md is honest about R-27 — keep them consistent.
