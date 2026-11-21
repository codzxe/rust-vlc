# AUDIT: apps/cli/src/containers/avi.rs
| Field | Value |
|---|---|
| Auditor | W13-A08 |
| Date | 2026-11-21 |
| Lines / functions | 142 / 2 (both production; no in-file tests — coverage lives in the integration suites) |
| File verdict | REAL — the first-vertical lane, baseline-proven end-to-end; the two functions are correct against every invariant I could trace (including the demuxer's channel/rate/scale gates the harvest relies on) |
| Reachable from shipped entry point? | YES — `lumen` → run/dispatch → `demux_host::open_source` → `containers::open` (mod.rs:52 `"avi" => avi::open(stream, opts)`) → this `open`. Baseline: `avi-firstvertical-open-session.txt` (full lifecycle) + `rc-smoke-engine-only.log.txt` 6/6 + decode-bench `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `Read` avi.rs 1-142 (full); `rg -n "pub enum StreamFormat" -A 30 crates/demux/src/avi/header.rs` (exactly two variants — the harvest's match is exhaustive); `sed -n '755,860p' crates/demux/src/avi/header.rs` + `rg -n "MAX_PCM_CHANNELS" crates/demux/src/avi/header.rs` (channels gate: `> 64` → per-stream skip with damage, :349/:777; `channels==0 || samples_per_sec==0 || block_align==0` → skip, :826); `sed -n '258,300p' crates/demux/src/avi/header.rs` (TimeBase = `new(scale, rate)`, one tick = scale/rate seconds — the frame_ns formula's convention); `rg -n "pub struct TimeBase" -A 12 crates/media-types/src/time.rs` (num/den zero rejected at construction → no div-by-zero in frame_ns); `rg -n "fn avi\|avi_mjpeg_pcm" apps/cli/tests/container_conformance.rs` (integration lane :136); baseline citations above. |

## 1. One-paragraph verdict
This is the smallest container host in my set and the cleanest: 142 lines, two functions, both correct. `open` follows the shared shape (demux → start → harvest → clamp → pure seek query → SelectDemuxer) and is the lane the whole first-vertical claim rests on — and that claim is baseline-proven (full Playing lifecycle, 60/60 frames decoded both limbs in decode-bench). `harvest_facts` passes the demuxer's own `strf` truth through without a local copy of any tag/bits table, and every value I stress-tested is gated one layer down: channels are ceilinged at 64 with a damage note (`MAX_PCM_CHANNELS`, header.rs:349/777 — I traced this specifically because the harvest itself does NOT gate, and an unbounded channel count would OOM the 131_072-slot ring at open), zero channels/rate/block-align are skipped, the `frame_ns` division is protected by `TimeBase::new`'s zero-rejection, the `num × 1e9` product cannot overflow u64 from u32 inputs, and the scale/rate convention makes the frame-period formula exact (one tick = scale/rate seconds → period = num×1e9/den). The single defect is a cross-family inconsistency: `samples_per_sec` has no plausibility ceiling here (or in aiff), while the mkv harvest refuses rates above 4 096 000 — a malformed AVI declaring a 4.29 GHz sample rate flows into the audio session and clock math unchallenged.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/avi-firstvertical-open-session.txt` → `Buffering → Playing → Stopped → Idle`, EXIT=0; `rc-smoke-engine-only.log.txt` → 6/6 engine checks (AVI first-vertical only, by design); `mp4-smoke-decode-bench` contrast row → `v_frames=60 v_decoded=60 a_frames=60 a_decoded=60` on `avi_mjpeg_pcm.avi`.
— Verified by read: the harvest's ungated `Some((u32::from(channels), samples_per_sec))` is safe for channels — the demuxer skips any stream with `channels > MAX_PCM_CHANNELS (64)` or zero channels/rate/block_align before a track is minted (header.rs:776-782, 826-829), which is what keeps the 131_072-frame × channels ring allocation bounded (SpscRing::new, audio-output/src/ring.rs:70-78).
— Verified by read: `frame_ns = base.num × 1e9 / base.den` matches the demuxer's base convention exactly (`StreamDesc::time_base` = `TimeBase::new(scale, rate)`, "one tick = scale/rate seconds", header.rs:261-268); `TimeBase::new` rejects zero components (media-types time.rs:77-81) so the division cannot panic; `u64::from(u32::MAX) × 1e9 ≈ 4.3e18 < u64::MAX` so the saturating_mul never even saturates.

## 3. Findings

### F-001 [S4]
Where: apps/cli/src/containers/avi.rs:112 (with crates/demux/src/avi/header.rs:826-829 — the parse that only rejects zero)
Code:
```js
                (
                    codec,
                    Some((u32::from(channels), samples_per_sec)),
                    None,
                    None,
                )
```
(verbatim Rust:)
```rust
                (
                    codec,
                    Some((u32::from(channels), samples_per_sec)),
                    None,
                    None,
                )
```
What is wrong: The PCM geometry pair passes `samples_per_sec` through with no ceiling — the AVI strf parse rejects only `samples_per_sec == 0` (header.rs:826), so a malformed-but-parseable WAVEFORMATEX declaring 4_294_967_295 Hz reaches `open_audio_session(channels, 4.29e9)` and the session clock/position math. The mkv harvest twin refuses rates above 4_096_000 as implausible (mkv.rs `audio_pair`); aiff effectively has no ceiling either (integral/finite/≤ u32::MAX only) — three families, three policies, one consumer.
Why it matters: No crash or OOM follows (the ring scales with channels, not rate — verified), but the session opens with absurd clock arithmetic (a 60-frame file "lasts" 14 ns) instead of a typed refusal, and the plausibility contract differs per container for the same host seam.
What correct looks like: one shared plausibility gate in `TrackFact`/`audio_geometry` (channels ≤ 64, rate within a recorded plausible band) instead of per-family re-implementation — exactly the asymmetry mkv.rs#part2's open questions already flagged.
Confidence: HIGH — Verification: read of avi.rs + header.rs + mkv.rs + ring.rs.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `open` | 35-78 | REAL | — | Shared container shape; clamp + pure seek query; `i64::MAX/4` fallback only when span end Missing (length==0 AVIs); baseline-proven lane |
| `harvest_facts` | 84-142 | REAL | F-001 | MJPEG arm: correct frame-period math (scale/rate convention verified), dimensions from biWidth/\|biHeight\|; PCM arm: descriptor from the demuxer's own mapping, ungated rate (F-001), channels safe via MAX_PCM_CHANNELS; `None` arm → "unknown" codec → typed refusal downstream; `extra_data: None` correct for AVI (no out-of-band blob) |

## 5. Test honesty
No in-file tests. Integration coverage: `apps/cli/tests/container_conformance.rs:136 avi_family_lane` (fixture `generated/valid/avi_mjpeg_pcm.avi`), plus sync_scenarios/native_seek/ttf_seek/pipeline_accounting/session_script/bench_decode suites all open the AVI fixture (baseline-green) — the lane is exercised through the REAL binary and the REAL open path; the decode counts (60/60) are independent ffmpeg-measured truths, not packet-count arithmetic.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the registry's first wired family (D-018's first vertical…)" (1-2) | avi.rs | REAL | avi-firstvertical-open-session.txt + rc-smoke 6/6 + decode-bench 60/60 |
| "Everything else (skip wrapper, engine wiring, codec dispatch) is shared code" (8-10) | avi.rs | REAL | SelectDemuxer/demux_host + host.rs open_session read (my host.rs#part2 audit) |
| "MJPEG dimensions, PCM channels/rate through the demuxer's own descriptor mapping — not a local copy" (80-83) | avi.rs | REAL | header.rs StreamFormat::Pcm + PcmSample::from_wave_tag read |

## 7. Dead / duplicated / unreachable code
None. Both functions are live; the file is the minimal AVI-specific remainder it claims to be.

## 8. Dependencies & flags
Workspace-only (demux::avi, plugin_api, media_types, commands). No feature gates, no unsafe, no unwrap/expect/panic in the file.

## 9. Open questions for the next agent
- W13 demux_host chunk owner: the per-family plausibility-gate asymmetry (mkv gates rate ≤ 4 096 000 + channels ≤ 64; avi gates channels only; aiff gates via COMM parse; wav — unexamined by me) would be one `audio_geometry`-level gate; F-001 is its AVI symptom.
- The `unknown` codec string (117) reaches `HostDecoderProvider` — confirm it refuses typed (`cli.codec_unavailable`) rather than indexing a registry by the literal (my host.rs#part2 read of the provider path suggests refusal; not verified at the provider's match arm).
