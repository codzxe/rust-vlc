# AUDIT: crates/codecs/tests/fixtures/av1/tenbit_128x96_5f.ivf (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A02 |
| Date | 2026-11-23 |
| Type / size / sha256 | Duck IVF, codec AV01, 128x96, 5 frames; ffprobe: av1 Main, yuv420p10le (10-bit) / 897 bytes / 3087967d1b26820f002c564a231e6511538c486235e9598981cb6fee2fd36b68 |
| File verdict | REAL (typed-refusal boundary evidence, executed) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `file` -> Duck IVF AV01 128x96 5 frames; `ffprobe` -> av1/yuv420p10le/5 frames; `sha256sum`; `./target/debug/deps/av1_differential-745f0c9fef4e31ca tenbit --nocapture` -> 1 passed / 0 failed, 0.07s |

## 1. Verdict
Real 10-bit AV1 IVF fixture used as an honest BOUNDARY probe, not a decode lane: the consuming test (`tenbit_fixture_is_a_typed_refusal_not_a_misdecode`, av1_differential.rs:253-289) feeds all 5 packets through the real `Av1Decoder` and asserts every one refuses with `Unsupported` and code `av1.bitdepth`, that the refusal count is exactly 5, and that the sink stays empty (no fabricated frames). I ran it — pass. This is the correct use of a beyond-v1 fixture: pin the typed refusal site so a 10-bit stream can never misdecode silently.

## 2. Test usage (verified by execution)
`./target/debug/deps/av1_differential-745f0c9fef4e31ca tenbit --nocapture` -> **1/1 pass** (0.07s). Assertions are exact: first error code string equality ("av1.bitdepth"), refused == 5, sink.frames empty, kind == Unsupported. Mutation of the decoder's bit-depth gate in either direction (silent decode, or wrong error class/code) fails this test.

## 3. Matrix evidence
Backs the av1 decoder row's "10-bit refuses typed (av1.bitdepth), never a mislabel" claim. ffprobe independently confirms the fixture is genuinely 10-bit (yuv420p10le), so the boundary being pinned is the real one. No inflation for this asset.
