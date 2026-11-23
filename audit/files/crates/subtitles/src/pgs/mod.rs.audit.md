# AUDIT: crates/subtitles/src/pgs/mod.rs
| Field | Value |
|---|---|
| Auditor | W09-A04 |
| Date | 2026-11-23 (repo frozen; audit session 2025-07-14 clock) |
| Lines / functions | 1092 / 19 production fns + 11 tests + 6 test helpers |
| File verdict | UNREACHABLE (genuine, tested code; no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO — see §3 F-002 |
| Findings | S0: 0 S1: 1 S2: 0 S3: 3 S4: 3 |
| Commands run | `./target/debug/deps/subtitles-70f89de07410626f pgs::` → 32 passed, 0 failed (exit 0); `./target/debug/deps/pgs_fuzz_survival-40c8df5a95c36609` → 2 passed (exit 0); sed quote verifications (exit 0) |

## 1. One-paragraph verdict
The epoch state machine, segment dispatch, multi-fragment ODS assembly, and cue
rendering are real, panic-free (fuzz-verified), and covered by 11 honest unit
tests that assert pixel values and damage variants — I ran them. But the module
docs promise "never unbounded allocation" while `compose()` multiplies an
uncapped, dedup-blind PCS object-ref count (≤255, straight from a header byte)
by the 16 MiB worst-case RGBA canvas: a ~2 KiB display set can retain ~4 GiB.
Worse: nothing in any shipped binary can reach this file — `subtitles` is not a
dependency of `lumen-cli`, `desktop`, or `web`, so 1092 loc of PGS core are
dead weight compiled only by `cargo test`.

## 2. What actually works (verified)
— `./target/debug/deps/subtitles-70f89de07410626f pgs::` → `test result: ok. 32
passed; 0 failed` (covers all 11 tests in this file + palette/rle/segment).
— `./target/debug/deps/pgs_fuzz_survival-40c8df5a95c36609` → 2 passed
(seeded mutation + truncation sweep never panic, damage stays typed/bounded).
— Baseline cross-check: audit/runs/baseline/cargo-test-workspace.txt:5927-5937
(all pgs::tests ok) and :6078-6086 (pgs_fuzz_survival ok).

## 3. Findings

F-001 [S1]
Where: crates/subtitles/src/pgs/mod.rs:545-559 (with segment.rs:250 and rle.rs:32)
Code:
```
        for comp in &pcs.objects {
            let Some(object) = self.objects.get(&comp.id).cloned() else {
                self.damage(PgsDamage::ObjectMissing { object_id: comp.id });
                continue;
            };
            let (indices, rle_damage) =
                rle::decode_object(&object.rle, object.width, object.height);
```
(and)
```
            let mut rgba = Vec::with_capacity(indices.len() * 4);
```
What is wrong: `pcs.objects` is not deduplicated or capped — `parse_pcs` builds
one entry per reference from `data[10]` (≤255, segment.rs:250-253), and every
reference re-decodes the object and pushes its own RGBA copy into
`composed_objects`. Each canvas is capped at MAX_OBJECT_PIXELS = 4·1024·1024
(rle.rs:32), so one image is ≤16 MiB — but a PCS referencing the same object id
255 times (2051 bytes of PCS payload) plus one 2048×2048 ODS (~800 RLE bytes:
256 × 3-byte long runs) makes compose() allocate and RETAIN 255 × 16 MiB ≈
3.98 GiB, plus 255 sequential full-canvas decodes (~1 Gi pixel writes). The
doc's "Never … unbounded allocation" contract (mod.rs:55-59) is false in the
composition dimension: the caps it cites (canvas, 8 palettes, 64 objects) never
bound reference count. Why it matters: on a 3 GB machine this is an OOM kill
from a ~3 KiB subtitle track payload — a header-driven allocation bomb of
>1,000,000× amplification, precisely the class the module claims to exclude.
Today it is latent (crate unreachable, F-002); the moment a Matroska
S_HDMV/PGS track is wired to PgsDecoder it becomes remotely triggerable by any
malformed .mkv. What correct looks like: cap `pcs.objects` (reference cap, e.g.
64, or dedupe by id and re-place), and/or cap total composed RGBA bytes per
display set with a typed EpochBankFull-style damage event.
Confidence: HIGH — Verification: read (arithmetic: 255 × 4,194,304 px × 4 B =
4,278,190,080 B retained; decode loop per ref). OOM not executed
(cargo-build-budget).

F-002 [S3 — reachability; escalated separately as a product-level gap]
Where: apps/cli/Cargo.toml:47-63, apps/desktop/Cargo.toml:33-43, apps/web/Cargo.toml:54-68, root Cargo.toml:41
Code:
```
members = ["crates/*", "apps/*", "tools/xtask", "tests/integration/first-vertical", "tools/mem-audit", "tools/sweep"]
```
What is wrong: no shipped crate depends on `subtitles`. `rg 'subtitles'
--glob '**/Cargo.toml'` matches only crates/subtitles/Cargo.toml itself and
fuzz/Cargo.toml (fuzz/ is NOT in the members list). lumen-cli deps
(apps/cli/Cargo.toml:48-63) list demux/codecs/playback-engine/etc. — no
subtitles; desktop depends on lumen-cli + commands + renderer-wgpu; web
likewise. So PgsDecoder has zero call sites outside its own crate's tests/fuzz
harnesses. The PGS "core" is library-only. Why it matters: any matrix/doc row
claiming PGS playback in a shipped Lumen build is false-by-construction; 1092
loc of decoding logic ship as dead code. What correct looks like: wire
S_HDMV/PGS in the demuxer/playback path or mark the format UNSHIPPED in the
matrix. (Docs row docs/formats/sub-pgs.md is W19-A06's to audit; flag raised.)
Confidence: HIGH — Verification: read + Grep over every Cargo.toml.

F-003 [S3]
Where: crates/subtitles/src/pgs/mod.rs:609-612
Code:
```
        let end = match (next_pts_ns, start) {
            (Some(next), Some(start)) if next > start => Some(next - 1),
            _ => None,
        };
```
What is wrong: when the next display set has the SAME PTS as this one
(`next == start`, a legal zero-duration replacement — e.g. two sets at PTS 0)
the guard fails and the cue becomes open-ended (`end = None`), i.e. it never
expires, even though a successor set exists that should hide it. A subtitle
that should live 0 ns lives until end of stream. Why it matters: wrong
behavior in a realistic edge (back-to-back sets at identical timestamps);
PGS streams start at PTS 0 routinely. What correct looks like: emit a
zero-length span (start..start) or clamp to `next` with damage, not open-ended.
Confidence: MEDIUM — Verification: read (branch logic; no runtime repro —
cargo-build-budget).

F-004 [S3]
Where: crates/subtitles/src/pgs/mod.rs:655-660
Code:
```
    fn layout_for(video: (u32, u32), x: u16, y: u16) -> CueLayout {
        let (w, h) = (video.0.max(1), video.1.max(1));
        let u = Rational::new(u32::from(x), w).unwrap_or(Rational::ZERO);
        let v = Rational::new(u32::from(y), h).unwrap_or(Rational::ZERO);
        CueLayout {
            anchor: Anchor::new(u, v).unwrap_or(Anchor::BOTTOM_CENTER),
```
What is wrong: a PCS placing an object beyond the video extent (x=20000 on a
1920-wide video — 16-bit fields allow it) yields u>1; `Anchor::new` rejects
num>den (layout.rs:98-107) and the `unwrap_or` silently relocates the object to
BOTTOM_CENTER (centered, 90% down) with NO damage event. The module's own
contract says every malformation is typed; this one is swallowed and the
subtitle is silently teleported. Why it matters: hostile PCS gets a
predictable-but-wrong render instead of a typed skip; a reviewer grepping
damage ledgers will see "clean" for garbage input. What correct looks like:
damage event (e.g. PositionOutOfRange) and skip, like ObjectMissing.
Confidence: HIGH — Verification: read (Anchor::new bounds check cited).

F-005 [S4]
Where: crates/subtitles/src/pgs/mod.rs:239-241, 406, 415
Code:
```
    /// The epoch's windows (tracked for the state machine; the
    /// reference places objects by their PCS x/y directly).
    windows: Vec<segment::Window>,
```
What is wrong: `windows` is written (L415 `self.windows =
segment::parse_wds(seg.data);`) and cleared (L406) but never read anywhere —
write-only state, i.e. dead code the struct clones on every `compose()`
(`self.pcs.clone()` copies the decoder; `PgsDecoder: Clone` duplicates it).
What correct looks like: drop the field or actually validate object placement
against window bounds. Confidence: HIGH — Verification: read + rg "windows"
(3 hits: decl/clear/assign; zero reads).

F-006 [S4]
Where: crates/subtitles/src/pgs/mod.rs:334-344
Code:
```
        let (segments, used) = segment::walk_segments(payload, framing);
        if payload.is_empty() || used < payload.len() {
            // The walk stopped early (or never started): either a
            // bare header remnant or a size overrun past the last
            // whole segment.
            if segments.is_empty() {
                self.damage(PgsDamage::TruncatedSegment { kind: None });
            } else {
                self.damage(PgsDamage::LengthOverrun);
```
What is wrong: a payload ending in a 1–2 byte remnant (walk stops because
remaining < header_len, with earlier segments present) is reported as
`LengthOverrun` ("segment size overruns the payload") — but nothing overran;
the true condition is a truncated segment header. Misleading damage taxonomy,
no behavioral harm. Confidence: HIGH — Verification: read (walk_segments
header_len guard, segment.rs:122).

F-007 [S4]
Where: crates/subtitles/src/pgs/mod.rs:327-332, 385-391
Code:
```
        self.set_pts_ns = pts_90khz.and_then(|ticks| {
            Timestamp::from_parts(ticks, 1, 90_000)
                .ok()
                .and_then(|ts| ts.as_nanos().ok())
                .map(|ns| ns.0 as i64)
        });
```
What is wrong: an out-of-domain container PTS (|ticks| beyond Timestamp's
checked range) is silently converted to `None` — no damage event, the display
set reports "no PTS" and the cue builder makes it open-ended at the front
(Stamp::Missing). A hostile timestamp should be typed like every other
malformation. Practical risk is low (overflow needs ~266 years of 90 kHz
ticks). Confidence: HIGH — Verification: read (media-types/src/time.rs:155-157
checked conversion).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Display for PgsDamage::fmt` | 131-176 | REAL | — | 13 arms, matches enum; verified by test damage prints |
| `PgsDisplaySet::is_clean` | 216-219 | REAL | — | damage.is_empty(); trivially correct |
| `Default for PgsDecoder` | 257-261 | REAL | — | delegates to new() |
| `PgsDecoder::new` | 265-281 | REAL | — | zeroed state |
| `PgsDecoder::reset` | 286-288 | REAL | — | full re-init; no callers outside tests/fuzz (unreachable crate) |
| `PgsDecoder::damage` (private) | 290-296 | REAL | — | cap 64 + dropped counter; test damage_ledger_is_capped |
| `PgsDecoder::decode_display_set` | 302-308 | REAL | — | framing detect-once, sticky; sup test verifies |
| `PgsDecoder::set_framing` | 311-313 | REAL | — | no external callers in tree (API for future carriage) |
| `PgsDecoder::framing` | 316-318 | REAL | — | getter, used by test L793 |
| `PgsDecoder::decode_framed` | 320-380 | REAL | S4 | dispatch + missing-END fallback; F-006 mislabel; damage ledger take/reset correct |
| `PgsDecoder::pcs_segment` | 382-412 | REAL | — | PTS fallback + epoch flush on non-Normal; test-verified |
| `PgsDecoder::wds_segment` | 414-416 | REAL | S4 | writes dead `windows` field (F-005) |
| `PgsDecoder::pds_segment` | 418-446 | REAL | — | version no-op/replace, bank cap 8, matrix from current PCS height; matrix is None→601 if PDS precedes first PCS (malformed-only) |
| `PgsDecoder::ods_segment` | 448-517 | REAL | — | size cap w*h≤4Mi px (L469), bank cap 64, fragment overrun clipping saturating-safe; `remaining - frag` can't underflow (frag=min) |
| `PgsDecoder::compose` | 521-571 | REAL | S1 | F-001 ref-count amplification; palette 256-entry lookup is bounds-safe |
| `Default/PgsCueBuilder::new` | 583-593 | REAL | — | unit struct |
| `PgsCueBuilder::cues` | 601-649 | REAL | S3 | F-003 same-PTS open-ended; z saturating_add only at u16 edge |
| `PgsCueBuilder::layout_for` | 655-668 | REAL | S3 | F-004 silent BOTTOM_CENTER fallback |
| `TRANSPARENT_BLACK` const | 674 | REAL | — | used by palette.rs default fill |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| epoch_lifecycle_full_display_set | YES | asserts exact white pixels (235,128,128→255s), geometry, framing |
| epoch_normal_reuses_objects_palette_update | YES | asserts palette version replace semantics (entry1 transparent, entry2 black) + pts_ns 1e9 |
| empty_composition_clears_and_epoch_start_flushes | YES | asserts flush → PaletteMissing after epoch start |
| multi_fragment_ods_assembles | YES | 3-fragment assembly, exact rgba len 16 |
| palette_missing_drops_set_object_missing_skips | YES | asserts drop vs skip distinction |
| truncated_and_unknown_segments_are_typed | YES | 3 damage variants incl. LengthOverrun |
| rle_overrun_and_short_fragment_are_typed | YES | clipped run still renders + FragmentWithoutHeader |
| invalid_object_size_is_typed | YES | 0x0 object rejected |
| sup_framing_uses_segment_pts_when_container_silent | YES | segment PTS fills in (1e9 ns) |
| cue_builder_spans_and_layout | YES | exact spans, anchor rationals 960/1920, open-ended variant |
| damage_ledger_is_capped | YES | asserts dropped:16 overflow marker |
All vectors are hand-crafted by the decoder's author against ffmpeg semantics
from memory — no differential oracle exists (admitted at L682-687 "HONEST
LABEL"). Tests prove internal consistency, not reference fidelity. Palette
constants (BT.709 column) are UNVERIFIED against real ffmpeg output.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Never … unbounded allocation (object canvases capped …)" | mod.rs:55-59 | FALSE (composition refs uncapped) | F-001: 255 refs × 16 MiB |
| "It never panics; every malformation … is typed" | mod.rs:51-53 | SHALLOW | fuzz survival passes (ran), but F-004 out-of-extent position and F-007 bad PTS are silent, not typed |
| "8 palettes / 64 objects per epoch — the reference caps" | mod.rs:58-59 | UNVERIFIED | no ffmpeg source in sandbox; cannot confirm the reference has these caps |
| "AcquisitionPoint and EpochContinue release them too (the reference flushes on every non-zero state)" | mod.rs:28-31 | REAL-UNPROVEN | matches ffmpeg pgssubdec flush_cache behavior as documented; no oracle run |
| "half-open [t0, t1) spans … one nanosecond before the next set's PTS" | mod.rs:42-44 | REAL | cues() L609-612 + test cue_builder_spans_and_layout (ran, ok) |

## 7. Dead / duplicated / unreachable code
- Whole module unreachable from shipped binaries (F-002).
- `windows` field: write-only (F-005).
- `set_framing`/`reset`: no in-tree callers outside the crate (future-carriage API).

## 8. Dependencies & flags
Crate deps: media-types (time types), testkit + media-clock dev-only. No
features, no unsafe, no I/O — matches the "std-only" contract in
crates/subtitles/Cargo.toml:27-30. No feature gates hide this code; it is
plainly compiled and tested but never linked into a product.

## 9. Open questions for the next agent
- W19-A06: does docs/formats/sub-pgs.md claim shipped PGS support? If yes → S0
  matrix fraud given F-002.
- Whoever audits demux/Matroska: is S_HDMV/PGS track handling present anywhere?
  (rg found none; a PGS track in an .mkv is presumably ignored/error at open.)
- Verify ffmpeg's real palette/object caps to close the L58-59 UNVERIFIED claim.
