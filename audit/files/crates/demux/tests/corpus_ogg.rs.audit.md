# AUDIT: crates/demux/tests/corpus_ogg.rs
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-13 |
| Lines / functions | 326 / 3 (corpus_dir helper + 2 tests) |
| File verdict | REAL-UNPROVEN (genuine differential with measured pins; cannot execute its assertions here — corpus absent) |
| Reachable from shipped entry point? | NO — integration test suite (tests/corpus_ogg.rs); exercises the shipped OggDemuxer |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p demux --test corpus_ogg -- --nocapture` → 2 passed in 0.01s, BOTH printed SKIP (corpus absent); `ls /tmp/my-project/corpus-v4/media` → nonexistent; LUMEN_CORPUS unset; baseline cross-check (audit/runs/baseline/cargo-test-workspace.txt:2216-2220 — identical 0.01s, 2 passed); rg for OggDemuxer accessor surface (all exist) |

## 1. One-paragraph verdict
The pins in this suite are real, measured ffmpeg truths (the worklog archive records
the live 2026-10-24 run: 4/4 PASS with the exact md5s/granules/packet counts in
TRUTHS — docs/worklog/archive-2026-10.md:8558,8590), and the assertions are strong
(codec identity, time base, EOS granule, duration arithmetic, payload md5 parity,
packet counts, per-serial counts, clean walk, link count, granule discontinuities).
But in this sandbox the corpus (/tmp/my-project/corpus-v4/media) does not exist, so
both tests return early and REPORT PASS — I proved this by running with --nocapture
(SKIP messages visible, 0.01s, "2 passed"). The orchestrator baseline shows the same
0.01s/2-passed signature for this suite, so the baseline's "4744 passed" includes
these two vacuous passes. The suite is honest about the skip on stderr and points at
the worklog, but pass/fail-wise a missing corpus is indistinguishable from a
verified differential.

## 2. What actually works (verified)
- `cargo test -p demux --test corpus_ogg -- --nocapture` → both tests compile, run,
  and print their documented SKIP lines; exit 0. The skip path is real and loud on
  stderr.
- Every OggDemuxer accessor the test drives exists and is public:
  serial_numbers/header_packets/packet_counts/links/granule_discontinuities/
  track_duration/damage_total (crates/demux/src/ogg/mod.rs:496-559) and
  demux::ogg::fuzz::fuzz_demux_ogg (ogg/fuzz.rs:50).
- Duration arithmetic in the test is correct: 192,312 × 1e9 / 48,000 = 4.0065 s;
  theora granule 2 × (2/1) = 4.0 s — matches the comments.
- The TRUTHS table matches the worklog's recorded live-measured table exactly
  (file, codec, granule, packets, md5 for all 4 lanes — cross-checked char-by-char
  against archive-2026-10.md:8590).

## 3. Findings
F-001 [S3]
Where: crates/demux/tests/corpus_ogg.rs:117-123 (and 284-287)
Code:
```
    let Some(dir) = corpus_dir() else {
        eprintln!(
            "corpus(ogg): /tmp/my-project/corpus-v4/media absent and LUMEN_CORPUS unset \
             — SKIP (the differential table for the 2026-10-24 instance is in the worklog)"
        );
        return;
    };
```
What is wrong: skip == pass. When the corpus is absent the test returns Ok and cargo
reports "ok"; the baseline run (audit/runs/baseline/cargo-test-workspace.txt:2216-
2220, 2 passed / 0.01s — identical to my --nocapture rerun with visible SKIPs) shows
this suite passing vacuously in the environment of record. Any regression in
OggDemuxer since 2026-10-24 is invisible to this suite here. The doc's "skips
loudly" is true only of stderr; the pass/fail signal carries no skip information
(no #[ignore], no panic, no marked-failure convention). What correct looks like:
`panic!`/`#[ignore = "corpus absent"]`/a dedicated skip-count mechanism, or checking
the corpus into the repo. Confidence: HIGH — Verification: ran (SKIP output pasted).
F-002 [S4]
Where: crates/demux/tests/corpus_ogg.rs:291-300
Code:
```
        // Deterministic mutation sweeps over the real bytes: every
        // 4 KiB-stride truncation prefix and single-bit flip.
        for cut in (0..bytes.len()).step_by(4096) {
            demux::ogg::fuzz::fuzz_demux_ogg(&bytes[..cut]);
        }
        for off in (0..bytes.len()).step_by(4096) {
            let mut mutated = bytes.clone();
            mutated[off] ^= 0x01;
```
What is wrong: the "single-bit flip" sweep flips only bit 0 of one byte every 4096
bytes — 1 of 8 bits at 1/4096 of offsets; page-level structures (Ogg page headers
sit at arbitrary offsets) are mostly untouched. Deterministic fuzz-lite by design,
but the comment's blanket "single-bit flip" overstates the coverage ~32,768× (8 bits
× 4096 stride). Confidence: HIGH — Verification: read.
F-003 [S4]
Where: crates/demux/tests/corpus_ogg.rs:171-183
Code:
```
        // The codec headers are stashed verbatim (decode-domain
        // parameters live there — channels/dimensions, see the module
        // docs): one serial, its headers present with the right magic.
        let serials = ogg.serial_numbers();
        assert_eq!(serials.len(), 1, "{}", truth.file);
        let headers = ogg.header_packets(serials[0]);
        assert!(!headers.is_empty(), "{}: headers stashed", truth.file);
```
What is wrong: the comment promises headers (plural) "with the right magic", but
only headers[0]'s magic is asserted (OpusHead / 0x01"vorb" / 0x80"theo"); the second
and third codec headers (OpusTags, vorbis comment 0x03/codebook 0x05, theora
comment/tables) are never magic-checked — a demuxer that dropped or reordered
headers[1..] would still pass. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: crates/demux/tests/corpus_ogg.rs:304-308
Code:
```
    for subdir in ["dirty", "garbage"] {
        let dir = dir.join(subdir);
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
```
What is wrong: missing/unreadable dirty/ and garbage/ dirs are silently skipped
(same vacuous-pass class as F-001, smaller blast radius — the sweep count only
appears on stderr). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CorpusTruth/TRUTHS | 39-103 | REAL | — | matches worklog live-measured table exactly |
| corpus_dir | 107-113 | REAL | — | LUMEN_CORPUS override else /tmp default |
| corpus_oggs_match_the_ffmpeg_truth | 115-273 | REAL-UNPROVEN | S3 | skips-as-passes here (F-001); assertions strong, unexecutable |
| corpus_oggs_survive_the_fuzz_harness | 282-325 | REAL-UNPROVEN | S4 | same skip; sweeps coarse (F-002), silent subdir continue (F-004) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_oggs_match_the_ffmpeg_truth | YES **iff corpus present** — NO here | md5/count/granule/base pins are real differentials; corpus absent → early return |
| corpus_oggs_survive_the_fuzz_harness | YES iff corpus present | no-panic contract over real + mutated bytes; absent → early return |
Corpus provenance: ffmpeg-authored corpus-v4 (143 files), last verifiably present
2026-10-24 (worklog); regeneration procedure documented in docs/plans; not in-repo.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every pinned value below was measured this session from the corpus-v4 instance" | corpus_ogg.rs:10-12 | REAL-UNPROVEN | worklog archive-2026-10.md:8590 records the identical table from the live run; cannot re-measure (corpus gone) |
| "this suite **skips loudly**" | corpus_ogg.rs:13-15 | SHALLOW | loud on stderr, silent in pass/fail (F-001; ran it) |
| "Ogg carries no container-level channel-count … codec-domain bytes we stash verbatim" | corpus_ogg.rs:18-25 | REAL | RFC 3533/7845 container model; header_packets() accessor exists |
| "both sides count data packets only" (packets pin) | corpus_ogg.rs:54-57 | UNVERIFIED | requires the corpus + ffprobe side-by-side |

## 7. Dead / duplicated / unreachable code
None. (mod common include uses md5_hex + open_via_input — both consumed.)

## 8. Dependencies & flags
No feature gates. Env coupling: LUMEN_CORPUS + /tmp/my-project/corpus-v4/media
(sandbox-external dependency — the root cause of F-001).

## 9. Open questions for the next agent
- Is any in-repo artifact regenerating corpus-v4 (xtask?) — if yes, the suite could
  be made hermetic and F-001 moot. Docs/plans/asf-selfbuild.md:31 references the
  same /tmp path (143 files) — same fragility across all corpus_* suites.
- Whoever audits crates/demux/src/ogg/*: the md5 pins here are your only external
  truth for payload parity; treat post-2026-10-24 ogg changes as UNVERIFIED by
  this suite.
