# AUDIT: fuzz/corpus/fuzz_sub_vtt/seed_vtt_hostile.vtt
| Field | Value |
|---|---|
| Auditor | W16-RR4 (recovery; original agent W16-A05) |
| Size / sha256 | 65 B / dcdfadbddd28093ee0dbfdf7452089482565aa0617e0d5ca64e7c7122af85c2c |
| Type | hand-authored hostile WebVTT (ASCII header + invalid-UTF-8 payload) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — fuzz corpus seed (campaign unwired) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 sha256/size/repr dump; rg consumers (99:99:99 unique to this file + sibling seeds); baseline greps |

## 1. One-paragraph verdict
Exactly the hostility the name promises, in 65 bytes: a WEBVTT signature, one cue
whose timing line is maximally out-of-range and reversed (`99:99:99.999 -->
00:00:00.000`), hostile cue settings (`vertical:lr`, `line:-10%`), and a payload
of `\xff\xfe\xff\x00` (UTF-16LE BOM + NUL — invalid UTF-8, exercising the harness's
`String::from_utf8_lossy` path, vtt/fuzz.rs:29). Consumed via the corpus-dir
convention by fuzz_sub_vtt only; that lane has no committed run.

## 2. What actually works (verified)
— python3 repr dump (verbatim): `b'WEBVTT\n\n99:99:99.999 --> 00:00:00.000
vertical:lr line:-10%\n\xff\xfe\xff\x00\n'`; 65 B; sha256 above.
— The timing pattern `99:99:99` occurs nowhere else in tree except the sibling
hostile seeds (seed_cue_hostile.cue, seed_ass_hostile.ass) and netfuzz_sweep's
TIMESTAMP-MAP soup — i.e., no test copies this file's content: the corpus copy is
the only artifact with these bytes.
— Registry wiring: fuzz_sub_vtt row (fuzz/README.md:83) → subtitles::vtt::fuzz::
fuzz_sub_vtt; corpus presence gate green (full_gate.txt:57-58, 35 targets).
— The harness the seed would feed ran its stable sweeps green in baseline
(cargo-test-workspace.txt:5986-5987, vtt::fuzz::tests::empty_and_marker_inputs_
survive / hostile_wide_inputs_survive — inline inputs, not this file).

## 3. Findings
F-001 [S3]
Where: fuzz/corpus/fuzz_sub_vtt/seed_vtt_hostile.vtt (no in-tree reader) vs crates/subtitles/src/vtt/fuzz.rs:38-62
Code: `#[test] fn hostile_wide_inputs_survive() { fuzz_sub_vtt(&vec![0xff; 4096]); ... }`
What is wrong: unexecuted fuzz lane — the seed is fed to nothing on stable: the
in-crate sweeps build inline hostile inputs, subtitles' fuzz_survival.rs seeds
from an inline `seed_document()` (:34), and no test reads fuzz/corpus/. Only the
never-run nightly campaign (fuzz/README.md:165-168 "No such run has happened yet")
would consume it. Same S3 family as seed_ass_hostile.ass / seed_cue_hostile.cue /
seed_wav_rf64 (W16-A01) and seed_adpcm_yamaha.bin (W16-RR2).
Why it matters: the corpus gate proves presence, not execution; this seed's
specific hostile geometry (reversed out-of-range timing + BOM bytes) exercises
nothing until the Lead wires the campaign.
Confidence: HIGH — Verification: read (rg over crates/ for the file name and its
content fingerprints; zero non-audit hits).

## 4. Matrix evidence
doc 04 §8.5 "hand-authored ... seeds (CC0, ours — the generated malformed files
arm)" — this file is that arm for WebVTT; README:99-110 corpus policy. No
feature-matrix row cites it.

## 5. Open questions for the next agent
- Its sibling seed_vtt_plain.vtt rides the same orphan pattern — same F-001 shape
  when assigned.
