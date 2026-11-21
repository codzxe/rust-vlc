# AUDIT: crates/demux/src/ps/mod.rs#part1 (lines 1–619)
| Field | Value |
|---|---|
| Auditor | W05-A07 |
| Date | 2026-11-21 |
| Lines / functions | 619 audited (assigned ≈1–609; extended to 619 = end of `parse_pes_full`, a natural function boundary — part2/A06 starts at 620) / 30 items in table |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen open <ps/vob>` → media-probe signature `mpegps` (`00 00 01 BA`, crates/media-probe/src/signature/phase5.rs:178) → apps/cli/src/containers/mod.rs:71 `"mpegps" => ps::open` → apps/cli/src/containers/ps.rs:144 `PsDemuxer::new` → `start()`/`next_event()` → `parse_pack_at`/`parse_pes_full`/`stamp_of`/`parse_stamp` (this chunk). Baseline executed: ps::tests 26/26 (nofailfast log :5735), ps_fixtures 4/4 (:2178–2181), containers::ps 4/4 (:2830–2833), real-binary CLI test `mpegps_container_opens_and_accepts_the_session` (:3044), module doctest (:6370) — all `ok`. |
| Findings | S0:0 S1:0 S2:1 S3:2 S4:4 |
| Commands run | rg "PsDemuxer" (callers: containers/ps.rs:144, ps_fixtures.rs:193, ps/tests.rs, ps/fuzz.rs); rg "EXTENSIONS|ps::probe" over crates+apps (no callers); rg ".current_scr(|.scr_extent(|.is_mpeg2(|.pool_bytes(" (test/fuzz-only or none); sed -n + python hex verification of every quote; rg in ts/pes.rs for the swallow-refusal contrast; read avi/riff.rs:357-369 `read_prefix_into` (clamps to length, returns short count, no panic) and avi/header.rs:385-401 `DamageLog`; cat crates/demux/Cargo.toml (no [features], std-only); sed of baseline nofailfast log for PS test blocks. |

## 1. One-paragraph verdict
This chunk is real, reachable, and executed: pack/PES header parsing with correct SCR/PTS bit extraction (both MPEG-1 and MPEG-2 flavors, verified against the field layouts), honest typed errors, bounded reads through the clamping `SourceReader`, and baseline runs that pass 26 unit + 4 fixture + 4 container-host + 1 CLI test plus the doctest. But the MPEG-1 PES branch misses the one guard its MPEG-2 sibling and the TS sibling both enforce — the parsed header length is never checked against the declared packet length — so `payload_len = declared_len - header_len` underflows on malformed input: a debug build panics, a release build wraps to ~2^64 and either aborts the session with a typed "payload too large" or silently ends the track pre-scan. Secondary defects: stamps fabricated from never-read stack zeros when a malformed MPEG-2 header length contradicts its own flags, a cluster of dead public observables, and lenient acceptance of codes the TS demuxer refuses typed.

## 2. What actually works (verified)
- Pack-header SCR/mux-rate/stuffing extraction, both flavors — verified by reading the bit arithmetic against the `'01'`/`'0010'` layouts (mod.rs:439-482) and by baseline `ps::tests::mpeg1_pack_roundtrips_scr_and_rate`, `mpeg2_pack_roundtrips_scr_rate_and_stuffing` (`ok`, nofailfast :1835 block).
- `parse_stamp` 3/15/15 split arithmetic (mod.rs:274-278) — baseline `ps::tests::stamp_bytes_roundtrip_with_the_parser ... ok`.
- The declared_len==0 unbounded-PES case returns payload_len 0 and never reads unbounded (mod.rs:505-517) — baseline `ps::tests::zero_pes_length_is_damage_noted_never_an_unbounded_read ... ok`.
- Private-1 substream routing table (Ac3/Dts/Lpcm/Spu ranges + strips 4/7/1, mod.rs:146-173) — baseline `ps::tests::private1_ac3_strips_substream_and_three_filler_bytes`, `private1_dts_lpcm_and_spu_route_with_their_strips ... ok`; fixture suite `fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit ... ok` (:2180).
- Reachability of the whole lane through the real binary — baseline `mpegps_container_opens_and_accepts_the_session ... ok` (:3044), which runs `lumen open` on `ps-mp2-video-audio.mpg` and asserts `ok: accepted session=1`.

## 3. Findings

### F-001 [S2]
Where: crates/demux/src/ps/mod.rs:576-610 (missing guard) and 609-610 (underflow)
Code:
```
                0b0001..=0b0011 => {
                    let wide = buf[at] >> 4 == 0b0011;
                    let need = at + if wide { 10 } else { 5 };
                    if got_h < need {
```
and
```
        let data_start = pos + 6 + header_len;
        let payload_len = u64::from(declared_len) - header_len;
```
What is wrong: The MPEG-1 branch validates the stamp window only against bytes-to-EOF (`got_h < need`), never against the PES's own declared packet length. When the parsed MPEG-1 header length (stuffing + stamp marker, up to 16+10 bytes) exceeds `declared_len` (as small as 1), line 610 computes `u64::from(declared_len) - header_len` with header_len > declared_len. The MPEG-2 branch in the same function enforces exactly this (`if want > usize::from(declared_len)` at 533-536), and the TS sibling demuxer enforces it too (crates/demux/src/ts/pes.rs:359-366, with baseline `ts::pes::tests::header_swallowing_the_declared_packet_is_refused ... ok` :1860) — only the PS MPEG-1 lane lacks it.
Why it matters: On a 23-byte crafted or damaged MPEG-1 file (pack header + PES with declared_len=1 whose first payload byte is a stamp marker), the subtraction panics in debug builds ("attempt to subtract with overflow") and wraps to ≈2^64 in release. In release the wrapped value flows (a) into the pre-scan at line 709 `pos = pes.data_start + pes.payload_len` → pos explodes past length → the `while pos + 4 <= length` loop exits with NO damage note — remaining streams are silently never registered (truncated track list, no error); and (b) into `emit_pes_packet` (part2, lines 888-894) where `want > MAX_PACKET_BYTES` raises a typed `payload_too_large` that kills the whole session — one damaged packet aborts playback instead of being damage-skipped, contradicting the module's own recovery ethos. The release-only baseline (`--release`, INDEX.md footnote) means `ps::tests::fuzz_lite_mutations_of_the_committed_fixtures_never_panic ... ok` (:1841) cannot detect this: release arithmetic wraps silently, so the "never panic" guarantee was never tested against the profile that panics.
What correct looks like: mirror the MPEG-2 guard before line 610 — if `header_len > u64::from(declared_len)`, return `error::bad_pes_header(pos)` (or a damage-noted skip), exactly as ts/pes.rs:359-366 does; add an MPEG-1 fixture test with declared_len smaller than its stamp window.
Confidence: HIGH — Verification: read (line-by-line; `read_prefix_into` semantics confirmed at avi/riff.rs:357-369 — clamps to length, so `got_h` counts bytes to EOF, not to packet end); release/debug behavior per Rust overflow defaults. No runtime repro (cargo forbidden by hard rule 8).

### F-002 [S3]
Where: crates/demux/src/ps/mod.rs:549-556 (with 531-537)
Code:
```
            match buf[1] >> 6 {
                0b10 => p = Some(parse_stamp(&buf[3..8], pos)?),
                0b11 => {
                    p = Some(parse_stamp(&buf[3..8], pos)?);
                    d = Some(parse_stamp(&buf[8..13], pos)?);
                }
                _ => {}
            }
```
What is wrong: The PTS_DTS flags are honored without validating that `hl` (the declared PES header data length, buf[2]) actually contains the stamp bytes: with flags `0b10` and hl < 5 (e.g. hl = 0), only `want = 3 + hl` bytes are ever read into `buf`; `buf[3..8]` (and `buf[8..13]` for `0b11` with hl < 10) are the stack array's zero-initialized tail — bytes that were never read from the file.
Why it matters: A malformed MPEG-2 PES that sets the PTS flag but declares hl=0 yields `pts: Some(0)` — a fabricated timestamp presented as real, where the correct behavior (per the module's "typed, never silent" doctrine, mod.rs:1-3) is a `bad_pes_header` refusal. A bogus zero stamp can also poison A/V-sync watermarks downstream. Blast radius: malformed input only; no panic (slices stay inside the 258-byte array).
What correct looks like: require `hl >= 5` for `0b10` and `hl >= 10` for `0b11` before parsing (or parse only if the stamp fits within `want`), else typed `bad_pes_header`.
Confidence: HIGH — Verification: read; `buf` is `[0u8; MAX_PES_HEADER]` (523) and only `buf[..want]` is filled (537).

### F-003 [S3]
Where: crates/demux/src/ps/mod.rs:121-127 (and 392-403)
Code:
```
pub const EXTENSIONS: &[&str] = &["mpg", "mpeg", "ps", "vob", "m2p"];

/// The probe: does the stream start with a pack start code?
#[must_use]
pub fn probe(header: &[u8]) -> bool {
    header.starts_with(&[0x00, 0x00, 0x01, 0xBA])
}
```
(line 124 byte-verified via python hex: `235b6d7573745f7573655d` = `#[must_use]`)
What is wrong: `ps::probe` and `ps::EXTENSIONS` have zero callers anywhere in crates/ or apps/ (rg over both trees: the only references are the definitions, docs, and media-probe's independent mirror comment at signature/phase5.rs:178 "mirrors `demux::ps::probe`"). The shipped dispatch path probes through media-probe signatures and string ids (`containers/mod.rs:71 "mpegps"`), never through these. Likewise `PsDemuxer::pool_bytes` (401-403) and `PsDemuxer::damage_total` (392-396) have no callers at all (every `damage_total()` caller in the repo is on the AVI/WAV/OGG/ADTS demuxers); `current_scr`/`scr_extent`/`is_mpeg2` are called only from ps/tests.rs and ps/fuzz.rs.
Why it matters: Dead public API that self-describes as "the R3 registry row" but is wired to no registry — a reader auditing "how does a .vob get opened" through this file is pointed at machinery the binary never runs, while the real gate lives in a hand-mirrored table in another crate that can drift (media-probe cannot depend on demux, so its mirror is unverifiable by type). The unused observables (`pool_bytes`, `damage_total`) are the D-1 budget/damage instrumentation — nothing observes them for PS.
What correct looks like: either delete the dead surface or wire the registry to it (single source of truth for extensions/probe); the PS damage/pool observables should be consumed by the container host or tests (as AVI's are) or dropped.
Confidence: HIGH — Verification: rg over crates/*.rs and apps/*.rs for `EXTENSIONS`, `ps::probe`, `\.current_scr\(`, `\.pool_bytes\(`, `damage_total`; ps/tests.rs:21 imports only `CLOCK, PsDemuxer, parse_stamp`.

### F-004 [S4]
Where: crates/demux/src/ps/mod.rs:429-437
Code:
```
        let mpeg2 = buf[4] & 0xC0 == 0x40;
        if !mpeg2 && buf[4] & 0xF0 != 0x20 {
            // Neither the MPEG-2 '01' nor the MPEG-1 '0010' prefix.
            return Err(error::bad_pack(pos));
        }
        let need = if mpeg2 { 14 } else { 12 };
        if got < need {
```
What is wrong: `buf[4]` is inspected before `got >= 5` is known. When a pack start code sits at EOF-4 bytes (got == 4), `buf[4]` is the zero-initialized tail, both prefix tests fail, and the caller gets `bad_pack` (InvalidStructure) instead of the honest `truncated` — the truncation arm at 435-437 only fires for got ≥ 5 with a recognized prefix.
Why it matters: Error-kind drift on the degenerate case: a file cut exactly at a pack start code reports "structurally invalid" rather than "truncated", misdirecting diagnostics. Baseline `ps::tests::truncated_pack_is_the_typed_truncation` (tests.rs:357) only exercises got=6, so the got=4 leg is untested.
What correct looks like: check `got < 5` (return `truncated(pos+5, pos+got)`) before touching `buf[4]`.
Confidence: HIGH — Verification: read; test coverage checked at tests.rs:355-360.

### F-005 [S4]
Where: crates/demux/src/ps/mod.rs:279-282 (and 451-453, 462-464)
Code:
```
    if value >> 33 != 0 {
        return Err(error::bad_pes_header(pos));
    }
    i64::try_from(value).map_err(|_| error::bad_pes_header(pos))
```
What is wrong: These width checks are vacuous. The assembled `value` is built from masked fields totaling exactly 33 bits (3+8+7+8+7 in parse_stamp; both SCR extractions are identical), so `value >> 33` is provably always 0 and `i64::try_from` of a u64 < 2^33 always succeeds. Same for both `scr_base >> 33 != 0` arms (451-453, 462-464). Meanwhile crates/demux/src/ps/error.rs:47-50 documents `bad_pack` as covering "SCR field bits outside their declared widths (… every extraction is width-checked before it is trusted)" — a documented protection that cannot fire.
Why it matters: Dead defensive branches plus a doc claim that overstates the validation; today's reader is misled about what is guarded, and the dead checks suggest a masking bug would be caught when it would not be.
What correct looks like: Either delete the dead checks and correct the error.rs doc, or derive the masks from named constants so the checks and the arithmetic cannot disagree.
Confidence: HIGH — Verification: read + arithmetic (each OR-term's maximal value sums to 2^33−1; field widths computed by hand).

### F-006 [S4]
Where: crates/demux/src/ps/mod.rs:549-556 (the `_ => {}` arm)
Code:
```
                0b11 => {
                    p = Some(parse_stamp(&buf[3..8], pos)?);
                    d = Some(parse_stamp(&buf[8..13], pos)?);
                }
                _ => {}
```
What is wrong: MPEG-2 PTS_DTS_flags `0b01` is forbidden by the same header-format table the TS sibling demuxer cites; this code silently treats it as "no stamps" via the catch-all arm. The TS demuxer refuses it typed: crates/demux/src/ts/pes.rs:355-357 `0b01 => return Err(short_header())`, with baseline `ts::pes::tests::forbidden_pts_dts_flags_refuse_typed ... ok` (:1864).
Why it matters: Cross-family inconsistency in strictness for one shared header format: a stream the TS lane brands structurally broken demuxes silently here. Low blast radius (no data fabricated — the packet is merely treated as unstamped).
What correct looks like: `0b01 => return Err(error::bad_pes_header(pos))`, matching ts/pes.rs, plus a test.
Confidence: MEDIUM — Verification: read + the TS contrast verified in-repo (ts/pes.rs:353-357); the ISO 13818-1 wording itself is not available in this sandbox (cited from the TS sibling's comment).

### F-007 [S4]
Where: crates/demux/src/ps/mod.rs:586-588
Code:
```
                    let first = parse_stamp(&buf[at..at + 5], pos)?;
                    if buf[at] >> 4 == 0b0001 {
                        d = Some(first);
```
What is wrong: The MPEG-1 arm accepts high nibble `0b0001` as a DTS-only stamp. No test covers it (rg over ps/tests.rs: no `0001`/DTS-only MPEG-1 PES case; the module's own pinned-conventions list at mod.rs:33-46 documents only the `0x0F`-unstamped byte, the private-1 filler, and the marker-bit leniency — never `0001`), and whether real MPEG-1 muxers emit DTS-without-PTS headers is UNVERIFIED here (no ISO 11172-1 text or corpus sample in the sandbox).
Why it matters: An untested, spec-dubious parsing branch in the input-trust boundary; if `0001` is as forbidden in MPEG-1 headers as `01` is in MPEG-2 flags, this is another silent-acceptance divergence from the TS lane.
What correct looks like: pin the decision one way — a fixture test asserting the accepted behavior with a comment citing the spec clause, or a typed refusal.
Confidence: LOW — Verification: read; UNVERIFIED spec status (blocked: no spec text in sandbox).

## 4. Per-function table (chunk lines 1–619)
| Function / item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs | 1-96 | REAL | — | claims cross-checked below (§6) |
| `MAX_PACKET_BYTES` | 99-102 | REAL | — | 64 KiB > u16 max 65 535 — ceiling never clips a legal PES ✓ |
| `MAX_RECOVERY_SCAN_BYTES` | 104-107 | REAL | — | consumed by part2 `resync_from` (A06) |
| `CLOCK` | 110 | REAL | — | 90_000 ✓ |
| `MAX_PES_HEADER` | 112-114 | REAL | — | 3+255=258; `buf[..want]` can never exceed it ✓ |
| `MAX_MPEG1_STUFFING` | 116-118 | REAL | — | 16; bounds slice math (`at+10 ≤ 26`) ✓ |
| `EXTENSIONS` | 121 | UNREACHABLE | S3 | F-003: no callers; media-probe keeps its own mirror |
| `probe` | 124-127 | UNREACHABLE | S3 | F-003: no callers |
| `PrivateKind` enum | 132-142 | REAL | — | DVD-book ranges documented |
| `PrivateKind::of_substream` | 146-154 | REAL | — | 0x80-87/0x88-8F/0xA0-A7/0x20-3F ✓; tested via private1 tests |
| `PrivateKind::strip` | 157-163 | REAL | — | 4/7/1 — pinned bit-exact by baseline fixture tests |
| `PrivateKind::descriptor` | 166-173 | REAL | — | honest names (ac3/dts/pcm_dvd/dvd_subtitle); decode refusal is downstream typed |
| `PsStream` | 177-189 | REAL | — | data holder; first/last PTS filled in part2 |
| `StreamKey` | 192-202 | REAL | — | routing key |
| `PackHeader` | 205-217 | REAL | — | data holder |
| `PesHeader` | 220-233 | REAL | — | data holder; `payload_len` field is the F-001 victim |
| `StickyShape::of` | 244-250 | REAL | — | replayable error shape; no data loss |
| `StickyShape::mint` | 252-254 | REAL | — | re-mints identical error |
| `stamp_of` | 260-265 | REAL | — | 33-bit ticks at 1/90000 ≈ 9.5e13 ns — fits; fallback leg defensive |
| `parse_stamp` | 270-283 | REAL | S4 | arithmetic correct (3/15/15 split; prefix bits ignored by design — callers validate); F-005 vacuous width checks |
| `PsDemuxer` struct | 310-332 | REAL | — | plain state |
| `PsDemuxer::new` | 343-361 | REAL | — | rejects unseekable/unknown-length/empty via SourceReader (avi/riff.rs); pool 2×64 KiB bounded |
| `stream_descriptors` | 368-370 | REAL | — | used by containers/ps.rs:218 (harvest_facts) ✓ |
| `current_scr` | 374-376 | REAL-UNPROVEN | S3 | test/fuzz-only callers (F-003) |
| `scr_extent` | 381-383 | REAL-UNPROVEN | S3 | test-only (F-003) |
| `is_mpeg2` | 388-390 | REAL-UNPROVEN | S3 | test-only (F-003) |
| `damage_total` | 392-396 | UNREACHABLE | S3 | zero callers (F-003) |
| `pool_bytes` | 398-403 | UNREACHABLE | S3 | zero callers (F-003) |
| `fail` / `replay` | 406-417 | REAL | — | sticky-error plumbing, idempotent |
| `parse_pack_at` | 422-494 | REAL | S4 | SCR/mux-rate/stuffing extraction verified correct both flavors; F-004 prefix-before-length; stuffing skip bounded by `end > length` check ✓ |
| `parse_pes_full` | 497-619 | REAL | S2/S3 | F-001 (MPEG-1 missing guard → underflow), F-002 (stamps past hl), F-006 (forbidden '01' accepted); MPEG-2 guard at 533-536 correct; `read_prefix_into` clamping means short reads never panic |

## 5. Test honesty (tests in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| module doctest (mod.rs:296-309) | YES | asserts 2 tracks + a Packet event from a synthetic stream; ran in baseline (:6370 `ok`) |
| `ps::tests` 26/26 (:1835-1860, :5735) | YES for covered lanes | roundtrip tests assert exact SCR/rate/stamp values — mutating the extraction arithmetic breaks them |
| `fuzz_lite_mutations_of_the_committed_fixtures_never_panic` (:1841) | NO for the F-001 class | ran under `--release` (INDEX.md footnote) where overflow wraps instead of panicking; the "never panic" claim is untested against debug builds, and the release wrap itself (silent scan truncation) is not asserted either |
| `truncated_pack_is_the_typed_truncation` (tests.rs:357) | PARTIAL | covers got=6 only; the got=4 misdiagnosis (F-004) passes |
| ps_fixtures 4/4 (:2178-2181) | YES | bit-for-bit payload pins against ffmpeg extractions |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the differential suite pins the demuxed payloads against ffmpeg's own raw extractions bit-for-bit" | mod.rs:12-14 | REAL | baseline `fixture_payload_streams_match_ffmpeg_extractions_bit_for_bit ... ok` (:2180) |
| "decode refusal is a typed `Unsupported` at the codec registry — the demuxer never silently drops them" | mod.rs:51-54 | REAL (demux side) | descriptors exposed via `stream_descriptors` (ps.rs:218); no silent drop path in this chunk |
| "typed, never silent" (module header) | mod.rs:1-3 | SHALLOW | F-002 fabricates a 0-stamp silently; F-006 silently accepts a forbidden flag pair — doctrine holds on the happy path only |
| "`PES_packet_length == 0` … is a damage-noted skip, never an unbounded read" | mod.rs:74-76 | REAL | mod.rs:505-517 returns payload_len 0 + damage note; baseline `zero_pes_length_is_damage_noted...` ok |
| "every extraction is width-checked before it is trusted" | ps/error.rs:47-50 | FALSE (vacuous) | F-005: the width checks cannot fire — the claim describes dead code as protection |

## 7. Dead / duplicated / unreachable code
- `EXTENSIONS` (121), `probe` (124-127), `PsDemuxer::damage_total` (392-396), `PsDemuxer::pool_bytes` (398-403): zero callers anywhere (F-003).
- `current_scr`/`scr_extent`/`is_mpeg2`: test/fuzz-only.
- Probe logic duplicated by hand in crates/media-probe/src/signature/phase5.rs:178 — no type-level link, drift possible.
- Vacuous branches: `value >> 33` / `scr_base >> 33` checks (279-281, 451-453, 462-464), `i64::try_from` (282) (F-005).

## 8. Dependencies & flags
- demux crate has NO [features] section (crates/demux/Cargo.toml read) — the PS module is compiled unconditionally in every build including the shipped CLI; no feature-gating risk in this file.
- Cross-family reuse of `crate::avi::riff::{PayloadSlots, SourceReader}` and `crate::avi::header::DamageLog` (lines 89-90) — same-crate layering, documented as deliberate ("the AVI W2-07 budget precedent", mod.rs:63-68). `read_prefix_into` clamps reads to stream length (avi/riff.rs:357-369), which is why no short-read panic exists in this chunk.
- std-only, no unsafe, no unwrap/expect in lines 1-619 (consistent with the crate's C5 claim in Cargo.toml).

## 9. Open questions for the next agent (A06 — ps/mod.rs#part2, lines 620-1220)
- F-001's release-mode damage flows through part2: verify `resync_from`, the scan loop exit (`while pos + 4 <= length` — the huge-pos exit adds no damage event), and `emit_pes_packet`'s `payload_too_large` sticky behavior — recommend the guard fix land with part2's tests.
- `probe_video_codec`/`probe_audio_codec` (766+) read ≤64-byte peeks — verify the `.min(64)` clamp handles F-001's wrapped `payload_len` (line 767 suggests it does).
- `register_key`'s first/last PTS bookkeeping and the seek clamp (`seek_clamps_into_the_scr_extent...`) — check 33-bit SCR wraparound handling.
- ps/fuzz.rs (A02): if the fuzz harness builds in debug, F-001 is an immediately reachable panic for it.
