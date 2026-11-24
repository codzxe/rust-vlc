# AUDIT: tests/fixtures/generated/malformed/wav_pcm_s16le_44k1_st__trunc3
| Field | Value |
|---|---|
| Auditor | W16-A09 |
| Date | 2026-11-24 |
| Size / sha256 (recomputed) | 17,679 B / 3d4a775b9a590cba… (16-hex prefix; full digest in audit/runs/W16-A09-batchmeta.txt) |
| File verdict | REAL (data fixture; derivation independently re-executed) |
| Reachable from shipped entry point? | N/A (test data) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

What it is: WAV class-B truncation derivative (extension-less family naming):
byte-exact prefix of the valid parent — python `trunc3 == valid[:17679]` -> True;
fmt(16) + LIST(26) + data-header at 70 intact, so the cut lands mid-`data`
(17,601 payload bytes = 8,800.5 stereo s16 samples — an odd-byte partial-sample
cut, which is the point of variant "trunc3").

Consumer: crates/demux/tests/wav_aiff_fixtures.rs:662-667 pins the prefix:
`let trunc3 = testkit::fixture_bytes("generated/malformed/wav_pcm_s16le_44k1_st__trunc3").must();
 assert_eq!(&trunc3[78..78 + 17_600], &parent_payload[..17_600], "trunc3 prefix");`
(verified against recomputed bytes: data payload starts at 78).
Generated-vs-handmade: generator-derived (xtask fixtures family).
Matrix-evidence: none. S4: same ledger gap as the WAV family siblings — absent
from tests/fixtures/CORPUS.md (28 rows) and MANIFEST.toml (28 `path =` entries),
so no `xtask fixtures --check` drift gate covers it.
