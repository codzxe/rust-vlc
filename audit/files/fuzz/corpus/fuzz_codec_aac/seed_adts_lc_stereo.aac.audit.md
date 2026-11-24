# AUDIT: fuzz/corpus/fuzz_codec_aac/seed_adts_lc_stereo.aac
| Field | Value |
|---|---|
| Auditor | W16-A06 |
| Date | 2026-11-24 |
| Kind | Fuzz seed corpus file (ADTS AAC-LC stereo) |
| Size / sha256-prefix | 25219 bytes / 1c934f5b8f71aace (verified locally) |
| File verdict | REAL — ffmpeg-authored ADTS stream, doubled into fuzz_demux_adts |
| Reachable from shipped entry point? | NO — fuzz seed only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 sha256, header sniff (`\xff\xf1P\x80...Lavc61.19.101` = ADTS syncword + Lavc encoder tag) |

## 1. Verdict
Real ADTS stream: first bytes `\xff\xf1\x50\x80` is a valid ADTS syncword
(0xFFF1, MPEG-4, no CRC), and the encoder tag `Lavc61.19.101` inside the
first frame proves external-encoder (ffmpeg) authorship — consistent with
the seed policy of copying committed ffmpeg-authored fixtures. The identical
file also sits in fuzz/corpus/fuzz_demux_adts/ (both target dirs list
`seed_adts_lc_stereo.aac`), which is the registry's deliberate
cross-pollination. Consumed via cargo-fuzz convention; presence gated
(baseline full_gate.txt:57-58 PASS).

## 2. What actually works (verified)
- Directory listing: `fuzz/corpus/fuzz_codec_aac` = [seed_adts_fuzz_base.aac,
  seed_adts_lc_mono.aac, seed_adts_lc_stereo.aac]; same filename in
  fuzz/corpus/fuzz_demux_adts/ — verified via python3 os.listdir.
- Header bytes verified (ADTS sync + Lavc tag).

## 3. Findings
None. (Whether codec-aac and demux-adts seeds are byte-identical is
UNVERIFIED — not needed for this verdict; only presence + validity matter.)
