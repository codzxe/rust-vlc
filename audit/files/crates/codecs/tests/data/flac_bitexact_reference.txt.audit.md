# AUDIT: crates/codecs/tests/data/flac_bitexact_reference.txt (reference data)
| Field | Value |
|---|---|
| Auditor | W06-A10 |
| Date | 2025-07-14 |
| File verdict | REAL (independently re-derived by me) |
| Rows | 4 fixture sections + 3 corpus sections (1 full row, 1 `not_flac`, 1 `metadata_only`, 1 `pending-at-test-time` sha) |
| Consumed by | tests/flac_bitexact.rs (4/4 pass — executed earlier in this audit), flac_seek.rs, flac_fuzz_survival.rs |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `sha256sum` × 4 fixtures → all 4 recorded `sha256` values MATCH; `ffmpeg -f s16le | md5sum` on the seektable fixture → `61726f3206de0b9383c16f46dcdceb76` = recorded `pcm_s16le_md5`; same on flac_s16_stereo_44100 → `e0f96555d1465acbd566af4259f52c92` = recorded; `ffmpeg -f s32le | md5sum` on the 24-bit fixture → `e8e8caf60af492fe8bcbf78532f3739b` = recorded `pcm_s32le_md5`; python3 STREAMINFO extraction → md5 field `61726f32...` = recorded `streaminfo_md5` AND = the ffmpeg decode digest |

## 1. One-paragraph verdict
Reference data with real provenance and no inflation: the header records the exact ffmpeg 7.1.5 generation commands, the dump commands, the in-tree seektable-insertion script, and the honest admission that corpus files are /tmp-resident (SKIP semantics; the 0046 corpus sha is literally `pending-at-test-time`). I independently re-derived the evidence chain with the sandbox's own ffmpeg and sha256sum: all four fixture file hashes match, three decoded-PCM MD5 chains match, and the STREAMINFO-embedded MD5 equals both the recorded digest and ffmpeg's own decode — verifying the file's central claim (the FLAC MD5 convention + decoder byte layout cross-confirmation) end-to-end. The 24-bit s32le left-justify convention is documented and matches ffmpeg's behavior (verified). first_samples pin block starts at 4608 (matches STREAMINFO min/max blocksize 4608 seen in my byte dump of the fixture). Zero findings.

## 2. What actually works (verified)
— every command in the table above, all exact matches. This is the strongest form of reference data: re-derivable by anyone with ffmpeg, not a blessed output.

## 3. Findings
None. Checked: file hashes (4/4), PCM digests (3 chains), STREAMINFO MD5 convention (1 extraction), blocksize/first_samples consistency, corpus-row honesty markers, and the consumer suite (flac_bitexact 4/4 executed earlier).

## 4. Doc claims (this IS a doc/data file)
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every fixture's PCM essence is authored by the external ffmpeg 7.1.5-0+deb13u1 encoder" | L11-13 | REAL | re-derived digests with sandbox ffmpeg match |
| "for every file the STREAMINFO md5 field equals the native-width dump ... independently confirming the FLAC MD5 convention" | L18-22 | REAL | verified on the seektable fixture: file-embedded md5 == ffmpeg s16le digest == recorded value |
| "ffmpeg 7.1.5's flac muxer writes NO seektable ... blocks are STREAMINFO/VORBIS_COMMENT/PADDING" | L15-17 | REAL | consistent with my block walk of the final fixture (STREAMINFO/VORBIS_COMMENT/SEEKTABLE after in-tree insertion) |
| corpus rows SKIP when absent | L7-9 | REAL | SKIP behavior observed live in sibling suites |

## 5. Open questions for the next agent
None.
