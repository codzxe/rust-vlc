# AUDIT: crates/demux/src/ps/test_support.rs
| Field | Value |
|---|---|
| Auditor | W05-A03 |
| Date | 2026-11-21 |
| Lines / functions | 153 / 10 (mem_stream, stamp_bytes, mpeg1_pack, mpeg2_pack, mpeg2_pes, mpeg1_pes, program_end, ac3_substream_payload, demo_stream, ramp) |
| File verdict | REAL (test-support; builders byte-verified against real ffmpeg output and committed fixtures) |
| Reachable from shipped entry point? | NO — by design ("Test-support surface, not player surface", line 6): rg shows zero player-path callers; consumed by ps/tests.rs, the doctest at ps/mod.rs:301, and integration suites. Note: it IS compiled into the shipped lib (`pub mod test_support;` at ps/mod.rs:94, un-gated — F-003). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | ffmpeg 7.1.5 generation (`-f mpeg` MPEG-1 PS, `-target pal-dvd` VOB) + python byte-level pack/PES inverse-decode and rebuild-compare (faithful port of the builders' masks/shifts); python scan of committed fixtures ps-vob-{ac3,dts,lpcm}.vob private-1 payload prefixes; rg caller/payload-size sweeps (max ramp(982)); rg baseline log for ps::tests (37 lines) and Doc-tests (absent); sed -n quote verification |

## 1. One-paragraph verdict
The best-verified test-support file I have audited in this wave: I rebuilt real ffmpeg 7.1.5 pack headers byte-for-byte with python ports of these builders (7/7 MPEG-1 packs, 32/32 MPEG-2 packs identical; MPEG-1 PES with PTS identical; the no-PTS MPEG-2 shape identical), and the private-1 AC-3 payload shape matches the committed `ps-vob-ac3.vob` bytes exactly (substream byte + 3 filler + AC-3 syncword `0b 77` at +4). The builders honestly forge ffmpeg's deviations from ISO (the `01` trailing rate bits — measured on every pack of a fresh encode; the `0x0F` MPEG-1 no-stamp byte) and the test suite pins those forges (baseline 1882/1883/1888, all ok). Three S4s: a silent `u16::MAX` length clamp that would corrupt oversized synthetic PES packets (latent — largest caller payload is 982 B), no builder coverage for PTS+DTS/padded PES headers (real ffmpeg shapes, delegated to fixtures and hand-rolled bytes), and the module ships in the release binary despite being test-only.

## 2. What actually works (verified)
- `mpeg1_pack` / `mpeg2_pack`: byte-exact vs fresh ffmpeg 7.1.5 output — generated `/tmp/ps1.mpg` (`-f mpeg`, testsrc+mp2) and `/tmp/ps2.vob` (`-target pal-dvd`, mpeg2video+mp2); inverse-decoded each real pack's SCR/mux_rate and rebuilt with a faithful python port of the builder: `rebuild == real: True` for the first pack of each flavor, all 7 MPEG-1 packs and all 32 MPEG-2 packs carry the forged/claimed marker patterns (`0x01` tail on MPEG-1, `0xF8` stuffing-nibble on MPEG-2).
- `mpeg1_pes`: byte-exact vs the real MPEG-1 audio PES in `/tmp/ps1.mpg` (sid 0xC0, PTS=47618, 2037 B payload) — `rebuild == real: True`.
- `mpeg2_pes` no-PTS shape: real ffmpeg writes `80 00 01 ff` (flags2=0, hlen=1, single 0xFF stuffing) — exactly the builder's lines 87-93 shape, comment "marker bit 0 — ffmpeg's convention" measured true.
- `ac3_substream_payload`: committed fixture `crates/demux/fixtures/ps/ps-vob-ac3.vob` first 0xBD payload = `80 06 00 01 0b 77 …` — substream byte `0x80`, three filler bytes, then the AC-3 syncword at +4; the builder's layout is the real one and the demuxer's 4-byte strip is fixture-pinned.
- `stamp_bytes`: 3/15/15 split with markers verified by my decode/rebuild roundtrip and the executed test `mpeg1_stamped_pes_roundtrips_pts` (baseline 1885 ok).
- Executed: 37 `ps::tests` lines in baseline test-workspace.log.txt (1876-1894 shown) all `ok`, including the fixture-lane tests (`fuzz_lite_clean_fixtures_drain_without_terminating_errors` 1880, `fuzz_lite_mutations_of_the_committed_fixtures_never_panic` 1881) and every builder-forge pin (1882, 1883, 1886, 1887, 1888).
- Panic/alloc audit: fixed-size arrays and small vecs; `u16::try_from(..).unwrap_or(..)` avoids the crate's no-panic/unwrap lint without panicking; no indexing beyond construction; no unsafe.

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/ps/test_support.rs:85 (and the same shape at 109)
Code:
```
    let len = u16::try_from(total).unwrap_or(u16::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    out.push(0x80); // '10' flags
```
What is wrong: a payload larger than 65532 (mpeg2) / 65530 (mpeg1) bytes silently clamps the declared PES length to `u16::MAX` while the builder still appends the full payload — the emitted packet's declared extent and real extent disagree, and everything after the declared end becomes stray bytes the demuxer must resync over. A test author building an oversized frame (AC-3/LPCM frames can exceed 64 KB in the wild) gets a fixture that is not the fixture they wrote, with no error.
Why it matters: silent fixture corruption in the one module whose job is deterministic fixtures; the failure would surface as a confusing demuxer test failure (or worse, a false pass of the resync path), not as a builder error. Latent today — the largest payload any current caller passes is `ramp(982, …)` (verified by sweep over ps/tests.rs) — so this is a trap, not an active bug.
What correct looks like: `assert!(total <= u16::MAX as usize - …)` in test-support code (a panic is the honest outcome), or return `Option<Vec<u8>>`/`Vec<u8>` with a documented truncation marker the caller must opt into.
Confidence: HIGH — Verification: read + caller-size sweep (max 982 B; nothing near the clamp).

### F-002 [S4]
Where: crates/demux/src/ps/test_support.rs:81-97 (builder scope)
Code:
```
pub fn mpeg2_pes(stream_id: u8, payload: &[u8], pts: Option<u64>) -> Vec<u8> {
    let header_len = if pts.is_some() { 5 } else { 1 };
```
What is wrong: the builder can only express the two minimal header shapes (PTS-only hlen=5, or no-PTS hlen=1+0xFF). Real ffmpeg writes shapes it cannot produce: PTS+DTS video PES (`80 c1 0e …`, measured on my fresh pal-dvd encode and implied by `mpeg2_pts_dts_pair_parses_both` at baseline 1889, which must hand-roll its bytes), and padded headers (the committed `ps-vob-ac3.vob` audio PES carries hlen=9 = PTS + 4 stuffing bytes, measured: `00 00 01 bd 07 ec 80 81 09 21 00 03 79 25 …`).
Why it matters: future test authors covering those demuxer arms cannot use the builder and will hand-roll (as the PTS+DTS test already does) — the builder's "deterministic container shapes" promise quietly stops at the two most common shapes; fixture-lane coverage of the padded/PTS+DTS forms rests entirely on the three committed .vob files.
What correct looks like: an `enum PesStamp { None, Pts(u64), PtsDts(u64,u64) }` parameter (plus optional stuffing), or an explicit doc line naming the shapes the builder does not forge.
Confidence: HIGH — Verification: ran (fresh ffmpeg encode + fixture byte dumps; both real shapes shown above).

### F-003 [S4]
Where: crates/demux/src/ps/test_support.rs:6 vs ps/mod.rs:94
Code:
```
//! Test-support surface, not player surface.
```
What is wrong: the claim is API intent, not compilation reality — `pub mod test_support;` (ps/mod.rs:94) is not `#[cfg(test)]`-gated, so these builders compile into the shipped `demux` lib and the release binary. (Necessary as-is: integration tests under `tests/` import them, and the crate-wide pattern — avi/adts/ivf/ogg/es all do the same, verified by rg — depends on it.)
Why it matters: a few KB of dead test code in the player binary and a doc line that reads as a build claim; if someone later "fixes" the doc by cfg-gating the module, every integration suite breaks.
What correct looks like: doc wording "no player-path callers; compiled un-gated so integration suites can share it (the crate-wide test_support pattern)", or a `#[doc(hidden)]`+feature split if binary size ever matters.
Confidence: HIGH — Verification: read (mod decl) + rg (avi/adts/ivf/ogg/es twins; zero player-path callers).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mem_stream | 17-21 | REAL | — | Pure delegation to avi::test_support (the shared seam); the duplication is one indirection, documented. |
| stamp_bytes | 26-35 | REAL | — | 3/15/15+markers split; roundtrip-verified (§2); masks correct for 33-bit values (`ticks & 0x1_FFFF_FFFF`). |
| mpeg1_pack | 40-55 | REAL | — | Byte-exact vs ffmpeg; forges the `01` tail (comment + test 1883 pin it; measured 7/7 real packs). |
| mpeg2_pack | 60-77 | REAL | — | Byte-exact vs ffmpeg (32/32); stuffing param ≤7 bytes honored via `0xF8 \| n`; SCR-ext hard-coded 0 (`out.push(0x01)`) — matches every real pack measured. |
| mpeg2_pes | 81-97 | REAL | S4 (F-001, F-002) | No-PTS shape == real ffmpeg; minimal-only scope; silent u16 clamp. |
| mpeg1_pes | 102-118 | REAL | S4 (F-001) | Byte-exact PTS shape vs real ffmpeg; `0x0F` no-stamp byte measured/pinned (test 1882). |
| program_end | 122-124 | REAL | — | The ISO end code `00 00 01 B9`. |
| ac3_substream_payload | 129-134 | REAL | — | Shape == committed fixture bytes (§2); consumed by private1_ac3 test (baseline 1892 ok). |
| demo_stream | 139-146 | REAL | — | Executed via test 1879; the doctest embedding it (ps/mod.rs:301) is UNVERIFIED — the baseline log contains no Doc-tests section. |
| ramp | 151-153 | REAL | — | Deterministic filler; trivially correct. |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| (No tests in this file — the consumers are ps/tests.rs, 19+ tests.) | — | The consumer suite is honest: roundtrip tests (1884/1885/1887) would fail on any mask/shift mutation of these builders; forge-pins (1882/1883/1888) would fail if the builder started emitting spec-true bytes instead of ffmpeg's; the fixture lanes (1880/1881, real .vob bytes) ground the builders' claims against real muxer output. |
| Doctest at ps/mod.rs:301 | UNVERIFIED | Not present in the baseline log (no Doc-tests section); the unit test 1879 covers the same function, so nothing rides solely on the doctest. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The trailing two bits: the spec says markers `11`, ffmpeg writes `01` — the builder forges ffmpeg's" | test_support.rs:51-53 | REAL | Measured: 7/7 packs of a fresh `-f mpeg` encode end in `…\|0x01`; pinned by test 1883. |
| "marker bit 0 — ffmpeg's convention" (0x00 flags2 + 0xFF stuffing) | test_support.rs:88/93 | REAL | Measured: real pal-dvd no-PTS PES = `80 00 01 ff …`; pinned by test 1888. |
| "the ffmpeg no-stamp marker" (0x0F) | test_support.rs:114 | REAL | Measured in real MPEG-1 stream shapes; pinned by test 1882. |
| "substream byte + the 3-byte filler + frame data" (private-1 AC-3) | test_support.rs:126-127 | REAL | Committed fixture bytes: `80 06 00 01 0b 77 …` (syncword at +4). |
| "Test-support surface, not player surface" | test_support.rs:6 | SHALLOW | API-true (no player callers), compile-false (F-003). |
| "round-trips with its parse" (stamp_bytes) | test_support.rs:23-24 | REAL | Executed roundtrip test 1885 + my decode/rebuild. |

## 7. Dead / duplicated / unreachable code
- No dead functions (all 10 have callers in ps/tests.rs, the doctest, or sibling suites).
- `mem_stream` is a one-line duplication of avi::test_support::mem_stream — deliberate shared-seam indirection, documented at line 12-13.

## 8. Dependencies & flags
- std (Arc) + plugin_api::SourceStream only; no external deps, no unsafe, no feature gates. Matches the C5 claim; `#[must_use]` on all pure builders.

## 9. Open questions for the next agent
1. ps/mod.rs owner: the demuxer's PTS+DTS and padded-header arms are covered by hand-rolled bytes + the three .vob fixtures only — if the fixture generator ever regenerates without `-target pal-dvd`, the padded lane disappears silently (fixtures are committed, so low risk).
2. Whoever owns the avi twin (`crate::avi::test_support`): the same u16 clamp shape exists there for RIFF chunk sizes? (Out of my file list — one rg would settle it.)
3. Baseline runner: capture Doc-tests sections in future runs — the ps/mod.rs:301 doctest is currently unexecuted evidence.
