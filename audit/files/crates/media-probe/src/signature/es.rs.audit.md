# AUDIT: crates/media-probe/src/signature/es.rs
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-21 |
| Lines / functions | 602 / 11 (+15 tests, 4 test helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → apps/cli/src/demux_host.rs:444 `ProbeRegistry::with_builtins()` → registers Mp3/Mp2/Annexb/FlacSignature at signature/mod.rs:142-145 → `probe_stream` → ids `mp3`/`mp2`/`h264`/`hevc`/`annexb`/`flac` → apps/cli/src/containers/mod.rs arms `mp3`,`mp2`,`h264`,`hevc`,`flac`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |

## 1. One-paragraph verdict
This is the rare file in this audit that does what it says: four raw-ES probe signatures
(MP3, MP2, Annex-B H.264/HEVC with whole-window NAL discrimination, FLAC) with honest,
documented gaps (Layer I unprobed, ambiguous NAL shapes → sub-threshold `annexb` family
id that the registry refuses, no-SPS windows miss), all registered in the builtin
registry and wired to real container arms. All 15 module tests pass in the baseline
(orchrun-test-workspace-nofailfast.log.txt, media-probe unittests: 80 passed, 0 failed).
The defects found are a test suite that fails to pin the load-bearing forbidden-bit
guard (a specific mutation survives green), an ID3v2 skip that never validates the tag
version and can misanchor on version-2 tags, and two narrow heuristic edge cases —
none user-visible in the normal lanes.

## 2. What actually works (verified)
- All 15 `signature::es::tests::*` pass — verified by
  `sed -n '3309,3420p' audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt`
  (media_probe unittests: "80 passed; 0 failed"; es test lines 40-54 of that slice).
- Registry wiring is real: signature/mod.rs:142-145 registers all four; containers
  registry has arms for every minted id (`mp3`,`mp2`,`h264`,`hevc` → es::open_*;
  `flac` → the honest typed refusal `open_flac`, apps/cli/src/containers/es.rs:258-279,
  read in full — the "no raw-FLAC demuxer" claim in this file's docs at es.rs:64-71 is
  TRUE: crates/demux/src/es/ contains aac/annexb/h264/hevc/mp3 only, no flac).
- ADTS/MPEG-audio disjointness is real and tested from this side (es.rs:364-374:
  layer-`00` windows route to `adts` through the registry).

## 3. Findings

### F-001 [S3] The forbidden-bit guard is not pinned by any test — mutation survives green
Where: crates/media-probe/src/signature/es.rs:229-232
Code:
```rust
                if b & 0x80 != 0 {
                    // forbidden_zero_bit set: invalid in both codecs.
                    h264_alive = false;
                    hevc_alive = false;
                } else {
```
What is wrong: Deleting this branch leaves all 15 tests passing. Every test vector with
a start-code head ≥ 0x80 (`0xBA`, `0xB3`, `0xE0`, `0x80` in
`mpeg_system_start_codes_stay_unprobed`, es.rs:508-513) also dies through the type-range
checks (`0xBA & 0x1F` = 26 → h264 dead; `0xBA >> 1` = 93 > 40 → hevc dead). No vector
exercises the shape where ONLY the forbidden bit kills: head `0x81..0x8F` (h264 type
1..15, e.g. `00 00 01 88` reads as H.264 PPS → `h264_param = true`).
Why it matters: With the guard removed (or broken by a refactor), a single 4-byte run
`00 00 01 8x` in any binary payload would mint `h264` at confidence 70 and route the
file to the H.264 demuxer. The guard is load-bearing for exactly the payload-noise case
the module docs say it exists for ("the guard that keeps chance `00 00 01 00` runs in
container payloads … from claiming the family", es.rs:44-47) — but the noise shape it
is quoted for (`00`) is already killed by the type-0 check; the shape only IT can kill
is untested.
What correct looks like: A test vector with a start code followed by `0x88` (or any
0x81..0x8F) asserting `None`/probe-miss.
Confidence: HIGH — Verification: read all test vectors (es.rs:336-601); enumerated
b & 0x1F and b >> 1 for every ≥0x80 vector; mutation reasoned, not executed (no cargo
runs per rules).

### F-002 [S3] `id3v2_end` accepts any "ID3" prefix without a version check; the size read is only correct for v2.3/v2.4
Where: crates/media-probe/src/signature/es.rs:155-170
Code:
```rust
fn id3v2_end(prefix: &[u8]) -> Option<usize> {
    if prefix.len() < 10 || &prefix[0..3] != b"ID3" {
        return None;
    }
    let size_bytes = [prefix[6], prefix[7], prefix[8], prefix[9]];
```
What is wrong: The version byte `prefix[3]` is never inspected. The 4-byte syncsafe
size at bytes 6..10 is the v2.3/v2.4 layout; an ID3v2.2 tag (3-byte size field) makes
`size` a composite of size and frame bytes, so `end` is computed from payload data.
When that bogus `end` lands inside the window on bytes shaped like an MPEG-audio frame
header (`0xFF 0xEx` with valid layer/bitrate/sampling bits — plausible in tagged cover
art of a NON-MPEG file), `mpeg_audio_anchor` mints `mp3` for a non-MPEG file.
Why it matters: A wrong format id routes the open path to the wrong demuxer — the exact
failure class a probe exists to prevent. Narrow (v2.2 tags are rare; the 4-byte pattern
constraint is tight), but it is a silent wrong-name, not a miss.
What correct looks like: Validate `prefix[3]` is 3 or 4 (v2.3/v2.4) before trusting the
4-byte size; treat v2.2 as out-of-scope → `None` (miss, the honest default everywhere
else in this module).
Confidence: MEDIUM — Verification: sed -n '155,170p' (quote verified); v2.2 3-byte size
field per the ID3v2.2.0 spec; the false-anchor path is reasoned, not executed.
Confidence on the code defect (no version check): HIGH; on real-world reachability: LOW.

### F-003 [S4] HEVC type ceiling treats unspecified NAL types 41..=63 as impossible
Where: crates/media-probe/src/signature/es.rs:241-242
Code:
```rust
                    if hevc_type > 40 {
                        hevc_alive = false; // past VCL+params+SEI: not HEVC
```
What is wrong: HEVC `nal_unit_type` is 6 bits (0..=63); 41..=47 and 48..=63 are
*unspecified* in the spec, not invalid. The module doc repeats the overstatement
("HEVC-impossible when its type (b >> 1) exceeds 40", es.rs:37-39). A stream carrying
an unspecified-type NAL loses the HEVC hypothesis — a probe miss (never a wrong name:
h264 dies independently on those bytes).
Why it matters: Legal-if-unusual HEVC streams probe-miss and fall to
`probe.no-demuxer`; the doc's "impossible" claims spec authority it doesn't have.
What correct looks like: Either accept 41..=63 as alive-but-non-param (miss only when
no VPS/SPS/PPS in window), or document the ceiling as a conservative heuristic, not
spec text.
Confidence: HIGH (code behavior) — Verification: sed -n '241,245p'; type width
`b >> 1` read at es.rs:235.

### F-004 [S4] Annex-B walk keeps scanning after both hypotheses are dead
Where: crates/media-probe/src/signature/es.rs:225-251
Code:
```rust
        while i + 4 <= prefix.len() && seen < MAX_WALK_STEPS {
            if prefix[i] == 0 && prefix[i + 1] == 0 && prefix[i + 2] == 1 {
```
What is wrong: Once `h264_alive == false && hevc_alive == false` the outcome is
necessarily `None` (es.rs:276), but the loop keeps byte-scanning to the window end (up
to the 64 KiB default window). No early exit.
Why it matters: Wasted work per probe on every non-Annex-B file that carries one
deadly start code (e.g., every MPEG-PS file: pack header `00 00 01 BA` sets the
forbidden bit at the first step, then the whole window is scanned anyway). The probe
runs on every open.
What correct looks like: `if !h264_alive && !hevc_alive { break; }` after each
classification.
Confidence: HIGH — Verification: read the loop (verified above); outcome match arms
es.rs:255-277 confirm None is forced.

### F-005 [S4] `i += 4` can skip a start code that overlaps the consumed header byte
Where: crates/media-probe/src/signature/es.rs:247-250
Code:
```rust
                i += 4;
            } else {
                i += 1;
            }
```
What is wrong: After a match at `i` (start code `i..i+2`, header `i+3`), the scan
resumes at `i+4`, so a start code at `i+3..i+5` (legal only when the first NAL's
header byte is `0x00` — H.264 type 0, HEVC TRAIL_N — i.e. a zero-length NAL) is never
classified. Example: `00 00 01 00 00 01 40 …` never sees the VPS at the second start
code; the walk ends with no param evidence → miss, where a full scan names `hevc`.
Why it matters: Pathological-but-legal bytestreams (zero-length NALs) probe-miss.
Blast radius: tiny; failure mode is a miss, not a wrong id.
What correct looks like: Advance `i += 3` and let the `else` arm's `i += 1` walk over
the header byte, or rescan from `i + 3`.
Confidence: MEDIUM — Verification: trace by hand on `00 00 01 00 00 01 40`: match at
0, header 0x00 (h264 dead), i=4; scan at 4 checks `[4],[5],[6]` = `00,01,40` → no
match; the `00 00 01` at `[3..5]` was skipped.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `anchored` (helper) | 100-107 | REAL | — | Local shape builder, score 70, no logic. |
| `mpeg_audio_anchor` | 128-149 | REAL | S3(via F-002) | 11-bit sync, version-01 reject, layer 01/10 only, bitrate 0/15 reject, sampling-11 reject — mirrors demux `parse_head` claims; anchor offset depends on F-002's version-blind ID3 skip. |
| `id3v2_end` | 155-170 | REAL | S3 | F-002. Syncsafe guard + end-within-window guard correct; no overflow (size ≤ 2^28). |
| `Mp3Signature::probe` | 179-182 | REAL | — | Layer-III gate; `at + 4` consumed. |
| `Mp3Signature::extensions` | 184-186 | REAL | — | `["mp3"]` hint list. |
| `Mp2Signature::probe` | 197-200 | REAL | — | Layer-II gate; disjoint from mp3 by the layer bit only — correct. |
| `Mp2Signature::extensions` | 202-204 | REAL | — | `["mp2"]`. |
| `AnnexbSignature::probe` | 214-278 | REAL | S4 ×3 | Bounded (256 steps); F-001 guard, F-003 ceiling, F-004 no-early-exit, F-005 skip. Decisive-only-with-param-set logic verified sound; both-alive → sub-threshold `annexb` (registry refuses, no arm — containers/mod.rs:56-58 comment). |
| `AnnexbSignature::extensions` | 280-282 | REAL | — | 5 ids. |
| `FlacSignature::probe` | 292-299 | REAL | — | `fLaC` + type-0 + 34-byte body (24-bit BE length bytes 0,0,34). Correct per FLAC spec. |
| `FlacSignature::extensions` | 302-304 | REAL | — | `["flac"]`. |
| tests: `ok`/`registry`/`config`/`probe` helpers | 315-332 | REAL | — | Test-only; registry-level probing through the real builtin set (not a mock of the subject). |
| 15 tests | 336-601 | REAL | S3 (gap) | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| layer3_header_probes_mp3 | YES | Asserts id `mp3` + confidence 70 on a concrete header. |
| layer2_header_probes_mp2 | YES | Asserts id `mp2`. |
| mpeg25_layer2_probes_mp2 | YES | Version-bits-00 lane asserted. |
| adts_layer00_is_not_mpeg_audio | YES | Layer-00 must not anchor; registry must still say `adts`. |
| structurally_invalid_headers_stay_unprobed | YES (4 shapes) | Free-format, index 15, sampling 3, version reserved, Layer I each asserted as miss. |
| id3_prefixed_frame_anchors_after_the_tag | YES | Asserts mp3 + consumed == window len. |
| oversized_id3_tag_is_a_miss_not_a_guess | YES | 99,682-byte tag vs window → probe.no-demuxer. |
| h264_start_codes_probe_h264 | YES | SPS-kills-HEVC discrimination asserted (0x67 → hevc type 51). |
| hevc_start_codes_probe_hevc | YES | VPS-kills-H.264 asserted (0x40 → h264 type 0). |
| ambiguous_nal_shapes_name_the_family_sub_threshold | YES | Asserts error kind carries `annexb`, score == SCORE_FAMILY_ONLY, 50 < threshold 60. Strong test. |
| mpeg_system_start_codes_stay_unprobed | PARTIAL | Pins the four vectors but not the forbidden-bit-only shape — F-001: the guard's deletion survives this suite green. |
| start_codes_without_parameter_sets_name_nothing | YES | The measured `flip16` false-positive shape (64× `00 00 01 00`) and slice-only window both asserted as miss. |
| flac_marker_probes_flac | YES | Marker + block shape → `flac`, confidence 70. |
| flac_near_misses_stay_unprobed | YES | 4 near-miss shapes each asserted as miss. |
| new_signatures_do_not_steal_other_families | YES | Registry-level: avi/ogg/matroska windows still select their own ids. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "No raw-FLAC demuxer exists — crates/demux/src/es covers h264/hevc/mp3/aac only" | es.rs:64-66 | REAL | ls crates/demux/src/es → aac, annexb, h264, hevc, mp3 (+ error/fuzz/mod/tests); open_flac (containers/es.rs:258-279) is the typed refusal, exactly as documented. |
| "an ADTS frame can never anchor here and an MPEG-audio frame can never anchor there" | es.rs:19-22 | REAL | Layer-bit check es.rs:137-141; test es.rs:364-374 proves both directions. |
| "a window with no parameter set names nothing" | es.rs:47-48 | REAL | Match arms es.rs:255-277 require param evidence; test es.rs:544-557. |
| "the class-B `flip16` AVI derivative carries five of them, all zero heads" (corpus measurement) | es.rs:44-46 | UNVERIFIED | Corpus fixture not present in tree; the behavioral consequence IS tested (es.rs:545). |
| "corpus lanes (`0019-raw-video.h264`, `0044-raw-video.265`)" | es.rs:51 | UNVERIFIED | Corpus dir is a sandbox path (/tmp/my-project/corpus-v4/media, containers/es.rs:298); baseline ran without corpus evidence for these lanes — code-level equivalents (es.rs:431-452) pass. |

## 7. Dead / duplicated / unreachable code
- None. All four signatures are registered (signature/mod.rs:142-145) and every minted
  id has a registry arm or a documented refusal. No duplication found.
- `_meta: &SourceMeta` ignored in all four `probe` impls — a documented design
  decision (content wins, es.rs:56-58), not dead code.

## 8. Dependencies & flags
- No feature gates; media-probe is on the shipped open path (demux_host.rs:444).
  Std-only. No unsafe. `MAX_WALK_STEPS` = 256, `SCORE_MAGIC_ANCHOR` = 70,
  `SCORE_FAMILY_ONLY` = 50 (signature/mod.rs:72,91,93 — read).

## 9. Open questions for the next agent
- F-002: does any corpus file carry an ID3v2.2 tag? (Determines whether the false
  anchor is reachable in the corpus or purely theoretical.)
- The `annexb` family id: confirm no other signature can ever mint it (rg
  `FormatId::annexb` — only this file; containers/mod.rs documents it never reaches
  the registry).
