# AUDIT: crates/demux/tests/mkv_attachments_chapters.rs
| Field | Value |
|---|---|
| Auditor | W04-A06 |
| Date | 2026-11-23 |
| Lines / functions | 436 / 8 (2 oracle differential tests + ffprobe scrapers + ffmpeg fixture generator) |
| File verdict | SPLIT: generated-fixture differential REAL (executed live, byte-exact); corpus differential REAL-UNPROVEN (skips everywhere the corpus is absent — including here and CI) |
| Reachable from shipped entry point? | Test binary (CI lane); code under test (`demux::mkv`) reachable via containers/mod.rs:56 "matroska" |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `cargo test -p demux --test mkv_attachments_chapters` → 2 passed (corpus lane skipped loudly; generated lane ran live); `-- --nocapture` → "SKIP: corpus-v4 absent" + "PASS d2a-generated.mkv"; `ffprobe -show_chapters` on the generated fixture → ns-unit chapters confirmed |

## 1. One-paragraph verdict
The generated-fixture half of this suite is genuinely strong: it builds a real MKV at run time with ffmpeg (two chapters via ffmetadata, a JPEG cover and a text file via `-attach`), then differentiates against live ffprobe (count, per-chapter ns start/end, titles, attachment filename/mimetype) and additionally asserts byte-exact inline `FileData` for both attachments and exact chapter nanoseconds — all executed here against ffmpeg 7.1.5 and passing. The comparison units are right: I verified ffprobe reports matroska chapters in a 1/1e9 time_base (raw ns), matching `atom.start_ns`, so the "chapter times do not scale with TimestampScale" doc claim holds for the generated lane. The corpus half — the card's named adversarial lanes (`0007-chapter-bomb.mkv`, `0017-meta-bomb-av.mkv`) — lives only at `/tmp/my-project/corpus-v4` on the corpus machine and silently becomes a no-op pass everywhere else, so the only hostile chapter coverage never runs in a reproducible environment. The JSON scraping is hand-rolled but fails loud (a scraping regression produces -1/None mismatches, not false passes).

## 2. What actually works (verified)
- `cargo test -p demux --test mkv_attachments_chapters` → `2 passed; 0 failed` — with `--nocapture`: `SKIP: corpus-v4 absent` (honest, loud) and `PASS d2a-generated.mkv: 2 chapters, 2 attachments (ffprobe-matched)` + `PASS d2a-generated.mkv: 2 chapters + 2 byte-exact attachments`.
- Inspected the generated fixture (`/tmp/lumen-d2a-fixtures/d2a-generated.mkv`) with ffprobe directly: chapters at time_base 1/1000000000, start/end 0/1500000000 and 1500000000/4000000000 ns — exactly what the test asserts, so the ns-vs-ns differential is unit-correct.
- The generator is a real ffmpeg invocation chain (lavfi testsrc+libx264+aac, `-map_chapters 2`, two `-attach` lanes with mimetype/filename metadata) — not a mock; failures `assert!` loudly.

## 3. Findings
F-001 [S3]
Where: crates/demux/tests/mkv_attachments_chapters.rs:36-42 and 246-249
Code:
```
fn corpus_dir() -> Option<PathBuf> {
    let dir = std::env::var("LUMEN_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/my-project/corpus-v4"));
    let media = dir.join("media");
    media.is_dir().then_some(media)
}
```
```
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP: corpus-v4 absent (LUMEN_CORPUS unset, /tmp/my-project/corpus-v4 missing)");
        return;
    };
```
What is wrong: the entire corpus differential — including the card's named adversarial lanes `0007-chapter-bomb.mkv` and `0017-meta-bomb-av.mkv` — is gated on a developer-machine directory that does not exist in this sandbox and (per the baseline's identical pass count) not in CI either. The skip is loud and honest, but the consequence is that the *only* hostile-input coverage for chapters/attachments (bomb-scale chapter tables) never executes in any reproducible lane; the suite's doc sells "the real-corpus `.mkv` lanes" as a feature of the file. Why it matters: chapter-bomb regressions would pass every gate. What correct looks like: commit a small chapter-bomb fixture (the generated lane proves in-repo ffmpeg generation is possible) or gate the corpus lane behind the same `LUMEN_REQUIRE_ORACLE` strictness knob used by mkv_fixtures. Confidence: HIGH — Verification: ran (skip observed live; `/tmp/my-project/corpus-v4` absent).

F-002 [S4]
Where: crates/demux/tests/mkv_attachments_chapters.rs:60-77 (and the twin at 104-124)
Code:
```
    for chunk in text.split("\"id\":").skip(1) {
```
What is wrong: hand-rolled ffprobe JSON scraping. A scraping regression (field rename, ffprobe output change) yields `-1`/`None` values that mismatch and fail loudly — no false-pass path found — but the error messages then blame the demuxer ("c1(probe) start X vs -1"), which wastes debugging time. Confidence: HIGH — Verification: read + live ffprobe JSON inspected.

F-003 [S4]
Where: crates/demux/tests/mkv_attachments_chapters.rs:53-57
Code:
```
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-show_chapters", "-of", "json"])
        .arg(path)
        .output()
        .must();
```
What is wrong: the corpus lane calls ffprobe via `.must()` (panic on spawn failure) without the `oracle_available()` guard its sibling suite uses — an environment with the corpus present but ffprobe absent panics instead of skipping. Inconsistent guard discipline across the two MKV suites. Confidence: HIGH — Verification: read (mkv_fixtures.rs:120-123 uses the guard; this file never imports testkit's oracle helpers).

F-004 [S4]
Where: crates/demux/tests/mkv_attachments_chapters.rs:308-322 and 333-376
Code:
```
    if !cover.exists() {
        run &[
```
```
    if !out.exists() {
```
What is wrong: fixture caching in the shared temp dir — a stale `d2a-generated.mkv` from an older run/ffmpeg is reused without regeneration. Self-consistent (ffprobe truth is re-measured on the same bytes), so no false-pass risk; only staleness hygiene. Confidence: HIGH — Verification: read + dir listing shows the cached artifacts.

## 4. Per-function table (test functions)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| corpus_dir | 36-42 | REAL | S3 | machine-local gate (F-001) |
| ffprobe_chapters / ffprobe_attachments | 52-125 | REAL | S4 | scraping, fails loud (F-002/F-003) |
| assert_lane_matches_ffprobe | 128-242 | REAL | — | count + zipped field differential + packet-flow sanity; solid |
| corpus_attachments_chapters_differential_vs_ffprobe | 244-289 | REAL-UNPROVEN | S3 | skipped here/CI; named bomb lanes unverified (F-001) |
| generated_fixture | 293-378 | REAL | S4 | real ffmpeg generation; caching (F-004) |
| generated_fixture_attachments_chapters_differential | 380-436 | REAL | — | executed live: byte-exact FileData + exact ns chapters + titles + mime |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| corpus_attachments_chapters_differential_vs_ffprobe | YES *where it runs* | live ffprobe differential per lane — but it runs only on the corpus machine (F-001) |
| generated_fixture_attachments_chapters_differential | YES | live ffprobe differential + byte-exact attachment bytes + exact chapter ns/titles; executed here |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "chapters … start/end nanoseconds (… chapter times do not scale with TimestampScale; verified live)" | mkv_attachments_chapters.rs:6-8 | REAL | ffprobe on the generated fixture shows 1/1e9 time_base, ns values; differential passed live |
| "the corpus regenerates with fresh seeds (chapter-bomb included)" | mkv_attachments_chapters.rs:19-20 | REAL-UNPROVEN | corpus absent here; skip observed |
| "the suite skips loudly" | mkv_attachments_chapters.rs:21-22 | REAL | "SKIP: corpus-v4 absent …" printed on --nocapture |

## 7. Dead / duplicated / unreachable code
- The ffprobe JSON `field` closure is duplicated verbatim across `ffprobe_chapters` and `ffprobe_attachments` and again in mkv_fixtures.rs `probe_packets` — three copies of the same scraper (extract-to-common debt).

## 8. Dependencies & flags
- Test-only: common (open_via_input), demux::mkv::{MkvDemuxer, attachments::FileData}, testkit::Must, ffmpeg/ffprobe on PATH. No feature gates.

## 9. Open questions for the next agent
- F-001: does any committed fixture carry a chapter-bomb shape? If not, the mkv chapters parser's adversarial coverage is corpus-machine-only — the mkv-files agent should verify `chapters.beyond_duration`/damage paths against something reproducible.
- The attachments byte-exact check asserts `FileData::Inline` — what is the cut-over to the non-inline variant (large attachments), and is that path covered anywhere reproducible?
