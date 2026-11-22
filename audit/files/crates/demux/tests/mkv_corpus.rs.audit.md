# AUDIT: crates/demux/tests/mkv_corpus.rs

| Field | Value |
|---|---|
| Auditor | W05-A06 |
| Date | 2026-11-13 |
| Lines / functions | 293 / 6 units (1 helper fn corpus_dir, 2 structs, 2 fns, 2 tests) |
| File verdict | SHALLOW |
| Reachable from shipped entry point? | N/A (integration test). Runs under `cargo test -p demux`. In CI it compiles, runs, and **passes vacuously** — same gating as corpus_mp4.rs: no workflow sets `LUMEN_CORPUS` (grep of `.github/workflows/` → zero hits), default dir `/tmp/my-project/corpus-v4/media` absent in this sandbox (verified `test -d` → ABSENT, while `ffprobe`/`ffmpeg` ARE present at `/usr/bin` — the corpus bytes are the only missing ingredient). The skip is `eprintln!` + `return` on a plain `#[test]` → counted in W00's "1751 pass". |
| Findings | S0:0 S1:0 S2:1 S3:2 S4:2 |
| Commands run | `Read` full file; `sed -n '8,11p;184,199p;249,252p'` (quote verification, exit 0); `test -d /tmp/my-project/corpus-v4` → ABSENT; `which ffprobe ffmpeg` → both present; `rg 'mkv_corpus\|0005-muxed-av\.mkv\|0016-mkv-video\|0042-meta-bomb-av' docs/worklog/` → aggregate line only (archive-2026-10.md:9970), no per-file table; `rg -o mkv/webm lanes` from `docs/corpus/corpus-v4-smoke-manifest-20261023.json` (exit 0). No cargo (adaptation). |

## 1. One-paragraph verdict

The MKV/WebM corpus differential — and it is designed *better* than its MP4 sibling: the ffprobe truth is measured live at run time (`Command::new("ffprobe")`, not pinned constants, so the "regenerates with fresh seeds" model is coherent), files open through the real `input::FileAccess` stack, damage events (`Recovered`) and demux errors fail lanes outright, and the seek test genuinely asserts resume (`assert!(got > 0)`) plus landing bound — the assertions the MP4 seek test lacks. But it shares the fatal environment gating: the corpus lives at a sandbox-private `/tmp/my-project/corpus-v4`, `LUMEN_CORPUS` is set by nothing in CI, so in every reproducible environment both tests take the skip path and report green while comparing nothing. Two of the module doc's claimed comparison dimensions are not implemented at all ("per-stream packet-timestamp ceilings", "keyframe count where ffprobe exposes it" — collected into tuples at lines 159-163, never asserted), and the hand-rolled JSON scraper degrades to vacuous PASS when ffprobe's shape drifts (missing fields are skipped, not failed). The promised per-file PASS/FAIL table in the worklog does not exist (grep: zero hits on lane filenames; only an aggregate "mkv_corpus 2/0" line). SHALLOW: the strongest differential design in the demux crate, structurally unable to run anywhere that counts.

## 2. What actually works (verified)

- Live-oracle design (no pinned truths): `ffprobe_streams` (50-89) shells out per file with `-count_packets -of json`; the comparison at 180-218 asserts track count, per-stream packet count, codec identity, and track kind. With corpus + ffprobe present, a demux miscount or codec mislabel fails the suite (`assert_eq!(fail, 0)` at 221). Mutation reasoning (UNVERIFIED by run): break the block walker's lacing and packet counts diverge → FAIL lane.
- Damage sensitivity: `Ok(DemuxEvent::Recovered {..})` sets `clean = false` (165-167) which forces a FAIL (209-210) — a concealing demuxer cannot pass the differential. Demux `Err` fails the lane (169-173). Packet-storm guard at 176.
- The seek round-trip (227-292) is a real AC: drains to learn the last stamp, reanchors at mid-point, asserts packets flow afterwards (`assert!(got > 0, "{name}: no packets after mid-point reanchor")`, line 280) and the landing is at-or-before target+1 (line 282). This is what corpus_mp4.rs's seek test should have been.
- Real access stack: `open_via_input` (common/mod.rs:546-550) — verified in the corpus_mp4.rs audit.
- Environment probed by me: `ffprobe` and `ffmpeg` are installed in this sandbox (`which` → `/usr/bin/ffprobe`, `/usr/bin/ffmpeg`) — the suite would actually run its oracle here if the corpus directory existed; it does not (`/tmp/my-project/corpus-v4` ABSENT).

## 3. Findings

### M1 [S2] The differential passes vacuously in CI and fresh clones — identical gating defect as corpus_mp4.rs
Where: crates/demux/tests/mkv_corpus.rs:33-39, 105-108
Code:
```rust
fn corpus_dir() -> Option<PathBuf> {
    let dir = std::env::var("LUMEN_CORPUS")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from("/tmp/my-project/corpus-v4"));
```
and
```rust
    let Some(media) = corpus_dir() else {
        eprintln!("SKIP: corpus-v4 absent (LUMEN_CORPUS unset, /tmp/my-project/corpus-v4 missing)");
        return;
    };
```
What is wrong: the suite's value depends on a directory that exists only on the author's sandbox. Verified: `/tmp/my-project/corpus-v4` ABSENT here; no CI workflow or script sets `LUMEN_CORPUS` (grep, zero hits); the manual-dispatch `compat-matrix.yml` corpus sweep drives `tests/compat`, not this suite. Both tests are plain `#[test]`s whose skip path `return`s `Ok`.
Why it matters: the module's headline claim — "every `.mkv`/`.mka`/`.webm` lane of the randomized real corpus ... compared against **live ffprobe truth**" (lines 1-6) — executes nowhere reproducible. W00's "tests 1751 pass" includes these two as passes that compared nothing. The one corpus artifact committed in-repo is a file *list* (`docs/corpus/corpus-v4-smoke-manifest-20261023.json`), not the media.
What correct looks like: commit small mkv/webm fixtures (the crate already commits adts/ps fixture bytes) and always run the live differential against them.
Confidence: HIGH — Verification: `test -d`, `which`, repo-wide grep (all exit codes observed).

### M2 [S3] Two of the doc's claimed comparison dimensions are never asserted — keyframes and timestamp ceilings are collected and dropped
Where: crates/demux/tests/mkv_corpus.rs:5-6, 18 (claims) vs 135-163 (collection) vs 184-208 (comparison)
Code:
```rust
//! per-stream codec identity, packet counts (`ffprobe -count_packets`),
//! and per-stream packet-timestamp ceilings.
```
and the collection:
```rust
        let mut ours: Vec<(u64, i64, i64, u64, String)> = Vec::new(); // (n, first, last, keyframes, codec)
```
```rust
                    slot.0 += 1;
                    slot.3 += u64::from(p.keyframe);
                    if let Stamp::Present(ts) = &p.pts {
                        slot.1 = slot.1.min(ts.ticks);
                        slot.2 = slot.2.max(ts.ticks);
                    }
```
What is wrong: the comparison loop (184-208) asserts only `slot.0` (packet count), `slot.4` (codec), and kind. `slot.1`/`slot.2` (first/last packet timestamps — the doc's "packet-timestamp ceilings") and `slot.3` (keyframes — the doc's "keyframe count where ffprobe exposes it", line 18) are computed for every packet and never compared against anything; `ffprobe_streams` does not even scrape a keyframe field. They surface only inside the FAIL println at line 217.
Why it matters: the module doc promises a four-dimension differential; two dimensions are dead weight that costs per-packet work (`min`/`max` on every packet) and asserts nothing. A demuxer that stamps every packet at t=0 or flags every block a keyframe passes this suite clean. Doc-vs-code FALSE claim (§3.4: test names/promises exceeding the body).
What correct looks like: scrape `nb_read_packets` per stream plus ffprobe's first/last packet pts and keyframe flags (`-show_packets -show_entries packet=pts_time,flags`) and assert them, or delete the claim and the dead tuples.
Confidence: HIGH — Verification: read — every use of `slot.1`/`slot.2`/`slot.3` traced (only lines 137, 159-163, and the debug println).

### M3 [S3] The ffprobe scraper degrades to vacuous PASS on oracle-shape drift
Where: crates/demux/tests/mkv_corpus.rs:68-87 (scraper) and 185, 193, 199 (skip-on-missing)
Code:
```rust
        streams.push(ProbeStream {
            index: streams.len(),
            codec_name: field("codec_name").unwrap_or_default(),
            kind: field("codec_type").unwrap_or_default(),
            nb_read_packets: field("nb_read_packets").and_then(|v| v.parse().ok()),
        });
```
with the guards:
```rust
            if let Some(expected) = ps.nb_read_packets
                && slot.0 != expected
```
```rust
            if !slot.4.is_empty() && slot.4 != ps.codec_name {
```
What is wrong: every field the hand-rolled JSON scrape fails to find becomes empty/`None`, and every comparison whose truth-side is empty/`None` is silently skipped (185, 193, 199). If a future ffprobe renames a field, emits a different JSON layout, or the scrape otherwise misses, lanes PASS with only the track count compared. This exact failure class has already bitten this file: the worklog records that `ProbeStream` fields were once unread and "the doc-claimed codec-identity assertion was NOT actually asserted, only packet counts" (docs/worklog/archive-2026-10.md:8619) — the current code fixed the *symptom* (fields now compared when present) but kept the *mechanism* (absent truth → skipped comparison → green).
Why it matters: a differential whose oracle-parse failure converts to a pass is error swallowing (§3.2). The one hard invariant left is the track count.
What correct looks like: `field(...)` missing on a probed stream must FAIL the lane ("oracle shape drift"), not skip.
Confidence: HIGH — Verification: read + worklog citation.

### M4 [S4] The "per-file PASS/FAIL table ... in the worklog entry" does not exist
Where: crates/demux/tests/mkv_corpus.rs:9-11
Code:
```rust
//! The comparison is measured at run time (not pinned): the corpus
//! regenerates with fresh seeds, and the per-file PASS/FAIL table for
//! this instance lives in the worklog entry. When the corpus is
//! absent (and no `LUMEN_CORPUS` override) the suite **skips loudly**.
```
What is wrong: grep of `docs/worklog/` for the corpus's mkv lane filenames (`0005-muxed-av.mkv`, `0016-mkv-video`, `0042-meta-bomb-av` — names taken from the committed manifest `docs/corpus/corpus-v4-smoke-manifest-20261023.json`) returns zero hits. The only worklog evidence is the aggregate gate line "mkv_corpus 2/0 (live-ffprobe differential, LUMEN_CORPUS=/tmp/my-project/corpus-v4)" (archive-2026-10.md:9970). No per-file table exists.
Why it matters: same dangling-evidence-pointer pattern as corpus_mp4.rs C4 — the skip's stated mitigation is fiction; the differential's evidence is one unsourced aggregate line.
What correct looks like: commit the per-lane table (the test already prints `PASS {name}: ...` lines — capture them).
Confidence: HIGH — Verification: grep, zero hits (exit code observed).

### M5 [S4] No-cues seek lanes count as PASS without testing anything
Where: crates/demux/tests/mkv_corpus.rs:249-252
Code:
```rust
        if demuxer.cue_index().is_none() {
            println!("PASS {name}: seek (no cues — SlowLinear path untested here)");
            continue;
        }
```
What is wrong: a lane without a Cues index prints `PASS` and increments the pass counter without performing any seek; `assert!(pass > 0)` (line 291) can be satisfied entirely by such lanes (a corpus of cue-less files would green the seek test while testing zero seeks).
Why it matters: tally inflation; the message text is honest ("SlowLinear path untested here") but the verdict line is not.
What correct looks like: print `SKIP` and track a separate skip counter; assert `tested > 0`.
Confidence: HIGH — Verification: read.

## 4. Per-unit table

| Unit | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc | 1-19 | SHALLOW | S3 | Claims 4 comparison dimensions; 2 unimplemented (M2). |
| `corpus_dir` | 33-39 | REAL | S2 | Env override else sandbox `/tmp` default; absent → skip (M1). |
| `ProbeStream` (struct) | 42-48 | REAL | S4 | `index` is the scrape ordinal, not ffprobe's index field (assigned `streams.len()` at 82) — used only in messages; latent mismatch if ffprobe ever numbers non-sequentially. |
| `ffprobe_streams` | 50-89 | REAL-UNPROVEN | S3 | Hand-rolled JSON scrape; missing fields → empty → comparisons skipped (M3); `String::from_utf8_lossy` on stdout; `output().must()` panics loudly if ffprobe is absent (correct-fail, only reachable when the corpus exists). Not executed by me (corpus absent). |
| `kind_probe_name` | 94-101 | REAL | — | Total mapping of the 4 MediaKind variants; unmapped kinds would surface as kind mismatches, not smoothing — honest. |
| `corpus_matroska_differential_vs_ffprobe` | 103-222 | SHALLOW | S2 | Real live differential with damage sensitivity, storm guard, per-lane PASS/FAIL printout, hard `assert_eq!(fail, 0)` — all gated on the absent corpus (M1); keyframes/timestamps collected-not-asserted (M2); `ours[handle as usize]` bounds-checked manually (152-156) — better than corpus_mp4's raw index. |
| `corpus_matroska_seek_roundtrip` | 227-292 | SHALLOW | S4 | Real AC (resume + landing asserted) — the mp4 suite's missing half is present here; no-cues lanes inflate the tally (M5); `reanchor` + `next_event().must()` — panics on error (correct-fail). |

## 5. Test honesty

| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| corpus_matroska_differential_vs_ffprobe (103) | YES with corpus+ffprobe: counts/codec/kind/damage all hard-asserted (`assert_eq!(fail, 0)`). NO in CI/fresh clone (vacuous skip, M1). A keyframe/timestamp regression passes even WITH the corpus (M2). | Environmental gating + missing dimensions. |
| corpus_matroska_seek_roundtrip (227) | YES with corpus: `got > 0` and landing bound asserted per lane. NO in CI (skip); no-cues lanes always "pass" (M5). | Gating + tally inflation. |
| Attribution to W00 baseline | UNVERIFIED | No cargo; skip mechanism guarantees "pass" wherever the corpus is absent — verified absent here. |

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every `.mkv`/`.mka`/`.webm` lane ... compared against **live ffprobe truth**" | mkv_corpus.rs:1-6 | SHALLOW | True only on the author's sandbox; corpus absent in CI and here (M1). |
| "... and per-stream packet-timestamp ceilings" | mkv_corpus.rs:5-6 | FALSE | slot.1/slot.2 collected (160-163), never asserted (M2). |
| "keyframe count where ffprobe exposes it" | mkv_corpus.rs:18 | FALSE | slot.3 collected (159), never asserted; ffprobe scrape has no keyframe field (M2). |
| "the per-file PASS/FAIL table for this instance lives in the worklog entry" | mkv_corpus.rs:9-10 | FALSE | Zero worklog hits on lane filenames; only aggregate line archive-2026-10.md:9970 (M4). |
| "the corpus regenerates with fresh seeds" | mkv_corpus.rs:9 | REAL-UNPROVEN | Coherent with the live-measure design (unlike corpus_mp4's pinned truths); generator not run by me. |
| (worklog) "mkv_corpus 2/0 (live-ffprobe differential, LUMEN_CORPUS=/tmp/my-project/corpus-v4)" | archive-2026-10.md:9970 | UNVERIFIED | No run possible; the 2/2 pass claim is one aggregate line with no per-file table anywhere (M4). |
| (worklog, history) "the doc-claimed codec-identity assertion was NOT actually asserted" — since fixed | archive-2026-10.md:8619 | REAL | The fix is in the current code (193-198); the same silent-skip mechanism remains for other fields (M3). |

## 7. Dead / duplicated / unreachable code

- `slot.1`, `slot.2`, `slot.3` of the `ours` tuples: collected per packet, asserted nowhere (M2) — dead computation on the tested path.
- `ProbeStream.index`: ordinal assigned, never ffprobe's actual index — message-only.
- `corpus_dir` duplicates the skip convention of every sibling corpus suite (corpus.rs, corpus_wav.rs, corpus_mp4.rs, ...) — five copies of the same gating logic, one per file.

## 8. Dependencies & flags

- Runtime deps: `ffprobe` on PATH (external binary), the corpus directory. Build deps: `plugin_api`, `media_types`, `demux::mkv::MkvDemuxer`, `common::open_via_input` → `input`, `testkit::Must`. No feature flags. The external-binary dependency is undeclared in Cargo.toml (it cannot be) — the `.must()` panic on absence is the only guard, and it is unreachable in CI because the corpus gate fires first.

## 9. Open questions for the next agent

- mkv/mod.rs owner (seek path): `reanchor` vs the `Demuxer::seek` trait method used by corpus_mp4.rs — two different seek APIs exercised by the two corpus suites; which one does the shipped CLI host call? (apps/cli/src/containers/mkv.rs owner should answer; UNRESOLVED here.)
- Claim-ledger agents: does any feature-matrix row cite mkv_corpus.rs? Its CI-invisible pass has the same evidence-integrity problem as corpus_mp4.rs.
- tools/corpus owner: is there any committed, deterministic recipe to regenerate corpus-v4's mkv lanes (seed + gen_corpus.py pin) so the differential can run anywhere? The manifest JSON lists files but no seeds.
