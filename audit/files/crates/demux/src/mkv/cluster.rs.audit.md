# AUDIT: crates/demux/src/mkv/cluster.rs
| Field | Value |
|---|---|
| Auditor | W05-A06 |
| Date | 2026-11-23 |
| Lines / functions | 541 / 13 (8 production + 5 test) |
| File verdict | SPLIT — block/timecode machinery REAL (executed); lacing semantics FALSE-as-conformance (count byte inverted vs Matroska/ffmpeg, executed against the repo's own oracle) |
| Reachable from shipped entry point? | YES — `apps/cli/src/containers/mod.rs:57` → `containers/mkv.rs:147 MkvDemuxer::new` → `next_event` → `next_block_packets` (mod.rs:673/740) → `cluster::parse_block_with` / `parse_timecode` / `absolute_ticks`. Desktop lane via `ln::host::CliHost`; web wasm lane does NOT reach mkv (AVI-only wiring). |
| Findings | S0: 0 S1: 1 S2: 1 S3: 0 S4: 4 |
| Commands run | `cargo test -p demux --lib mkv::` → 50 passed; probe6 (`audit/scratch/probe6_mkv_w05a06.rs`) lacing cases executed → fixed/xiph/ebml count-byte 2 (ffmpeg-legal) → `packets=2 sizes=[3,3]` / `[4,12]` / `[3,8]`, damage=0; ffprobe on `fixed_c3.mkv`/`xiph_c3.mkv` → `Error parsing frame sizes`, count=2 variants parse |

## 1. One-paragraph verdict
This file is the root cause of Wave-4's S1 "MKV lacing inversion" (W04-A06's escalation) — CONFIRMED, EXECUTED, and located precisely: all three lacing codings read the count byte as the literal frame count (`frame_count = count as usize` with no `+1`) where Matroska defines it as frames−1, and the size-field counts inherit the same off-by-one (count−1 coded sizes instead of count). ffmpeg — this repo's own designated oracle, executed live in the mkv test suites — rejects the repo-convention encoding outright and parses the spec encoding; feeding the real `MkvDemuxer` spec-legal laced blocks silently misparses them (2 packets instead of 3, wrong sizes, zero damage events), which is frame-boundary corruption on every laced block of every real-world MKV (mkvmerge laces audio by default). The in-tree unit tests pin the inverted semantics with bodies crafted to be valid only under them, so the 50 passing tests protect the bug. Everything else here — the block header grammar (track vint value with marker stripped, i16 BE relative timecode, flag bits), cluster `Timecode` parsing, `absolute_ticks` overflow/negative checks, overrun-as-typed-error — is genuine, careful, and executed; but the file's central spec claim ("the three lacing codings") is inverted, and its BlockGroup-keyframe doc contract ("derived from ReferenceBlock absence by the caller") is false at both ends (see mod.rs F-002, executed `key=[false]`).
## 2. What actually works (verified)
- `cargo test -p demux --lib mkv::` → `test result: ok. 50 passed; 0 failed`.
- probe6 control: 3 MiB unlaced block → exactly one 3,145,728-byte packet — track vint value parse, i16 timecode, flags, `body_offset`, and mod.rs's positional frame re-read all agree byte-exactly.
- probe6 lacing cases execute the codings end-to-end (both count conventions) — the machinery runs; only the semantics are wrong (F-001).
- Read-verified: overrun (`declared > rest`) and fixed-remainder (`!rest.is_multiple_of`) are typed `bad_structure` (pinned by tests `lacing_overrun_is_typed_mismatch`, `fixed_lacing_remainder_is_refused` — both semantics-agnostic and honest); EBML delta bias `(1 << (7·n − 1)) − 1` matches the libmatroska/ffmpeg formula; `parse_vint_value` bounds `at + len ≤ data.len()` before any slice; the manual track-vint loop terminates (a `0x00` head is rejected by `parse_vint_id` first) and `data.len() < after_track + 3` is checked before the body slice.
## 3. Findings
F-001 [S1] — W04-A06 Wave-4 S1 adjudicated: CONFIRMED, root cause here
Where: crates/demux/src/mkv/cluster.rs:258 (Fixed), 287 (Xiph), 320 (EBML); pinned by tests at 471, 487, 499
Code:
```
            let frame_count = body[0] as usize;
```
```
    let frame_count = count as usize;
    sizes.clear();
    sizes.reserve(frame_count.saturating_sub(1));
```
```
    let frame_count = count as usize;
    if frame_count == 0 {
        return Err(0);
    }
```
What is wrong: Matroska's Block lacing defines the count byte as **number of frames − 1** (spec §6.3.6; ffmpeg/libmatroska implement `laces = *data++ + 1`). All three codings use the byte literally and read one fewer coded size than spec (Xiph/EBML: `frame_count − 1` fields instead of `frame_count`; Fixed: `frame_count` frames instead of `frame_count + 1`), so every laced block splits at wrong boundaries and the last "frame" absorbs what are really two frames. Executed evidence (mine, this run): (a) `MkvDemuxer` on ffmpeg-legal blocks (count byte 2 = 3 frames): fixed → `packets=2 sizes=[3, 3]` (expect 3×[2,2,2]), xiph → `[4, 12]` (expect [4,5,6]), **ebml → `[3, 8]` (expect [3,3,4] — the EBML coding was not in W04-A06's executed set)** — all with `recovered=0 damage=0`: silent corruption; (b) ffmpeg 7.1.5 on the byte-identical fixtures: `fixed_c3.mkv` and `xiph_c3.mkv` (repo convention, count=3 for 3 frames) → `Error parsing frame sizes`; the count=2 variants parse the lace. (c) W04-A06's independent execution (probe5) reproduced the same fixed/xiph numbers. Why it matters: laced audio is the mkvmerge default — ordinary files misparse with zero damage signal; the same bug made the test fixture (mkv_fixtures.rs) spec-invalid, and both in-tree test layers pin the inversion (below). What correct looks like: `let frame_count = count as usize + 1;` in all three (and the EBML/Xiph size loops read `frame_count − 1` fields — which then equals the count byte), tests rewritten with count bytes 2/2/2 expecting 3 frames, mkv_fixtures.rs `laced_fixture` rewritten in lockstep (W04-A06's open item). Confidence: HIGH — Verification: ran (probe6 + ffprobe, outputs above).

F-002 [S2] (cross-file with mod.rs F-002 — counted there and here; one defect, two lying docs)
Where: crates/demux/src/mkv/cluster.rs:58-59 (contract), 217 (value)
Code:
```
    /// The keyframe flag (SimpleBlock bit 0x80; BlockGroup: derived
    /// from `ReferenceBlock` absence by the caller).
```
```
        keyframe: simple && flags & FLAG_KEYFRAME != 0,
```
What is wrong: for BlockGroup blocks `parse_block_with` always yields `keyframe: false`, and the promised caller-side derivation does not exist in `mod.rs` (the BlockGroup walk never inspects `ID_REFERENCE_BLOCK`). Executed: BlockGroup block without ReferenceBlock → packet `key=[false]`; identical bytes as SimpleBlock with the keyframe bit → `key=[true]`. Every BlockGroup packet in the shipped demuxer is non-keyframe. Confidence: HIGH — Verification: ran.

F-003 [S4]
Where: crates/demux/src/mkv/cluster.rs:132-135
Code:
```
            let value =
                ebml::uint_from_be(&bytes).ok_or_else(|| error::bad_structure(child.data_pos))?;
            return Ok(ClusterTimecode {
                ticks: value as i64,
```
What is wrong: `value as i64` wraps a u64 timecode above i64::MAX to negative — no panic, but the cluster then fails per-block in `absolute_ticks` (`sum < 0` → typed `timecode_overflow`) and is damage-skipped, instead of being classified at parse (a `i64::try_from` would mint the same typed error at the honest site). Blast radius: >292-year cluster timestamps only. Confidence: HIGH — Verification: read (wrap behavior is the documented `as` semantics; not executed — crafting a >i64::MAX timecode file served no additional proof beyond the read).

F-004 [S4]
Where: crates/demux/src/mkv/cluster.rs:241-252 vs 233-239
Code:
```
        Lacing::Xiph => {
            let (frame_count, consumed) = xiph_sizes(body, &mut scratch.sizes)?;
            if frame_count == 0 {
                return Err(consumed);
            }
```
```
        Lacing::Ebml => {
            let (frame_count, consumed) = ebml_sizes(body, &mut scratch.sizes)?;
```
What is wrong: degenerate count byte 1 behaves differently per coding under the current semantics: Xiph → 1 frame (zero coded sizes, remainder-only), EBML → 2 frames (first coded size + remainder). Under spec semantics (count+1 → 2 frames) both agree; as shipped the two codings disagree on the same byte. Footnote to F-001 — resolves itself under the `+1` fix. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/demux/src/mkv/cluster.rs:201-210; 155-161
Code:
```
        3 => Lacing::Ebml,
        // lacing_bits is a 2-bit field (0..=3); the arm is unreachable
        // by construction — refused as a malformed block, never a
        // panic (dirty-world posture, 8.A1 sweep).
        _ => return Err(bad(data.len())),
```
What is wrong: the `_` arm is dead (a 2-bit field cannot exceed 3 — the comment admits it), and `parse_block` (the non-scratch wrapper) has no production caller (mod.rs uses `parse_block_with`; rg shows only this file's tests). Neither harms; both are the hygiene debt the sweep blessed. Confidence: HIGH — Verification: read + rg.

F-006 [S4]
Where: crates/demux/src/mkv/cluster.rs:396, 413
Code:
```
    let declared: usize = sizes.iter().sum();
```
```
        at += size;
```
What is wrong: on a 32-bit-usize target (wasm32) the sum of coded sizes (each ≤ body.len()+255 after the continuation guard, up to 254 of them ≈ 8 GiB) overflows u32 — in release it wraps, making `declared` small and the "last = remainder" frame huge/wrong instead of the typed overrun error; `at += size` can likewise wrap frame starts. Latent only: the shipped wasm lane does not reach mkv (apps/web wires AVI exclusively); native 64-bit lanes cannot overflow (≤ 2^33). Confidence: MEDIUM — Verification: read (bound arithmetic; not executed on a 32-bit target — no wasm runtime in sandbox).
## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FLAG_KEYFRAME / FLAG_LACING_MASK consts | 26-28 | REAL | — | mask 0x06 >>1 = spec bits 1..2 (verified against spec table + probe encodings) |
| BlockFrame / BlockScratch / ParsedBlock | 31-75 | REAL | — | scratch-recycle model (5.X2b) genuine, executed via control case |
| Lacing enum | 78-88 | REAL code, FALSE doc | S1 | coding values correct; the implemented count semantics are not the spec's (F-001) |
| ClusterTimecode | 91-97 | REAL | — | |
| parse_timecode | 108-143 | REAL | S4 | unknown-size child refused; size ≤ 8; absence typed; `as i64` wrap (F-003) |
| parse_block | 155-161 | REAL, test-only | S4 | no production callers (F-005) |
| parse_block_with | 167-224 | REAL | S1/S2 | header grammar correct (executed); lacing dispatch feeds F-001; keyframe contract broken (F-002); dead `_` arm (F-005) |
| lace_frames | 229-278 | FALSE-as-conformance | S1 | Fixed count inversion (F-001); exact-division and zero-count guards typed and honest |
| xiph_sizes | 283-311 | FALSE-as-conformance | S1 | count inversion + one-fewer sizes (F-001); continuation guard sane; terminator overflow guarded |
| ebml_sizes | 316-357 | FALSE-as-conformance | S1 | count inversion (F-001); first-size +1 correct; delta bias formula correct; negative size refused |
| parse_vint_value | 360-384 | REAL | — | len cap 8, bounds checked before slice |
| build_frames | 389-420 | REAL | S4 | overrun typed; remainder rule; 32-bit sum latent (F-006) |
| absolute_ticks | 428-436 | REAL | — | checked add + negative refusal (executed by test + every probe block) |
| tests (8) | 453-541 | 3 INVERTED / 5 honest | S1 | see §5 |
## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| no_lacing_single_frame | YES | grammar + offsets asserted byte-exactly |
| xiph_lacing_splits_and_remainder_last | **INVERTED** | body `vec![3, 5, 0xFF, 0x2D]` + expects 3 frames — valid only under the inverted semantics; a correct `+1` fix (4 frames: [5,300,160,0-remainder]) fails it; the test body is spec-degenerate under the fix |
| fixed_lacing_divides_exactly | **INVERTED** | `[4u8, …8 bytes]` expects 4 frames; spec says 5 (and 8 % 5 ≠ 0 → the same body is a spec parse error) |
| ebml_lacing_sizes_decode | **INVERTED** | `vec![3, 0x82, 0xBF]` expects 3 frames; under spec (4 frames) the third size vint (0xAA) decodes a −21 delta → invalid — the body only parses under the bug |
| lacing_overrun_is_typed_mismatch | YES | declared > rest → typed code asserted; semantics-agnostic |
| fixed_lacing_remainder_is_refused | YES | non-dividing payload refused; semantics-agnostic |
| short_block_header_is_typed | YES | exact code asserted |
| absolute_ticks_checks_the_timeline | YES | negative + i64::MAX overflow both asserted |
Mutation check (no edit performed): flipping the three `frame_count` bindings to `+ 1` fails exactly the three INVERTED tests — the suite certifies the inversion.
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`cluster` — `Cluster`/`SimpleBlock`/`BlockGroup` + the three lacing codings (Xiph, EBML, fixed)" | mkv/mod.rs:9-10 | REAL code / FALSE-as-conformance | codings exist and run; count semantics inverted vs the repo's own oracle (F-001, executed) |
| "Lacing mismatches (frame-count vs data, a Xiph size overrun, a final frame shorter than declared) are typed bad_structure" | cluster.rs:11-14 | PARTIAL | overrun/remainder: REAL (tests pass); "frame-count vs data" mismatches are NOT detected under the inversion — spec-legal counts silently misparse (F-001, damage=0); a "final frame shorter than declared" cannot occur (last is computed as remainder, possibly 0-length and emitted) |
| "Relative timecodes are i16 on disk; the absolute stamp is cluster + relative … checked against the timeline" | cluster.rs:15-18 | REAL | absolute_ticks + test |
| "BlockGroup: derived from ReferenceBlock absence by the caller" | cluster.rs:58-59 | FALSE | no caller derives it; executed key=[false] (F-002) |
| "EBML vint-coded sizes (first frame size − 1 encoded)" | cluster.rs:84-85 | REAL | `checked_add(1)` at 332 matches; delta bias matches ffmpeg |
## 7. Dead / duplicated / unreachable code
- `parse_block` wrapper: test-only (production path is `parse_block_with`).
- `_ => return Err(bad(...))` lacing arm: unreachable by construction (documented).
- `Lacing` discriminant is carried in `ParsedBlock.lacing` but no production consumer branches on it after `lace_frames` (diagnostics only) — harmless.
## 8. Dependencies & flags
- Depends only on `media_types` (EngineError) + in-crate `ebml`/`error`. No feature gates, no unsafe, no unwrap/expect (testkit `Must` is test-only). usize-width hazards noted at F-006 (wasm32 latent).
## 9. Open questions for the next agent
- The fix must land in lockstep: cluster.rs (+1 ×3), the three inverted unit tests, and mkv_fixtures.rs's `laced_fixture`/two tests (W04-A06 F-001) — any partial fix turns green tests red and vice versa.
- Does the corpus differential lane (machine-local, corpus-v4) contain any mkvmerge-authored laced file? If yes, its packet-count oracle must already disagree with this demuxer — check `tests/mkv_corpus.rs` lane results when the corpus is present (W04-A08 marked the lane REAL-UNPROVEN on this machine).
- `emit_block`'s 16 MiB frame drop (mod.rs F-004) interacts with F-001's fix: under spec counts a lace can legally carry more, larger frames — re-check the ceilings together.
