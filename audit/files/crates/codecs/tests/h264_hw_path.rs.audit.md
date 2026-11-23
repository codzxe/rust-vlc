# AUDIT: crates/codecs/tests/h264_hw_path.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 968 / 20 tests + 13 helpers/fakes |
| File verdict | REAL (with one tautological assert and a self-skipping corpus lane) |
| Reachable from shipped entry point? | N/A (integration test). NOTE: the code under test (H264Decoder) is NOT registered in the shipped engine (baseline mp4-smoke engine-info: decoders = mjpeg, pcm_s16le; H.264 MP4 exits "no decoder"), so the tested surface is not reachable from lumen-cli playback today |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | executed prebuilt binary `../../target/debug/deps/h264_hw_path-f4e0fd72a3109215 --nocapture --test-threads=1` -> 20 passed / 0 failed, 0.03s, exit 0 (corpus test printed SKIP); rg citations in docs/feature-matrix.toml:416; sed-verified quotes |

## 1. One-paragraph verdict
A real integration suite for the H.264 hardware decode-path seam, honestly scoped: the header declares the adapter is FAKE and the suite pins plumbing (parser -> AU assembly -> session feed -> opaque-handle frames -> typed fallbacks), not real hardware decode. I executed it: 20/20 pass in 0.03s. The feed-path tests assert real observable behavior (AU counts, pts plumbing, surface handles, configure calls, typed error codes, stale-generation drops, lag/flush draining, policy trail shapes) and would fail if the plumbing broke. Two defects: one assertion pretends to verify bounded accumulation but only checks a Debug string contains "H264Decoder" (tautology), and the corpus lane silently self-skips when the corpus file is absent — which is the case in this sandbox, so its 26-VCL ground-truth assertions never ran here.

## 2. What actually works (verified)
— verified by `../../target/debug/deps/h264_hw_path-f4e0fd72a3109215 --nocapture --test-threads=1` -> `test result: ok. 20 passed; 0 failed; ... finished in 0.03s`, exit 0.
— verified by baseline audit/runs/baseline/cargo-test-workspace.txt:1127-1151 -> same 20/20 in 0.03s.
— cross-checked docs/feature-matrix.toml:416: matrix note admits "runtime hardware decode NOT exercised in sandbox [VERIFY]" — consistent with a fake-adapter suite; no matrix row cites fuzz_lite as bounded-accumulation evidence (rg found no citation), so the tautology below is not propagated as evidence.

## 3. Findings
F-001 [S3]
Where: crates/codecs/tests/h264_hw_path.rs:816-819
Code: (verbatim)
```
    // Bounded accumulation held (MP §12): the pending list cannot
    // exceed the cap even under adversarial unit floods.
    let pending = format!("{decoder:?}");
    assert!(pending.contains("H264Decoder"));
```
What is wrong: the comment claims the test verifies the pending-list cap (a memory bound), but the assertion only checks that the Debug rendering contains the type name. It cannot fail for any accumulation behavior; the bound is NOT verified anywhere in this suite. Why it matters: a reader of the suite will believe MP §12's pending cap has test coverage; it has none here. What correct looks like: expose the pending count (or a max-watermark) on the decoder and assert it <= cap after the 5k-packet flood. Confidence: HIGH — Verification: read + ran (test passes trivially).
F-002 [S3]
Where: crates/codecs/tests/h264_hw_path.rs:699-707
Code: (verbatim)
```
    let path = std::path::PathBuf::from(
        std::env::var("LUMEN_CORPUS").unwrap_or_else(|_| "/tmp/my-project/corpus-v4".to_string()),
    )
    .join("media/0019-raw-video.h264");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("SKIP h264 hw-path corpus: 0019-raw-video.h264 absent");
        return;
    };
```
What is wrong: the only full-real-stream test self-skips (returns Ok) when the corpus is absent — the default path /tmp/my-project/corpus-v4/media/0019-raw-video.h264 does not exist in this sandbox (ls: No such file), and my run printed the SKIP line, so the 26-VCL / 26-AU / frame_num-monotonic assertions (L741-767) never executed here; baseline "ok" for this test proves nothing about the corpus lane in this environment. The skip is loud (eprintln) but the harness still reports pass. Confidence: HIGH — Verification: ran (SKIP printed); file-absence verified with ls.
F-003 [S4]
Where: crates/codecs/tests/h264_hw_path.rs:748-754
Code: (verbatim)
```
    assert!(
        matches!(
            out,
            DecodeOutcome::NeedMoreInput | DecodeOutcome::Recovered { .. }
        ) || sink.frames.len() >= 26
    );
    assert!(sink.frames.len() >= 26, "every picture produced a surface");
```
What is wrong: the first assertion is subsumed by the second; dead conditional clutter. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/codecs/tests/h264_hw_path.rs:605-608
Code: (verbatim)
```
    assert!(matches!(
        out,
        DecodeOutcome::Emitted { frames: 1 } | DecodeOutcome::Recovered { .. }
    ));
```
What is wrong: after the truncated-length-prefix damage, either success or recovery is accepted — the test does not pin which, so a regression that flips Emitted to Recovered here goes unnoticed (the pcm suite pins outcomes exactly; this one does not). Confidence: HIGH — Verification: read.
F-005 [S4]
Where: crates/codecs/tests/h264_hw_path.rs:785-820
Code: (excerpt, L803-813)
```
    for i in 0..5_000u32 {
        let len = (rng.next() % 512) as usize;
```
What is wrong: fuzz-lite is absence-of-panic only, on ≤512-byte packets, mostly killed at first-NAL parse (whole suite runs in 0.03s); damage accounting for h264 rests on the two hand-built cases in damage_is_counted_recovery (L566-609). Weaker than the PCM survival bar but the header does not claim more. Confidence: HIGH — Verification: read + ran.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MustErr::must_err | 28-40 | REAL | - | err-half helper, track_caller |
| annexb / avcc_record / avc_sample / packet | 68-113 | REAL | - | fixture builders; avcC form is the real-world one |
| FakeAccel / FakeSession (HwAccelerator + H264HwDecoder impls) | 132-228 | REAL | - | logs configure/AUs/flush, injectable failures, optional 1-frame lag; honest in-test stand-in |
| FakeBridge / AlienBridge | 234-260 | REAL | - | downcast seam; alien bridge returns None |
| RecordingSink | 263-284 | REAL | - | asserts OpaqueHandle storage (unreachable! on CPU bytes — correct) |
| config | 286-297 | REAL | - | |
| no_accelerator_is_typed_unsupported_never_silent | 301-315 | REAL | - | pins h264.hw.required + Unsupported |
| accelerator_without_bridge_is_typed_no_bridge | 317-333 | REAL | - | pins h264.hw.no_bridge |
| wrong_codec_descriptor_is_refused_typed | 335-343 | REAL | - | h264.codec_descriptor |
| avc1_descriptor_aliases_are_accepted | 345-361 | REAL | - | avc1.64000d fourcc alias |
| annexb_stream_feeds_aus_and_emits_opaque_frames | 365-431 | REAL | - | AU-boundary detection, pts, surfaces 1/2/3, configure(1 SPS,1 PPS,(64,48)), 3 AUs, flush — strong |
| avcc_extradata_configures_and_length_prefixed_samples_decode | 433-479 | REAL | - | configure-time param sync + lag pipeline + Drained-on-reflush |
| multi_slice_picture_is_one_au | 481-517 | REAL | - | two VCLs same picture key -> one AU |
| decode_before_configure_is_typed_bad_state | 521-529 | REAL | - | h264.not_configured on decode+flush |
| stale_packets_are_dropped_on_sight | 531-563 | REAL | - | generation guard: zero decode work |
| damage_is_counted_recovery_and_decoder_stays_usable | 565-609 | REAL | - | forbidden-bit NAL -> Recovered{skipped:1}; truncated length prefix bounded; see F-004 |
| hw_session_failure_is_typed_engine_error | 611-658 | REAL | - | h264.hw.decode; DeviceLost -> DecodeCause::HardwareLost |
| hw_configure_failure_is_typed_unsupported | 660-677 | REAL | - | h264.hw.configure + Unsupported |
| alien_bridge_leaves_the_stream_unclaimed | 679-694 | REAL | - | |
| corpus_0019_annexb_full_stream_survives... | 698-768 | REAL-UNPROVEN (here) | S3 | self-skips without corpus (F-002); assertions strong when present |
| SplitMix64 (dup) | 772-782 | REAL | - | third copy of SplitMix64 across test files (duplication, see §7) |
| fuzz_lite_random_packets_never_panic | 784-820 | REAL | S3 | absence-of-panic verified; bounded-accumulation assert is a tautology (F-001) |
| fuzz_lite_configure_with_random_extradata_never_panics | 822-851 | REAL | - | 2k random extradata, typed outcomes |
| from_setting_software_only_default_is_off_and_refuses_typed | 855-890 | REAL | - | exact OFF trail shape pinned (Probed/Inactive DisabledByPolicy) |
| from_setting_rejects_spellings_outside_the_vocabulary | 892-902 | REAL | - | |
| from_setting_prefer_hardware_records_a_typed_trail | 904-945 | REAL | - | host-independent trail contract |
| engine_accelerator_precedes_the_self_negotiated_one | 947-967 | REAL | - | engine accel wins over policy None |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| 17 plumbing/policy tests | YES | exact outcome/codes/log-shape asserts; e.g. dropping the generation guard fails stale_packets (SessionLog.aus non-empty) |
| damage_is_counted_recovery... | PARTIALLY | first two cases exact; final case accepts two outcomes (F-004) |
| corpus_0019... | NOT IN THIS ENV | self-skip; needs LUMEN_CORPUS (F-002) |
| fuzz_lite_random_packets_never_panic | panic -> YES; accumulation bound -> NO (F-001 tautology) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "a fake hardware adapter ... proves the feed path end-to-end" | header L1-12 | REAL | 20/20 executed; fake adapter is in-test and clearly labeled |
| "Capability negotiation + no-device/permission fallback host-tested (codecs suite incl. h264_hw_path ...); runtime hardware decode NOT exercised in sandbox [VERIFY]" | docs/feature-matrix.toml:416 | REAL | matches suite content and my run |
| fixtures "real ffmpeg-authored bytes from the corpus's 0019-raw-video.h264" | header L7-9 | UNVERIFIED | corpus file absent in sandbox; constants' provenance cannot be re-derived |

## 7. Dead / duplicated / unreachable code
SplitMix64 is duplicated: tests/h264_hw_path.rs:772-782, tests/pcm_fuzz_survival.rs:160-186, and (per later files) likely others — a shared test-util candidate. No dead tests.

## 8. Dependencies & flags
Test-only; imports codecs::video::h264::hw (in-tree), codec_api::HwAccelerator seam. No feature gates in this file.

## 9. Open questions for the next agent
- decode.rs (my file 3) must confirm: with_bridges/from_setting wiring, pending-list cap existence (F-001 asks where the cap is actually enforced + whether any test pins it).
- Is h264 registered in any shipped decoder registry? Baseline engine-info says no — flag for the vertical-scope owner.
