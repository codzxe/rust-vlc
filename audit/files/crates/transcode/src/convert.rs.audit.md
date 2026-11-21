# AUDIT: crates/transcode/src/convert.rs (whole file)
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-21 |
| Lines / functions | 449 / 15 (5 prod fns + 1 const + 1 test helper + 7 test fns + 1 doc'd match arm) |
| File verdict | UNREACHABLE (real, correct, tested library code in a dependency-island crate: no crate or app in the workspace depends on `transcode`) |
| Reachable from shipped entry point? | NO. Exhaustive proof: `rg -n "transcode" --glob 'Cargo.toml' -l .` → only `crates/transcode/Cargo.toml` itself (no `[dependencies]` entry anywhere names it); `rg -ln "transcode::" --glob '*.rs'` outside `crates/transcode` → zero hits. The crate is a `crates/*` workspace member (compiled + tested by the gate) but nothing links it into `lumen` or `lumen-tauri`. Internal callers of this file: runner.rs:74,931-934 only. Feature matrix is honest: `ui-dialog-convert` = not_started ×4, evidence "" (docs/feature-matrix.toml:2913-2922, AC "no capability shown that is not implemented"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n "transcode" --glob 'Cargo.toml' -l .` (no dependents) · `rg -ln "transcode::" --glob '*.rs'` (zero external uses) · `rg -n -B4 -A14 "fn frames_in" crates/media-types/src/frame.rs` (the caller's whole-frame/zero-channel guard) · `sed -n '890,952p' crates/transcode/src/runner.rs` (the sole caller) · baseline `orchrun-test-workspace-nofailfast.log.txt:5817-5823` (all 7 convert::tests ok inside the transcode target's 50 passed, :5866) · hand-verified the s24 sign-extension, u8→s16/s24/s32 widenings, and endian-agnostic identity paths |

## 1. One-paragraph verdict
The conversion math is correct — I hand-traced every arm: the s24 sign-extension parks the payload at the top of i32 and arithmetic-shifts back down (0xFFFFFF → −1 → −256 after <<8, exactly what the test pins), the u8 widenings produce (u−128)<<{8,16,24} with both extremes in range, the identity paths are byte-exact on little-endian hosts and correct byte-swaps on big-endian ones, the planar→interleaved permutation indexes both layouts correctly, and the lossy pairs are refused typed with a pinned error code. The seven tests are all discriminating and ran green in the baseline run. The two defects are defensive gaps, not wrong math: `convert_to_wav` divides by `channels * width` without asserting `channels > 0` (the sole caller's `frames_in` check happens to guard it), and `emit`'s catch-all arm silently emits nothing for an illegal pair, outsourcing bug detection to a downstream length check. And the whole 449-line file — like the whole 6,527-line crate around it — is linked into nothing: a dependency island the matrix honestly marks not_started.

## 2. What actually works (verified)
— All 7 `convert::tests` green in baseline `orchrun-test-workspace-nofailfast.log.txt:5817-5823` (transcode lib target: 50 passed, 0 failed, `:5866`), release profile.
— Hand-verified `read_native`'s s24 path (134-146): little-endian host reads (lo, mid, hi) = (w[0], w[1], w[2]); big-endian reads (w[2], w[1], w[0]); `((value << 8) as i32) >> 8` sign-extends from bit 23 — the comment's warning about a plain `as i32` leaving the high byte unsigned is arithmetically right, and the test's `0xFF,0xFF,0xFF → −256` confirms the widened value.
— Hand-verified u8→s16/s24/s32 (191-202): u=0 → −32768/−8388608/−2147483648, u=255 → 32512/0x7F0000/0x7F000000 — every extreme fits the target width; the test pins (−32768, 0, 32512) for s16.
— Hand-verified the endian-agnostic identity: `from_ne_bytes(...).to_le_bytes()` is a no-op on LE and a full swap on BE; `identity_passthrough` correctly requires `HOST_IS_LITTLE` so a BE host always converts (122-126); the passthrough test asserts `== HOST_IS_LITTLE` so it is meaningful on both hosts.
— Verified the scratch-reuse policy empirically claimed by the test: `scratch_is_reused_not_accumulated` (428-439) asserts stable capacity across 5 conversions — the D-1 allocation policy is pinned, not just documented.
— Verified the sole caller's guard chain: runner.rs:923-926 requires `format.frames_in(samples.len()) == Some(_)` (media-types frame.rs:77-83 returns `None` when `frame_bytes() == 0` — i.e. channels == 0 — or on a ragged payload) BEFORE calling `convert_to_wav`, so the module precondition holds at the one call site that exists.

## 3. Findings

### F-001 [S4]
Where: crates/transcode/src/convert.rs:238-240
Code:
```
    let channels = format.channels as usize;
    let width = format.sample.bytes_per_sample() as usize;
    let frames = samples.len() / (channels * width);
```
What is wrong: `channels * width` is a divisor with no zero guard in this function. `width` is always ≥ 1 (every `SampleFormat` has bytes), but `channels == 0` makes this a division-by-zero panic. The doc contract (229-231) states the caller verified "a whole number of sample frames and the pair is legal" — it does not state `channels > 0`, and the guard that actually prevents the panic lives in ANOTHER crate's method (`AudioFormat::frames_in`, media-types frame.rs:79, returns `None` when `frame_bytes() == 0`) invoked by the sole caller (runner.rs:923-925) before this function runs.
Why it matters: the safety of a `pub(crate)` function depends on an invariant enforced one crate away; a second caller (or a refactor of the runner) that skips the `frames_in` check inherits a panic on a zero-channel `AudioFormat`. A `debug_assert!(channels > 0)` would localize the contract.
What correct looks like: `debug_assert!(channels > 0, "zero-channel audio format");` at function entry (or a typed refusal), plus a doc line naming the precondition.
Confidence: HIGH — Verification: read (both the division and the caller's guard chain traced; no other caller exists).

### F-002 [S4]
Where: crates/transcode/src/convert.rs:218-222
Code:
```
        // Every other pair was refused by `conversion_legal` before
        // any byte moved; reaching here is a logic bug in this
        // module, and the honest answer is to skip nothing silently —
        // the writer's whole-frame check catches any length drift.
        _ => {}
```
What is wrong: the catch-all arm silently appends nothing for an (source, target) pair that escaped `conversion_legal`. The comment claims the writer's whole-frame check catches the resulting length drift — that is only true if the drift crosses a frame boundary; a skipped whole frame in a multi-frame payload yields a shorter-but-still-whole payload, which a length-multiple check accepts. The module's own doctrine (line 18: "a transcode engine must never quantize silently") is exactly what this arm does on the logic-bug path.
Why it matters: a future edit that adds a legal pair to `conversion_legal` but forgets the `emit` arm (or vice versa) produces silently-shortened audio instead of a loud failure at the source.
What correct looks like: `unreachable!("conversion_legal admitted an unhandled pair")` (the arm is documented as a logic bug — make it fail like one), or at minimum `debug_assert!(false, …)`.
Confidence: HIGH — Verification: read (arm traced; the comment's own justification analyzed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `HOST_IS_LITTLE` (const) | 36-38 | REAL | — | `cfg!(target_endian)` — compile-time, never guessed. |
| `source_descriptor` | 44-54 | REAL | — | 6-way identity table; used by error minting (86-89). |
| `identity_target` | 57-67 | REAL | — | SampleFormat → PcmSample bijection. |
| `resolve_target` | 75-91 | REAL | — | Source-selection short-circuit; typed refusal with both descriptors; test-pinned code string. |
| `conversion_legal` | 95-116 | REAL | — | 13 legal pairs = 6 identity + 6 int widenings + 1 float widening; matches the doc matrix exactly. |
| `identity_passthrough` | 121-126 | REAL | — | requires LE + interleaved + identity kind. |
| `read_native` | 130-154 | REAL | — | s24 sign-extension hand-verified; F32/F64 return 0 (documented, unused). |
| `read_f32` | 157-159 | REAL | — | trivial. |
| `emit` | 164-224 | REAL | S4 (F-002) | 13 arms hand-verified; catch-all silently skips. |
| `convert_to_wav` | 232-255 | REAL | S4 (F-001) | planar/interleaved indexing correct; scratch clear+reserve (the `- scratch.len()` at 244 is a dead subtraction — len is 0 post-clear); integer division truncates ragged tails per the documented caller contract. |
| `tests::format` (helper) | 264-271 | REAL | — | 48 kHz fixture. |
| `target_resolution_matrix` | 273-310 | REAL | — | 6 identities + 4 widenings + 4 typed refusals with the exact code string. |
| `interleaved_identity_is_passthrough…` | 312-321 | REAL | — | endian-agnostic assertion (`== HOST_IS_LITTLE`). |
| `planar_to_interleaved_permutes_frame_major` | 323-344 | REAL | — | exact [L0 R0 L1 R1] permutation. |
| `widening_conversions_are_exact` | 346-402 | REAL | — | s16→s32 over [-32768, 32767]; u8→s16 over [0, 128, 255]; s24→s32 (−1 → −256); f32→f64 exact. |
| `native_identity_roundtrips_bytes` | 404-425 | REAL | — | i32::MIN/MAX through the identity; length equality pinned. |
| `scratch_is_reused_not_accumulated` | 427-439 | REAL | — | capacity stability across 5 calls. |
| `shared_bytes_deref_gives_the_payload_for_passthrough` | 441-448 | REAL | — | view-semantics pin for the zero-copy path. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| target_resolution_matrix | YES | exact PcmSample equality on both accept and refuse sides; the code string `transcode.wav.sample-conversion` is pinned. |
| interleaved_identity_is_passthrough… | YES | passthrough boolean pinned against the compile-time endianness; planar and non-identity negatives pinned. |
| planar_to_interleaved_permutes_frame_major | YES | exact 4-sample order — any indexing bug permutes wrongly. |
| widening_conversions_are_exact | YES | exact widened values at both extremes of every widened width. |
| native_identity_roundtrips_bytes | YES | i32::MIN/−1/0/MAX round-trip + length equality (a swap bug on LE hosts would flip values). |
| scratch_is_reused_not_accumulated | YES | capacity equality across calls — a re-allocating implementation fails. |
| shared_bytes_deref… | YES (weakly) | pins Deref semantics the writer relies on; trivial but honest. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Two conversions are always legal and lossless" + "lossless widenings only" matrix | convert.rs:5-18 module doc | REAL | `conversion_legal`'s 13 pairs match the doc matrix 1:1; widening math hand-verified. |
| "a transcode engine must never quantize silently" | convert.rs:16-18 | REAL (refusals) / SHALLOW (logic-bug arm) | typed refusals pinned by test; `emit`'s catch-all skips silently (F-002). |
| "zero-copy passthrough … every other pair converts into a caller-owned scratch Vec … zero allocations after the first" | convert.rs:20-27 | REAL | identity_passthrough gate + runner.rs:931-943 take/return of the scratch; capacity pinned by test. |
| `ui-dialog-convert` not_started (the crate has no shipped surface) | docs/feature-matrix.toml:2913-2922 | REAL (honest) | no dependents (rg); the "no capability shown that is not implemented" AC is currently satisfied. |

## 7. Dead / duplicated / unreachable code
The entire crate is a dependency island (header proof). Within the file, no dead functions — all five prod fns are called (source_descriptor/resolve_target from error paths and the runner; conversion_legal from resolve_target; identity_passthrough/convert_to_wav from runner.rs:931-939).

## 8. Dependencies & flags
Depends on `demux::wav::format::PcmSample` (a demux-crate type used as the WAV vocabulary — a mild upward dependency of transcode on demux, consistent with the crate's decode-to-WAV purpose), `media_types` frame/error types, `crate::error`, `crate::graph::WavPcmTarget`. No feature gates; no unsafe; no panics outside F-001's guarded divisor. Tests green in baseline.

## 9. Open questions for the next agent
- A07 (job.rs) / A04 (runner.rs parts): does `WavPcmWriter::write_pcm` (wav.rs, A03's) actually reject a payload whose length is not a whole number of frames — the check F-002's comment leans on?
- Whoever owns product direction: the crate is 6,527 lines of tested, unlinked infrastructure; the matrix's phase_target = 7 UI (`ui-dialog-convert`) is the only prospective consumer. If phase 7 slips, this is the largest dead island in the repo — worth a wave-17 note.
