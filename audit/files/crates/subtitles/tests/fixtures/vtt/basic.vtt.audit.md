# AUDIT: crates/subtitles/tests/fixtures/vtt/basic.vtt (text fixture)
| Field | Value |
|---|---|
| Auditor | W09-A10 |
| Date | 2026-11-23 |
| Size / type | ~142 bytes, UTF-8 WebVTT text, LF endings, no BOM (od -c: starts `WEBVTT\n\n`) — verified |
| File verdict | REAL (fixture, consumer test passes in baseline; all asserted values hand-checked) |
| Reachable from shipped entry point? | NO — test fixture; consumed via `include_str!` by crates/subtitles/tests/vtt_acceptance.rs:30 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `od -c` (header + LF endings, no BOM); read of consumer test vtt_acceptance.rs:1-88; baseline citation (:6143-6146) |

## 1. Blunt verdict
A clean 3-cue WebVTT fixture exercising exactly what its consumer (`ffmpeg_basic_cues_times_text_equal_truth`, baseline :6146 ok) asserts: short `MM:SS.mmm` timestamps (00:01.000→1_000_000 µs ✓, 01:02.004 = 1 min 2.004 s → 62_004_000 µs ✓ — hand-checked), the `&amp;` entity (decoded to "&"), multiline text, `<i>`/`<b>` inline tags, and id-less cues (auto ids 1,2,3 asserted). The parser is expected to produce ZERO damage on it — asserted. Content is internally consistent; every asserted value matches the bytes.

## 2. Notes
- The fixture doubles as honest evidence of a parity gap: the test asserts `CueText::Plain("Italic and bold text")` — i.e., the VTT parser STRIPS `<i>`/`<b>` to plain text rather than producing styled runs (the test helper at vtt_acceptance.rs:42 even states "A styled payload (never produced by this parser) fails every equality below"). VLC renders WebVTT inline styling; this parser drops it (the parser's own honesty, owned by the vtt module auditor — the fixture faithfully pins the dropping behavior).
- `&amp;` handling: decoded to "&" per the assertion — the six WebVTT named escapes minus `quot` per the long.vtt note (vtt_acceptance.rs:15-16).
- Auto-id behavior pinned: `vec![1, 2, 3]` for id-less cues — matches the auto_id counter start found in my model/cue.rs audit (vtt/mod.rs:143).

## 3. Findings
F-001 [S4] Provenance is self-attested: the consumer doc claims "three ffmpeg-generated WebVTT documents (`ffmpeg -c:s webvtt`, 7.1.5)" (vtt_acceptance.rs:1) but no ffmpeg exists in this sandbox to regenerate/diff, and no regeneration check is committed. Byte shapes are consistent with ffmpeg's webvtt encoder (short timestamp form under 1 h, `&amp;` preserved, tags preserved). UNVERIFIED, same class as ac_basic.ass.

## 4. Fixture-to-claim map
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| basic.vtt parses clean, 3 cues, exact times/text | vtt_acceptance.rs:46-73 | REAL | Baseline :6146 ok; values hand-checked against fixture bytes. |
| "ffmpeg-generated ... webvtt" provenance | vtt_acceptance.rs:1 | REAL-UNPROVEN | Consistent shape, no regeneration possible in sandbox (F-001). |
| "keeps `&amp;`-family escapes and `<i>`/`<b>` tags for OUR stripper" | vtt_acceptance.rs:9-10 | REAL | Fixture bytes + Plain-text assertion (tags are stripped, not styled). |
