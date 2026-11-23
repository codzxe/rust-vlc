# AUDIT: crates/codecs/tests/fixtures/breadth/ws_st.bin
| Field | Value |
|---|---|
| Auditor | W06-A06 |
| Date | 2025-06-27 |
| Lines / functions | binary (512 bytes) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (test fixture; adpcm_breadth_differential.rs:113 row + :311 shape check) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | file ("data" — raw payload, expected); stat (512); od header (ramp visible); direct run of the built breadth suite -> 4/4 ok |

## 1. One-paragraph verdict
A 512-byte hand-crafted Westwood (`adpcm_ima_ws`) stereo payload, and its documented provenance is visibly true: the file opens with an exact +7 arithmetic byte ramp (`00 07 0e 15 1c 23 2a …`) — unmistakably synthetic, matching the suite's row "`adpcm_ima_ws` stereo 22 050 | `ws_st.bin` | 1 × 512 | hand-crafted ramp | forced decoder, `-ac 2`" (adpcm_breadth_differential.rs:24). I executed the built breadth suite: 4/4 pass, so the row's pinned decode MD5 (`ec91fc42a66bee03edf43a5e91b80932`) held against the suite's ffmpeg differential.

## 2. What actually works (verified)
- Geometry: 512 == 1 × 512 (suite :311 lists ws_st.bin in the shape check — passed).
- Ramp provenance: od header shows the +7 ramp — "hand-crafted ramp" is literal.
- `./target/debug/deps/adpcm_breadth_differential-13f5db9ab9dabd4b --nocapture` → 4/4 ok (the ws lane's pinned MD5 asserted and held).

## 3. Findings
None. Checked: size, ramp pattern vs documented provenance, consuming suite executed green, consumers (breadth suite only).

## 4-9. (binary asset — tables N/A; consuming suite's pins hold; no dead code; no deps; no open questions)
