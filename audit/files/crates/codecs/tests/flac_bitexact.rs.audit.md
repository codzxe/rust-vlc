# AUDIT: crates/codecs/tests/flac_bitexact.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 254 / 4 tests |
| File verdict | REAL (executed, byte-exact differential + oracle-free signature) |
| Reachable from shipped entry point? | N/A (test); FlacDecoder is shipped (pipeline_host.rs:253) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | executed prebuilt `flac_bitexact-0363eae80343785c --nocapture --test-threads=1` -> 4 passed / 0 failed, 0.21s (corpus test SKIP printed); baseline cited (identical) |

## 1. One-paragraph verdict
The strongest test shape in my assignment set: a five-step chain per fixture — ffprobe-agreeing facts, ffprobe-pinned frame split and per-frame pts, contract-path PCM MD5 vs the recorded ffmpeg dump (width-lifted for 24-bit), the encoder's OWN STREAMINFO MD5 (an oracle-free end-to-end signature), and free-surface/contract-surface byte identity — plus a seektable resolution test, the fuzz-found channel-mismatch panic class pinned as typed, and a corpus lane with byte-exact + typed-rejection rows. Executed live: 4/4 in 0.21s. A single wrong decoded byte fails this suite twice (ffmpeg MD5 and STREAMINFO MD5). Only hygiene notes.

## 2. What actually works (verified)
— verified by executing the prebuilt binary: `test result: ok. 4 passed; 0 failed; ... finished in 0.21s`, corpus test printed its SKIP (corpus absent).
— verified baseline audit/runs/baseline/cargo-test-workspace.txt:1066-1074 -> same 4/4.
— chain-of-custody: the reference file (tests/data/flac_bitexact_reference.txt, 5501 bytes) records generation commands + MD5s; the suite's assertions consume them strictly (`.must()` parses).

## 3. Findings
F-001 [S4]
Where: crates/codecs/tests/flac_bitexact.rs:199-204
Code: (verbatim)
```
    let Some(root) = corpus_dir() else {
        eprintln!("corpus: LUMEN_CORPUS unset and /tmp/my-project/corpus-v4 absent — SKIP");
        return;
    };
```
What is wrong: the corpus lane (0046 24-bit differential, 0106 metadata-only, 0071 ID3 mislabel) self-skips in this environment — its assertions never ran here; baseline "ok" for this test proves nothing about the corpus rows on this host. Confidence: HIGH — Verification: ran (SKIP printed).
F-002 [S4]
Where: crates/codecs/tests/flac_bitexact.rs:113-117
Code: (verbatim)
```
            bps if bps <= 24 => {
                let lifted = md5_hex(&lift_s24_to_s32le(&pcm));
                assert_eq!(lifted, expect["pcm_s32le_md5"], "{name}: s24→s32le PCM");
            }
            _ => unreachable!("{name}: no 32-bit fixture in this set"),
```
What is wrong: 32-bit FLAC has no committed fixture — the bps-32 decode path is untested by this suite (self-admitted by the unreachable arm). Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| fixtures_decode_byte_exact_against_the_ffmpeg_reference | 49-141 | REAL | - | the 5-step chain; executed; every MD5 assert is a hard pin |
| seektable_carrier_exposes_six_verified_points | 143-162 | REAL | - | resolution-verify rule per point; executed |
| channel_mismatch_on_a_decodable_frame_is_typed_not_panic | 168-194 | REAL | - | fuzz-found panic class pinned typed (flac.frame.channels, Corrupt); executed |
| corpus_flacs_decode_byte_exact_or_reject_typed | 199-254 | REAL-UNPROVEN (here) | S4 | self-skips without corpus (F-001); rows are strong when present (byte-exact + typed ID3 rejection) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| fixtures_decode... | YES | double MD5 oracles (ffmpeg + encoder signature) + pts/split pins; a 1-byte decode error fails |
| seektable... | YES | per-point resolution assert |
| channel_mismatch... | YES | exact code+kind; unreachable! on Ok |
| corpus... | NOT IN THIS ENV | self-skip |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "decodes byte-exactly against the recorded ffmpeg 7.1.5 reference" | L1-5 | REAL | executed; sandbox ffmpeg version matches the reference header's recorded author version |
| "the encoder's own integrity signature, an end-to-end check that needs no oracle at all" | L19-21 | REAL | step 4 compares md5(pcm) to the STREAMINFO md5 field |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
Test-only; testkit::Must; common::md5_hex. No feature gates.

## 9. Open questions for the next agent
- Add a 32-bit FLAC fixture to close F-002 (the decoder's 32-bit path — if it exists — is unpinned).
