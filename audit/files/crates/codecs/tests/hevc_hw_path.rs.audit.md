# AUDIT: crates/codecs/tests/hevc_hw_path.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-02-14 |
| Lines / functions | 961 / 17 tests + ~13 helpers/fixtures |
| File verdict | REAL as a plumbing test; proves NO HEVC decoding (fake adapter, fabricated frames) |
| Reachable from shipped entry point? | N/A (integration test); the HevcDecoder it exercises is UNREACHABLE from shipped entry points (see nal.rs audit F-003) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `./target/debug/deps/hevc_hw_path-c0de0d66a701dd51 --nocapture` → 17 passed; 0 failed, exit 0 (corpus row SKIP-observed) |

## 1. One-paragraph verdict
This suite genuinely exercises the HevcDecoder plumbing — Annex-B and
length-prefixed framing, §7.4.2.4.4 AU-boundary detection, configure-time
parameter-set sync, typed refusals (`hevc.hw.required`/`no_bridge`/
`codec_descriptor`/`not_configured`), stale-generation drop, counted damage
recovery — against a FAKE hardware adapter, and it passes 17/17 live. But
answering the orchestrator's hostile question directly: it proves plumbing
only. The fake session fabricates frames (opaque handle = incrementing
counter, size from the AU or a hard-coded `(360, 640)` fallback), no real
HevcHwBridge exists anywhere in-tree (only test fakes; decode.rs:157
"truthful today: zero adapters are installed in-tree"), so zero pixels are
ever decoded by anything this file touches — and the decoder itself is
unreachable from both shipped hosts anyway. One assert is a flat tautology
dressed up as a bounded-accumulation proof, and the flagship full-corpus
test silently no-ops in this environment.

## 2. What actually works (verified)
— verified by `./target/debug/deps/hevc_hw_path-c0de0d66a701dd51 --nocapture`
→ `test result: ok. 17 passed; 0 failed` (ran by me, exit 0), including the
typed-refusal rows, the damage-recovery row, both fuzz-lite rows, and the
multi-segment AU row with real x265-derived byte fixtures.

## 3. Findings

F-001 [S3]
Where: crates/codecs/tests/hevc_hw_path.rs:920-924
Code:
```rust
    let _ = decoder.flush(&mut sink);
    // Bounded accumulation held (MP §12): the pending list cannot
    // exceed the cap even under adversarial unit floods.
    let pending = format!("{decoder:?}");
    assert!(pending.contains("HevcDecoder"));
}
```
What is wrong: the comment claims "the pending list cannot exceed the cap
even under adversarial unit floods" — the assert checks only that the debug
string of the decoder contains the literal "HevcDecoder". That is a
tautology: it passes for any Debug-able decoder regardless of pending-list
size, cap, or absence thereof. If the cap were removed tomorrow this test
would still pass. The bounded-accumulation property (MP §12) is asserted
NOWHERE.
Why it matters: evidence-string inflation — a reader auditing the MP §12 row
would believe a bound is pinned here.
What correct looks like: expose the pending count (test-support accessor)
and assert it <= the documented cap after the flood loop.
Confidence: HIGH — Verification: ran + read.

F-002 [S3]
Where: crates/codecs/tests/hevc_hw_path.rs:805-813
Code:
```rust
fn corpus_0044_annexb_full_stream_survives_and_feeds_every_picture() {
    let path = std::path::PathBuf::from(
        std::env::var("LUMEN_CORPUS").unwrap_or_else(|_| "/tmp/my-project/corpus-v4".to_string()),
    )
    .join("media/0044-raw-video.265");
    let Ok(bytes) = std::fs::read(&path) else {
        eprintln!("SKIP hevc hw-path corpus: 0044-raw-video.265 absent");
        return;
    };
```
What is wrong: silently returns Ok when the corpus is absent — and it is
absent (`test -d /tmp/my-project/corpus-v4` → ABSENT). My live run printed
`SKIP hevc hw-path corpus: 0044-raw-video.265 absent` and the harness still
reported `ok`. The module doc (:16-17) explicitly promises "the full-file
corpus test below feeds the untruncated stream" — in this environment that
promise is never exercised; the only full-stream evidence is the pinned
56-VCL claim, which itself only runs when the corpus exists.
Why it matters: baseline "17 passed" includes a test that verified nothing;
the strongest end-to-end claim of the card is environment-gated to off.
Confidence: HIGH — Verification: ran (skip observed live).

F-003 [S4]
Where: crates/codecs/tests/hevc_hw_path.rs:79-84
Code:
```rust
/// Picture 2's IDR_N_LP slice segment (48-byte prefix).
const REAL_TRAIL_A_PREFIX: &[u8] = &[
    0x28, 0x01, 0xaf, 0x0a, 0x0b, 0xb2, ...
```
What is wrong: the names `REAL_TRAIL_A_PREFIX`/`REAL_TRAIL_B_PREFIX` say
TRAIL (NAL types 0/1) but the payloads start `0x28 0x01` →
`(0x28 >> 1) & 0x3f` = 20 = IDR_N_LP, as their own doc comments admit. The
stream is all-IDR, so the names describe a slice class that never occurs in
these fixtures. Names lie; readers correlating "TRAIL" with the non-IRAP
paths will mis-read the AU-boundary assertions.
Confidence: HIGH — Verification: read (header arithmetic).

F-004 [S4]
Where: crates/codecs/tests/hevc_hw_path.rs:231-235
Code:
```rust
        let frame = HevcHwFrame {
            surface: self.next_surface.fetch_add(1, Ordering::SeqCst),
            size: au.display_size.unwrap_or((360, 640)),
            pts: au.pts,
        };
```
What is wrong: the fake session's frame geometry defaults to a hard-coded
`(360, 640)` when the AU carries no display size. Test-only fabrication and
honest per the module doc ("fake hardware adapter"), but every downstream
assertion of "360×640" in this suite is then partly self-fulfilling: the
decoder forwards the SPS-derived size, and where it doesn't, the fake
supplies the expected answer anyway.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MustErr | 36-48 | REAL | - | negative-unwrap helper |
| fixtures (7 consts) | 55-103 | REAL-UNPROVEN | S4 | claimed x265 corpus provenance UNVERIFIED (corpus absent); pinned bytes internally consistent |
| annexb / hvcc_sample / packet | 105-137 | REAL | - | builders |
| FakeAccel/FakeSession (configure/decode/flush) | 156-256 | REAL | S4 | fabricates handles/sizes (F-004) |
| FakeBridge/AlienBridge | 262-288 | REAL | - | downcast seam, claim-refusal row |
| RecordingSink | 291-312 | REAL | - | asserts OpaqueHandle (CPU path would unreachable!) |
| no_accelerator_is_typed_unsupported_never_silent | 329-343 | REAL | - | ran ok |
| accelerator_without_bridge_is_typed_no_bridge | 345-361 | REAL | - | ran ok |
| wrong_codec_descriptor_is_refused_typed | 363-371 | REAL | - | ran ok |
| hvc1_descriptor_aliases_are_accepted | 373-389 | REAL | - | ran ok |
| annexb_stream_feeds_aus_and_emits_opaque_frames | 393-479 | REAL | - | strong: boundary/pts/surface/SEI-attachment assertions |
| hvcc_extradata_configures_and_length_prefixed_samples_decode | 481-531 | REAL | - | lagging session + double-flush covered |
| multi_segment_picture_is_one_au | 533-586 | REAL | - | degenerate multi-slice pinned |
| decode_before_configure_is_typed_bad_state | 590-598 | REAL | - | ran ok |
| stale_packets_are_dropped_on_sight | 600-632 | REAL | - | generation drop verified against session log |
| damage_is_counted_recovery_and_decoder_stays_usable | 634-681 | REAL | - | 0xffff_ffff truncated prefix — pins 64-bit typed path (32-bit would panic, see nal.rs F-001) |
| orphan_continuation_segment_is_counted_damage | 683-715 | REAL | - | ran ok |
| hw_session_failure_is_typed_engine_error | 717-764 | REAL | - | Decode + DeviceLost→HardwareLost mapping |
| hw_configure_failure_is_typed_unsupported | 766-783 | REAL | - | ran ok |
| alien_bridge_leaves_the_stream_unclaimed | 785-800 | REAL | - | ran ok |
| corpus_0044_..._every_picture | 804-873 | SHALLOW (here) | S3 | F-002 skip no-op; 56-VCL ground truth unreachable |
| fuzz_lite_random_packets_never_panic | 889-925 | REAL | S3 | panic-freedom genuine; "bounded accumulation" assert is a tautology (F-001) |
| fuzz_lite_configure_with_random_extradata_never_panics | 927-961 | REAL | - | 2k random hvcC-shaped blobs, ran ok |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 15 non-corpus tests | YES | exact DecodeOutcome/typed-code assertions on real byte fixtures |
| corpus_0044_... | NO (in this env) | silent skip; would only fail with corpus present |
| fuzz_lite_random_packets... | PARTLY | catches panics; the "bounded" comment asserts nothing (F-001) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "proves the feed path end-to-end — parser → access-unit assembly → session feed → opaque-handle frames" | :1-7 module doc | REAL (with a fake session) | 17/17 ran live |
| "the full-file corpus test below feeds the untruncated stream" | :16-17 | SHALLOW | corpus absent; test skipped in my run |
| "typed-unsupported fallback ... never a silent skip" | :5-7 | REAL | no_accelerator/no_bridge/alien rows pass |
| "Bounded accumulation held (MP §12)" | :921-922 | FALSE (as evidence) | assert is `pending.contains("HevcDecoder")` — tautology (F-001) |
## 7. Dead / duplicated / unreachable code
The HevcDecoder under test is unreachable from shipped entry points (no
"hevc" arm in apps/cli/src/pipeline_host.rs nor apps/web/src/host.rs).
No dead helpers inside the test file.
## 8. Dependencies & flags
Integration test of `codecs` crate; uses codec-api prelude, media-types.
No feature gates; runs in default `cargo test -p codecs` (baseline line 1184).
## 9. Open questions for the next agent
- decode.rs: is the pending-AU list actually capped (the F-001 claim)? Verify
  the real bound and where it is enforced.
- hw.rs (not assigned): confirm zero in-tree HevcHwBridge impls besides test
  fakes (rg found none outside crates/codecs).
