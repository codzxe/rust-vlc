# AUDIT: tests/fixtures/generated/malformed/aiff_pcm_s16be_44k1_st__garbage0
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 39430 bytes / n.a. (5 assigned chunks L1-3304) |
| File verdict | REAL (data verified byte-level) |
| Reachable from shipped entry point? | NO (test fixture) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | python3 byte-surgery verification (exit 0); sed/rg on consumer (exit 0) |

## 1. One-paragraph verdict
Real class-B malformed AIFF derivative (garbage-append). Byte head
`464f524d 000089fe 41494646 434f4d4d` = `FORM` size 0x89fe (35326, body-after-8) + `AIFF` + `COMM`.
I verified the derivation myself: exact 35334-byte prefix of parent
generated/valid/aiff_pcm_s16be_44k1_st.aiff, then exactly 4096 appended bytes (seeded random,
256 distinct values, not 0xEE — the 0xEE claim exists only in the OGG family comments and is
true only there). Parent sound data survives byte-identically, matching the consumer's assertion.

## 2. What actually works (verified)
- python3: `garbage[0:35334] == parent` -> True; `len(garbage) - len(parent) == 4096` -> True.
- Consumer assertion exists and would fail if the fixture were not an exact prefix:
  crates/demux/tests/wav_aiff_fixtures.rs:681-687 (verified sed -n):
  `let agarbage = testkit::fixture_bytes("generated/malformed/aiff_pcm_s16be_44k1_st__garbage0").must();`
  `assert_eq!(&agarbage[54..54 + 35_280], asound, "aiff garbage0: identical sound");`
- Pinned outcome table: crates/demux/tests/wav_aiff_fixtures.rs:564-569 `name: "garbage0"` /
  `bytes: 35_280` / `terminal: ""` (full demux, garbage invisible).

## 3. Findings
F-001 [S3]
Where: this file + tests/fixtures/generated/MANIFEST.toml:3 (`file_count = 28`)
What is wrong: this fixture is one of ~88 files under tests/fixtures/generated/ (116 media
files on disk, manifest lists 28) with NO generator, NO MANIFEST row, NO drift-gate coverage.
tests/fixtures/CORPUS.md:10-11 claims "Every file under `generated/` ... authored in-tree by
the deterministic generator in tools/xtask/src/cmd/fixtures.rs" — false for this file: rg for
`aiff` in tools/xtask/src/cmd/fixtures.rs matches nothing but a comment.
Why it matters: silent drift/corruption of this fixture would pass `cargo xtask fixtures
--check`; the pinned-outcome tests would then test corrupted-input behavior by accident.
What correct looks like: extend the manifest/generator to the WAV/AIFF/OGG/MKV/3GP families or
stop claiming tree-wide coverage in CORPUS.md.
Confidence: HIGH — Verification: ran (byte-compare + rg counts).

## 4. Provenance / consumers
- Provenance: hand/agent-authored byte surgery (comment crates/demux/tests/wav_aiff_fixtures.rs:37-42,
  "generated from the two PCM parents by deterministic byte surgery (verified this session ...)");
  NO committed generator exists for it (rg over tools/ + scripts/ — no aiff writer).
- Consumers: crates/demux/tests/wav_aiff_fixtures.rs:681 (byte-equality test),
  AIFF_DERIVATIVES table (line 564) via `aiff_mutated_family_outcomes_are_pinned_exactly` (line 597).
- Matrix evidence: none found for this exact file (rg over docs/corpus — no hit). The
  compat-matrix raw-rows jsonl does not row it. UNVERIFIED: whether soak/compat lanes use it.

## 5. Open questions
- None for this artifact.
