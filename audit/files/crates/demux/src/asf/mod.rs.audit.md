# AUDIT: crates/demux/src/asf/mod.rs
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-23 |
| Lines / functions | 780 / 24 |
| File verdict | UNREACHABLE (real, working demuxer code — no shipped entry point reaches it) |
| Reachable from shipped entry point? | NO. apps/cli/src/containers/mod.rs registry has arms for adts/aiff/avi/es/ivf/mkv/mp4/ogg/ps/ts/wav — no `asf`; media-probe has zero ASF signature (crates/media-probe/src/signature/ = avi.rs, es.rs, mp4.rs, phase5.rs); no crate outside demux references `demux::asf`/`AsfDemuxer` (grep across crates/ apps/ = no hits) |
| Findings | S0: 0 S1: 0 S2: 2 S3: 2 S4: 1 |
| Commands run | `cargo test -p demux asf` -> exit 0, 36 unit tests passed (1.19s); `cargo test -p demux --doc asf` -> exit 0, doctest `asf::AsfDemuxer (line 119)` passed; grep for consumers/unwrap/panic sites; sed quote verification |

## 1. One-paragraph verdict
This is a fully implemented ASF/WMV/WMA demuxer — header object tree, packet walk with fragment reassembly and preroll-subtracted stamps, Simple-Index seek plus a bounded linear fallback — with typed error paths throughout and genuinely passing tests (36 unit + 1 real doctest, my run). But it is dead weight in the shipped product: the CLI container registry has no `asf` arm, the prober has no ASF signature, and no other crate consumes it. Worse, the module doc claims the ASF-a card landed "the media-probe signature" — media-probe contains no ASF code at all. And the packet walk has an unguarded allocation: the FPO's `max_packet_size` (raw u32, no ceiling) and the Data Object's declared size (u64, never validated against the real file length) feed straight into a `vec![0u8; len]` read of up to 4 GiB — a crafted 100-byte file can demand a 4 GiB allocation before any byte is read. Today that is unreachable; the day someone wires ASF into the registry it becomes a remotely triggerable OOM. The arithmetic on the seek path also mixes saturating and non-saturating ops on the same unbounded file-parsed `preroll_ms`.

## 2. What actually works (verified)
— verified by `cargo test -p demux asf` -> `running 36 tests ... test result: ok. 36 passed; 0 failed` and `cargo test -p demux --doc asf` -> `1 passed` (my runs, exit 0).
— verified by grep: no `unwrap()`/`expect(`/`panic!` in non-test mod.rs code (the house-rule claim at line 35 holds for this file); all 24 listed error codes exist via `error::` helpers; `MAX_INDEX_BYTES` (1 MiB) and `MAX_LINEAR_SEEK_PACKETS` (4096) ceilings are enforced (lines 604-605, 737-738).

## 3. Findings
F-001 [S2 — S1 the moment ASF is wired]
Where: crates/demux/src/asf/mod.rs:464-468 (with crates/demux/src/avi/riff.rs:267-268, crates/demux/src/asf/header.rs:716, 1035-1041)
Code:
```
        let at = payload_offset + index * packet_size;
        let bytes = self
            .reader
            .read_exact_or_end_at(packet_size as usize, at)?
            .ok_or_else(|| error::truncated(at + packet_size, at))?;
```
What is wrong: `packet_size` is the FPO's `max_packet_size` (header.rs:716 `max_packet_size: guid::u32le(payload, 72)` — raw, no ceiling; build_walk only refuses 0 and min≠max), and `packets_total` is bounded only by the DECLARED Data Object size (walk_top_level records `size` without validating it against `stream_len`, header.rs:1035-1041). `read_exact_or_end_at` pre-allocates the whole buffer (`riff.rs:268 let mut out = vec![0u8; len];`) before reading. A ~100-byte crafted file declaring max_packet_size = 0xFFFFFFFF and a large Data Object size forces a single ~4 GiB allocation → allocation abort/OOM on small-memory targets. Why it matters: header-driven unbounded allocation is the exact S1 pattern (MP §12 is this crate's own stated discipline); no test pins a ceiling (asf/tests.rs pins variable-size refusal, not huge-fixed-size). Severity held at S2 ONLY because no shipped path reaches this file. What correct looks like: clamp packet_size against `data.size` bounded by `stream_len` (or a MAX_PACKET_BYTES ceiling with a typed refusal). Confidence: HIGH — Verification: read (allocation path traced end to end; no run executed — UNVERIFIED as an actual OOM: would need a crafted fixture, not attempted).

F-002 [S2]
Where: crates/demux/src/asf/mod.rs:5-8 vs apps/cli/src/containers/mod.rs:53-71 and crates/media-probe/src/signature/
Code:
```
//! List, Extended Content Description, a bounded Header Extension
//! child walk — plus the [`probe`] gate and the media-probe
//! signature.
```
What is wrong: The doc claims the media-probe signature landed. media-probe has no ASF code (signature dir: avi.rs, es.rs, mp4.rs, phase5.rs; zero `asf`/`wmv`/`wma` hits), and the CLI registry (containers/mod.rs:55-71) has no `asf` arm — a .wmv file is probed as nothing and refused before this demuxer can ever run. Why it matters: 780 lines of demuxer (plus header/data/guid/error/test_support modules) are shipped dead code; the doc tells the reader the probe wiring exists when it does not. What correct looks like: either the signature + registry arm, or a doc that says "probe wiring pending". Confidence: HIGH — Verification: read + grep (zero consumers outside crates/demux).

F-003 [S3]
Where: crates/demux/src/asf/mod.rs:749-751 (with header.rs:713 `preroll_ms: guid::u64le(payload, 56),`)
Code:
```
            if u64::from(parsed.send_time_ms) >= target_ms as u64 + preroll {
                landing_packet = index;
                landing_ms = u64::from(parsed.send_time_ms).saturating_sub(preroll);
```
What is wrong: `preroll` is the raw unbounded u64 from the file; `target_ms as u64 + preroll` is plain `+` — overflow panics in debug, wraps in release (wrong landing at packet 0). The sibling sites use saturating arithmetic (line 253 `target_ms.saturating_add(preroll_ms)`, line 724 `.saturating_sub(preroll)`) — this one site does not. Why it matters: inconsistent overflow discipline on the same parsed field; a fuzz corpus with preroll_ms = u64::MAX kills the debug harness. What correct looks like: `target_ms.saturating_add(preroll)`. Confidence: HIGH — Verification: read (arithmetic traced; not executed — UNVERIFIED as a panic).

F-004 [S3]
Where: crates/demux/src/asf/mod.rs:570-572
Code:
```
    fn ticks_for(track: &Track, base: TimeBase, pts_ms: u32) -> i64 {
        let _ = track;
        let ms = i64::from(pts_ms);
```
What is wrong: `track` is an ignored parameter (the signature promises track-dependent conversion, the body never reads it); the ms→sample-tick conversion truncates (`checked_div(1000)`, no rounding) losing up to ~1 ms of stamp precision per packet. Why it matters: misleading API shape and small systematic stamp drift on the audio lane (bounded by the container's own ms granularity, hence S3 not S2). What correct looks like: drop the parameter or use it; round-to-nearest on the division. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/demux/src/asf/mod.rs:581-583
Code:
```
            ms.checked_mul(rate)
                .and_then(|t| t.checked_div(1000))
                .unwrap_or(i64::MAX)
```
What is wrong: Overflow clamps to `i64::MAX` ticks — an absurd-but-accepted stamp instead of a typed refusal. Blast radius: only reachable with pathological ms×rate products. Confidence: MEDIUM — Verification: read.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| probe | 96-98 | REAL | — | 16-byte GUID window check, correct |
| SimpleIndex::parse | 205-245 | REAL | — | closing-size + zero-interval refusals, count cross-check |
| SimpleIndex::bucket_of | 250-256 | REAL | — | saturating add, bucket clamp |
| AsfDemuxer::new | 295-305 | REAL | — | delegates SourceReader contract checks |
| header / file_properties / streams | 309-324 | REAL | — | map_or(&[]) empty-before-start |
| stream_record_for_track | 329-332 | REAL | — | handle→index direct mapping (assumes TrackId == slot index, true by build_tracks) |
| codec_list / metadata / data_object / top_objects | 336-358 | REAL | — | harvest seams; zero shipped consumers (F-002) |
| damage_total | 363-365 | REAL | — | |
| pcm_fact | 371-379 | REAL | — | PCM-lane seam, correct None shape |
| build_tracks | 382-415 | REAL | — | span UNKNOWN until stamps; base 1/rate (rate 0 → TimeBase::new err → bad_waveformatex — honest) |
| build_walk | 418-455 | REAL | S2 | min==max fixed-size enforcement; no packet-size ceiling (F-001) |
| walk_one_packet | 459-525 | REAL | S2 | unbounded pre-read allocation (F-001); undeclared-stream payloads skipped not fatal — good |
| drain_held | 532-565 | REAL | — | hold-and-flush at EOS, preroll arithmetic duplicated (see F-003 sibling) |
| ticks_for | 570-585 | REAL | S3 | ignored param + truncation (F-004), i64::MAX clamp (F-005) |
| load_simple_index | 594-621 | REAL | — | 1 MiB ceiling, cached Option<Option> — correct |
| reanchor | 626-633 | REAL | — | clamps to packets_total, clears assembler/queue |
| start | 637-649 | REAL | — | idempotent clone-on-restart |
| next_event | 651-697 | REAL | — | queue → packet; exhaustion → drain_held once → EOS; typed not-started |
| seek | 699-761 | REAL | S3 | indexed bucket landing + bounded linear fallback; overflow site F-003 |
| seekability | 763-779 | REAL | — | Fast iff SIMPLE_INDEX object present (even if its parse later fails — acceptable hint semantics) |

## 5. Test honesty
Tests live in asf/tests.rs (another agent's file). From this file's perspective: 36 filtered tests + the mod.rs doctest pass (my run). The doctest (lines 117-141) is a real executable example asserting `pcm_s16le` and immediate `EndOfStream` — it would fail if the header walk or event loop regressed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "ASF-c landed seeking ... ffmpeg-measured layout" | mod.rs:14-16 | REAL | unit tests pass; layout constants in SimpleIndex::parse |
| "plus the probe gate and the media-probe signature" | mod.rs:7-8 | FALSE | media-probe has no ASF signature (grep = 0 hits); no registry arm |
| "no unwrap/expect/panic! in non-test code" | mod.rs:35 | REAL | grep of mod.rs/header.rs/data.rs/guid.rs: only doc-text mentions |
| "bounded memory (MP §12): the header read is ceiling-checked" | mod.rs:36-37 | SHALLOW | header read is ceiling-checked (MAX_HEADER_BYTES) but the PACKET read is not (F-001) |

## 7. Dead / duplicated / unreachable code
The entire module is unreachable from shipped entry points (F-002) — real code, dormant. `IndexEntry::packet_count` is `#[allow(dead_code)]` (line 174) — carried, unused, disclosed.

## 8. Dependencies & flags
Reuses `crate::avi::riff::SourceReader` (line 67) — the AVI RIFF reader as shared demux I/O layer; pragmatic reuse, layering smell documented nowhere. No feature gates; `pub mod fuzz` / `pub mod test_support` compiled unconditionally like every demux family.

## 9. Open questions for the next agent
- asf/tests.rs owner: is there a test with a huge fixed `max_packet_size`? If not, F-001 is also a test gap.
- Registry owner: was an ASF wiring unit (6.OPW-asf) planned? containers/mod.rs's §5 order list ends at es — ASF never appears.
