# AUDIT: crates/media-probe/src/signature/avi.rs
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-21 |
| Lines / functions | 783 / 36 (9 prod fns, 3 consts, 12 test helpers, 15 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → `apps/cli/src/demux_host.rs:444` `ProbeRegistry::with_builtins()` → `signature/mod.rs:132` registers `AviSignature` → `media_probe::probe_stream` (`lib.rs:203-210`) → `AviSignature::probe`) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 1 |
| Commands run | `rg -n "AviSignature" crates/ apps/ tests/` (callers: mod.rs:30,132; tests/registry_roundtrip.rs:21,244); `sed -n '104,129p'/'140,160p' avi.rs` (saturation chain quotes); `sed -n '750,830p' crates/demux/src/avi/header.rs` (demuxer 0xFFFE GUID resolution — actual site 787-812); `rg -n "MJPG\|mjpa\|mjpb" crates/demux/src/avi/` (codec filter header.rs:715); `rg -n "pcm_" header.rs` (naming vocabulary 151-156); `rg -n "MAX_WALK_STEPS\|SCORE_" signature/mod.rs` (consts at 72,91-96); baseline `orchrun-test-workspace-nofailfast.log.txt:3309-3348` (all 15 file tests ran ok) |

## 1. One-paragraph verdict
This is a genuine, executed, defensively-written probe signature: RIFF/`AVI ` anchor, bounded hdrl/strl walk with step ceiling (MAX_WALK_STEPS=256), size-clamped windows, zero allocations from attacker-controlled sizes, and a conservative hint vocabulary that demonstrably mirrors the demuxer (PCM table = `header.rs:151-156`; MJPG-only video filter = `header.rs:715`; extensible-GUID resolution = `header.rs:787-812`). All 15 tests executed and passed in the baseline run. The one real defect: the walk loop's *non-saturating* `pos + 8` condition defeats the file's own documented 32-bit saturation invariant (D-9) — on wasm32/32-bit targets a lying chunk size near `u32::MAX` saturates `pos` to `usize::MAX` and the next loop-condition add overflows (debug: add-overflow panic; release: wrap → slice-index panic). Shipped 64-bit builds cannot reach it (`u32::MAX as usize + 8` never overflows u64), which is the only reason this is S2, not S1.

## 2. What actually works (verified)
— All 15 tests in this file executed and passed: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:3309-3348` (`running 80 tests` for media_probe lib; every `signature::avi::tests::*` line reads `... ok`).
— Stage agreement with the demuxer verified in source: extensible GUID resolution (probe `sniff_wave` 0xFFFE→GUID@24 ↔ demux `header.rs:787-812`), PCM naming (`pcm_descriptor` ↔ `header.rs:151-156`), MJPG handler filter (`header.rs:715` `if handler.as_bytes() != b"MJPG"` skips mjpa/mjpb exactly as the probe refuses to hint them).
— Bounded-walk claims verified: `MAX_WALK_STEPS = 256` (mod.rs:72); no allocation anywhere takes a parsed size (hints `Vec` grows only per strl, ≤ steps).

## 3. Findings
### F-001 [S2]
Where: crates/media-probe/src/signature/avi.rs:104 and 140 (loop conditions), 124-127 and 156-159 (saturating advance)
Code:
```rust
    while pos + 8 <= end && steps < MAX_WALK_STEPS {
```
```rust
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
What is wrong: the advance chain saturates — but on a 32-bit target (`usize == u32`, the comment's own wasm32 case) `pos` saturates to `usize::MAX` when a chunk declares a size near `u32::MAX`, and the *next* evaluation of the loop condition `pos + 8 <= end` performs a non-saturating, non-checked add that overflows: debug builds panic on the add; release builds wrap to `7 <= end` (true, since `end ≥ 32` whenever the walk runs), and the body then executes `&prefix[pos..pos + 4]` with `pos = usize::MAX` → slice-index panic. The comment's invariant ("must saturate, never wrap") is true of the chain and false of the loop header that consumes its result. Identical pattern in `walk_strl` (lines 140, 156-159).
Why it matters: a ~40-byte crafted AVI prefix panics the prober on any 32-bit target. Today's shipped builds are 64-bit (baseline: x86_64 release CLI; `orchrun-platform-macos-crosscheck.log.txt` aarch64+x86_64 both EXIT=0), so it is latent — but the web column of the feature matrix is a claimed platform (parity report: web=0/252), `media-probe` is deliberately std-only for exactly that portability, and the file *documents* this precise case as handled. The fuzz-style test `builtin_signatures_never_panic_on_arbitrary_windows` (mod.rs:471-493) sweeps byte values on a 64-byte window but runs on 64-bit only, so it cannot catch this.
What correct looks like: make the loop condition overflow-proof: `while end.saturating_sub(pos) >= 8 && steps < MAX_WALK_STEPS` (or `pos.checked_add(8).is_some_and(|p| p <= end)`), and add a CI check/test on a 32-bit target (wasm32) before the web platform is claimed.
Confidence: HIGH — Verification: read (arithmetic traced for 64-bit — safe: `u32::MAX as usize + 8` cannot overflow u64 — and for wasm32 — panics); no 32-bit build exists to execute (UNVERIFIED by run, blocked: repo has no wasm32 target config and running builds is out of scope).

### F-002 [S4]
Where: crates/media-probe/src/signature/avi.rs:166 (and the same stale range in the test comment at 607)
Code:
```rust
/// exactly the demuxer's own resolution (`header.rs:756-776`): a
```
What is wrong: the cited line range is stale — the demuxer's WAVEFORMATEXTENSIBLE GUID resolution actually lives at `crates/demux/src/avi/header.rs:787-812` (verified with `sed -n '780,830p'`: `let effective_tag = if format_tag == WAVE_FORMAT_EXTENSIBLE { ... }`); the test comment at line 607 cites `header.rs:756-780`, a third stale variant. The *claim* itself is true; only the coordinates rot.
Why it matters: line-number citations in comments are how later auditors/waves verify stage agreement; stale ones send the reader to unrelated code (the `auds` strf field reads at 750-780) and erode trust in the surrounding commentary.
What correct looks like: cite the block by name (the "resolve effective WAVE format tag" arm of the auds handler in `header.rs`) or the current range, and fix both copies.
Confidence: HIGH — Verification: read (sed on header.rs 750-830).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `AviSignature::extensions` | 50-55 | REAL | — | `&["avi"]`, honest comment that no demux-side EXTENSIONS const exists. |
| `AviSignature::probe` | 57-91 | REAL | — | Anchor `RIFF`+`AVI ` at 0/8 (len<12 declines); `_meta` ignored by design (magic-first). `end = 20.saturating_add(declared).min(prefix.len())` (73) — lie clamps to window. Ladder 70/80/90/95 verified against mod.rs:91-96. No panic path on 64-bit. Reachable (see header). |
| `walk_hdrl` | 99-130 | REAL | S2 | Bounded steps ≤256; clamped payload ends; zero-size chunk advances 8 (no stall). F-001: `pos + 8` (104) overflows after a saturating advance on 32-bit. |
| `walk_strl` | 135-162 | REAL | S2 | Same structure; `strh` needs ≥8 payload bytes (146) before slicing fccType/fccHandler — bounds-checked. F-001 applies identically (140, 156-159). |
| `sniff_wave` | 175-197 | REAL | — | 16-byte WAVEFORMATEX minimum (178-180); 0xFFFE requires ≥40 physical bytes (186-188) and reads GUID at fixed offset 24 (189) — byte-identical to demuxer header.rs:787-801; unresolved GUID → None (stage agreement). |
| `classify_stream` | 212-233 | REAL | — | let-chains (edition 2024, toolchain 1.99 ok); only `vids`+`MJPG` and `auds`+known-PCM hint; `TrackHint::new(...).ok()` cannot swallow a real name (table-pinned by test). |
| `pcm_descriptor` | 238-248 | REAL | — | 6 rows; names byte-identical to demuxer vocabulary header.rs:151-156 (verified by rg). |
| `u32_le` / `u16_le` | 251-258 | REAL | — | Correct LE reads; callers pre-establish bounds. |
| consts `WAVE_FORMAT_EXTENSIBLE`/`SUBFORMAT_PCM_GUID`/`SUBFORMAT_IEEE_FLOAT_GUID` | 33-43 | REAL | — | 0xFFFE + both `{...89B71}` GUIDs match demuxer header.rs:361-375 (both files read). |
| Test helpers `ok/chunk/list/strh/strf_wave/strf_bitmap/full_header/probe/hint_kinds/w215_*` | 264-364, 548-601 | REAL | — | Faithful mini-writer: WORD-align pad (280-282), LIST size = form+children (296) pinned to the committed fixture arithmetic (4+64+124+100=292, extent 312 — recomputed, correct). |

## 5. Test honesty (15 tests, all executed ok in baseline :3309-3348)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| magic_only_anchors_at_seventy | YES | Asserts exact score 70 + consumed 12; mutation of anchor/score breaks it. |
| short_or_foreign_windows_do_not_anchor | YES | RIFX/WAVE/AVIX/short windows → None; a laxer anchor fails it. |
| hdrl_list_confirms_structure | YES | Exact rung 80 at the 24-byte boundary. |
| avih_confirms_primary_header | YES | Rung 90 at 32 and 52 bytes (class-B trunc1). |
| full_header_sniffs_both_tracks | YES | Asserts both hints' kinds/names and consumed == 312 == full.len(); a shrunk extent base (16 instead of 20) yields consumed 308 ≠ 312, and a grown base (24) overruns into the appended `movi` LIST in the second half of the test, also breaking `prefix_consumed == full.len()`. |
| lying_list_size_clamps_to_the_window_but_keeps_earlier_hints | YES | Corrupts size top byte; asserts clamp + surviving hints + consumed == window.len(). |
| strh_without_strf_yields_no_audio_hint | YES | No-wave → no hint, rung 90. |
| pcm_width_table_names_like_the_demuxer | YES | All 6 table rows end-to-end + MP3 tag 0x0055 unhinted. |
| extensible_strf_hints_the_pcm_family_like_the_demuxer | YES | 4 GUID×width cases; fails if GUID offset/resolution regresses (W2-15 d2 regression pin). |
| unresolved_extensible_forms_stay_unhinted_like_the_demuxer | YES | Truncated (30-byte) and foreign-GUID (MULAW) payloads → no hint. |
| unknown_video_handlers_stay_unhinted | YES | XVID → no invented codec. |
| field_based_mjpeg_variants_stay_unhinted | YES | mjpa/mjpb → no hint, score 90 — pins the header.rs:715 agreement. |
| zero_size_chunks_cannot_stall_the_walk | YES | JUNK size-0 chunk; run ends, score 90 (would hang/fail on a stall regression). |
| every_truncation_is_a_sane_verdict | YES | Exhaustive prefix sweep 0..=len: rung whitelist + consumed ≤ cut — catches any panic/mis-rung on truncations. |
| all_descriptor_rows_are_valid_hints | YES | Every table row constructs a valid TrackHint. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "magic 70, +LIST hdrl 80, +avih 90, +track hints 95" | module doc :16-17 | REAL | mod.rs:91-96 consts; tests pin each rung (baseline ok). |
| "RIFF forms other than `AVI ` (e.g. `WAVE`) do not anchor" | module doc :17-19 | REAL | probe():60-62 + short_or_foreign test. |
| "Sizes are never trusted into allocations" | module doc :11-12 | REAL | No size-derived allocation; window/step/clamp bounds only (read). |
| "the demuxer resolves the same GUID (`header.rs:756-776`)" | sniff_wave doc :166 | REAL (stale citation → F-002) | Actual resolution header.rs:787-812 (sed-verified); semantics identical. |
| "`mjpa`/`mjpb` deliberately stay unhinted (stage-agreement)" | classify_stream doc :207-211 | REAL | header.rs:715 `handler.as_bytes() != b"MJPG"` skip + demux test `mjpa_handler_is_filtered_like_any_unsupported_handler` (tests.rs:1065). |
| "45 of the current corpus draw's PCM streams are extensible" | sniff_wave doc :171-172 (W1-03 census) | UNVERIFIED | I did not count the corpus draw; no run cited. Not re-run (rule 8). |
| "on 32-bit targets (wasm32) a lying size near u32::MAX must saturate, never wrap (D-9)" | walk_hdrl comment :119-123 | **FALSE as an end-to-end guarantee** | The chain saturates but `pos + 8` (104) then overflows → panic on 32-bit (F-001). True on 64-bit only. |

## 7. Dead / duplicated / unreachable code
None. All 9 production functions are called (probe → walk_hdrl → walk_strl → sniff_wave/classify_stream → pcm_descriptor; u32_le/u16_le used throughout). Duplicated GUID/0xFFFE constants exist in demux header.rs:361-375 — intentional stage-mirroring across the crate boundary (media-probe is zero-dep by C5); noted, not actionable here.

## 8. Dependencies & flags
`media-types`, `plugin-api` (path deps); dev: testkit, input. No features touch this file (`contract` feature is src/contract.rs only). No unsafe, no unwrap/expect in production code — matches the crate Cargo.toml claim "No unsafe, no unwrap/expect/panic/todo" (tests use `unreachable!`, permitted).

## 9. Open questions for the next agent
- Does any shipped lane compile media-probe for a 32-bit target (web/tauri mobile)? If yes, F-001 becomes S1. The web matrix column is currently 0/252 (xtask-parity-report baseline) — resolve before web claims land.
- The W1-03 "45 extensible streams" census number is repeated twice in this file (:171, :559) with no committed artifact — worth a provenance check by the corpus-wave agent.
