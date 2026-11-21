# AUDIT: crates/demux/src/wav/mod.rs#part1 (lines 1–554)
| Field | Value |
|---|---|
| Auditor | W04-A07 |
| Date | 2026-11-21 |
| Lines / functions | 554 (of 1109; part1) / 18 units |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen` CLI → `apps/cli/src/demux_host.rs:362 open_source` → media-probe signature hit → `apps/cli/src/containers/mod.rs` "wav" arm → `apps/cli/src/containers/wav.rs:50 WavDemuxer::new` → `Demuxer::start/next_event` (part2); executed: `containers::wav::tests::*` 6 tests ok in baseline `orchrun-test-workspace-nofailfast.log.txt:2837-2845`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `rg -n "WavDemuxer" apps/ crates/` (callers); `rg -n "wav::probe"` across apps/+crates/ (0 production callers); `sed -n` + `od -c` on quoted ranges (Bash display mangles `[x` sequences — `od -c` is authoritative; Read tool output verified identical); `rg -n "BW64"` crates/media-probe/ (magic branch untested); `ls tests/fixtures/generated/malformed \| rg "^(aiff\|wav)"` (18 files); corpus-skip analysis below |

## 1. One-paragraph verdict
The WAV demuxer's first half — module constants, the 12-byte probe, the struct
and its observability accessors, and the top-level RIFF/RF64/BW64 walk with
sentinel/`ds64` resolution — is real, carefully damage-accounted code with the
strongest external evidence anchor in this demux family: committed
ffmpeg-authored fixtures, a 12-lane ffmpeg-measured differential corpus
(`crates/demux/tests/corpus_wav.rs`), and a doctest executed in the baseline.
Two evidence-hygiene defects matter: the corpus differential silently no-ops
(skip-as-pass) when its fixture dir is absent, so the baseline's "ok" cannot
prove it ran; and the demuxer's own `probe()` is production-dead — routing
actually uses a hand-maintained mirror in media-probe whose BW64 branch no
test has ever executed. The walk itself correctly clamps every declared
extent, resolves the RF64 sentinel through `ds64`, and counts damage on every
surrendered byte at the container edge.

## 2. What actually works (verified)
- Doctest `wav::WavDemuxer` (line 216): builds a minimal PCM16 WAV, starts,
  asserts `codec == "pcm_s16le"` and a keyframe packet — **ok** in baseline
  `orchrun-test-workspace-nofailfast.log.txt` Doc-tests demux block (8/8 ok).
- `containers::wav::tests::{all_wav_fixture_flavors_open, pcm_fixture_opens_with_expected_facts,
  adpcm_extra_data_flows_into_the_decoder, broken_fmt_chunk_refuses_at_the_demux_stage,
  start_time_clamps_and_skips, corpus_wav_lanes_decode_through_the_open_path}` — all
  **ok** in baseline (:2837-2845) — the open path (probe → demux) is exercised
  through real code (with the corpus-lane caveat of F-001).
- `crates/demux/tests/corpus_wav.rs` + `wav_aiff_fixtures.rs`: 2 + N tests
  **ok** in baseline (:2011-2018 and demux integration section); the fixtures
  are committed ffmpeg products with bit-exact MD5 truths recorded in-repo
  (`TRUTHS` array) and a 12/12 PASS table committed at
  `docs/formats/container-wav.md:39-56`.
- `SourceReader::new` preconditions (reject unknown-length/unseekable/empty)
  inherited from the AVI riff module — verified by read (riff.rs:219-237).

## 3. Findings

### F-001 [S3]
Where: crates/demux/src/wav/mod.rs:11-15 (claim) + crates/demux/tests/corpus_wav.rs:192-199 (mechanism)
Code (the claim):
```rust
//! committed fixtures under `tests/fixtures/generated/` are ffmpeg
//! products; the differential suite pins the demuxed payload against
//! ffmpeg's raw dumps, and the corpus suite runs the twelve real
//! randomized WAVs).
```
Code (the mechanism, corpus_wav.rs:192-199):
```rust
fn corpus_wavs_match_the_ffmpeg_truth() {
    let Some(dir) = corpus_dir() else {
        eprintln!(
            "corpus(wav): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (the differential table for the 2026-10-24 instance is in the worklog)"
        );
        return;
    };
```
What is wrong: The corpus differential returns `Ok` (test passes, asserts
nothing) when `/tmp/my-project/corpus-v4/media` is absent. The doc comment
says it "skips loudly", but `cargo test` captures stdout/stderr of passing
tests — the eprintln is invisible in every passing run. That directory does
not exist in this sandbox (`ls /tmp/my-project/` → audit-workspace, scripts,
target, tool-results, wt — no corpus-v4), and nothing in the baseline logs
distinguishes execution from skip (the suite shows "ok … finished in 0.00s").
The same skip-as-pass pattern is in `apps/cli/src/containers/wav.rs:519-528`
(`corpus_wav_lanes_decode_through_the_open_path`).
Why it matters: This suite is the demuxer's primary external conformance
anchor (the module doc's "differential suite pins the demuxed payload").
A green CI/baseline run with the corpus absent proves nothing about WAV
demux correctness, while looking identical to a real 12/12 pass. The
committed 12/12 table (docs/formats/container-wav.md:39) is a point-in-time
record, not re-executed evidence.
What correct looks like: Fail (or `#[ignore]` with an explicit env-gated
runner) when the corpus is absent in a context that claims evidence — or at
minimum print the skip via a mechanism cargo shows (e.g. write a marker file
the gate checks), and record in the baseline INDEX which corpus tests
actually executed.
Confidence: HIGH — Verification: read + `ls /tmp/my-project/` + baseline log
grep (no corpus-v4 dir; "ok" lines indistinguishable from skip).

### F-002 [S3]
Where: crates/demux/src/wav/mod.rs:140-147 (dead copy) + crates/media-probe/src/signature/phase5.rs:62-71 (live mirror)
Code:
```rust
pub fn probe(header: &[u8]) -> bool {
    if header.len() < 12 {
        return false;
    }
    let magic = FourCC::new([header[0], header[1], header[2], header[3]]);
    let form = FourCC::new([header[8], header[9], header[10], header[11]]);
    matches!(magic, RIFF | RF64 | BW64) && form == FORM_WAVE
}
```
What is wrong: `demux::wav::probe` has **zero production callers** (rg across
apps/ and crates/: only wav's own tests reference it). The routing decision —
which files reach this demuxer at all — is made by `media-probe`'s
`WavSignature`, a hand-maintained byte-mirror ("mirrors `demux::wav::probe`",
phase5.rs:56) that cannot share code across the crate edge. The mirror's
RIFF and RF64 branches are pinned by `registry_roundtrip.rs` fixtures, but
**no BW64 fixture exists anywhere in the repo**, so the BW64 arm of the copy
that actually routes files has never been executed by any test; meanwhile the
dead demux-side copy's BW64 acceptance IS tested (wav/tests.rs:860) — the
test coverage is on the copy that cannot matter.
Why it matters: `EXTENSIONS` advertises `bw64` in both places (mod.rs:132,
phase5.rs:70-72); a one-character typo in the live mirror's BW64 magic would
make every BW64 file fail at the probe stage ("probe.no-demuxer") while the
demuxer could parse it — and no test would catch it.
What correct looks like: Either route the demuxer's probe as the single
truth (a shared signature table both crates consume), or pin the mirror
byte-for-byte with a parity test over the full magic set (RIFF/RF64/BW64 ×
WAVE/non-WAVE) in media-probe's own suite.
Confidence: HIGH — Verification: read + rg (callers, fixtures, tests);
registry_roundtrip.rs read (only RIFF/RF64 WAV fixtures).

### F-003 [S4]
Where: crates/demux/src/wav/mod.rs:468-474
Code:
```rust
                } else if let Some(bytes) = self.reader.read_exact_or_end_at(4, payload_start)? {
                    let raw = [bytes[0], bytes[1], bytes[2], bytes[3]];
                    let declared = u64::from(u32le(&raw, 0));
                    if declared > 0 {
                        self.fact_frames = Some(declared);
                    }
                }
```
What is wrong: When the 4-byte fact payload straddles EOF
(`read_exact_or_end_at` → `None`), the fact chunk is silently dropped — no
damage note for the undelivered bytes (contrast: `fact-too-small` at :467
and every other cut path in this file notes its loss).
Why it matters: The module's damage contract ("never silent") is breached in
a narrow corner: a file whose extent ends exactly inside a `fact` payload
under a to-stream-end sentinel loses the frame-count declaration without a
record. The duration then falls back to the byte extent — behaviorally
correct, but the consumer cannot see the loss.
What correct looks like: `else { self.damage.note(4, "fact-past-eof"); }`
on the `None` arm.
Confidence: MEDIUM — Verification: read (`read_exact_or_end_at` Option
semantics traced via riff.rs:285 and its AVI call sites).

### F-004 [S4]
Where: docs/formats/container-wav.md:74-75 (claim) vs the tree
Code (the claim):
```markdown
+FB: wav_*
mutated families (flip/garbage/trunc ×5) with pinned exact outcomes
+ fuzz-harness survival; 20 malformed fixtures committed.
```
What is wrong: The tree carries 18 wav/aiff malformed fixtures (9
`wav_pcm_s16le_44k1_st__*` + 9 `aiff_pcm_s16be_44k1_st__*`, `ls
tests/fixtures/generated/malformed/`), and the pinning test iterates exactly
9 WAV derivatives (`WAV_DERIVATIVES: [WavDerivative; 9]`,
wav_aiff_fixtures.rs:368). "20" matches nothing countable in the tree.
Why it matters: Evidence-count drift in the record doc that the
feature-matrix row cites ("differential tables cited to 5.A2") — the audit
cannot tell whether 2 fixtures were lost or the count was wrong at write
time.
What correct looks like: Correct the count to 18 (or restore the missing
two with their pinned outcomes).
Confidence: HIGH — Verification: `ls` + `rg -n "WAV_DERIVATIVES"`.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc + constants (MAX_PACKET_BYTES 64 KiB, MAX_INFO_BYTES 64 KiB, MAX_INFO_VALUE_BYTES 1 KiB, magics, SIZE_SENTINEL, EXTENSIONS) | 1-132 | REAL | — | Bounds documented with rationale; EXTENSIONS matches media-probe's list (verified). |
| `probe` | 140-147 | REAL | S3 (F-002) | Correct logic (od -c verified); production-dead; mirror analysis in F-002. |
| `RiffInfo` | 152-158 | REAL | — | Lossy UTF-8 by documented policy (advisory display text). |
| `DataRegion` | 161-170 | REAL | — | declared vs effective end split mirrors AVI MoviRegion. |
| `Ds64` | 173-178 | REAL | — | plain data. |
| `StickyShape::{of, mint}` | 190-202 | REAL | — | Verbatim re-implementation of avi::StickyShape — duplication across demuxers (AVI has its own; noted, not a finding: the AVI one is private, so no shared path without API change). |
| `WavDemuxer::new` | 278-299 | REAL | — | SourceReader::new preconditions; pool 2 × MAX_PACKET_BYTES ✓ matches doc. |
| `format` / `fmt_chunk_bytes` / `info` / `is_rf64_like` / `fact_sample_length` / `damage_total` / `pool_bytes` | 304-349 | REAL | — | Read-only observability accessors, all `#[must_use]`; safe pre-start (Options stay None). |
| `frames` | 355-362 | REAL | — | saturating arithmetic; FLAC → None honestly (`samples_per_unit` None). |
| `fail` / `surface_damage` / `emit` | 365-386 | REAL | — | Same damage-record-first contract as AVI; `emit` ordering logic identical. |
| `parse_container` (first half: head classification, sentinel resolution, top-level walk dispatch) | 389-554 | REAL | S4 (F-003) | Head: <12 bytes + recognized magic prefix → truncation, else not-wav (honest D-9 flavor) ✓. RIFF sentinel (plain RIFF) → damage-noted "to stream end" ✓. data sentinel resolved via ds64 else to-stream-end with note ✓. ds64: outside-RF64 → damage + skip; inside → extent regrown when sentinel ✓. duplicate data/fmt → first-wins + damage ✓. LIST<4 → "list-too-small" (contrast AVI's rec arm, mod.rs#part2 F-004 — here it is handled). Past-extent chunk → clamp + "chunk-extends-past-riff" (live-data excepted — attributed once as data-extends-past-stream in part2 :598-607) + walk end: documented "no resync convention" trade — loud typed errors for missing fmt/data follow in part2. `pos`/`next` saturating, bounded by extent ≤ length — no overflow. fact None-arm silence = F-003. |

## 5. Test honesty (tests for this file live in wav/tests.rs (A04/A10) and the integration suites)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| doctest WavDemuxer (:216-237) | YES | asserts codec string + keyframe packet; passed in baseline (Doc-tests demux 8/8). |
| corpus_wavs_match_the_ffmpeg_truth | ONLY WHEN CORPUS PRESENT | F-001: absent corpus → skip-as-pass; otherwise pins codec/rate/channels/frames + payload MD5 per file (12 lanes) — discriminating. |
| containers::wav::tests (6, baseline :2837-2845) | YES (except corpus lane, F-001) | open-path tests on committed fixtures with expected facts. |
| registry_roundtrip WAV/RF64 | YES | real fixture bytes through the builtin probe registry (RIFF+RF64 pinned; BW64 not — F-002). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the corpus suite runs the twelve real randomized WAVs" | mod.rs:14-15 | REAL-UNPROVEN (in baseline) | Test + 12 measured truths exist; execution conditional on /tmp corpus (absent here); committed 12/12 table is the provenance (F-001). |
| "Declared sizes are hints; the stream is authority … damage-noted, the shorter extent wins" | mod.rs:33-43 | REAL | Walk verified: sentinel resolution (:445-457), extent clamps (:425-431, :542-552), RF64-without-ds64 refusal (part2 :561-563). |
| "An RF64/BW64 head without its mandatory `ds64` is the one structural lie that refuses at `start`" | mod.rs:41-43 | REAL | part2 :561-563 (`error::bad_structure`) — read-verified (boundary file half, A08 owns the line). |
| container-wav row: linux/windows/macos "implemented"; "Linux-only evidence, CI 3-OS pending" | docs/feature-matrix.toml:220-231 | REAL (linux) / REAL-UNPROVEN (win/macos) | Row's own evidence string admits Linux-only; no win/macos run exists in baseline. |
| "RF64 > 4 GB untested [VERIFY]" (notes) | docs/feature-matrix.toml:231 | honest gap | Consistent with corpus (no >4 GiB lane); no fixture exceeds 4 GiB. |

## 7. Dead / duplicated / unreachable code
- `demux::wav::probe` — production-dead (F-002).
- `StickyShape` — duplicated per demuxer (avi + wav identical shape; a third
  copy may exist in other demuxers — cross-family observation for the wave
  synthesizer).
- media-probe `WavSignature` — deliberate mirror (no crate edge), partially
  pinned (F-002).

## 8. Dependencies & flags
Reuses `crate::avi::riff::{SourceReader, PayloadSlots, FourCC, pad_for, u32le}`
and `crate::avi::header::DamageLog` — cross-format reuse, no new deps, no
unsafe, no feature gates. Pool bounded 2 × 64 KiB (D-1 observable).

## 9. Open questions for the next agent
- wav/mod.rs#part2 (A08): does the post-walk `data-extends-past-stream`
  accounting interact correctly with a sentinel `data` in a plain RIFF whose
  trailing bytes contain another chunk (LIST after data)? (The sentinel's
  to-EOF extent swallows trailing chunks as audio per the convention —
  intended, but worth a test.)
- Whoever owns media-probe: add the BW64 parity/fixture pin (F-002).
- Is there a third `StickyShape` copy in asf/ogg/mp4 demuxers? If yes, raise
  to S3 duplication at wave synthesis.
