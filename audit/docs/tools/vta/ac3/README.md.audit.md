# CLAIM AUDIT: tools/vta/ac3/README.md
| Field | Value |
|---|---|
| Auditor | W17-A09 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Claims extracted | 12 |
| Verdict counts | TRUE: 10 (9 verified against main.rs / the dependency record; ffmpeg version verified live on this host) PARTIAL: 1 UNVERIFIED: 1 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Every mechanism claim in this README is backed by real code at real lines in
the harness it documents, and every verdict claim is backed by the
dependency record: oxideav-ac3 is genuinely absent from every workspace
manifest (NOT ADMITTED), the archived-FAILED status matches record §D's
corpus-lane row (maxΔ 316 LSB, DIVERGENT — byte-found at
oxideav-ac3-0.0.11.md:101), the syncwalker's syncword/table/834-836-stride
claims all exist in src/main.rs, the hostile battery is exactly the five
named cases, cplprobe really censuses cplinu/dithflag, and the required
ffmpeg 7.1.5 is verifiably on this host's PATH (I ran ffmpeg -version ->
7.1.5-0+deb13u1). One pointer is imprecise: "see record §D for the exact
generation commands" — §D exists and holds the lane table (ch/rate/bitrate)
but not the ffmpeg command lines; the reader must reconstruct 7 of 8 lane
commands from the table plus the README's one example. Harness execution
itself is UNVERIFIED here (no cargo).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "STATUS: ARCHIVED — NOT ADMITTED. The differential gate FAILED (corpus lane maxΔ 316 LSB)" | :3-5 | TRUE | oxideav-ac3-0.0.11.md:101 corpus row "**316** \| **DIVERGENT**" (§D at :90); verdict :147 "BLOCKED" |
| 2 | "`oxideav-ac3` is NOT a dependency of any Lumen workspace crate" | :5-6 | TRUE | `rg -i oxideav Cargo.toml Cargo.lock crates/codecs/Cargo.toml` -> 0 hits |
| 3 | "Standalone (own `[workspace]` — NOT a member of the Lumen workspace)" | :9 | TRUE | tools/vta/ac3/Cargo.toml:1 `[workspace]` (empty) + package vta-ac3 0.0.0 |
| 4 | "syncword `0x0B77` + the ATSC A/52B §5.4.1.4 Table 5.18 frame-size formula ... 44.1 kHz alternating ... 834/836-byte strides" | :14-18 | TRUE | main.rs:12-13 syncword/fscod/frmsizecod; :49-65 `frame_len_bytes` table with 44.1kHz `row[(frmsizecod % 2)]` alternation; :21 "834/836-byte strides" |
| 5 | "Resyncs (counted) when the predicted boundary misses the syncword" | :19 | TRUE | main.rs:74 "boundary does not carry the 0x0B77 syncword" resync path; resyncs counted (harness audited REAL by W17-A10) |
| 6 | "make_decoder (channels = passthrough) → send_packet/receive_frame → Frame::Audio interleaved s16le ... 1536 samples/ch" | :20-23 | TRUE | main.rs:28 `use oxideav_ac3::decoder::make_decoder;`; :7-8 s16le + differential doc; :19 "1536/44100-second frames" |
| 7 | "Differential: ffmpeg pcm_s16le ... full-stream sample compare ... + MD5s" | :24-26 | TRUE | record §D:92-95 same pipeline ("reference ffmpeg 7.1.5 -i F -map 0:a -c:a pcm_s16le -f s16le"); MD5 parity discussion :127-129 |
| 8 | "Hostile: truncated frame, garbage mid-stream, zero-length, reserved fscod, reserved frmsizecod, garbage-only — each ... under catch_unwind" | :27-29 | TRUE | main.rs:240-259 the exact five cases (`truncated_frame`, `garbage_midstream`, `zero_length`, `corrupt_fscod_reserved` (fscod=3), frmsizecod=63 reserved, `garbage_only`); :25 `catch_unwind` |
| 9 | "cplprobe: per-block cplinu / dithflag side-info census via the crate's own parser" | :30-32 | TRUE | main.rs:297-322 `fn cplprobe` counting `s.cplinu` (:315) and `s.dithflag` (:316) |
| 10 | "Run: cargo run --release / -- hostile / -- cplprobe <lane>" | :47-50 | TRUE | main.rs:172 hostile dispatch, :176 cplprobe dispatch; default differential mode |
| 11 | "see record §D for the exact generation commands" | :37-38 | PARTIAL | record §D exists (:90) and holds the LANE TABLE (ch/rate/bitrate per lane) but NOT the per-lane ffmpeg commands — the README's own §Run carries one example; 7 of 8 lanes' commands must be inferred. Pointer imprecision, not a false pointer (W17-A10's main.rs audit logged this as an S3 "false pointer"; the section exists, so this re-audit records it as S4 imprecision) |
| 12 | "Requires ffmpeg/ffprobe 7.1.5 on PATH; writes out_*.s16 / ff_*.s16 into /tmp/vta-ac3" | :53-54 | TRUE (version, live) / run UNVERIFIED | `ffmpeg -version` -> "ffmpeg version 7.1.5-0+deb13u1"; `command -v ffprobe` -> /usr/bin/ffprobe; the harness itself needs cargo (absent in this sandbox) — not executed |

## 3. Contradictions
None. README, src/main.rs, and the dependency record agree on status
(ARCHIVED/NOT ADMITTED/BLOCKED), on the failed differential, and on every
mechanism.

## 4. Fabrication / inflation indicators
NONE. The README's status block is anti-inflation: it leads with the
failure verdict (maxΔ 316), forbids wiring the crate in, and points to the
record. The only defect is the §D generation-commands pointer precision.

## 5. Matrix rows
N/A — harness README, not a matrix chunk.
