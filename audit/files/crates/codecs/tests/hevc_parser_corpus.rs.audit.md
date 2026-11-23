# AUDIT: crates/codecs/tests/hevc_parser_corpus.rs
| Field | Value |
|---|---|
| Auditor | W06-A09 |
| Date | 2026-11-27 |
| Lines / functions | 441 / 6 helpers + 3 tests |
| File verdict | REAL-UNPROVEN (well-designed live-oracle differential, but every test skips green in this environment — baseline "ok" asserted nothing; authoring-session evidence lives only in the worklog) |
| Reachable from shipped entry point? | N/A (test binary). The parser under test (crates/codecs/src/video/hevc/parser/) is another agent's scope; shipped HEVC decode is refused (`no decoder for hevc`, baseline mp4-smoke) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | ls /tmp/my-project/corpus-v4/media (absent, exit 2); rg baseline + worklog; sed quote verification |

## 1. One-paragraph verdict
The test design is right: parse real ffmpeg-authored HEVC lanes (raw Annex-B, TS-extracted ES, MP4 hvcC record) with the clean-room parser and assert profile/level/resolution/pix_fmt against live ffprobe, plus a 2000-round mutation property pass. But the corpus directory this suite depends on (`/tmp/my-project/corpus-v4/media`, overridable via `LUMEN_CORPUS`) does not exist in this sandbox — I verified — so all three tests take their `eprintln!("SKIP ...")` + `return` path and finish "ok" while asserting NOTHING. The baseline's three green rows (cargo-test-workspace.txt:1210-1212) are therefore skip-greens, not evidence. The genuine evidence is the authoring-session worklog table (docs/worklog/archive-2026-10.md:9593-9600: 0091 Main/63/480x360/yuv420p PASS, 0100 hvcC field table, both entry-form lanes PASS), which I cannot re-execute here (corpus regeneration is a heavy operation outside my budget — UNVERIFIED). The skip behavior is loudly documented in the file header, so this is an evidence gap, not fraud.

## 2. What actually works (verified)
- Corpus absence verified: `ls /tmp/my-project/corpus-v4/media/` → "No such file or directory" (exit 2) → `corpus_dir()` returns None in this environment.
- Baseline :1207-1212: the three tests ran and returned ok (via the SKIP path — no corpus, and the run cannot have had `LUMEN_CORPUS` set either, since the worklog notes disk pressure forced minimal profiles).
- Worklog evidence located: docs/worklog/archive-2026-10.md:9593-9600 records the authoring-session PASS table and the `cargo test -p codecs --all-features` = 479 passed (19 hevc lib tests + 3 corpus integration tests) run that backs the design.
- ffprobe/ffmpeg ARE present in this sandbox (`/usr/bin/ffmpeg`, `/usr/bin/ffprobe`) — only the corpus bytes are missing.

## 3. Findings
F-001 [S3]
Where: crates/codecs/tests/hevc_parser_corpus.rs:212-216 (and the same shape at :283-292, :331-340)
Code:
```
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP hevc raw corpus: corpus-v4 absent (LUMEN_CORPUS unset)");
        return;
    };
```
What is wrong: skip-and-return-Ok. When the corpus is absent (the case in this sandbox and in the baseline run) the test finishes green having asserted nothing — the baseline's "4744 passed" includes these three as empty passes. The mutation property pass (2000 rounds × flips/truncations through SPS/VPS/PPS parsers, :252-280) is the strongest hostile evidence in the file and it never ran here. Why it matters: any regression in the HEVC parser's field extraction would not be caught by the baseline lane; the only evidence is a dated worklog table. What correct looks like: `#[ignore]` with an explicit "requires corpus" marker so the skip is visible in test summaries, or a committed minimal fixture lane (like the ADPCM breadth suite does — 12 committed files). Confidence: HIGH — Verification: ran ls (absent) + read baseline + read worklog; the tests' real execution UNVERIFIED (no corpus; regeneration = cargo-build-budget).

F-002 [S4]
Where: crates/codecs/tests/hevc_parser_corpus.rs:1-5 vs 187
Code:
```
//! ffprobe truth** — profile, tier, level, resolution, chroma/pix_fmt
```
vs
```
    let tier = if ptl.tier { "High" } else { "Main" };
```
What is wrong: the header claims tier is asserted against the oracle; the code only prints tier into the row log (it appears in the `eprintln!` at :195-207) — no comparison ever uses it (ffprobe's `profile` field encodes profile, not tier). Doc overstates the asserted set. Confidence: HIGH — Verification: read (tier is only referenced in the log format).

F-003 [S4]
Where: crates/codecs/tests/hevc_parser_corpus.rs:19-21 vs 402-419
Code:
```
//! When the corpus root (or ffmpeg) is absent the suite **skips
//! loudly** (the corpus regenerates with fresh seeds; PASS/FAIL
//! tables live in the worklog entry, not pinned constants).
```
vs
```
    assert_eq!(cfg.level_idc, 93);
    ...
    assert_eq!(sps.cropped_size().must(), (1024, 768));
```
What is wrong: the 0100 test DOES pin constants (level 93, 1024×768, exactly 4 arrays of types [32,33,34,39]) while the header says constants are not pinned. If corpus regeneration with "fresh seeds" ever varies encoding parameters for lane 0100, this test fails for provenance reasons rather than parser defects. The pins are probably stable lane parameters, but the stated philosophy and the code disagree. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| corpus_dir | 35-41 | REAL | — | LUMEN_CORPUS override; is_dir check |
| ffmpeg_available | 43-50 | REAL | — | ffprobe -version probe |
| ffprobe_video | 63-93 | REAL | — | field parser with `name=` prefix matching; tolerant defaults |
| parse_annexb | 109-148 | REAL | — | first-parameter-set-wins; slice prefix over SPS/PPS seams |
| assert_facts | 154-209 | REAL | S4 | compares profile/level/size/pix_fmt; tier logged only (F-002); returns PASS/FAIL row |
| corpus_raw_annexb_0044... | 211-281 | REAL-UNPROVEN (skipped here) | S3 | + 2000-round mutation property (never ran in baseline) |
| corpus_ts_0091... | 283-329 | REAL-UNPROVEN (skipped here) | S3 | ffmpeg -c copy -f hevc extraction; extraction failure also skips |
| corpus_mp4_0100... | 331-441 | REAL-UNPROVEN (skipped here) | S3 | hvcC hex-dump parser + field-pinned assertions (F-003) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_raw_annexb_0044 | YES when corpus present; NO in this environment (skip-green) | asserts live ffprobe fields + mutation survival |
| corpus_ts_0091 | YES when corpus+extraction present; NO here | same |
| corpus_mp4_0100 | YES when corpus present; NO here | field-exact hvcC assertions |
All three: the skip path prints a message but cannot fail — a silently-green suite in the shipped CI lane.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "real ffmpeg-authored bytes ... asserted against live ffprobe truth — profile, tier, level, resolution, chroma/pix_fmt" | :1-5 | REAL-UNPROVEN (tier never asserted — see F-002) | worklog:9593-9600 records the authoring PASS; not re-executable here |
| "skips loudly ... not pinned constants" | :19-21 | PARTLY FALSE | skip is loud (eprintln) but green; 0100 pins constants (F-003) |
| "0038/0099 AVI+HEVC lanes skipped: ffprobe reports codec unknown" | :15-17 | REAL (worklog-corroborated) | worklog:9599 same statement |

## 7. Dead / duplicated / unreachable code
None intra-file. The entire suite is environmentally unreachable (no corpus) — see F-001.

## 8. Dependencies & flags
Test-target; shells out to ffprobe/ffmpeg (present in sandbox). No features. `LUMEN_CORPUS` env var is the only external knob (never set in baseline evidence).

## 9. Open questions for the next agent
- Who owns tools/corpus regeneration, and can a minimal committed HEVC lane (0044-like, ~100 KB) be checked in so this suite always runs (the ADPCM breadth precedent)?
- hevc/parser/*.rs audits (other agents): their baseline-green unit tests are the only currently-executed HEVC parser evidence; cross-check that they don't also depend on the corpus.
