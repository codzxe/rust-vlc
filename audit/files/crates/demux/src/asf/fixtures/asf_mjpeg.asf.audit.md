# AUDIT: crates/demux/src/asf/fixtures/asf_mjpeg.asf
| Field | Value |
|---|---|
| Auditor | W04-A05 |
| Date | 2026-11-21 |
| Lines / functions | 26,321-byte binary / n/a (test asset) |
| File verdict | REAL (test asset: committed, sha-pinned, byte-reproducible from its in-repo recipe) |
| Reachable from shipped entry point? | N/A (test asset). Consumed by `fixture_family_walks_match_the_measured_truth` (asf/tests.rs:1127, entry at :1138) and `fixture_family_cross_checks_against_live_ffprobe` (asf/tests.rs:1325) — both executed ok in baseline (orchrun-test-workspace-nofailfast.log.txt:1567 and :1953). The consuming demux module itself is UNREACHABLE from shipped entry points (see guid.rs audit F-001: no ASF probe signature, no `asf` arm in containers::open) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | sha256sum (pin match) · ffmpeg 7.1.5 regeneration from the tests.rs:991 recipe into audit/analysis/gen/ (byte-identical sha 1454c0a3…) · python3 audit/analysis/asf_parse.py (independent structural walk; all 14 FixtureTruth facts at tests.rs:989-1002 matched) · ffprobe -show_entries stream=codec_name (`mjpeg` ✓) · od -A d -t x1 of the SP EC field (F-001 evidence) · rg `EC_NONE\|EC_AUDIO_SPREAD\|known_name` (blast radius) |

## 1. One-paragraph verdict
The bytes are genuine, provably: the sha256 pin (tests.rs:1491) matches, an independent regeneration from the exact in-repo recipe under ffmpeg 7.1.5 (the pin version) reproduces the file byte-for-byte, my own structural walk matches all fourteen typed facts in the `FixtureTruth` entry (header 585/5 children, FPO 26321/8 packets/3100 preroll/3200 packet size, Data Object 585/25650/8, Simple Index 26235/86, video 64x48 MJPG 24bpp 9216 size_image 0 extradata), and live ffprobe names the stream `mjpeg`. This is the strongest fixture provenance in the demux crate — unlike the adts family (no recipes), every fact here is recipe-reproducible and oracle-cross-checked. The defects are around the file, not in it: the fixture's own Stream Properties bytes refute two `guid.rs` constants whose comments claim to have been "byte-verified against ffmpeg 7.1.5 output" (F-001, S3 — the audit's headline find), and the plan's promised integrity/decode gates for this family never landed (F-002/F-003). The whole lane is dead code in the shipped binary.

## 2. What actually works (verified)
- sha256 `1454c0a3a62b640e60ddf2f56a8fab53048210cdc023be3b1753e7946331dd61` == pin `1454c0a3a62b640e` (tests.rs:1491) — verified by sha256sum.
- Regeneration: `ffmpeg -y -v error -f lavfi -i testsrc=size=64x48:rate=10:duration=1 -c:v mjpeg -b:v 200k` (tests.rs:991) in audit/analysis/gen/ → identical sha256. The committed bytes ARE ffmpeg 7.1.5 output of the committed recipe.
- Independent walk (audit/analysis/asf_parse.py, offsets cross-checked against header.rs:708-716/729-748/1040-1052): HEADER 585 B / 5 declared children (FPO, Header Extension 156, ECD 98, SP video 129, Codec List 68) · FPO file_size 26321 = real size, 8 packets, preroll 3100, min=max=3200, flags 0x02 (not broadcast, seekable) · DATA at 585 size 25650 count 8 reserved `01 01` · SIMPLE_INDEX at 26235 size 86 · 585+25650=26235, +86=26321 exact · SP stream 1, video type GUID, TSD 51 B → 64x48, data_size/biSize 40, bit_count 24, fourcc `MJPG`, size_image 9216, extradata 0 — all == tests.rs:993-1001.
- ffprobe oracle: `codec_name` = `mjpeg` (== tests.rs:1001 `ffprobe_codecs`).
- Baseline execution: differential test ok (nofailfast log 1567), ffprobe cross-check ok (log 1953), asf module doctest ok (log 6368).

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/asf/guid.rs:164-170 (contradicted by this fixture's bytes at offset 428-443, and by every audio fixture's SP EC field)
Code:
```
/// Audio-spread error correction type (observed on every audio SP).
pub const EC_AUDIO_SPREAD: Guid = Guid([
    0x50, 0xCD, 0xC3, 0xBF, 0x8F, 0x61, 0xCF, 0x11, 0x8B, 0xB2, 0x00, 0xAA, 0xB4, 0xE2, 0x20, 0xC0,
]);
/// No error correction type (the video SPs' observed type field is
/// the all-zero GUID in ffmpeg output).
pub const EC_NONE: Guid = Guid([0u8; 16]);
```
What is wrong: Both constants are wrong against the very fixtures the module's header comment (guid.rs:11-13: "the GUIDs marked *observed* were byte-verified against ffmpeg 7.1.5 output this session") claims they were verified on. Actual committed bytes (od -A d -t x1): this fixture's video SP (object at 388) EC field at 428 = `00 57 fb 20 55 5b cf 11 a8 fd 00 80 5f 5c 44 2b` = 20FB5700-5B55-11CF-A8FD-00805F5C442B (the ASF spec's no-error-correction GUID) — NOT the all-zero GUID; every audio fixture (e.g. asf_adpcm_ms.wma SP at 278, EC at 318) = `50 cd c3 bf 8f 61 cf 11 8b b2 00 aa 00 b4 e2 20` = BFC3CD50-618F-11CF-8BB2-00AA00B4E220 — the constant is missing the `00` after `AA` and appends a phantom `C0` (renders ...00AAB4E220C0). The same false claim is restated at header.rs:242-243 ("audio-spread observed on audio, the zero GUID on video") and the plan doc carries a malformed 10-hex-tail rendering (docs/plans/asf-selfbuild.md:169).
Why it matters: `Guid::known_name()`'s EC rows (guid.rs:95-96) can never fire on real ffmpeg output — both rows are dead in practice, and any future logic that starts matching on the EC type would branch on constants that no real file carries. The "byte-verified this session" provenance system that the file's credibility rests on is broken for these two entries. (My own prior guid.rs audit repeated the claim that all 15 constants were verified — corrected by appended note in that audit file.)
What correct looks like: `EC_AUDIO_SPREAD = [50 CD C3 BF 8F 61 CF 11 8B B2 00 AA 00 B4 E2 20]`, `EC_NONE = [00 57 FB 20 55 5B CF 11 A8 FD 00 80 5F 5C 44 2B]` (or delete both if EC names stay unused), and fix the three comments.
Confidence: HIGH — Verification: od byte dumps of the committed fixtures + sha-pinned (the fixtures ARE ffmpeg 7.1.5 output — proven by my byte-exact regeneration); constant usage rg'd (only known_name + guid tests; no parse path branches on them → no functional impact, hence S3 not S2).

### F-002 [S4]
Where: docs/plans/asf-selfbuild.md:443-469 (§4.4) vs the landed test suite
Code:
```
- **Files:** `crates/demux/tests/{asf_corpus.rs (skips loudly — zero corpus lanes today,
  §1.1 — becomes live when/if C-V6 lands), asf_fixtures.rs}`; fixtures under
  `crates/demux/fixtures/asf/` + `tests/fixtures/generated/` (xtask MANIFEST entries,
  sha256-pinned — the avi/mp4 convention)
```
What is wrong: The plan's gate design for this family — xtask MANIFEST registration (the avi/mp4 drift-gate convention) and "decode-exact lanes for pcm/adpcm/mjpeg (ffmpeg `-f md5` oracle, the parity-v4 convention)" (§4.4, line 468) — never landed: the fixtures live at `src/asf/fixtures/` (not `crates/demux/fixtures/asf/`, not `tests/fixtures/generated/`), the `xtask fixtures --check` gate covers 55 files under `tests/fixtures` only (gate-fixtures___check.log.txt: "no drift — 55 file(s) verified under …/tests/fixtures"; no asf/wma/wmv match in the log), and no decode/md5 lane for any asf fixture exists anywhere (rg `-f md5` lanes: ogg/wav/aiff/ivf/mp3/vorbis/alac/adpcm/mp2 corpus files only). The §4.4 lane label "**decodable video**" for this file (line 456) is therefore aspirational: the fixture is demuxed, never decoded.
Why it matters: Plan-vs-code drift: a reader auditing ASF coverage via the plan believes gate-time integrity checks and decode-exact oracles exist; neither does. (Mitigation: the in-test sha pins at tests.rs:1138 do enforce byte integrity at test time — stronger than the adts family — and the ffprobe packet-count oracle is live when ffprobe exists.)
What correct looks like: Either land the promised gates or amend the plan/§4.4 with what actually shipped (in-test pins, demux-only differential).
Confidence: HIGH — Verification: plan text read; gate log read; rg across crates for asf decode lanes (none).

### F-003 [S4]
Where: crates/demux/src/asf/tests.rs:1100-1104
Code:
```
fn generate(dir: &std::path::Path, truth: &FixtureTruth) -> std::path::PathBuf {
    let path = dir.join(truth.file);
    if path.exists() {
        return path;
    }
```
What is wrong: `fixture_dir()` (tests.rs:1083-1098) returns the in-tree `src/asf/fixtures/` whenever it exists, and `generate()` then writes a regenerated fixture INTO the crate source tree when a committed file is missing (a deleted `asf_mjpeg.asf` reappears as an untracked binary in `src/` during `cargo test`). On a machine with ffmpeg != 7.1.5 this manifests as a hard sha-assert failure (the exact CI failure mode ASF-d was created to avoid, tests.rs:1073-1078) rather than the documented loud skip (which only fires when the whole directory is absent AND ffmpeg is absent).
Why it matters: Tests that write into the source tree are a side effect that surprises `git status` and can mask a deliberately-removed fixture; the "byte-stability cannot be delegated to a system dependency" rationale is only half-enforced.
What correct looks like: Regenerate only into a temp dir; treat a missing committed file as a hard failure regardless of ffmpeg availability.
Confidence: HIGH — Verification: code path read (fixture_dir → generate); regeneration behavior exercised by my audit/analysis/gen run (written to audit/, not src/).

## 4. Per-function table (asset checks)
| Check | Result | Notes |
|---|---|---|
| Exists / size sane | YES / 26,321 B | Largest family member; matches FPO file_size field and plan §4.4 |
| Structure matches the truth table | YES (14/14 facts) | Independent walk (asf_parse.py) == tests.rs:989-1002 |
| sha256 pin | MATCH | 1454c0a3a62b640e… (tests.rs:1491) |
| Recipe reproducibility | BYTE-EXACT | ffmpeg 7.1.5 + tests.rs:991 recipe → identical sha |
| Referenced by code/docs | YES | tests.rs:990 (truth), :1491 (pin); plan §4.4:456; worklog.md:1608 |
| Content sanity | YES | Damage-free walk asserted and passed in baseline (tests.rs:1253-1257) |
| Determinism / drift gate | TEST-TIME ONLY | sha pin inside the test; NOT under xtask fixtures gate (F-002) |
| Hostile value | N/A | Clean-file fixture by design (family's hostile cases are synthesized elsewhere) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `fixture_family_walks_match_the_measured_truth` (tests.rs:1127-1322) | YES | Per fixture: sha pin, 14 typed structural facts, damage==0, per-track emission>0, first pts == measured, monotonic pts, seek+resume>0 (this file: Simple Index present → seek lane runs, tests.rs:1304-1320). Mutation test: change the demuxer's FPO preroll read offset by one field and the preroll assert (tests.rs:1201) fails. Executed ok (baseline log 1567). |
| `fixture_family_cross_checks_against_live_ffprobe` (tests.rs:1324-1426) | YES (when ffprobe present) | Live oracle: codec names, stream count, per-track packet counts vs `-count_packets`. I re-ran the codec-name oracle by hand: `mjpeg` ✓. Executed ok (baseline log 1953). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "asf_mjpeg.asf 26,321 1454c0a3a62b640e" | plan §4.4 table (asf-selfbuild.md:456) | REAL | sha256sum + size verified |
| Lane "**decodable video**" for asf_mjpeg.asf | plan §4.4:456 | SHALLOW | mjpeg IS a shipped decoder (engine-info), but no decode lane exists for this fixture (F-002); demux-only differential, and the ASF lane is unreachable in the shipped binary anyway |
| "decode-exact lanes for pcm/adpcm/mjpeg (ffmpeg -f md5 oracle)" | plan §4.4:468 | PAPER-ONLY | No such lane anywhere (rg); only ogg/wav/ivf/mp3/etc. corpus lanes exist |
| "the GUIDs marked *observed* were byte-verified against ffmpeg 7.1.5 output this session" | guid.rs:11-13 | FALSE (for the 2 EC constants) | F-001: this fixture's video SP EC field is 20FB5700-…, not the all-zero constant |

## 7. Dead / duplicated / unreachable code
The asset itself is alive as a test input; its consumer module is dead in the shipped product (guid.rs audit F-001: no probe signature, no registry arm, zero references outside crates/demux). The F-001 constants are dead-in-practice observability rows.

## 8. Dependencies & flags
None (data file). Consumed only under cfg(test).

## 9. Open questions for the next agent
- If the ASF lane is ever wired: the demuxer's `error_correction_type` records will carry the REAL GUIDs (20FB5700-…/BFC3CD50-…00AA00B4E220) while guid.rs names only the wrong ones — fix constants before any EC-matching logic lands.
- A01's three family members (asf_pcm_s16le.wma, wma_wmav2_mono8k.wma, wmv_wmv2_64x48.wmv) — same family evidence applies (regeneration recipe for each is in tests.rs:918-1065).
