# AUDIT: crates/codecs/tests/fixtures/sine48_lc_mono.aac (binary asset)
| Field | Value |
|---|---|
| Auditor | W06-A01 |
| Date | 2026-11-24 |
| Type / size / sha256 | MPEG ADTS, AAC v4 LC, 48 kHz, monaural (per `file`) / 13201 bytes / 6e11e8c04db377f2892fbd07642c6bf001b596cb12b24394992a089f61417051 |
| File verdict | REAL (load-bearing fixture, consumed by five test surfaces + two docs; the consuming suites pass live) |
| Reachable from shipped entry point? | N/A (test fixture; not shipped) — the decoder it exercises (`aac-lc`, symphonia) is shipped |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## 1. Blunt verdict
A genuine 13 KiB ADTS AAC-LC sine fixture with a multi-suite consumer graph and independently-reproducible oracle pins; the suites that assert its bytes run green live.

## 2. Evidence
- Consumers (rg): `crates/codecs/tests/aac_bitexact.rs:38` (FIXTURES list; window-exact decode + tolerance rows), `crates/codecs/tests/aac_corpus_differential.rs:183`, `apps/cli/src/containers/adts.rs:254,281,325` (open-path facts: 48 000 Hz, 1 ch, 142 packets), `apps/cli/src/host.rs:1748` (integration), `crates/media-probe/tests/registry_roundtrip.rs:126`.
- Reference pins: `crates/codecs/tests/data/aac_bitexact_reference.txt:33-38` — rate 48000, 1 channel, 142 frames, 145 408 samples/channel, `lumen_pcm_md5 = e8f06389...`, `ffmpeg_pcm_md5 = 0b5b973f...`, `max_sample_delta = 1` (honest 1-LSB tolerance row, not fake bit-exactness).
- Doc/matrix evidence: `docs/formats/audio-aac.md:58` and `docs/worklog/archive-2026-10.md:8906` ("53 (0.04%) | 1 LSB | PASS" differential row).
- Ran live: `timeout 90 ./target/debug/deps/aac_bitexact-1e99e0008b8875c2 --test-threads=2` → `2 passed` (the fixture's recorded window asserted exactly).

## 3. Notes
- The prior aac_bitexact audit (W06 wave, audit/files/crates/codecs/tests/aac_bitexact.rs.audit.md:17) already reproduced this fixture's ffmpeg oracle MD5 independently — the pin is externally reproducible, not self-generated.
- No provenance concern: ffmpeg-authored test fixture per the aac format doc's fixture inventory.
