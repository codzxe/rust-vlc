# AUDIT: crates/demux/src/asf/test_support.rs (file: 283 lines)
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-21 |
| Lines / functions | 283 / 11 units (`mem_stream`, 4 `push_*` appenders, `file_properties_payload`, `audio_stream_properties_payload`, `child`, `minimal_asf_audio`, `waveformatex`, `SimpleIndexSpec` + `IndexEntry`, `asf_with_packets`) |
| File verdict | REAL (test-support surface; the subject it feeds — AsfDemuxer — is UNREACHABLE from shipped entry points, see my signature/mod.rs F-001) |
| Reachable from shipped entry point? | This file: NO by design (test-support; module doc :3-6 says "nothing in the demuxer itself uses it" — verified: consumers are asf/tests.rs (37 builder refs), asf/fuzz.rs:37, and the asf/mod.rs doctest). Its consumers executed green in baseline: 36 `asf::*` tests (:1555+) and the doctest `asf::AsfDemuxer (line 119) ... ok` (:6368). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `Read` full file; GUID verification via python3 over ALL 11 `crates/demux/src/asf/fixtures/*`: the audio stream-type GUID written at :68-72 (`40 9E 69 F8 4D 5B CF 11 A8 FD 00 80 5F 5C 44 2B`) appears in all 8 audio-carrying ffmpeg-generated fixtures (offsets 302/545/557) and the `41 …` variant appears in ZERO — the byte is correct, ffmpeg-pinned; `rg -n "STREAM_TYPE_AUDIO" -A 2 asf/guid.rs` (:152-153 constant matches the builder byte-for-byte — internal consistency confirmed); `rg -n "test_support::" asf/*.rs` (consumers); `sed -n` on every quoted range. |

## 1. One-paragraph verdict
Clean, spec-shaped byte builders: the File Properties payload is the exact
80-byte ASF layout, Stream Properties is the 54-byte-plus-TSD form with the
correct audio and no-error-correction GUIDs, object frames carry GUID+size
with every size field patched to the real totals, and the Simple Index
layout matches the ffmpeg-measured 6-byte entry form. I byte-verified the
audio stream-type GUID against all eleven real ffmpeg-generated fixtures in
the crate — the builder, the demuxer constant (guid.rs:152-153), and every
real fixture agree byte-for-byte (my initial suspicion of a 0x40/0x41
spec-typo was refuted by the fixtures: 0x40 is what ffmpeg writes). The two
findings are leftover cruft: a tautological `debug_assert` plus dead
`let _ =` silencer in `minimal_asf_audio`, and that same builder's FPO
declaring one data packet while its Data Object holds zero — a fixture that
lies to itself about packet count. Context worth restating: this whole
module tests an ASF demuxer that no shipped entry point can reach (the
probe registry has no ASF signature — signature/mod.rs F-001).

## 2. What actually works (verified)
— Byte-layout verification (this audit, python3 over fixtures + manual
layout arithmetic):
  - FPO payload :45-59 = 16+6×8+4×4 = 80 bytes, field order matches ASF
    (File ID, File Size, Creation Date, Packet Count, Play/Send Duration,
    Preroll, Flags, Min/Max packet size, Max bitrate).
  - Header Object :121-132 = GUID+size(8)+count(4)+reserved(2) = 30-byte
    header; size patched as 30+children.
  - Data Object :134-149 = 16+8+16+8+2 = 50-byte fixed part, size patched
    at data_start+16..24.
  - Simple Index :256-276 = 56 + 6n bytes, size patched.
  - WAVEFORMATEX :163-173 = 18 bytes, avg-bytes/sec computed from
    align×rate.
  - Audio stream-type GUID: present verbatim in wma_wmav2_mono8k.wma,
    asf_adpcm_ms.wma, asf_pcm_s16le.wma, wma_wmav1_st44k.wma (offset 302),
    asf_pkt8000.wmv, asf_stream_broadcast.wmv, wmv_wmv2_wma_av.wmv
    (545/557) — 8/8 audio fixtures; the 0x41 variant: 0/11 files.
— All consumers green in baseline: 36 `asf::*` tests, doctest :6368, fuzz
  survival tests.
— `mem_stream` correctly reuses the AVI test seam (one SourceStream
  implementation, not two — the doc's claim).

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/asf/test_support.rs:151-156
Code:
```rust
    let header_size = 30 + children.len();
    let fpo_payload_at = 30 + 24;
    let total = end as u64;
    out[fpo_payload_at + 16..fpo_payload_at + 24].copy_from_slice(&total.to_le_bytes());
    debug_assert_eq!(header_size, 30 + children.len());
    let _ = header_size;
```
What is wrong: `debug_assert_eq!(header_size, 30 + children.len())` compares
a variable against the very expression that defined it two lines up —
tautologically true, verifying nothing. Then `let _ = header_size;` exists
only to silence the unused-variable warning the dead assert introduced.
Leftover from a refactor (an earlier version presumably used `header_size`
in the patch arithmetic).
Why it matters: dead assertions train readers to skip asserts; the file is
explicitly public teaching surface ("rustdoc examples … the fuzz harness
needs deterministic shapes").
What correct looks like: delete both lines (the only real consumer of the
30+24 arithmetic is `fpo_payload_at`, which is used).
Confidence: HIGH — Verification: read; rg confirms `header_size` has no
other use.

### F-002 [S4]
Where: crates/demux/src/asf/test_support.rs:111 vs :145
Code:
```rust
        &file_properties_payload(0x2, 0, 3200, 1),
```
(:145:) `    push_qword(&mut out, 0); // Total data packets`
What is wrong: `minimal_asf_audio` writes the File Properties Object with
Data Packets Count = **1** (fourth field argument) while the Data Object it
immediately appends declares Total data packets = **0** and contains no
packet bytes. The fixture contradicts itself about how many packets exist.
Why it matters: the builder's doc sells "the smallest shape every walk path
must accept" — it silently bakes in an FPO-vs-Data-Object count mismatch as
acceptable input. If the demuxer ever trusts the FPO count (progress
estimation, pre-allocation), every test built on this fixture has already
ratified the lie. `asf_with_packets` (:213) does NOT have this defect — it
passes `packets.len()`.
What correct looks like: `file_properties_payload(0x2, 0, 3200, 0)` — or a
comment explaining the deliberate mismatch if it is one.
Confidence: HIGH — Verification: read of both sites; no comment marks the
mismatch as intentional.
Blast radius: fixture semantics only; current demuxer walks the Data
Object's own count (36 tests green with the mismatch present).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-6 | REAL | — | "nothing in the demuxer itself uses it" verified (consumers: tests/fuzz/doctest only) |
| `mem_stream` | 19-21 | REAL | — | delegates to AVI seam — no duplication |
| `push_guid`/`push_qword`/`push_dword`/`push_word` | 24-41 | REAL | — | trivial LE appenders; correct endianness (verified against fixtures) |
| `file_properties_payload` | 45-59 | REAL | — | exact 80-byte ASF FPO layout; File Size patched later by callers |
| `audio_stream_properties_payload` | 64-88 | REAL | — | 54-byte fixed + TSD; audio + no-EC GUIDs byte-verified vs fixtures and guid.rs constants |
| `child` | 91-97 | REAL | — | GUID+size(24+payload) frame — correct ASF object header |
| `minimal_asf_audio` | 105-158 | REAL | S4 ×2 | F-001 (tautological assert + dead let), F-002 (FPO count 1 vs Data Object 0); patches Data Object size and FPO File Size correctly (offset arithmetic re-derived: 30+24+16..24) |
| `waveformatex` | 163-173 | REAL | — | correct 18-byte WAVEFORMATEX; avg bytes/sec = align×rate |
| `IndexEntry`/`SimpleIndexSpec` | 179-192 | REAL | — | type + spec struct; documented ffmpeg-measured entry form |
| `asf_with_packets` | 201-283 | REAL | — | full builder; every size field patched to real totals (FPO count = packets.len(), Data Object size/count, Simple Index size/count); same correct patch offsets as minimal |

## 5. Test honesty
This file IS test support; its honesty = do the builders build what they
claim? Verified by byte arithmetic (§2) and by the real-fixture GUID
cross-check. The consumers' test-honesty belongs to the tests.rs chunks
(A02/A06/A07).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the ffmpeg-measured 6-byte entry form … the number is non-decreasing" | :176-178 | REAL-UNPROVEN | layout verified against Simple Index bytes in fixtures would need an index-bearing fixture check — none of the 11 fixtures was checked for the index object in this audit (UNVERIFIED: entry monotonicity claim); layout arithmetic consistent |
| "Public … on purpose: rustdoc examples cannot see #[cfg(test)] items, and the fuzz harness needs deterministic shapes" | :3-5 | REAL | doctest (asf/mod.rs:125,136) + fuzz.rs:37 both consume it; both green in baseline |
| "nothing in the demuxer itself uses it" | :5-6 | REAL | rg: zero non-test consumers |
| "the 80-byte layout this unit pinned — see header's module docs" | :43-44 | REAL | arithmetic: 16+48+16 = 80 ✓ |

## 7. Dead / duplicated / unreachable code
- F-001's `header_size`/assert/`let _` trio — dead.
- No duplication: mem_stream delegates; builders are shape-distinct.
- The entire ASF lane (this file's reason to exist) is unreachable from
  shipped entry points — cross-ref signature/mod.rs F-001 (S2).

## 8. Dependencies & flags
- `plugin_api::SourceStream` (workspace); std-only otherwise. Compiled
  unconditionally into the demux crate (pub mod) — production binary
  carries test-support code (documented, deliberate; same pattern as
  avi/wav test_support — a workspace-wide decision, not this file's
  defect).

## 9. Open questions for the next agent
- A02/A06/A07 (asf/tests.rs chunks): do any tests assert the FPO's
  declared packet count is honored (or ignored)? F-002's mismatch should be
  either pinned as intentional or fixed.
- A05 (asf/guid.rs): the STREAM_TYPE_AUDIO constant matches ffmpeg output
  (verified here against 8 fixtures) — but the spec record
  docs/formats/container-asf.md should agree; the card's "[VERIFY-P0] spec
  pin" (matrix row 207 notes) is the place the 0x40-vs-spec question was
  settled — confirm the record says 0x40.
- Whoever audits the ASF fuzz harness: fuzz.rs feeds arbitrary bytes
  through mem_stream — survival only, no correctness oracle (baseline
  `asf::fuzz` tests green).
