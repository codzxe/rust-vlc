# AUDIT: fuzz/corpus/fuzz_codec_av1/seed_ivf_av1.ivf
| Field | Value |
|---|---|
| Auditor | W16-A03 | Date | 2026-11-24 |
| Bytes / sha256(16) | 1,937 / fa649131fde4eaa9 |
| File verdict | REAL (hand-authored/skeleton seed) |
| Reachable? | YES — cargo-fuzz corpus convention; presence enforced by fuzz_registry_check.sh:64 |
| Findings | S0:0 S1:0 S2:0 S3:0 S4:0 |

## 1. Verdict
Seed for fuzz_codec_av1. NOT a copy or prefix of any committed av1/vp9 ivf
fixture (python3 prefix scan over crates/codecs/tests/fixtures/{av1,vp9} —
no match). Header verified as a real IVF container: `44 4b 49 46` = "DKIF",
version 0, header length 0x20, codec FourCC "AV01" (bytes 8-11). Provenance
class: hand-authored/skeleton seed per fuzz/README.md:106-107 ("hand-authored
text/cookie/skeleton seeds"). 1,937 B — well under the 50 KB corpus budget.
## 2. Verified
— python3: magic parsed; no fixture parent found (distinct from the
testsrc2/rgb/smpte/tenbit ivf fixtures).
— Consumer: fuzz/Cargo.toml bin fuzz_codec_av1 (single-seed corpus — the
only target in my set with exactly 1 seed).
## 3. Findings
None.
