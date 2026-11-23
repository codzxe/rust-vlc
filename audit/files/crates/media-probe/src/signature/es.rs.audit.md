# AUDIT: crates/media-probe/src/signature/es.rs
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-23 |
| Lines / functions | 602 / 9 prod fns (4 signatures) + 15 tests |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — apps/cli/src/demux_host.rs:444-446 `ProbeRegistry::with_builtins()` + `probe_stream` on the open path; signatures registered at crates/media-probe/src/signature/mod.rs:142-145 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | python3 byte-verified fixture-head comparison (h264/hevc windows vs crates/media-probe/fixtures/*.264/.265); python3 reads; rg registration + refusal wiring; baseline citations :3573-3708, :3690-3696 |

## 1. One-paragraph verdict
Genuinely implemented, honestly bounded probe signatures for the raw-ES
family. The MPEG-audio anchor implements the correct half-header gate
(11-bit sync, layer bits, reserved-version/free-bitrate/reserved-sampling
rejections — bit math re-verified by hand), the ID3v2 skip is syncsafe-
correct and window-contained, the Annex-B discriminator's
hypothesis-killing + parameter-set-evidence design is sound (I traced the
kill logic: H.264 SPS 0x67 → HEVC type 50 dies; HEVC VPS 0x40 → H.264
type 0 dies; ambiguous windows mint only a sub-threshold family id the
registry refuses), and the FLAC gate matches the spec's STREAMINFO-first
shape. All 15 unit tests plus the registry roundtrip over the REAL
committed fixtures pass in baseline. The one defect is a comment-level
provenance lie: two test windows claim to be "the committed fixture's
head" but are not byte-identical to the committed fixtures.

## 2. What actually works (verified)
- All 15 `signature::es::tests` pass: baseline cargo-test-workspace.txt
  :3573-3708 (package media-probe, exit 0), e.g.
  `adts_layer00_is_not_mpeg_audio ... ok`,
  `start_codes_without_parameter_sets_name_nothing ... ok`.
- Registry roundtrip over the REAL committed fixtures (not hand windows):
  tests/registry_roundtrip.rs:190-201 drives
  `fixtures/annexb_probe_h264.264` → "h264" and
  `fixtures/annexb_probe_hevc.265` → "hevc" (+ mp3/mp2/flac rows);
  `every_landed_container_probes_to_its_format_id ... ok` baseline :3695.
- FLAC refusal claim verified end-to-end: the doc's "no raw-FLAC demuxer
  exists … typed refusal naming that gap" matches
  apps/cli/src/containers/es.rs:258-270 (`open_flac` returns the typed
  refusal with `format=flac` detail) and containers/mod.rs:62 `"flac" =>
  es::open_flac(stream, opts)`.
- Bit-math re-derivation (read): layer bits `(h[1]>>1)&0x3`, version
  `(h[1]>>3)&0x3`, ID3 syncsafe 7-bit shifts, footer flag `prefix[5] &
  0x10` — all correct.

## 3. Findings
F-001 [S4]
Where: crates/media-probe/src/signature/es.rs:428-433 (and 442-448)
Code:
```
    /// (the committed fixture's head: SPS `0x67`, then PPS/slices).
    #[test]
    fn h264_start_codes_probe_h264() {
        let window = [
            0x00, 0x00, 0x00, 0x01, 0x67, 0x64, 0x00, 0x0D, 0x00, 0x00, 0x00, 0x01, 0x68, 0xEA,
```
What is wrong: the comment attributes the window to "the committed
fixture's head", but the committed fixture
crates/media-probe/fixtures/annexb_probe_h264.264 begins
`00 00 00 01 67 F4 00 0A 91 9B ...` (python3 hex read) — the test window's
`67 64 00 0D` matches nothing in the fixture head. Same for the HEVC test
(442-448): the first 24 bytes match annexb_probe_hevc.265, then the window
splices `1E 00 00 01 42` over the fixture's `1E 95 98 09 00 00 00 01 42`.
Hand-typed approximations labeled as fixture bytes — a provenance claim
that does not hold, though the real fixtures ARE probed separately via
registry_roundtrip.rs:190-201 (so behavior is covered; only the comment is
false). What correct looks like: either quote the fixture bytes exactly or
drop the attribution. Confidence: HIGH — Verification: python3 byte
comparison of both fixtures vs both windows.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| anchored (helper) | 100-107 | REAL | — | magic-rung hit shape |
| MpegLayer (enum) | 111-117 | REAL | — | L3/L2 only; L1 + layer-00 recorded gaps (doc :23-26, :126-127) |
| mpeg_audio_anchor | 128-149 | REAL | — | bit math re-verified; `prefix.get(at..at+4)?` bounds-safe |
| id3v2_end | 155-170 | REAL | — | syncsafe-checked, footer-aware, window-contained (`end <= prefix.len()`) |
| Mp3Signature::probe + extensions | 178-187 | REAL | — | L3 gate |
| Mp2Signature::probe + extensions | 196-205 | REAL | — | L2 gate |
| AnnexbSignature::probe | 213-278 | REAL | — | O(n) scan, MAX_WALK_STEPS=256 bound, no allocation, `i+4<=len` guards every index; hypothesis-kill + param-evidence logic traced correct; ambiguous → SCORE_FAMILY_ONLY (50) sub-threshold |
| AnnexbSignature::extensions | 280-283 | REAL | — | 264/h264/265/h265/hevc |
| FlacSignature::probe + extensions | 291-305 | REAL | — | `fLaC` + type-0 + 34-byte body; `prefix.len()>=8` guard |
| 15 tests | 307-601 | REAL | S4 | F-001 comment provenance; otherwise strict (direct signature asserts + registry-level + error-kind asserts) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 15 | YES | exact format ids, confidence values, error codes/kinds, direct-signature score asserts (483-497) |
| registry_roundtrip (other file, covers this one) | YES | real committed fixture bytes → exact id + threshold |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "an ADTS frame can never anchor here and an MPEG-audio frame can never anchor there" | :19-22 | REAL | layer-00 returns None (:140); test :364-374 proves both directions |
| "a window with no parameter set names nothing" | :46-47 | REAL | :259-276 `_ => None` guard; test :544-557 (64× `00 00 01 00`) |
| "corpus `0071-cover-audio.flac` carries a 99,682-byte ID3v2 tag" | :80-82 | REAL-UNPROVEN | corpus is external (LUMEN_CORPUS, es_corpus.rs:52); consistent with containers/es.rs:533 test and the 0x06 0x0A 0x62 syncsafe encode in test :418 (decodes to 99,682) |
| "No raw-FLAC demuxer exists … typed refusal" | :64-68 | REAL | apps/cli/src/containers/es.rs:258-270 refusal; no flac module in crates/demux/src/es (ls verified) |
| "the committed fixture's head" (test comments) | :428, :442 | FALSE (as byte attribution) | F-001 python3 comparison |

## 7. Dead / duplicated / unreachable code
None. All four signatures are registered (signature/mod.rs:142-145) and
exercised through the shipped open path.

## 8. Dependencies & flags
media-probe crate, std-only; no feature gates; no unsafe (rg verified).
Constants from super: MAX_WALK_STEPS=256 (signature/mod.rs:72),
SCORE_MAGIC_ANCHOR=70 (:91), SCORE_FAMILY_ONLY=50 (:93).

## 9. Open questions for the next agent
- crates/media-probe/src/contract.rs (my next file): the score/threshold
  contract these hits ride.
- The recorded Layer-I gap and MVC 16..=20 gap are honest but mean real
  .mp1/MVC files probe-miss — a product-level coverage note, not a defect
  of this file.
