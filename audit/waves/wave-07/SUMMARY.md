# WAVE 07 SUMMARY — audio codecs + audio-engine

| Field | Value |
|---|---|
| Agents | A01–A10 all complete |
| Files audited | 70 files (78 rows), 78/78 covered |
| Findings | S0:0 S1:1 S2:12 S3:57 S4:101 (171 total, from headers) |
| Citation gate | 149 checked, 3 rewrap defects, 0 fabrications |
| Tree purity | clean |

## Engine map (who actually decodes audio)
PCM: in-house (byte-exact, 39 tests). MP3: symphonia-bundle-mp3 =0.6.1.
MP2: in-house (differential vs live ffmpeg). AAC: symphonia-codec-aac =0.6.1
(LC only; implicit HE-AAC silently decodes half-rate — S2). FLAC: claxon
0.4.3 (32-bit FLAC undecodable — S2; frame-bps gate missing — S3).
Vorbis: symphonia-codec-vorbis 0.6.1. ALAC: symphonia-codec-alac (ffmpeg-MD5
bit-exact, executed). Opus: ruopus 0.1.2 (4-ch order row contradicts RFC
7845 — FR/BL swapped — S2). ADPCM ms/ima/oki/yamaha/swf: in-house ports,
bit-exact vs live ffmpeg (independently re-derived). Resampler: LINEAR,
±1000ppm drift-only, NO 44.1->48 conversion (convert.rs contains no SRC;
pipeline refuses rate mismatch) — dormant in shipped build anyway.

## Worst findings
1. [S1] mp3/xing.rs parse_vbri indexes rest[14..21] after checking
   len<14 -> panic on crafted VBRI (module unreachable from shipped hosts;
   library-API defect) (W07-A04).
2. [S2] The ENTIRE user-facing volume/EQ/mute/balance surface is dead:
   audio.set-volume refused by every shipped host; zero production callers
   of set_volume/Gain/Eq3Band; mixer permanently UNITY; EQ = real RBJ
   biquads wired to NOTHING (W07-A02/A06/A07/A08).
3. [S2] mix.rs has NO clipping on the live path — "stage D saturating
   clamp" doc claim FALSE; only clamp exists in dead code (W07-A09).
4. [S2] Volume::from_db(771+) stores +inf gain — "always finite" invariant
   false (executed proof) (W07-A02).
5. [S2] MP3 packet-pts timeline hole (529 samples ≈ 12ms after frame 1,
   every decode); LAME gapless facts never fed (extra_data: None
   hard-coded); unconditional 529-sample latency trim on mp1/mp2 (W07-A08).
6. [S2] Vorbis Emitted{frames:0} on held head — codec-api contract
   violation whitewashed by test comment; contract suite only ever runs
   vs MjpegDecoder (W07-A03).
7. [S2] adpcm_ms delta clamp 32767 diverges from live ffmpeg on loud-legal
   input (executed: -1,29438 vs 32767,32767) (W07-A03).
8. [S2] convert.rs zero-channel panic in its own error path (probe-verified
   0/0) (W07-A05).
9. [KEY] Audio verdict: codecs are REAL (mostly external crates wrapped
   honestly, differentials bit-exact), but the AUDIO EXPERIENCE is a null
   sink with no volume control, no EQ, no real device, dormant resampler,
   and open-loop clock (Wave 3). What a user HEARS: nothing, by
   architecture (ADR-005 era).
