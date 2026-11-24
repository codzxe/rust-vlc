# AUDIT: tests/fixtures/generated/malformed/aiff_pcm_s16be_44k1_st__trunc0
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 12 bytes |
| File verdict | REAL (derivation verified byte-level) |
| Reachable from shipped entry point? | NO (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 prefix check (exit 0) |

## 1. Verdict
12-byte truncation derivative: bytes `464f524d 000089fe 41494646` = `FORM` size 0x89fe +
`AIFF` — exactly the parent's first 12 bytes (verified: parent prefix match True). The cut
lands immediately after the form type: no COMM, no SSND — the demuxer must refuse typed.
Consumed by crates/demux/tests/wav_aiff_fixtures.rs AIFF_DERIVATIVES (`name: "trunc0"`,
terminal "demux.aiff.truncated", fails_at_start: true — verified read of the table).

## 2. Consumers
AIFF mutated-family walk (`aiff_mutated_family_outcomes_are_pinned_exactly`,
wav_aiff_fixtures.rs:597) — ran and PASSED in the committed baseline (PACKAGE demux
exit_code=0). Also the byte-surgery self-proof test (wav_aiff_fixtures.rs:675-700 region).

## 3. Findings
F-001 [S3] — same systemic manifest-gap (outside the 28-file drift gate; no generator row;
CORPUS.md:10-11's tree-wide claim FALSE here). Confidence: HIGH.

## 4. Open questions
None.
