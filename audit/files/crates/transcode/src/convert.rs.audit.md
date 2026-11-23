# AUDIT: crates/transcode/src/convert.rs
| Field | Value |
|---|---|
| Auditor | W10-A10 |
| Date | 2026-11-23 |
| Lines / functions | 449 / 7 production fns + 7 tests |
| File verdict | UNREACHABLE (real, correct conversion core — transcode crate island) |
| Reachable from shipped entry point? | NO — sole caller crates/transcode/src/runner.rs:934 (after its own whole-frame re-check at 921-925); no manifest depends on `transcode` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p transcode --lib convert::` -> 7 passed / 0 failed (exit 0); read of runner.rs seam; `sed -n` quote verification |

## 1. One-paragraph verdict
The conversion matrix is exactly what the docs claim and the arithmetic is right: identity plus lossless widenings only (u8→s16/s24/s32, s16→s24/s32, s24→s32 via value-preserving shifts with correct s24 sign-extension; f32→f64 exact), every lossy pair a typed `transcode.wav.sample-conversion` refusal, planar→interleaved as a correct frame-major permutation, byte-order normalization via from_ne/le pairs, and a caller-owned scratch Vec reused per frame. All 7 tests pass under my run, with boundary-value exactness (i16::MIN, u8 0/128/255, s24 −1). Like the rest of the crate it is unreachable from any shipped binary, the catch-all arm in `emit` would produce a silent empty file (not a caught drift) if the legality table and emit ever disagreed, and `convert_to_wav` itself divides by `channels*width` with no zero guard (the runner seam validates first).

## 2. What actually works (verified)
— verified by `cargo test -p transcode --lib convert::` -> `test result: ok. 7 passed; 0 failed` — matrix resolution (6 identities, 4 widenings ok; 4 lossy pairs refused with exact code), planar permutation [L0 R0 L1 R1], widening exactness incl. u8 silent-at-128 and s24 −1 → −256, native-identity byte roundtrip, scratch reuse with stable capacity.
— verified by read: runner.rs:921-925 re-checks whole-frame discipline (`format.frames_in` → `wav_partial_frame` refusal) before convert_to_wav at :934 — the doc's "caller has already verified" holds at the only call site.

## 3. Findings
F-001 [S3]
Where: crates/transcode/src/convert.rs:232 (pub(crate) fn convert_to_wav) + repo manifests
Code:
```
pub(crate) fn convert_to_wav(
    samples: &[u8],
    format: &AudioFormat,
    target: PcmSample,
    scratch: &mut Vec<u8>,
) {
```
What is wrong: reachability, same as wav.rs F-001 — the entire conversion lane lives in a crate no shipped binary or sibling crate depends on; only runner.rs (itself test-only reachable) calls it. No doc claims otherwise (matrix `ui-dialog-convert` = not_started), so: tested library island, not fraud.
Confidence: HIGH — Verification: ran (manifest + call-site grep).
F-002 [S4]
Where: crates/transcode/src/convert.rs:218-223
Code:
```
        // Every other pair was refused by `conversion_legal` before
        // any byte moved; reaching here is a logic bug in this
        // module, and the honest answer is to skip nothing silently —
        // the writer's whole-frame check catches any length drift.
        _ => {}
```
What is wrong: the safety-net claim is half false. If `conversion_legal` ever admitted a pair `emit` does not handle, every sample would be skipped, `scratch` stays empty, and `write_pcm(&[])` accepts it: 0 is a multiple of any frame size, so the writer's whole-frame check does NOT catch it — the result is a structurally valid WAV with a silent zero-length data chunk, the exact "quantize silently" failure the module forbids. The comment says the check "catches any length drift"; it only catches partial drift, not total. What correct looks like: unreachable!()/panic in the catch-all (this is an internal invariant, not input-driven) or an error return.
Confidence: HIGH — Verification: read (control flow through write_pcm's `ensure_room` with add=0: `0.is_multiple_of(frame)` is true).
F-003 [S4]
Where: crates/transcode/src/convert.rs:238-240
Code:
```
    let channels = format.channels as usize;
    let width = format.sample.bytes_per_sample() as usize;
    let frames = samples.len() / (channels * width);
```
What is wrong: no zero guard — `channels == 0` panics with divide-by-zero. Safe today because the only caller validates channels ≥ 1 via WavPcmWriter::new before any frame is converted, but the function's contract is "caller verified whole frames", not "caller verified channels" — a new caller gets a panic instead of a typed refusal. Also a torn tail here is silently floor-divided away (again caller-dependent).
Confidence: HIGH — Verification: read (runner.rs order confirmed: writer::new at :590/:904 precedes sink accept).
F-004 [S4]
Where: crates/transcode/src/convert.rs:441-448
Code:
```
    fn shared_bytes_deref_gives_the_payload_for_passthrough() {
        // The zero-copy path writes the frame's own bytes; prove the
        // view semantics the writer relies on (Deref = payload).
        let payload = SharedBytes::from_vec(vec![9u8; 6]);
        assert_eq!(payload.as_slice(), &[9u8; 6]);
        assert_eq!(payload.len(), 6);
    }
```
What is wrong: this tests media-types' `SharedBytes`, not this module — nothing in convert.rs is exercised. It pins the seam's assumption at best; as a row in convert.rs's test ledger it inflates coverage. The actual zero-copy passthrough decision (`identity_passthrough`) is tested elsewhere in-module, honestly.
Confidence: HIGH — Verification: read.
Coverage note (not a numbered finding): widening value-exactness is tested for u8→s16, s16→s32, s24→s32, f32→f64 — but NOT for u8→s24, u8→s32, s16→s24 (those pairs are only resolution-tested). The shifts are identical in shape, so risk is low.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| source_descriptor | 45-54 | REAL | — | Static vocabulary mapping |
| identity_target | 58-67 | REAL | — | 1:1 SampleFormat→PcmSample |
| resolve_target | 75-91 | REAL | — | Source→identity; Sample(wanted)→legality check, typed refusal |
| conversion_legal | 96-116 | REAL | — | Exhaustive match: 6 identities + 7 widenings; everything else false |
| identity_passthrough | 122-126 | REAL | — | LE host + interleaved + identity only |
| read_native | 130-154 | REAL | — | s24 sign-extension via `((value << 8) as i32) >> 8` — correct; floats return 0 (documented, unused) |
| read_f32 | 157-159 | REAL | — | ne-bytes |
| emit | 164-224 | REAL | S4 | All 13 legal pairs correct (shifts verified); catch-all F-002 |
| convert_to_wav | 232-255 | REAL | S4 | Planar/interleaved indexing correct; F-003 zero guard; scratch reserve-then-reuse correct |
| tests (7) | 257-449 | REAL | S4 | Strong value-exactness; one out-of-module test (F-004) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| target_resolution_matrix | YES | 14 pairs: 10 resolve, 4 refuse with exact code |
| interleaved_identity_is_passthrough_on_little_endian_hosts | YES | LE-conditional assert + planar/other-target negatives |
| planar_to_interleaved_permutes_frame_major | YES | Distinct L/R values, exact output order |
| widening_conversions_are_exact | YES | Boundary values, exact expected i16/i32/f64 |
| native_identity_roundtrips_bytes | YES | MIN/−1/0/MAX roundtrip, length equality |
| scratch_is_reused_not_accumulated | YES | Capacity stable across 5 calls (would fail on per-call alloc) |
| shared_bytes_deref_gives_the_payload_for_passthrough | NO (not this module) | Tests media-types (F-004) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Two conversions are always legal and lossless: byte-order normalization, planar→interleaved" | convert.rs:9-12 | REAL | emit/read_native + planar test |
| "lossless widenings only… every other pair is a typed refusal" | convert.rs:14-18 | REAL | conversion_legal 96-116 + matrix test |
| "identity pair … is a zero-copy passthrough" | convert.rs:22-24 | REAL | identity_passthrough (decision in runner.rs seam) |
| "cleared and reused per frame (zero allocations after the first)" | convert.rs:25-27 | REAL | scratch reuse test (capacity pinned) |
| "the writer's whole-frame check catches any length drift" | convert.rs:221-222 | FALSE (half) | Empty scratch passes the check (F-002) |
| "The caller has already verified the payload is a whole number of sample frames" | convert.rs:229-231 | REAL | runner.rs:921-925 verified |

## 7. Dead / duplicated / unreachable code
- Whole module unreachable from shipped entry points (F-001).
- `source_descriptor` is used only for error vocabulary (resolve_target refusal) — reachable in principle via runner; fine.

## 8. Dependencies & flags
- No feature gates. Deps: demux (PcmSample), media-types (SampleFormat/AudioFormat/AudioLayout). `HOST_IS_LITTLE` is a compile-time cfg — big-endian paths exist but are untestable on this host (UNVERIFIED by execution; verified by reading the branch logic).

## 9. Open questions for the next agent
- runner.rs (not in my list): confirm the passthrough path re-checks whole frames itself (comment at runner.rs:921 says "the passthrough path re-checks what…").
- If 7.E3 ever adds real encoders, the emit catch-all should become an unreachable!() — flag it then.
