# AUDIT: crates/media-probe/src/signature/mp4.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 624 / 11 (+12 in-file tests, 4 test helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → apps/cli/src/demux_host.rs:444-446 `media_probe::probe_stream(&registry,…)` with `ProbeRegistry::with_builtins()` → signature/mod.rs:133 `registry.register(Box::new(Mp4Signature), …)` → `Mp4Signature::probe`; the resulting format id drives containers/mod.rs:74 `"mp4" => mp4::open`) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | rg `Mp4Signature` (registered mod.rs:133) · rg `track_hints|TrackHint` across crates/ apps/ (consumed ONLY inside media-probe; CLI uses `report.format` alone, demux_host.rs:446) · rg `MAX_WALK_STEPS|SCORE_*` (mod.rs:72,91-96) · rg `EXTENSIONS` in demux/src/mp4/mod.rs:138-140 (mirror claim verified — identical 9 entries) · rg `min_score|from_score|no_demuxer` in lib.rs/registry (sub-minimum hit still named in the error — doc claim holds) · sed -n verification of every quote |

## 1. One-paragraph verdict
A real, registered, executed MP4/ISO-BMFF prober with a genuinely engineered scoring ladder (brand rungs 70/65/50, moov 80, mvhd 90, stsd hints 95), window-clamped box walks, saturating arithmetic throughout, and an unusually honest in-file test suite (exhaustive truncation ladder test, lying-size and lying-count cases) — all 11 tests ran green in the baseline media_probe suite (80 passed). Defects: `find_stsd_entry` self-recurses on nested `stbl` boxes with depth bounded only by window/8 — a hostile 64 KiB window drives ~8.2k stack frames and the accepted 1 MiB lookahead ceiling drives ~131k, a stack overflow; the `soun`/`twos` sample entry is mapped to `pcm_s16le` though `twos` is big-endian PCM (contradicting the file's own "only in-repo-ground-truth pairs" policy); and `prefix_consumed` under-reports bytes the top-level walk actually stepped over, skewing the registry's shorter-prefix tie-break.

## 2. What actually works (verified)
- Registered + executed: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3363-3374` — all 11 `signature::mp4::tests` ok; media_probe lib suite `test result: ok. 80 passed; 0 failed` (log line 3356 region).
- End-to-end probe→open lane executed: baseline INDEX.md §"MP4 SMOKE TEST" — `lumen open` on 4 sample MP4s produced `tracks 2 ready` / duration / MediaInfoReady (mp4-smoke-open-session.txt), proving the signature anchored real ffmpeg-written MP4s (ftyp/moov/mvhd/stsd walk found h264+aac tracks).
- Mirror claim verified: `demux::mp4::EXTENSIONS` (mp4/mod.rs:138-140) is byte-identical to this `extensions()` list.
- Score-ladder/registry interaction verified by read: sub-minimum (50) hits are still passed to `error::no_demuxer` (signature/mod.rs:203-208), so the doc claim "the no-hit error still names the family" is true.

## 3. Findings

### F-001 [S2]
Where: crates/media-probe/src/signature/mp4.rs:255-259 (recursion site), 219-223 (entry site)
Code:
```
        if kind == b"stbl"
            && let Some(found) = find_stsd_entry(prefix, pos + 8, box_end)
        {
            return Some(found);
        }
```
What is wrong: `find_stsd_entry` recurses into itself for every `stbl` child. `MAX_WALK_STEPS` (256, mod.rs:72) bounds only the `while` loop of each invocation, never the recursion depth. A hostile file whose `minf` contains a chain of nested `stbl` boxes (8 bytes each: size≥8 + "stbl") drives one recursive call per 8 window bytes: ~8,180 frames at the default 64 KiB window (config.rs:16-17), ~131,000 frames at the `MAX_LOOKAHEAD_CEILING` of 1 MiB (config.rs:19-22) that `ProbeConfig` accepts.
Why it matters: Stack exhaustion on attacker-controlled input through the shipped `lumen open` path (probe is step 2 of open, demux_host.rs:446). At the 1 MiB configured ceiling the overflow is certain (≈16 MB of frames on any stack); at the default 64 KiB it is ~1-2 MB of frames — within reach of a 2 MiB spawned-thread stack. Secondary: on 32-bit targets `pos.saturating_add(size)` can reach `usize::MAX`, after which `pos + 8 <= end` overflows (debug panic / release wrap) — 64-bit shipped targets don't hit this, but it is the same unbounded-walk root. No test covers nested `stbl` depth.
What correct looks like: Make the descent iterative with an explicit depth budget (e.g. `depth: u8` parameter, refuse past 2 — `minf > stbl > stsd` is the only legal shape per ISO 14496-12 §12), or thread a shared step counter through the walk family.
Confidence: HIGH on the unbounded structure (code-traced); MEDIUM on overflow-at-default-window (frame size and probe thread stack not executed — no cargo). Verification: read.

### F-002 [S3]
Where: crates/media-probe/src/signature/mp4.rs:278-279
Code:
```
    } else if handler == b"soun" && matches!(entry, b"sowt" | b"twos") {
        Some((MediaKind::Audio, "pcm_s16le"))
```
What is wrong: `twos` is QuickTime 16-bit **big-endian** PCM (the name is the byte-order statement; ffmpeg maps it to `pcm_s16be`); `sowt` is the little-endian twin. Mapping both to `pcm_s16le` mis-declares `twos` audio. It also contradicts the function's own policy comment (lines 267-271): "Only pairs with in-repo ground truth produce hints — the fixture corpus's `vide`+`jpeg` and `soun`+`sowt`" — `twos` has no in-repo ground truth, yet it is mapped.
Why it matters: The hint is advisory today (ProbeReport doc: "advisory; the demuxer owns truth", report.rs:275; the shipped CLI consumes only `report.format`, demux_host.rs:446 — verified by rg: nothing outside media-probe reads `track_hints`), so no current byte-swap. But the first consumer of the hint gets silently byte-swapped audio for `.mov` files, whose PCM tracks are commonly `twos`. Untested: no test exercises `twos`.
What correct looks like: Either drop `twos` from the match (the file's own additive-rows policy), or map it to a `pcm_s16be` descriptor when that codec exists.
Confidence: HIGH — Verification: read (fourcc semantics are QuickTime-documented; ffmpeg mapping is public ground truth; not executed here).

### F-003 [S4]
Where: crates/media-probe/src/signature/mp4.rs:100,113
Code:
```
                    consumed = consumed.max(moov_consumed);
```
```
            prefix_consumed: consumed,
```
What is wrong: `prefix_consumed` reports `max(ftyp_end, moov walk end)`, but the top-level loop (lines 83-107) steps over every box up to the window end (e.g. a trailing `mdat` header) — those bytes were inspected yet are not counted. The field's contract is "how many leading window bytes the signature actually inspected" (report.rs:257-258) and the registry tie-break is "shorter prefix wins" (mod.rs:215-219).
Why it matters: The metric under-reports, giving this signature an unearned advantage in same-score tie-breaks against a signature that honestly reports its full inspection; the observable contract ("always ≤ the configured max_lookahead … the depth bound made observable", report.rs:277-279) is skewed.
What correct looks like: Report the final `pos` the top-level walk reached (clamped), not just the moov-derived value.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Mp4Signature::extensions` | 42-48 | REAL | — | Mirror of demux::mp4::EXTENSIONS verified identical |
| `Mp4Signature::probe` | 50-115 | REAL | S4 | ftyp anchor at window start only (documented v0 posture); ftyp_end clamped; saturating adds; score ladder via `.max()` monotone; F-003 consumed metric |
| `KNOWN_BRANDS` + `is_known_brand` | 122-130 | REAL | — | 18 brands, common subset; unknown → family-only fallback (documented, tested `brand_ladder_is_honest`) |
| `compat_brands` | 135-139 | REAL | — | 4-aligned from byte 16; `end.saturating_sub(3)` guard; a lying oversize ftyp can scan past the real tag into following bytes (clamped to window; score inflation only 65-vs-70, both selectable — noted, not a finding) |
| `walk_moov` | 147-169 | REAL | — | mvhd fourcc confirms rung even with broken size (flag set before break — honest); step-bounded |
| `walk_trak` | 174-198 | REAL | — | Collects hdlr/entry only from mdia; last-writer-wins for handler/entry across multiple mdia boxes (multiple mdia per trak is illegal — fine) |
| `walk_mdia` | 202-227 | REAL | S2 | Entry point to F-001 recursion; hdlr offset pos+16..pos+20 correct (FullBox 4 + pre_defined 4); guard `box_end >= pos+20` keeps the slice in-bounds (box_end ≤ end ≤ prefix.len()) |
| `find_stsd_entry` | 231-263 | DEFECTIVE | S2 | F-001 unbounded self-recursion; stsd parse: version/flags 4 + count 4 + first entry fourcc, count sanity 1..=16, all offsets in-bounds-guarded |
| `classify_track` | 272-287 | DEFECTIVE | S3 | F-002 `twos`→pcm_s16le; `TrackHint::new(...).ok()` swallows an invalid-descriptor error into "no hint" (acceptable: static ASCII pairs) |
| `u32_be` | 290-292 | REAL | — | Panics on out-of-range `at`, but every caller guards `pos+8 <= end ≤ prefix.len()` — traced all five call sites |
| tests `box_bytes`/`fullbox`/`ftyp`/`hdlr`/`stsd`/`stbl`/`trak`/`moov`/`full_file`/`hint_kinds`/`find` | 307-392, 621-623 | REAL | — | Local builder mirrors the 2.A3 writer; independent of the code under test |

## 5. Test honesty (in-file `mod tests`, 12 tests)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `recognized_major_brand_anchors_at_seventy` (395-409) | YES | Pins score 70, format "mp4", consumed 28 on the exact fixture-shaped ftyp. Executed ok (baseline log 3369). |
| `short_or_foreign_windows_do_not_anchor` (412-422) | YES | Five non-anchor shapes incl. size<16, broken type, wrong box. ok (3370). |
| `brand_ladder_is_honest` (425-450) | YES | Compat-only 65, unknown 50, and the version-field-is-not-a-brand trap. ok (3363). |
| `moov_and_mvhd_climb_the_ladder` (453-472) | YES | Cut-window rungs 90/80. ok (3368). |
| `full_moov_sniffs_both_tracks` (475-494) | YES | Both hints `(Video,"mjpeg"),(Audio,"pcm_s16le")` and score 95, over partial and full file. ok (3366). |
| `moov_after_mdat_is_still_found` (497-524) | YES | moov-at-end within window found (95); pushed past window honestly stays 70. ok (3367). |
| `broken_box_sizes_end_the_walk` (527-550) | YES | Oversize moov lie clamps (still 95/2 hints); sub-header lie stops the walk (90, no hints). ok (3365). |
| `zero_size_box_terminates_the_walk` (553-562) | YES | size==0 → 80 rung only. ok (3374). |
| `unmapped_sample_entries_stay_unhinted` (565-579) | YES | avc1/mp4a produce no hints, score 90. ok (3371). Would NOT catch F-002 (twos is not among the tested entries). |
| `stsd_entry_count_lies_are_ignored` (582-601) | YES | 0x7FFFFFFF count → video hint dropped, audio intact. ok (3372). |
| `every_truncation_is_a_sane_verdict` (604-617) | YES | Exhaustive cut sweep: ladder-only scores, consumed ≤ cut. ok (3364). Strongest test in the file. |
| Gap | — | No nested-`stbl` depth test (F-001), no `twos` test (F-002), no test that a `free`/`skip` box between ftyp and moov is skipped (walk code handles it; untested). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "an oversize lie (the class-B `stszlie0` pattern … in `stsz`, a box the signature never opens) cannot move the scan" | mp4.rs:12-15 | REAL | `stsz` is only stepped over by size (find_stsd_entry loop, no stsz arm); tested indirectly by `broken_box_sizes_end_the_walk` |
| "recognized major brand 70, compatible only 65, family-only 50 — deliberately below the doc's default min_score = 60 … the no-hit error still names the family" | mp4.rs:17-21 | REAL | Constants mod.rs:91-93; DEFAULT_MIN_SCORE=60 config.rs:26-27; registry passes best hit to `error::no_demuxer` (mod.rs:203-208); `brand_ladder_is_honest` ok |
| "size == 0 … and size < 8 end a walk in v0 — the to-EOF extent is not followed; 64-bit largesize boxes are likewise not followed" | mp4.rs:22-27 | REAL | Breaks at 103-104/159-160/182-183/237-238; `u32_be` only (no largesize read); `zero_size_box_terminates_the_walk` ok |
| "Mirror of `demux::mp4::EXTENSIONS` … the round-trip suite pins the mirror" | mp4.rs:43-44 | REAL | demux/src/mp4/mod.rs:138-140 identical; `extension_lists_cover_their_fixtures` ok (baseline log 3420 region) |
| "Only pairs with in-repo ground truth produce hints — the fixture corpus's `vide`+`jpeg` and `soun`+`sowt`" | mp4.rs:267-271 | FALSE for `twos` | F-002: `twos` (and `mjpa`/`mjpb`) are mapped with no corpus ground truth |

## 7. Dead / duplicated / unreachable code
None found. All helpers called; the extension mirror duplication across crates is deliberate (layering, documented, round-trip-pinned).

## 8. Dependencies & flags
No feature flags, no unsafe, no external crates (media_types only). Window sizes and ceilings come from config.rs constants — resource-bound posture consistent with the docs' MP §12 claim.

## 9. Open questions for the next agent
- Does any shipped lane probe on a spawned thread with a 2 MiB stack (engine pipeline)? That decides whether F-001 at the default window is a live crash or only at ≥1 MiB configs.
- The AVI signature (avi.rs) reportedly has the same walk family — check whether it recurses unboundedly too (its `find_stsd_entry` twin is the `strl` walk).
- When a TrackHint consumer lands, F-002 becomes an audio byte-swap bug — re-rate then.
