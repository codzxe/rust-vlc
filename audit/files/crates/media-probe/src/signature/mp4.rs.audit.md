# AUDIT: crates/media-probe/src/signature/mp4.rs
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-23 |
| Lines / functions | 624 / 10 production + 10 tests (17 test builders/helpers) |
| File verdict | REAL (one wrong codec mapping contradicting the repo's own demuxer; otherwise solid) |
| Reachable from shipped entry point? | YES: `lumen` CLI open → media-probe `probe_stream` → `ProbeRegistry::with_builtins` (signature/mod.rs registers `Mp4Signature`); web app `apps/web/src/open.rs:230-232` probes with the same registry |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 4 |
| Commands run | `cargo test -p media-probe` → 80 lib + 8 doctests passed / 1 ignored; standalone recursion probe (audit/scratch/probe4_mp4_stbl.rs) executed at depths 100…131,072 |

## 1. One-paragraph verdict
The MP4 signature is a genuinely implemented, tightly bounded shallow `ftyp`/`moov` walker with a disciplined scoring ladder (50/65/70/80/90/95) and unusually good adversarial tests — including a byte-wise truncation sweep asserting only sane rungs and a stsd entry-count-lie test. All bounds math is `saturating_add`/window-clamped; I found no panic path (the nested-`stbl` recursion was my hostile candidate — executed at 131,072 depth, no overflow on x86). The one real defect is a fabricated codec mapping: `'twos'` is hinted `pcm_s16le`, but `'twos'` is big-endian PCM — the repo's own MP4 demuxer maps `b"twos" | b"lpcm" | b"raw " => "pcm_s16be"` (demux/src/mp4/mod.rs:455-456), so the probe and the demuxer disagree, and the probe violates its own stated policy ("Only pairs with in-repo ground truth produce hints" — no `twos` fixture exists). Today no shipped code consumes `track_hints`, so the wrong name is latent rather than user-visible; it is still wrong data in a public API.

## 2. What actually works (verified)
- `cargo test -p media-probe` → `80 passed; 0 failed` + `8 passed` doctests (1 ignored, pre-existing baseline).
- Executed the nested-`stbl` hostile case directly against the built rlib (audit/scratch/probe4_mp4_stbl.rs): depths 100 / 8,192 / 32,768 / 65,536 / 131,072 (the 1 MiB-window worst case) all return a sane `score=90` hit — no panic, no overflow, on both -O and debug builds and under a 2 MB stack limit.
- Header/brand arithmetic audited: `compat_brands` range `(16..end-3).step_by(4)` can never slice past the window; `hdlr`/`stsd` payload reads are guarded by `box_end >= pos+20` / `pos+24` before slicing; `u32_be` call sites all guarded by `pos+8 <= end`.
- Extensions mirror (9 entries) matches `demux::mp4::EXTENSIONS` (mp4/mod.rs:138-140) and is pinned by tests/registry_roundtrip.rs:239-245 — the doc claim is REAL.

## 3. Findings
F-001 [S3]
Where: crates/media-probe/src/signature/mp4.rs:276-282
Code:
```
    let descriptor = if handler == b"vide" && matches!(entry, b"jpeg" | b"mjpa" | b"mjpb") {
        Some((MediaKind::Video, "mjpeg"))
    } else if handler == b"soun" && matches!(entry, b"sowt" | b"twos") {
        Some((MediaKind::Audio, "pcm_s16le"))
    } else {
        None
    };
```
What is wrong: `'twos'` is QuickTime 16-bit **big-endian** PCM (ffmpeg: `PCM_S16BE`); the repo's own demuxer says so — `b"twos" | b"lpcm" | b"raw " => Some("pcm_s16be")` (crates/demux/src/mp4/mod.rs:455-456). The probe hints `pcm_s16le` for the same fourcc: an internal contradiction where one side is simply wrong. It also violates this module's own policy at 267-271 ("Only pairs with in-repo ground truth produce hints — the fixture corpus's vide+jpeg and soun/sowt"): there is no `twos` fixture, so the mapping is invented. Why it matters: any future consumer of `TrackHint` that trusts the codec name (the field's entire purpose) would select a little-endian decoder for big-endian audio — byte-swapped garbage output. Today no shipped code reads `track_hints` (rg over apps/ and crates/: producers only), so the blast radius is latent. What correct looks like: `b"sowt" => "pcm_s16le"` only, or add `b"twos" => "pcm_s16be"` once that descriptor exists in the registry. Confidence: HIGH — Verification: read (both mappings cited, side by side); the demuxer's mapping is the repo's own ground truth.

F-002 [S4]
Where: crates/media-probe/src/signature/mp4.rs:255-259
Code:
```
        if kind == b"stbl"
            && let Some(found) = find_stsd_entry(prefix, pos + 8, box_end)
        {
            return Some(found);
        }
```
What is wrong: `find_stsd_entry` recurses into nested `stbl` boxes with no depth bound — the only ceiling is window bytes ÷ 8 (default 64 KiB → 8,192 levels; config ceiling 1 MiB → 131,072 levels). Executed: all reachable depths survive on x86 (optimized frames are tiny; 131,072 @ 2 MB stack OK; debug build 32,768 @ 2 MB OK), so this is NOT a live crash here — but the safety is emergent (window size × compiler frame size vs thread stack), not enforced; a wasm build (1 MB default stack, apps/web does ship media-probe) with a raised `max_lookahead` (config.rs allows 1 MiB) would overflow. `MAX_WALK_STEPS` bounds iterations *per call*, not depth. What correct looks like: a `depth` parameter with a small explicit cap (stbl nesting is 1 in every real file). Confidence: HIGH — Verification: ran (depths executed, no crash) + read (recursion unbounded by steps counter).

F-003 [S4]
Where: crates/media-probe/src/signature/mp4.rs:53
Code:
```
        if prefix.len() < 16 || &prefix[4..8] != b"ftyp" {
            return None;
        }
```
What is wrong: the anchor requires `ftyp` at byte 0. Real-world ISO files occasionally open with a `free`/`skip`/`wide` box before `ftyp` (old QuickTime tooling); those files score no MP4 hit at all and fall to extension-based selection. ffmpeg's probe searches for `ftyp` within the buffer. Deliberate ("anchored … at the window start") and consistent with the crate's anchored-scan doctrine — a documented-shape limitation, not a lie. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: crates/media-probe/src/signature/mp4.rs:103-105
Code:
```
            if size < 8 {
                break;
            }
```
What is wrong: `size == 0` (ISO "box extends to end of file") and `size == 1` (64-bit `largesize`) both terminate the walk — a size-0 `moov` (legal as the final box) scores 80 instead of 95 with children unreachable. Documented recorded limitation (module docs 24-27) with a dedicated test (`zero_size_box_terminates_the_walk`); real-world moov is always sized. Confidence: HIGH — Verification: read + test cited.

F-005 [S4]
Where: crates/media-probe/src/signature/mp4.rs:56-70
Code:
```
        let ftyp_size = u32_be(prefix, 0) as usize;
        if ftyp_size < 16 {
            return None;
        }
        let ftyp_end = ftyp_size.min(prefix.len());
```
What is wrong: nothing exploitable (the clamp is correct), but `prefix_consumed = ftyp_end` reports the *declared* ftyp extent even in the family-only case (score 50, below `min_score` 60, never selectable) — a no-hit report's consumed figure is wasted precision; cosmetic. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Mp4Signature::extensions | 42-48 | REAL | — | mirror pinned by round-trip suite |
| Mp4Signature::probe | 50-115 | REAL | S4 | anchored ftyp (F-003); ladder honest; all walks window-clamped |
| is_known_brand | 128-130 | REAL | — | 18-brand subset, additive-documented |
| compat_brands | 135-139 | REAL | — | 4-aligned, partial-brand-safe range |
| walk_moov | 147-169 | REAL | — | mvhd-flag-before-break honest (documented) |
| walk_trak | 174-198 | REAL | — | last-mdia-wins (fine) |
| walk_mdia | 202-227 | REAL | — | hdlr guard `box_end >= pos+20` correct |
| find_stsd_entry | 231-263 | REAL | S4 | entry-count lie guard (1..=16) good; unbounded recursion (F-002) |
| classify_track | 272-287 | REAL | S3 | `twos` mapping wrong (F-001) |
| u32_be | 290-292 | REAL | — | guarded call sites |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| recognized_major_brand_anchors_at_seventy | YES | exact score/consumed |
| short_or_foreign_windows_do_not_anchor | YES | 5 non-anchor shapes |
| brand_ladder_is_honest | YES | 65/50 rungs + version-field-not-a-brand trap |
| moov_and_mvhd_climb_the_ladder | YES | 90/80 rungs at exact cuts |
| full_moov_sniffs_both_tracks | YES | both hints + 95 |
| moov_after_mdat_is_still_found | YES | moov-at-end + window-limited honesty |
| broken_box_sizes_end_the_walk | YES | huge moov size clamped; sub-header lie stops walk |
| zero_size_box_terminates_the_walk | YES | pinned limitation |
| unmapped_sample_entries_stay_unhinted | YES | avc1/mp4a unhinted at 90 |
| stsd_entry_count_lies_are_ignored | YES | audio hint survives, video dropped |
| every_truncation_is_a_sane_verdict | YES | byte-wise sweep, rung whitelist + consumed ≤ cut |
| (nothing tests `twos`) | — | the wrong mapping (F-001) is exactly where coverage stops |
| (nothing tests nested stbl) | — | the recursion bound is untested (F-002) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "an oversize lie … cannot move the scan" (stszlie0) | mp4.rs:12-15 | REAL | all walks clamp `box_end` to window; broken_box_sizes test |
| "recognized major brand 70 … +stsd track hints 95" | mp4.rs:17-22 | REAL | ladder tests |
| "size == 0 and size < 8 end a walk in v0 — recorded limitation" | mp4.rs:24-27 | REAL | zero_size test |
| "Only pairs with in-repo ground truth produce hints" | mp4.rs:267-271 | FALSE | `twos` has no in-repo ground truth and is mapped anyway (F-001) |
| "Mirror of demux::mp4::EXTENSIONS … the round-trip suite pins the mirror" | mp4.rs:43-44 | REAL | registry_roundtrip.rs:239-245; tables byte-identical |

## 7. Dead / duplicated / unreachable code
- The MP4 box-walk logic (size/fourcc/children) is a third independent implementation in-repo (probe signature, demux::mp4, and the 2.A3 fixture writer) — acceptable layering (no crate edge by design), noted as duplication debt.
- `track_hints` output currently has zero consumers outside media-probe's own tests — the entire hint machinery (walk_trak/walk_mdia/find_stsd_entry/classify_track, ~120 loc) is produced-and-unused in shipped code (the demuxer re-derives tracks at open). Not dead (public API + tests), but unwired.

## 8. Dependencies & flags
- media-types only (MediaKind, TrackHint). No feature gates. Registered at DEFAULT_PRIORITY_BUILTIN in signature/mod.rs.

## 9. Open questions for the next agent
- F-001: confirm whether the engine will ever consume `TrackHint` for decoder selection (the demux_host pre-pass pattern suggests it might); if yes, fix `twos` before that lands.
- F-002: if `max_lookahead` is ever raised above 64 KiB in any product config, re-run the stack safety math for wasm (apps/web ships media-probe on wasm32 with ~1 MB stacks).
