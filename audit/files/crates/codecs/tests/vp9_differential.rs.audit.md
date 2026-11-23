# AUDIT: crates/codecs/tests/vp9_differential.rs
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Lines / functions | 457 / 4 tests + 7 helpers |
| File verdict | REAL (test-evidence file; executed live with the ffmpeg oracle active — 4/4 pass, all five lanes per-frame bit-exact) |
| Reachable from shipped entry point? | Test file (not shipped); code under test (`Vp9Decoder`) IS shipped: apps/cli/src/pipeline_host.rs:313 `"vp9" => { ... Vp9Decoder::new() }`, feature `codec-vp9` in codecs `default` |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
This is the strongest class of test file in the repo: the admission differential for the shipped VP9 decoder, demuxing real committed IVF fixtures through the landed `demux::ivf`, decoding through the wired codec-api contract, and comparing against a **live** ffmpeg 7.1.5 `libvpx-vp9` oracle (pinned MD5 constants as fallback when ffmpeg is absent — and ffmpeg is present here, so the live branch ran). I executed it: 4/4 tests, all five 8-bit 4:2:0 lanes per-frame byte-identical to ffmpeg, the 10-bit profile-2 lane is 50/50 typed `vp9.bitdepth` refusals with zero fabricated frames, garbage-then-real recovers the full stream, and the off-by-one truncation sweep (125 packets) is panic-free with honestly-printed concealment ("1 concealed-decode, 124 recovered"). Only coverage gaps: profile-1 (4:4:4, the `vp9.layout` gate) is refused by the wrapper but exercised by no lane here, and per-frame identity is only checked on the live branch.

## 2. What actually works (verified)
- `timeout 115 ./target/debug/deps/vp9_differential-4ec38b32b2dc3e15 --test-threads=2 --nocapture` → `4 passed; 0 failed` (21.7s), output shows per-lane `stream MD5 ... == record, 125/125 frames identical` for all five lanes (live oracle active — `/usr/bin/ffmpeg` on PATH).
- The truncation sweep genuinely asserts typed-Ok on every outcome (`Err(err) => unreachable!`, L449) — a panic or untyped error fails the test by construction.

## 3. Findings

F-001 [S4 — profile-1 (4:4:4) refusal gate unexercised by this suite]
Where: vp9_differential.rs:65-96 (LANES), 98-102 (TENBIT_LANE)
Code:
```
const LANES: [Lane; 5] = [
    Lane {
        name: "testsrc2_320x240_125f",
```
What is wrong: the wrapper refuses profile 1 (4:2:2/4:4:4) typed as `vp9.layout` (codecs/src/video/vp9/mod.rs:37-38), but this differential has no profile-1 lane — the only non-profile-0 coverage is profile-2's `vp9.bitdepth`. The `vp9.layout` refusal path is untested here (possibly covered by unit tests in the vp9 module — outside this file; UNVERIFIED).
Confidence: HIGH — Verification: read + rg.

F-002 [S4 — pinned-constant fallback skips per-frame identity]
Where: vp9_differential.rs:308-329
Code:
```
        let mut identical = lane.frames as usize;
        if live {
```
What is wrong: when ffmpeg is absent the whole-stream MD5 is pinned (fine) but the per-frame identity count silently reports `lane.frames` without checking anything, and the per-frame assertion block is skipped — the eprintln then claims "{identical}/{n} frames identical" (L331) which in the no-ffmpeg branch is a fabricated-looking number (it equals the frame count by construction, not by measurement). Cosmetic honesty gap in the no-oracle environment only; this sandbox ran the live branch.
Confidence: HIGH — Verification: read (live branch executed in my run).

## 4. Per-function table (test file — helpers + tests)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Sink / fixture_path / ivf_packets | 105-140 | REAL | — | real demuxer, not a re-implementation |
| decode_all | 145-168 | REAL | — | configure → packets → flush, error codes collected |
| video_bytes | 171-182 | REAL | — | asserts CPU/I420 output shape |
| cached_run | 190-207 | REAL | — | once-per-process staging of all 6 lanes |
| ffmpeg_available / ffmpeg_decode_yuv420p | 210-244 | REAL | — | live oracle; forces `-c:v libvpx-vp9` |
| differential_lanes_vs_ffmpeg_libvpx_vp9 | 246-335 | REAL | S4 | counts/sizes/PTS-ladder/MD5/per-frame — all live-verified |
| tenbit_lane_is_a_typed_refusal_not_a_misdecode | 337-358 | REAL | — | 50/50 typed refusals, no frames |
| hostile_garbage_is_recovered_and_the_stream_survives | 360-413 | REAL | — | full decode after leading garbage |
| off_by_one_truncation_sweep_is_panic_free | 415-457 | REAL | — | 125 truncations, 0 panics |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| differential_lanes_vs_ffmpeg_libvpx_vp9 | YES | whole-stream + per-frame MD5 vs live ffmpeg; wrong pixel/size/pts/count fails |
| tenbit_lane_is_a_typed_refusal_not_a_misdecode | YES | exact refusal code ×50 + zero frames |
| hostile_garbage_is_recovered_and_the_stream_survives | YES | Recovered outcome match + full 125-frame decode after garbage |
| off_by_one_truncation_sweep_is_panic_free | YES | unreachable! on any Err; panics abort the test |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "bit-exact ... whole-stream MD5 equality plus per-frame identity" | vp9_differential.rs:12-14 | TRUE (live) | my run: 5/5 lanes per-frame identical |
| "Lane F ... typed vp9.bitdepth Unsupported refusal — never a mis-decode" | vp9_differential.rs:21-25 | TRUE | 50/50 refusals, 0 frames, live run |
| "engine-level 10-bit bit-exactness ... via tools/vta/vp9" | vp9_differential.rs:24-25, 98-101 | REAL-UNPROVEN | out of this file's scope; tools/vta not run here (UNVERIFIED) |

## 7. Dead / duplicated / unreachable code
None in-file. `identical` fallback value (F-002) is the only dead-ish path (live branch always taken on this host).

## 8. Dependencies & flags
Test-only; uses committed fixtures under tests/fixtures/vp9/, demux::ivf test_support, system ffmpeg as oracle. Code under test rides `codec-vp9` (default on).

## 9. Open questions for the next agent
- vp9 module owner: does any unit test cover the `vp9.layout` (profile-1) refusal? If not, one lane would close F-001.
- tools/vta/vp9 engine-level 10-bit claim — separate admission-record evidence, not re-verified here.
