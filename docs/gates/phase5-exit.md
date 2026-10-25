# Phase 5 Exit Gate — Breadth (waves 5.A–5.R + delta cards 5.X1/5.X2)

Status: **PASS WITH REGISTERED CAVEATS** — Lead sign-off 2026-10-24.
Verdict basis: doc 13 §8 (45 planned cards) + the 2026-10-24 delta log (2 added
cards) = 47 parent cards, all resolved; every number below is a real run logged
in `worklog.md` and `docs/gates/logs/` where applicable.

## 1. Evidence chain

| Gate | Result | Evidence |
|---|---|---|
| `cargo test --workspace --all-features` | **3213 passed / 0 failed** | this session's gate run (full completion incl. doctests) |
| `cargo clippy --workspace --all-targets --all-features -- -D warnings` | 0 warnings | this session |
| `cargo fmt --all -- --check` | clean | this session |
| fuzz registry diff | EMPTY (32 targets, doc 04 §8.5 naming) | `scripts/fuzz_registry_check.sh` (gate #6 in `full_gate.sh`); in-crate survival suites 1393/0 |
| records audit | 40 records, 83 format rows, zero missing | `docs/gates/phase5-records-audit.md` + `scripts/q2v_matrix_backfill.py` |
| perf snapshot | throughput/TTF/seek + hardware manifest | `docs/perf/breadth.md` + raw CSVs `docs/perf/raw/r1/` |
| algorithm audit (5.X1) | hotspot tables measured | `docs/perf/algorithm-audit.md` |
| memory audit (5.X2) | before/after alloc+RSS tables | `docs/perf/memory-audit.md` |

## 2. Card ledger (47 resolved)

Landed format cards (waves 5.A–5.O): A1 FLAC, A2 WAV+AIFF, A3 MP3/MP2-family,
B1 Ogg, B2 Vorbis, C1 MP4/MOV/3gp, C2 AAC-LC + ADTS/ADIF, D1 Matroska/WebM,
D2 MKV attachments/chapters/editions/tags, F1 H.264 parser (NAL/SPS/PPS +
slice/SEI), F2 H.264 hw decode-path (ADR-001), G1 HEVC parser, G2 HEVC
hw decode-path, H1 MPEG-1/2 sequence layer, H3 MJPEG polish, I1 MPEG-TS
(PSI + PES), I2 MPEG-PS/VOB, I3 ES wrappers + IVF, J1–J3 AVI-family audio
breadth (ADPCM ×8 descriptors, ALAC, MP2), L1/L2 subtitle compositor + cue
model, M1–M3 SRT/ASS/WebVTT, N1–N3 PGS/DVB+VobSub/CC-608-708 (ADR-012
self-build), O1–O3 M3U8-attrs/PLS/XSPF/CUE/probe+dir.
Investigations & decisions (honest records): H2-I MPEG-4 ASP, K2-I Opus,
K3-I ASF/WMA, K4-I AC-3/DTS, P1-I VP8/VP9, P2-I Theora + 7 blocked/deferred
records — each amends "no pure-Rust decoder" to "no *validated* pure-Rust
engine" with named Phase-6 validate-then-admit candidates (ruopus 0.1.2,
oxideav-mpeg4video 0.1.8, oxideav-ac3 0.0.11, rusty_vp9 0.1.3,
filmcraft-dnx 0.9.0).
Delta cards: X1 algorithm audit + MJPEG optimization, X2 memory audit + MKV
zero-alloc fix. Consolidation: R3 registry (probe signatures 2→10, 17-fixture
round-trip), REC-1/REC-2 (21 format records authored/audited), Q1 fuzz sweep,
Q2-V records audit, R1 perf snapshot.

## 3. Honest performance ledger (user mandate: fast + low memory)

| Metric | Phase-4 close | Phase-5 close | Evidence |
|---|---|---|---|
| heavy-mjpeg realtime | 0.292× | **2.08×–2.77×** (uhd4k/720p lanes; −34% decode ms) | X1a audit → X1b fix, 27-lane oracle re-verified bit-exact |
| uhd4k mjpeg end-to-end | — | **8.7× faster** than the X1a-audit baseline | R1 breadth |
| MKV demux allocations | 4.10 allocs/pkt (3.02× file walk bytes) | **0.11 allocs/pkt (1.02× file)** | X2a audit → X2b fix, differentials unchanged |
| audio decoders | — | FLAC 526×, MP3 3251×, MP2 6220×, Vorbis 1510×, AAC 1119× realtime | R1 breadth |
| AV1 (rav1d asm-off) | — | 68.4 Mpix/s = 0.292× libdav1d (policy gap, recorded) | E2 + R1 |
| SSE4.1 IDCT | claimed win | **removed** — measured net-loss 4/4 lanes, scalar kept | X1a |

## 4. Registered caveats (honest, blocking nothing downstream)

1. **VLC oracle absent sandbox-wide** — every differential is vs ffmpeg 7.1.5;
   VLC-lane comparisons remain pending-infra (Phase-3/4 precedent).
2. **TrueHD (K1) admission blocked** — `truehd` 0.7.2 purity PASS but rustc
   1.99.1 OOM-kills compiling bitstream-io 4.10.0's macro expansion
   (root-cause isolated, 20-line repro); scratch-patched differential was 4/4
   bit-exact. Unblock = upstream fix or Lead-approved ~60-line vendor patch.
3. **Open-path wiring** — the CLI's `open` still routes the D-018 first
   vertical only; per-container host wiring (mp4/mkv/ts/ogg/wav…) is the next
   card (R3 registered the probe surface; the honest typed refusal is tested).
4. **mp2 ±1 LSB** vs ffmpeg (Layer II rounding class, recorded per-row);
   MP3 corpus-MD5 table re-print owed to Q2-V follow-up.
5. **AV1 10-bit/4:4:4** = typed refusals (v0 scope); asm-on policy is a
   Phase-7 admission decision.
6. 3-OS, 4h-soak, nightly lanes: infra-blocked as at Phase-4 close.

## 5. Phase-6 launch list (doc 13 §9 + this phase's discoveries)

1. Streaming core (6.A1–A3): HTTP(S) client, ranged seekable resource, scheme
   registry — the biggest unstarted surface.
2. HLS (6.B), DASH (6.C), buffering/reconnect (6.D).
3. **Validate-then-admit wave** (new, from the K/P investigations): ruopus,
   oxideav-mpeg4video, oxideav-ac3, rusty_vp9, filmcraft-dnx — S spikes
   (purity + conformance + ffmpeg differential) then M admissions.
4. ASF container self-build (K3-I split proposal: header tree / data packets /
   index seeking).
5. AC-3/DTS IEC 61937 passthrough card (ADR-005 seam) after ADR-010 patent
   registration.
6. Open-path wiring + TTF/seek breadth re-measure once containers route.
7. ADPCM bench harness + MP3 corpus-table re-print (Q2-V follow-ups).

Phase 5 is closed. The Lead authorizes Phase 6.
