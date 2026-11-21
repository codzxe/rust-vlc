# AUDIT: crates/demux/src/mkv/cluster.rs
| Field | Value |
|---|---|
| Auditor | W05-A02 |
| Date | 2026-11-21 |
| Lines / functions | 541 / 25 (9 free fns, 5 types, 2 consts, 1 test helper, 8 tests) |
| File verdict | REAL (with one empirically-proven spec off-by-one in the lacing count) |
| Reachable from shipped entry point? | YES — `lumen open file.mkv` → `apps/cli/src/host.rs:904 open_source` → `containers/mod.rs:57 "matroska" => mkv::open` → `apps/cli/src/containers/mkv.rs:147 MkvDemuxer::new` → `mkv/mod.rs:706,740 cluster::parse_block_with`, `:515,874 parse_timecode`, `:782 absolute_ticks` (rg-verified callers). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 3 |
| Commands run | `rg -n "parse_timecode\|parse_block\|absolute_ticks" crates/demux/src/mkv/mod.rs` (call sites); `rg -n "MkvDemuxer::new" apps/cli/src/containers/mkv.rs` (wiring); `rg -n "mkv::cluster::tests" audit/runs/baseline/test-workspace.log.txt` (8/8 ok, lines 1775-1782); **empirical lacing probe**: `python3 /tmp/lace_probe.py` + `ffprobe -count_packets` (ffmpeg 7.1.5) over 4 hand-built one-block MKVs in /tmp — see F-001; `sed -n` verification of every quote. |

## 1. One-paragraph verdict
The Cluster layer is carefully engineered — bounded arithmetic everywhere, typed errors carrying absolute offsets, damage-not-refusal posture, scratch-buffer recycling — and every unit test runs green in baseline. But the single most important field it parses is read wrong: the lacing frame-count byte is the **number of frames minus one** per the Matroska spec (and per ffmpeg/libmatroska, the repo's own declared reference), and this code reads it as the absolute count. I proved it empirically: four one-block MKVs I built in /tmp, probed with the sandbox's ffprobe 7.1.5 (the same version the repo's corpus and baseline used), yield 3 packets for a count byte of 2 and 1 packet for a count byte of 0 — this code produces 2 frames (merging the last two) and refuses the count-0 lace respectively. Every real-world laced block (mkvmerge/libmatroska muxers lace audio by default) is mis-framed. The repo cannot see the bug because its ffmpeg-authored corpus contains zero laced blocks — a fact the fixture suite itself admits — and the hand-built lacing fixture bakes in the same off-by-one.

## 2. What actually works (verified)
— Bounds/overflow discipline, read: `xiph_sizes` guards the 255-continuation accumulation against `body.len()` (cluster.rs:304-306); `ebml_sizes` uses `checked_add`/`try_from`/negative checks (332-354); `build_frames` refuses `declared > rest` before subtracting (399-404); `absolute_ticks` uses `checked_add` and rejects negatives (429-435). No panics reachable from parsed sizes.
— All 8 unit tests ran green in baseline: `rg "mkv::cluster::tests" audit/runs/baseline/test-workspace.log.txt` → lines 1775-1782, all `ok`.
— Reachability proven by call chain (header table); the parse paths run on every `.mkv/.webm/.mka` open in the shipped CLI.
— Fixed/EBML/Xiph *damage* paths (overrun, non-dividing fixed payload, short header) are each typed and tested (`lacing_overrun_is_typed_mismatch`, `fixed_lacing_remainder_is_refused`, `short_block_header_is_typed` — baseline lines 1779, 1778, 1781).

## 3. Findings
### F-001 [S2]
Where: crates/demux/src/mkv/cluster.rs:254-262 (Fixed), 283-291 (Xiph), 316-323 (EBML)
Code (Xiph head; the other two are identical in shape):
```
fn xiph_sizes(body: &[u8], sizes: &mut Vec<usize>) -> Result<(usize, usize), usize> {
    let Some(&count) = body.first() else {
        return Err(0);
    };
    let frame_count = count as usize;
    sizes.clear();
    sizes.reserve(frame_count.saturating_sub(1));
    let mut at = 1usize;
    for _ in 0..frame_count.saturating_sub(1) {
```
and Fixed:
```
            let frame_count = body[0] as usize;
            let consumed = 1usize;
            if frame_count == 0 {
                return Err(0);
            }
```
What is wrong: The Matroska lacing count byte is the number of frames **minus one** (spec: "Number of frames in the lace minus one"; ffmpeg `matroska_parse_laces` computes `laces = *data + 1`; libmatroska `number_frames = count + 1`). This code treats the byte as the absolute frame count: it reads `frame_count − 1` coded sizes (should be `(count+1) − 1 = count`), produces `count` frames instead of `count + 1`, and refuses the legal degenerate lace `count == 0` (which is one frame) as `bad_structure`.
Empirical proof (run by me, 2026-11-21, sandbox ffmpeg 7.1.5-0+deb13u1 — the same 7.1.x line as the baseline corpus): four one-block PCM MKVs built in /tmp with the repo's own fixture grammar, probed with `ffprobe -v error -show_streams -count_packets -of json`:
- Fixed, flags 0x84, count byte 2, 6 payload bytes → ffprobe `nb_read_packets: 3` (3×2-byte frames). This code: 2 frames of 3 bytes.
- Xiph, flags 0x82, count byte 2, sizes [4,5], 14 payload bytes → ffprobe: 3 packets ([4,5,5]). This code: 2 packets ([4,9] — last two frames merged).
- EBML, flags 0x86, count byte 2, first size 0x82 (→3), delta 0xBF (→3), 9 payload bytes → ffprobe: 3 packets ([3,3,3]). This code: 2 packets ([3,6]).
- Fixed, count byte 0, 6 payload bytes → ffprobe: 1 packet. This code: typed `bad_structure` refusal of a legal block.
Why it matters: every laced block in a real-world Matroska/WebM file (mkvmerge, libmatroska and WebM muxers lace audio by default) is mis-framed: N frames delivered as N−1 packets with the last two merged (Xiph/EBML) or wrong sizes (Fixed). `emit_block` (mod.rs:765+) mints one `Packet` per `BlockFrame`, so frame boundaries, packet counts and per-frame cadence timestamps are all wrong downstream — mis-framed packets feed decoders garbage boundaries (odd-sized PCM frames, split H.264/VP9 access units). Packet counts also silently disagree with the repo's own "ffprobe truth" standard. The repo cannot catch this: `crates/demux/tests/mkv_fixtures.rs:2-5` states ffmpeg's muxer "writes zero laced blocks in every file we generate — measured", so the corpus differential (`corpus_matroska_differential_vs_ffprobe ... ok`, baseline test-workspace.log.txt:2141) exercises zero laced blocks, and the hand-built laced fixture (mkv_fixtures.rs:258-276) writes count byte 3 for 3 frames — the same off-by-one as the parser.
What correct looks like: `let frame_count = count as usize + 1;` in all three codings, sizes loop over `frame_count − 1` entries, `count == 0` accepted as a one-frame lace (whole remainder), and the unit tests + hand-built fixture re-based on spec-conformant bytes (count byte = frames − 1) plus one ffmpeg-verified golden.
Confidence: HIGH — Verification: ran (ffprobe 7.1.5 packet counts above) / read (spec convention as implemented by ffmpeg+libmatroska, the repo's declared reference at cluster.rs:340-343 "the bias libmatroska/ffmpeg share").

### F-002 [S4]
Where: crates/demux/src/mkv/cluster.rs:27-28
Code:
```
/// Lacing selector: bits 0..=1 of the flags byte.
pub const FLAG_LACING_MASK: u8 = 0x06;
```
What is wrong: The doc comment is false: mask `0x06` selects bits 1..=2 (LSB-numbered), not "bits 0..=1" (which would be `0x03`). The `Lacing` enum's own doc at line 77 ("bits 1..=0 → value") is equally garbled.
Why it matters: the mask itself is correct (matches ffmpeg's `(flags >> 1) & 0x03`); the documentation misleads the next implementer into shifting the wrong field — precisely the kind of drift that produces bugs like F-001.
What correct looks like: "bits 1..=2 of the flags byte (value after `>> 1`: 0 none, 1 Xiph, 2 fixed, 3 EBML)".
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/demux/src/mkv/cluster.rs:132-135
Code:
```
            let value =
                ebml::uint_from_be(&bytes).ok_or_else(|| error::bad_structure(child.data_pos))?;
            return Ok(ClusterTimecode {
                ticks: value as i64,
```
What is wrong: `value` is a full `u64` (the length check permits 8 bytes), and `as i64` silently wraps values above `i64::MAX` to negative `ticks`. Nothing at this layer rejects the wrap.
Why it matters: a corrupt/adversarial 8-byte Timecode with the high bit set yields a negative cluster base. Block stamps are saved by `absolute_ticks`' `sum < 0` check (cluster.rs:432-434), but `ticks` is also used raw at mkv/mod.rs:875 (`tc.ticks <= target_ticks` seek scan), where a wrapped negative compares as an ultra-early cluster. Contrived, contained — hence S4.
What correct looks like: `i64::try_from(value).map_err(|_| error::bad_structure(child.data_pos))?`.
Confidence: HIGH — Verification: read (mod.rs:875 usage confirmed by rg).

### F-004 [S4]
Where: crates/demux/src/mkv/cluster.rs:181-194
Code:
```
    let tid = ebml::parse_vint_id(data).map_err(|_| bad(0))?;
    let (track, after_track) = {
        let mut len = 1usize;
        let mut mask = 0x80u8;
        while data[0] & mask == 0 {
            mask >>= 1;
            len += 1;
        }
```
What is wrong: The track-number vint is parsed twice — once via `parse_vint_id` (whose value is discarded; only `tid.len` is used) and once by this hand-rolled loop. The inline loop has no length cap of its own: it terminates within 8 steps only because `parse_vint_id` already rejected `data[0] == 0` (ebml.rs:183-187) and capped length at 4 (ebml.rs:193-197). Remove the first call and the loop is an unbounded walk into `data[1..len]` panics. The invariant is real but implicit and undocumented.
Why it matters: duplicated marker-walk logic with cross-function safety coupling is regression bait; the two parses can silently disagree under future edits.
What correct looks like: one `parse_vint_value_at(data, 0)` (the function already exists below at line 360) reused for both number and width, with the ≤8-octet cap inside it.
Confidence: HIGH — Verification: read (ebml.rs:179-207 cross-checked).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| FLAG_KEYFRAME (const) | 25-26 | REAL | — | 0x80, matches spec/SimpleBlock |
| FLAG_LACING_MASK (const) | 27-28 | REAL | S4 | F-002: value right, doc comment wrong |
| BlockFrame | 31-36 | REAL | — | offset/len pair into block payload |
| BlockScratch | 43-49 | REAL | — | recycling scratch; moved out/in per block (mod.rs:710-712, 748-750) |
| ParsedBlock | 52-75 | REAL | — | body-relative frames, documented offset math |
| Lacing (enum) | 77-88 | REAL | — | spec table values 1=Xiph 2=Fixed 3=EBML (matches ffmpeg); doc prose garbled (F-002) |
| ClusterTimecode | 90-97 | REAL | S4 | `ticks` can wrap negative (F-003) |
| parse_timecode | 108-143 | REAL | — | bounded child walk, ≤8-byte uint, unknown-size child refused, missing Timecode → typed refusal; `as i64` wrap = F-003 |
| parse_block | 155-161 | REAL | — | convenience wrapper; production uses parse_block_with (tests/fuzz use this one) |
| parse_block_with | 167-224 | REAL | S4 | header bounds checked (174-176, 195-197); duplicated vint walk = F-004; unreachable match arm documented (206-209) |
| lace_frames | 229-278 | REAL (wrong count) | S2 | F-001 dispatch; None/empty-body → one zero-len frame (harmless) |
| xiph_sizes | 283-311 | REAL (off-by-one) | S2 | F-001: `frame_count = count` should be `count + 1`; 255-continuation guard correct |
| ebml_sizes | 316-357 | REAL (off-by-one) | S2 | F-001: same count bug; first-size −1 offset and 2^(7n−1)−1 delta bias match ffmpeg |
| parse_vint_value | 360-384 | REAL | — | ≤8 octets, in-bounds, 0x00 head refused |
| build_frames | 389-420 | REAL | — | declared-vs-rest check; underflow-safe; produces sizes.len()+1 frames (internally consistent with the wrong count) |
| absolute_ticks | 428-436 | REAL | — | checked_add + negative rejection, tested incl. i64::MAX |
| tests::block (helper) | 443-451 | REAL | — | builds ≤126 track vints |
| no_lacing_single_frame | 453-466 | REAL | — | would fail on header regression; unaffected by F-001 |
| xiph_lacing_splits_and_remainder_last | 468-483 | REAL (encodes F-001) | S2 | asserts 3 frames for count byte 3 — wrong convention baked into the test |
| fixed_lacing_divides_exactly | 485-493 | REAL (encodes F-001) | S2 | count byte 4 → asserts 4 frames; spec: 5 |
| ebml_lacing_sizes_decode | 495-510 | REAL (encodes F-001) | S2 | count byte 3 → 3 frames; spec: 4 |
| lacing_overrun_is_typed_mismatch | 512-519 | REAL | — | passes under either convention |
| fixed_lacing_remainder_is_refused | 521-527 | REAL | — | non-dividing payload refused; convention-independent |
| short_block_header_is_typed | 529-533 | REAL | — | 2-byte block refused |
| absolute_ticks_checks_the_timeline | 535-540 | REAL | — | negative + overflow paths |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| no_lacing_single_frame, short_block_header_is_typed, absolute_ticks_checks_the_timeline | YES | exact field/timeline pins |
| lacing_overrun_is_typed_mismatch, fixed_lacing_remainder_is_refused | YES | damage pins are convention-independent |
| xiph_lacing_splits_and_remainder_last, fixed_lacing_divides_exactly, ebml_lacing_sizes_decode | NO — they FAIL to catch the real bug | they assert the same off-by-one convention the parser implements; a spec-correct parser would FAIL these tests; against real ffmpeg-muxed laced bytes both code and tests are wrong together (proven: ffprobe 3 packets vs this code's 2 for count byte 2) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the three lacing codings (card 5.D1)" implemented | cluster.rs:1-2 module doc | SHALLOW/WRONG on real files | F-001: all three mis-frame real laced blocks (ffprobe-verified); unit tests green only because they encode the same wrong count |
| ffmpeg-authored corpus validates the mkv demux ("ffprobe truth") | mkv_corpus.rs:1-9 | REAL-UNPROVEN for lacing | corpus has zero laced blocks (mkv_fixtures.rs:2-5 admits); differential passed (baseline log:2141) without touching F-001's code path |
| "no laced blocks in every file we generate — measured" | mkv_fixtures.rs:2-5 | REAL | honest admission; explains why F-001 survived the differential |

## 7. Dead / duplicated / unreachable code
- Duplicated vint marker-walk (F-004): `parse_block_with` inline loop vs `parse_vint_value` vs `ebml::parse_vint_id` — three implementations of the same grammar in two files.
- `parse_block` (non-scratch) has no production caller (mod.rs uses `parse_block_with`); kept as the pub test/fuzz entry — acceptable, note only.
- Nothing unreachable otherwise: all public items consumed by mod.rs or tests.

## 8. Dependencies & flags
- No feature flags; no `unsafe`; no external crates (crate rules hold).
- Depends on `mkv::ebml` (parse_vint_id, parse_header_at, uint_from_be, ID_TIMECODE) and `mkv::error` — both in-crate.

## 9. Open questions for the next agent
- mkv/mod.rs `emit_block` (not my file): confirm how a mis-framed packet set surfaces downstream (cadence math at mod.rs:786-800 compounds F-001's wrong frame count).
- mkv_fixtures.rs `laced_fixture` (not my file): bakes in F-001's convention — must be re-based with the fix; also mkv_extras_wiring.rs:711 ("Xiph-laced block head claiming 2 frames") encodes the same convention.
- Request the orchestrator re-run after any fix: the corpus differential plus a new ffmpeg-verified laced golden (my /tmp probe files are reproducible from F-001's description; scripts at /tmp/lace_probe.py in this sandbox).
