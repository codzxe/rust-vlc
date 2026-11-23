# AUDIT: crates/media-probe/src/signature/avi.rs
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-02-13 |
| Lines / functions | 783 / 9 prod (15 tests + 10 helpers) |
| File verdict | REAL (with one platform-specific S1) |
| Reachable from shipped entry point? | YES — apps/cli/src/demux_host.rs:444-446 `ProbeRegistry::with_builtins()` → `probe_stream` → signature/mod.rs:132 `registry.register(Box::new(AviSignature))` → `probe()`; also apps/web/src/open.rs:232 (wasm32 lane) |
| Findings | S0: 0 S1: 1 S2: 0 S3: 0 S4: 1 |
| Commands run | `cargo test -p media-probe --lib` → 80 passed / 0 failed (exit 0; 15 avi tests); python3 u32-arithmetic simulation proving F-001; sed quote verification |

## 1. One-paragraph verdict
The probe is real and heavily tested: anchored `RIFF`+`AVI ` magic with a scored ladder,
window-clamped hdrl/strl sniffing, WAVEFORMATEXTENSIBLE GUID resolution agreeing with the
demuxer, and no invented codec names (XVID/mjpa/mjpb/MP3-tag stay unhinted — verified by
the tests I ran). But the loop-bound arithmetic that the module's own comment claims is
wasm32-safe is not: the saturating advance can park `pos` at `usize::MAX`, and the next
loop-condition evaluation `pos + 8` is a PLAIN add — on 32-bit targets it overflows
(debug: overflow panic; release: wraps to 7, passes the `<= end` check, then
`&prefix[usize::MAX..3]` panics on the always-checked slice bounds). A crafted ~40-byte
AVI prefix crashes the shipped web (wasm32) build's probe. The 64-bit test lane cannot
see this bug, which is exactly why all 15 tests pass.

## 2. What actually works (verified)
— `cargo test -p media-probe --lib` -> 80 passed, 0 failed; the 15 avi tests cover the
score ladder (70/80/90/95), non-anchoring foreign windows (`RIFX`, `WAVE`, `AVIX`, short),
lying-LIST-size clamping with surviving hints, zero-size-chunk no-stall, full truncation
sweep 0..=len, extensible-GUID PCM mapping, and unhinted unknown codecs.
— Anchor strictness (false-positive focus): `prefix.len() < 12 || prefix[0..4] != "RIFF"
|| prefix[8..12] != "AVI "` — no anchoring on RIFX/WAVE/AVIX/garbage (test
`short_or_foreign_windows_do_not_anchor` passes).

## 3. Findings
F-001 [S1]
Where: crates/media-probe/src/signature/avi.rs:104,124-127 (walk_hdrl) and :140,156-159 (walk_strl)
Code:
```
    while pos + 8 <= end && steps < MAX_WALK_STEPS {
        steps += 1;
        let fourcc = &prefix[pos..pos + 4];
```
...
```
        // RIFF chunks are WORD-aligned: header + size + one pad byte
        // when the payload is odd. A zero-size chunk still advances by
        // its header, so the walk cannot stall. Every advance is a
        // saturating chain: on 32-bit targets (wasm32) a lying size
        // near `u32::MAX` must saturate, never wrap (D-9).
        pos = pos
            .saturating_add(size)
            .saturating_add(8)
            .saturating_add(size & 1);
```
What is wrong: On wasm32 (usize = u32) a declared chunk size ≥ ~0xFFFFFFE0 saturates the
advance chain to `pos = usize::MAX (0xFFFFFFFF)`. The next loop-condition `pos + 8` is a
plain add: debug builds panic (overflow check); release builds wrap to 7, which is `<=
end`, so the body executes `&prefix[pos..pos + 4]` = `prefix[0xFFFFFFFF..3]` — a slice
index panic (bounds are always checked). The comment's claim "on 32-bit targets (wasm32)
a lying size near `u32::MAX` must saturate, never wrap (D-9)" is FALSE: saturating to
usize::MAX is itself the fault state because the guard is not checked/saturating
arithmetic. Proof (python3 u32 simulation): pos after advance = 0xFFFFFFFF;
`pos + 8` wrapped = 7 <= end(200) → True; slice [4294967295..3] on a 200-byte prefix →
PANIC. Why it matters: the probe consumes untrusted file bytes in the shipped web build
(apps/web/src/open.rs:232 `probe_stream`; apps/web/Cargo.toml:63 depends on media-probe;
baseline notes the wasm lane builds and browser-load passes) — a ~40-byte crafted
"RIFF....AVI LIST....hdrl <chunk with size 0xFFFFFFFF>" prefix is a remotely triggerable
panic/DoS on that platform. On 64-bit lanes (CLI/desktop) `pos` peaks at ~4.3e9, the
condition is false, the loop exits cleanly — no panic, and the host test suite can never
catch this. Identical pattern in `walk_strl` (lines 140, 156-159). What correct looks
like: `while pos.checked_add(8).is_some_and(|next| next <= end)` or clamp the advance
`pos = pos.min(end)` after the saturating chain (which also makes `pos.min(end)` at the
return redundant). Confidence: HIGH — Verification: read + arithmetic simulated
(python3); wasm32 execution itself UNVERIFIED (no wasm test lane run — cargo-build-budget;
the wasm32 usize semantics are definitional, not environmental).

F-002 [S4]
Where: crates/media-probe/src/signature/avi.rs:75-81
Code:
```
            if saw_avih {
                score = SCORE_PRIMARY_HEADER;
            }
            hints = walk_hints;
            if !hints.is_empty() {
                score = SCORE_TRACK_HINTS;
            }
```
What is wrong: The documented ladder (module doc :16-17 "magic 70, +`LIST hdrl` 80,
+`avih` 90, +track hints 95") implies cumulative rungs, but a header whose `strl` sniff
yields hints while `avih` is absent scores 95 — the top rung — without the primary
header ever being confirmed. Blast radius: probe score only (registry ranking), no crash.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| AviSignature::extensions | 50-55 | REAL | — | `["avi"]` |
| AviSignature::probe | 57-91 | REAL | — | anchor + ladder; `end` clamp at :72-73 is itself safe on wasm32 (saturating + min) |
| walk_hdrl | 99-130 | REAL | S1 | F-001: condition `pos + 8` plain add vs saturating advance |
| walk_strl | 135-162 | REAL | S1 | F-001: identical pattern |
| sniff_wave | 175-197 | REAL | — | ≥16 bytes for plain, ≥40 for extensible; GUID at fixed offset 24; unresolved → None |
| classify_stream | 212-233 | REAL | — | only vids+MJPG / auds+PCM hint; everything else unhinted |
| pcm_descriptor | 238-248 | REAL | — | 6-row table, pinned by test |
| u32_le / u16_le | 251-258 | REAL | — | call sites all guarded by prior length checks in this file |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| magic_only_anchors_at_seventy | YES | exact score/consumed asserts |
| short_or_foreign_windows_do_not_anchor | YES | 8 negative windows incl. AVIX, RIFX, WAVE |
| hdrl_list_confirms_structure / avih_confirms_primary_header | YES | rung-by-rung score asserts at exact cut lengths |
| full_header_sniffs_both_tracks | YES | hint identity + prefix_consumed == hdrl extent |
| lying_list_size_clamps... | YES | corrupted size byte, clamped-consumed + surviving hints |
| strh_without_strf / unknown_video_handlers / field_based_mjpeg | YES | no-hint asserts (stage agreement) |
| pcm_width_table / extensible_strf_hints / unresolved_extensible | YES | full tag×bits matrix + GUID mapping + negative GUIDs |
| zero_size_chunks_cannot_stall_the_walk | YES | terminates with score 90 (would hang/fail on a stall) |
| every_truncation_is_a_sane_verdict | YES | all cuts, rung-only scores, consumed <= cut |
| all_descriptor_rows_are_valid_hints | YES | table rows construct valid hints |
| (gap) | — | No test constructs a saturating-size chunk; none CAN catch F-001 on a 64-bit host — the bug is invisible to this suite |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Sizes are never trusted into allocations — every walk clamps to the window and stops on sizes that stall or explode" | :10-14 | REAL | no allocation from sizes; `end` clamped :73; steps guard 256 |
| "on 32-bit targets (wasm32) a lying size near `u32::MAX` must saturate, never wrap (D-9)" | :122-123 | FALSE | F-001 — the advance saturates but the loop guard wraps/panics |
| "RIFF forms other than `AVI ` (e.g. `WAVE`) do not anchor" | :17-19 | REAL | anchor check :60 + negative test passes |
| Ladder "+`avih` 90, +track hints 95" cumulative | :16-17 | PARTIAL | F-002 — hints can score 95 without avih |

## 7. Dead / duplicated / unreachable code
None. All 9 functions are live; test helpers mirror the xtask writer (documented as such,
line 272-274).

## 8. Dependencies & flags
media-types only. No feature gates. wasm32-relevant constants (MAX_WALK_STEPS=256,
signature/mod.rs:72) verified.

## 9. Open questions for the next agent
- Check whether the wasm lane's probe entry (`apps/web/src/open.rs:232`) wraps panics
  into JS errors or aborts the module (blast-radius refinement for F-001).
- CONFIRMED SAME PATTERN OUTSIDE THIS FILE: src/signature/mp4.rs uses the identical
  guard/advance pair — `while pos + 8 <= end` (:152,179,207,234) with
  `pos = pos.saturating_add(size)` (:166) and `size = u32_be(prefix, pos) as usize`
  (:154); the `if size < 8 { break; }` guard does NOT stop a size of 0xFFFFFFFF, so
  the same wasm32 saturation panic applies to the MP4 signature. Whoever audits mp4.rs
  should inherit F-001.
