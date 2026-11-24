# AUDIT: tests/fixtures/generated/malformed/aiff_pcm_s16be_44k1_st__trunc4
| Field | Value |
|---|---|
| Auditor | W16-A03 |
| Date | 2026-11-24 |
| Lines / functions | 3165 (binary; 35,322 bytes) / n/a |
| File verdict | REAL (data fixture, class-B truncation) |
| Reachable from shipped entry point? | NO (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | python3 sha256/md5 (de184540ab74640b…, md5 2afdc0c924d5af166349607e325bfaaa); python3 prefix-compare; rg consumers |

## 1. One-paragraph verdict
Class-B B1 truncation of the AIFF parent: byte-exact prefix of
generated/valid/aiff_pcm_s16be_44k1_st.aiff cut at 35,322 bytes (parent is
35,334 B; the last 12 bytes — 3 stereo 16-bit sample frames — are gone).
Verified byte-identical prefix. Consumed by the demux wav/aiff differential
suite with an explicit prefix assertion. Honest fixture; the family's
provenance is under-documented (see F-001).

## 2. What actually works (verified)
— python3: `trunc4 == aiff[:35322]` → True; sizes 35,334 → 35,322 (Δ12 B).
— Consumer assertion, crates/demux/tests/wav_aiff_fixtures.rs:687-692:
```
    let atrunc4 =
        testkit::fixture_bytes("generated/malformed/aiff_pcm_s16be_44k1_st__trunc4").must();
    assert_eq!(
        &atrunc4[54..54 + 35_268],
        &asound[..35_268],
        "aiff trunc4 prefix",
```
— Module doc records the surgery contract
(crates/demux/tests/wav_aiff_fixtures.rs:37-40):
`The \`__trunc0..4\`, \`__flip1/4/16\`, \`__garbage0\` derivatives were generated … truncations are exact parent prefixes at the recorded cut offsets`.

## 3. Findings
F-001 [S3]
Where: whole family (no in-tree generator); provenance doc gap.
Code: (crates/demux/tests/wav_aiff_fixtures.rs:38-41)
`generated from the two PCM parents by deterministic byte surgery (verified this session: ...`
What is wrong: the "session" that generated the derivatives is not a committed
tool. tools/xtask/src/cmd/fixtures.rs generates only the avi/mp4 families;
rg over tools/ and scripts/ for `aiff_pcm_s16be_44k1_st` finds no producer,
and tests/fixtures/CORPUS.md never mentions the aiff family.
Why it matters: the corpus cannot be regenerated or audited for drift from
source; a corrupted fixture would be silently re-pinned by the expectation
files. What correct looks like: a committed generator (xtask fixtures.rs
entry or tools/corpus script) + a CORPUS.md row.
Confidence: HIGH — Verification: read + rg.
F-002 [S4]
Where: filename (parent has `.aiff`, derivative has none; sibling
`__garbage0` also has none while `*.wav` derivatives keep no extension
either — but AVI/MP4 derivatives keep theirs).
What is wrong: inconsistent derivative naming convention across families.
Confidence: HIGH — Verification: ls tests/fixtures/generated/malformed/.

## 4. Matrix evidence
Cited as evidence in docs/corpus/compat-matrix-20261110.json/.raw-rows.jsonl
(rg hit) and scripts/quality/{23,27,28}_*.sh extension sweeps (".aiff" in
EXT set — but this file's extensionless name evades those sweeps; the valid
parent is what they count).
