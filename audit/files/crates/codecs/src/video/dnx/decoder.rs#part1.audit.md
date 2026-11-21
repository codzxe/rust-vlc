# AUDIT: crates/codecs/src/video/dnx/decoder.rs#part1 (lines 1–557)
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-21 |
| Lines / functions | 557 (of 1068) / 23 |
| File verdict | UNREACHABLE (real, differentially-tested code; no shipped entry point can produce its input) |
| Reachable from shipped entry point? | NO end-to-end — `DnxDecoder` is wired at apps/cli/src/pipeline_host.rs:326, but NO demuxer/probe/container lane in the tree mints the `dnxhd` descriptor (`rg "dnxhd" crates/demux/src crates/media-probe/src apps/cli/src` → only the pipeline arm + its comment). The arm's own comment admits it (see F-001). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "dnxhd|dnx" apps/cli/src crates/demux/src crates/media-probe/src` → zero descriptor producers; `sed -n` quote verification (decoder.rs 97-106, 306-321, 369-385; pipeline_host.rs 318-327); baseline: test-workspace.log.txt:535-544 (unit tests ok), :934-942 + test-workspace-allfeatures.log.txt:893-900 (dnx_differential 5 lanes + truncation ok); `rg -n "dnx" docs/feature-matrix.toml` |

## Chunk boundary note
Assigned part1 ≈ lines 1–534 of 1068; extended to 557 so `pack_into` (527–557) is not split mid-function. Part2 (W06-A02) = 558–1068: `packed_slot`, `impl Decoder` (configure/decode/flush), `mod tests`. Cross-file facts I verified for context but do not audit: configure resets `pictures = 0` (line 612), trait `decode` emits exactly one frame per packet.

## 1. One-paragraph verdict
This is the filmcraft-dnx wrapper's first half: engine-option pinning, the CBR truncation gate (the admission record's hostile flag answered with a typed `Truncated` refusal carrying exact expected/found byte counts), the full-depth `DnxPicture` mirror, and a typed-refusal output classifier that refuses every output the 8-bit pixel vocabulary cannot express losslessly (10/12-bit, 4:4:4, RGB, alpha, non-square PAR, BT.2020-CL) instead of silently downshifting. The code is genuine and it is the best-tested decoder in this wave's assignment: the 5-lane differential against a live ffmpeg oracle plus a real truncated-frame test all ran and passed in the baseline workspace runs. It is nonetheless UNREACHABLE from any shipped entry point: nothing in the tree produces a `dnxhd` track descriptor, so the `lumen` CLI can never route a file to it — the code's own wiring comment says so. Two real quality defects (per-frame full-depth allocation on the trait path; OutOfBand color hard-fill) are recorded below.

## 2. What actually works (verified)
— CBR truncation gate + typed refusals + conservative color mapping: `video::dnx::decoder::tests::{cbr_gate_refuses_short_packets_with_exact_numbers, color_volumes_map_conservatively, inexpressible_outputs_are_typed_refusals, plane_geometry_mismatch_surfaces_as_corrupt_data}` all `ok` in baseline run #3 (test-workspace.log.txt:535-544).
— Full differential vs ffmpeg 7.1.5: `dnx_differential` lanes L1–L5 (incl. 10-bit lane via the full-depth seam) + `truncated_real_frame_refuses_typed` all `ok` in baseline runs #3 and #4 (test-workspace.log.txt:934-942, test-workspace-allfeatures.log.txt:893-900). Lane integrity is pinned by committed per-lane MD5s of ffmpeg's own decode, tolerance ±2 LSB with sign-balance gates — these tests fail if the wrapper, packer, or engine drifts.
— Pack index math hand-verified: 4:2:2 YUYV macropixel walk `out[(row*cw+c)*4 .. +3]` peaks at exactly `w*h*2-1` = the classified slot length; plane bounds `y[h*w-1]`, `cb[h*cw-1]` (read, arithmetic checked).

## 3. Findings

### F-001 [S3]
Where: apps/cli/src/pipeline_host.rs:318-327 (the consumption point), crates/codecs/src/video/dnx/decoder.rs:269-271 (the gate)
Code:
```rust
            // raw VC-3 ES decodes statelessly. No container lane
            // mints the `dnxhd` descriptor yet (the mkv V_MS/FOURCC
            // and ASF futures own that) — the codecs-crate
            // differential is the lane; the decoder's `configure`
            // gate is the honest authority (the only accepted
            // descriptor, `dnxhd`).
            "dnxhd" => {
```
What is wrong: the decoder arm exists in the shipped CLI, but `rg` proves no demuxer, no probe signature, no container lane (`apps/cli/src/containers/*`, `crates/demux/src/*`, `crates/media-probe/src/signature/*`) ever emits the string `dnxhd` as a track codec descriptor. A user opening any DNxHD/VC-3 file — including a raw `.dnxhd` ES — cannot reach this decoder: the open path fails earlier with a container/codec-unknown error.
Why it matters: the feature-matrix row `codec-video-prores-dnxhd` (docs/feature-matrix.toml:581-591) carries `status = { windows = "partial", linux = "partial", macos = "partial" }` on evidence that is exclusively codec-crate tests; per this audit's rules a row is REAL only when a shipped path shows it working. Users reading "partial" can reasonably expect the shipped player to decode a DNxHD file; it cannot.
What correct looks like: either an ES/MP4/MKV lane that mints `dnxhd`, or matrix status cells that say not_started/blocked until a container lane exists (the row's own notes admit "MOV/MXF sample-entry plumbing is later-wave work").
Confidence: HIGH — Verification: read + rg producers + baseline logs.

### F-002 [S3]
Where: crates/codecs/src/video/dnx/decoder.rs:297-301, 341-356 (consumed by the trait `decode` at 629-631, part2)
Code:
```rust
    pub fn decode_picture(&mut self, data: &[u8]) -> Result<DnxPicture, EngineError> {
        let picture = decode_picture_ungated(data)?;
        self.pictures = self.pictures.saturating_add(1);
        Ok(picture)
    }
```
What is wrong: the trait path decodes every frame at full native depth into fresh owned `Vec<u16>` planes (`frame.y/cb/cr/alpha`, lines 341-356), packs into a pooled slot, then drops the planes. The `FRAME_SLOTS` pooling (module docs "Allocation discipline (D-1)", lines 102-106) covers only the packed output — the dominant allocation (≈ 1.5–3 × w×h×2 bytes per frame, ~6 MB transient at 1080p 4:2:2) is per-frame heap churn the pooling posture claims to prevent.
Why it matters: "steady-state decodes reuse a small rotation instead of allocating per frame" (module docs :77-79) is only true for the small packed buffer; the doc oversells the allocation discipline. Real but non-fatal, and unreachable today (F-001) — S3.
What correct looks like: a plane pool (reuse `Vec<u16>` allocations across frames via capacity-retaining reuse) or an engine API that decodes into caller-provided buffers.
Confidence: HIGH — Verification: read (decode_frame_with returns owned planes; no plane pooling anywhere in 1-557 or the trait path).

### F-003 [S4]
Where: crates/codecs/src/video/dnx/decoder.rs:369-385
Code:
```rust
/// ... `OutOfBand`
/// means the container was supposed to describe the volume — the
/// same conservative BT.709/limited fill, documented rather than
/// guessed per-frame. ...
        DnxColorVolume::Bt709 | DnxColorVolume::OutOfBand => Ok(ColorDescription {
            primaries: ColorPrimaries::Bt709,
```
What is wrong: `OutOfBand` exists precisely because the container owns the color description, but the wrapper hard-fills BT.709/limited and `configure` (part2, 607-609) "deliberately" consults no `extra_data` — the container's actual description is neither plumbed nor plumbed-able through this signature.
Why it matters: an OutOfBand-coded stream mastered as full-range or BT.601 gets mislabeled; the renderer converts per the wrong description. Documented and conservative (ffmpeg does the same default), hence S4 not S3.
What correct looks like: pass the container's color metadata through `DecoderConfig::extra_data` (or a future color side-channel) and honor it for OutOfBand.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/codecs/src/video/dnx/decoder.rs:309-316
Code:
```rust
    if let Ok(header) = filmcraft_dnx::probe(data)
        && let Some(unit) = header.coding_unit_size()
    {
        let units: u64 = if header.interlaced && !header.frame_encoding {
            2
```
What is wrong: the ×2 field-unit rule rides on the external engine's `interlaced`/`frame_encoding` probe semantics; the filmcraft-dnx crate source is not in this repo, so the mapping "interlaced && !frame_encoding ⇒ two coding units" is trusted from the admission record, not traced.
Why it matters: if the semantic is inverted for some header shape, the gate under- or over-requires bytes (false truncation refusals or a reopened concealment window). The differential's truncation test covers one real frame shape only.
What correct looks like: an in-repo unit test enumerating the four (interlaced, frame_encoding) combinations with hand-built headers pinning the units arithmetic.
Confidence: MEDIUM — Verification: UNVERIFIED (external crate semantics; record docs/dependencies/records/filmcraft-dnx-0.9.0.md is the claimed source of truth).

## 4. Per-function table (part1: lines 1–557)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-79 | REAL | — | claims cross-checked below (§6) |
| `ENGINE_OPTIONS` const | 97-100 | REAL | — | pins threads:false, spec_dequant:false per record |
| `FRAME_SLOTS` const | 106 | REAL | — | 4 packed slots |
| `DnxChroma` enum | 111-118 | REAL | — | Yuv420/Yuv422/Yuv444 |
| `DnxChroma::chroma_width` | 123-128 | REAL | — | div_ceil(2), 444 full |
| `DnxChroma::chroma_height` | 132-137 | REAL | — | 420 half, else full |
| `DnxColorVolume` enum | 142-151 | REAL | — | mirror of CLV |
| `DnxPicture` struct | 162-193 | REAL | — | full-depth u16 planes + rgb/alpha/par/cid |
| `DnxPicture::chroma_width/height` | 198-206 | REAL | — | delegates |
| `DnxDecoder` struct | 243-256 | REAL | — | derive(Default); generation/codec/slots/slot_next/pictures |
| `DnxDecoder::new` | 261-263 | REAL | — | |
| `DnxDecoder::accepts` | 269-271 | REAL | — | only `dnxhd` |
| `DnxDecoder::decoded_pictures` | 275-277 | REAL | — | counter reset at configure (612, part2) — consistent |
| `DnxDecoder::decode_picture` | 297-301 | REAL | S3 | F-002 allocation; counter only on success |
| `decode_picture_ungated` | 306-367 | REAL | S4 | F-004 field semantics; CBR gate u64-safe (no narrowing) |
| `color_description` | 378-398 | REAL | S4 | F-003 OutOfBand fill; Bt2020Cl typed refusal |
| `PackedOutput` struct | 401-406 | REAL | — | |
| `packed_output` | 411-485 | REAL | — | typed refusals for 10/12-bit, rgb, alpha, par≠(0,0)/(1,1), 444, odd 422 width; checked_mul/add |
| `limit_error` | 490-496 | REAL | — | defensive, documented unreachable |
| `validate_planes` | 501-522 | REAL | — | exact plane↔raster match, typed Corrupt on mismatch |
| `pack_into` | 527-557 | REAL | — | YUYV macropixel walk index-verified; I420 plane copy; `as u8` gated by bit_depth==8; Yuv444 arm empty (refused earlier) |
| doctest (struct docs) | 220-242 | REAL | — | asserts foreign-descriptor typed error; ran in baseline doctest pass |

## 5. Test honesty (part1-relevant tests; bodies in part2 + tests/dnx_differential.rs)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| tests::cbr_gate_refuses_short_packets_with_exact_numbers | YES | asserts exact expected/found numbers in the typed error |
| tests::inexpressible_outputs_are_typed_refusals | YES | pins every refusal arm of `packed_output` |
| tests::plane_geometry_mismatch_surfaces_as_corrupt_data | YES | feeds wrong-size planes, expects typed Corrupt |
| tests::color_volumes_map_conservatively | YES | pins `color_description` outputs |
| dnx_differential::lane_l1..l5 (ffmpeg oracle) | YES | per-plane ±2-LSB tolerance + MD5 lane integrity + byte-count parity — fails on any decode/pack drift |
| dnx_differential::truncated_real_frame_refuses_typed | YES | real half-truncated frame must produce `codecs.dnx.frame.short`, not Ok |
All ran `ok` in baseline runs #3/#4 (cited in header). Differential skips only when the ffmpeg binary is absent — it was present (lanes ran, not skipped).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "every packet is first probed; … shorter than the declared size (×2 for field-coded) ⇒ typed Truncated" | decoder.rs :34-49 | REAL | code 306-339; tests ok in baseline |
| "maps the engine's planar output losslessly where the vocabulary allows and refuses typed everywhere else" | decoder.rs :51-67 | REAL | `packed_output` 411-485 refusal table matches doc table 1:1 |
| "Packed output refills pooled SharedBytes slots … steady-state decodes reuse a small rotation instead of allocating per frame" | decoder.rs :75-79 | SHALLOW | pooling covers packed bytes only; full-depth planes re-allocated per frame (F-002) |
| "single-threaded … any budget is accepted and decode stays single-threaded" | decoder.rs :20-22 | REAL | ENGINE_OPTIONS threads:false (97-100); threads feature compiled out (Cargo.toml:158 default-features=false) |
| matrix row `codec-video-prores-dnxhd` status partial (win/linux/mac) | docs/feature-matrix.toml:589 | FALSE (for shipped reachability) | no container mints `dnxhd` (F-001); evidence is codec-crate tests only; row notes admit plumbing is "later-wave work" |

## 7. Dead / duplicated / unreachable code
The whole `dnx` module is production-unreachable (F-001): wired in the CLI match arm but no descriptor producer. No intra-part1 dead code found; every function is consumed by the trait path or the public full-depth seam.

## 8. Dependencies & flags
`filmcraft-dnx = { version = "=0.9.0", optional = true, default-features = false }` behind feature `codec-dnx` (crates/codecs/Cargo.toml:67, 158); `codec-dnx` is in `default` (line 51) → compiled into shipped builds, dead weight until a container lane exists. No unsafe in this file (crate-wide no-unsafe convention held).

## 9. Open questions for the next agent
- W06-A02 (part2): does the engine/pipeline escalate a per-frame decode `Err` (e.g. `codecs.dnx.frame.short`) to a session-fatal Error, or conceal-and-continue? The module doc promises "the decoder stays usable" — the ENGINE's behavior on Err is the open half.
- Container-lane owner: F-001 blocks any user-visible DNxHD playback; the mkv V_MS/FOURCC or an ES lane is the prerequisite.
- A02: `pack_into`'s Yuv444 arm is an empty body — confirmed unreachable (packed_output refuses 444 at 479-483 before any packing), so the empty arm is safe but deserves a debug_assert.
