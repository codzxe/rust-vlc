# AUDIT: crates/codecs/tests/fixtures/alac/alac_mono16.m4a
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | binary (24051 bytes) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture; lane row of crates/codecs/tests/alac_bitexact.rs:80) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | file; ffprobe; ffmpeg pcm_s16le decode -> md5sum (== documented pin); rg consumer trace |

## 1. One-paragraph verdict
Genuine ALAC fixture whose every documented fact reproduces independently: `file` → "ISO Media, Apple iTunes ALAC/AAC-LC (.M4A)"; ffprobe → codec alac, 44100 Hz, **mono**, 16 bits-per-raw-sample, 0.7 s — the filename's `mono16` claim is exact. My own ffmpeg decode yields s16le MD5 `a2a11b40df939a1d604fd71233cf83e8`, which is byte-identical to the pin documented in the consuming suite's table (alac_bitexact.rs:24). The baseline ran the alac suites green.

## 2. What actually works (verified)
- ffprobe facts == filename + suite table (alac, 44_100, mono, s16le).
- `ffmpeg -i alac/alac_mono16.m4a -map 0:a -c:a pcm_s16le -f s16le - | md5sum` → `a2a11b40df939a1d604fd71233cf83e8` == alac_bitexact.rs:24 pin (sed-verified).
- Consumers: alac_bitexact.rs (also alac_bench/alac_fuzz_survival per rg).

## 3. Findings
None. Checked: container type, codec/rate/channels/depth vs name, decode cleanliness, MD5 vs documented pin, consumers, no other doc citations (rg).

## 4-9. (binary asset — tables N/A; tests: the consuming bit-exact suite's pins hold; no dead code; no deps; no open questions)
