# AUDIT: crates/subtitles/tests/fixtures/fuzz/crlf2.srt (text fixture)
| Field | Value |
|---|---|
| Auditor | W09-A10 |
| Date | 2026-11-23 |
| Size / type | 156 bytes, pure-CRLF SRT text — verified by od + python byte-diff |
| File verdict | REAL (byte-exact CRLF rewrite of the canonical parent) |
| Reachable from shipped entry point? | NO — test fixture; consumed by tests/srt_fuzz_survival.rs `committed_corpus_survives` (all-37-files loop) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `od -c` (every line ends `\r\n`); python3 byte-diff vs fixtures/canonical/out2.srt: length 144→156 (+12), EXACT match of `parent.replace(b'\n', b'\r\n')`; baseline citation |

## 1. Blunt verdict
Byte-verified: `out2.srt` with all 12 LFs rewritten to CRLFs, nothing else — the name says exactly what it is (crlf2 = CRLF rewrite of parent 2). This exercises the parser's line-ending normalization on an otherwise perfectly clean 3-cue file (Hello world / Second line here / Quick flash cue). Consumer `committed_corpus_survives` passes in baseline (:6113); the parent itself is separately asserted clean by `canonical_parents_still_parse_exactly` (:6112).

## 2. Notes
- The name "crlf2" and the mutation policy ("CRLF rewrites", srt_fuzz_survival.rs:25) match the bytes — no discrepancy.
- Hostile check: no hidden second mutation (byte-diff proves single-class rewrite). The generator-script absence note from the xff3_048 audit applies here too.
