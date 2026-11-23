# AUDIT: crates/subtitles/tests/fixtures/vtt/long.vtt
| Field | Value |
|---|---|
| Auditor | W09-A04 |
| Date | 2026-11-23 (repo frozen) |
| Size / type | 180 B; `file`: "WebVTT subtitles, Unicode text, UTF-8 text" |
| Fixture verdict | REAL — content byte-verified against its consumer's exact assertions |
| Cited as evidence by | vtt_acceptance.rs:31 (`LONG`) -> `ffmpeg_long_form_hours_and_unicode_equal_truth` (vtt_acceptance.rs:76-100) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `cat` fixture; `./target/debug/deps/vtt_acceptance-0e0b6ef4ae2414a1` -> 4 passed (exit 0); hand re-derivation of all asserted µs values and entity decoding |

## 1. Blunt verdict
Real three-cue WebVTT fixture delivering exactly the surface its name and
doc row promise: the short `MM:SS.mmm` form (00:00.500 --> 00:02.000), the
long `HH:MM:SS.mmm` form over an hour (01:23:45.678 --> 01:23:50.000 and
01:23:51.000 --> 01:24:02.500), UTF-8 payload (日本語), and the entity
edge `&lt;tag&gt; &amp; &quot;quotes&quot;` — where `&lt;`/`&amp;` decode
but `&quot;` must stay literal (WebVTT defines exactly six named escapes;
`quot` is not one). Every asserted µs re-derived by hand: 01:23:45.678 =
5,025,678,000 ✓; 01:24:02.500 = 5,042,500,000 ✓. Sibling verification:
the vtt_acceptance.rs auditor regenerated all three ffmpeg-route fixtures
byte-exact with the sandbox's ffmpeg 7.1.5
(audit/files/crates/subtitles/tests/vtt_acceptance.rs.audit.md §1).

## 2. Verification
— Consumer run: `vtt_acceptance-0e0b6ef4ae2414a1` -> 4 passed (includes the
LONG test: 3 cues, `doc.damage` empty, exact spans and texts).
— Fixture bytes (cat): header + 3 cues exactly as above; no BOM, LF
terminators, matching the assertions' expected plain texts including the
literal `&quot;`.

## 3. Issues
None in the fixture. (Source-SRT provenance for the "ffmpeg re-order"
claim is reconstruction-only — covered by the vtt_acceptance audit, not a
fixture defect.)
