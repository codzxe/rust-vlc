# AUDIT: crates/codecs/src/audio/alac/decoder.rs#part1 (lines 1-442 of 884)
| Field | Value |
|---|---|
| Auditor | W07-A05 |
| Date | 2026-11-21 |
| Lines / functions | 442 / 14 (module docs + 10 fns + 3 structs) |
| File verdict (part1) | REAL |
| Reachable from shipped entry point? | YES — `lumen open x.m4a` → MP4 demux `crates/demux/src/mp4/mod.rs:450` `b"alac" => Some("alac")` (and MKV `crates/demux/src/mkv/header.rs:812` `A_ALAC`) → CLI host `apps/cli/src/pipeline_host.rs:293-297` `"alac" => self.seeded(..., AlacDecoder::new)` (magic cookie in `extra_data`); `codec-alac` default-on. `parse_alac_config`/`engine_for`/free surface also used by tests + fuzz; contract path uses the same `parse_alac_config_inner` + `decode_into`. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "max_frame_bytes"` crates (assigned once at decoder.rs:211, read nowhere — control-protocol hits are an unrelated crate); `rg -n "alac"` baseline orchrun log:311-320,813-833,6331 (unit + bitexact + fuzz + doctest all PASS); `rg -n "alac"` demux mp4/mkv (descriptor minting); `sed -n '60,110p' docs/dependencies/records/symphonia-alac-0.6.1.md` (engine admission record, R6 coverage); `sed -n '443,470p'` decoder.rs (chunk boundary). |

## 1. One-paragraph verdict
Part 1 is the ALAC wrapper's negotiation core and it is real: the container-prefix stripper (`skip_cookie_prefixes`/`find_alac_atom` — I re-derived termination and bounds: every iteration strictly shrinks the slice, all indexing in-bounds), the typed ALACSpecificConfig gates, the width-narrowing map, and an engine-error taxonomy whose truncation tripwire is string-matched but documented against the engine source. Bit-exactness against ffmpeg is proven by a passing baseline differential (`fixtures_bitexact_vs_ffmpeg ... ok`). The one genuine defect: `AlacConfig::max_frame_bytes` is parsed from the WRONG cookie offset (14 instead of 12 — self-inconsistent with the file's own correct `sample_rate` at 20 under the canonical 24-byte layout) and is dead on top of it, so a garbage value sits in a public struct field waiting for a future consumer. Two minor quality items: filler truncation detail and the lenient `frma` strip.

## 2. What actually works (verified)
- Baseline `tests/alac_bitexact.rs::fixtures_bitexact_vs_ffmpeg ... ok` (orchrun log:825) — the decode chain this part feeds is bit-exact against ffmpeg 7.1.5 on committed fixtures.
- Baseline unit tests for this range: `cookie_parse_accepts_the_passthrough_shapes`, `cookie_damage_is_typed_per_rule`, `output_width_follows_the_bit_depth`, `truncation_maps_to_the_typed_tripwire` all `ok` (log:311-315); fuzz `mutated_cookies_are_typed_not_panics`, `mutated_packets_never_panic_or_hang` `ok` (log:829-831); doctest `ok` (log:6331).
- `skip_cookie_prefixes` termination and index-safety re-derived by hand (see F-003 analysis): no infinite loop (an `at == 0` return from `find_alac_atom` is impossible because the `buf[4..8] == b"alac"` 12-byte strip would have fired first), no OOB (`at+8 < len` by `scan = min(len−8, 40)`).

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/audio/alac/decoder.rs:206-213
Code:
```rust
    let config = AlacConfig {
        frame_length: be32(0),
        bit_depth: body[5],
        rice_pb_mb_kb: (body[6], body[7], body[8]),
        num_channels: body[9],
        max_frame_bytes: be32(14),
        sample_rate: be32(20),
    };
```
What is wrong: under the canonical 24-byte ALACSpecificConfig layout (frameLength 0-3, compatibleVersion 4, bitDepth 5, pb/mb/kb 6-8, numChannels 9, maxRun 10-11, **maxFrameBytes 12-15**, avgBitRate 16-19, sampleRate 20-23 — the same layout the file's own correct `sample_rate: be32(20)` and `num_channels: body[9]` reads imply), `be32(14)` reads bytes 14-17 = (maxFrameBytes & 0xFFFF)≪16 | (avgBitRate ≫ 16) — not maxFrameBytes. On a real cookie (e.g. maxFrameBytes 0x2000, avgBitRate 0x000AC440-ish) the field yields a garbage ~0x2000_00xx value.
Why it matters: the field is dead today (`rg "max_frame_bytes"` over crates: assigned here, read nowhere — the struct doc even calls it "diagnostics"), so nothing breaks now; but it is a public `pub` struct field carrying a wrong number with a name that promises the spec field — the first consumer (a buffer-sizing heuristic, a probe display, a max-output sanity check) inherits garbage. Dead + misparsed + public = a trap.
What correct looks like: `max_frame_bytes: be32(12)` — or delete the field until something reads it.
Confidence: HIGH on the deadness (rg-verified), MEDIUM on the offset (canonical layout from the Apple reference decoder / ffmpeg `alac.c`; symphonia's own `MagicCookie::read` source not in-sandbox to cross-check — the repo's own consistent offsets 0/5/6-8/9/20 make offset 12 the only self-consistent choice).

### F-002 [S4]
Where: crates/codecs/src/audio/alac/decoder.rs:343-347
Code:
```rust
                    media_types::error::CorruptDetail::Truncated {
                        expected: 0,
                        found: packet_len as u64,
                    }
```
What is wrong: the truncation detail's `expected: 0` is filler — "expected 0 bytes, found N" says nothing about where the bitstream ran dry.
Why it matters: diagnostics/telemetry consume these details; the same filler pattern recurs in mp2/decoder.rs:139-144 (wave 7 cross-file pattern).
What correct looks like: `expected: <bytes the engine needed>` is unknowable post-hoc — use `CorruptDetail::InvalidStructure { offset: None }` or a dedicated detail carrying the packet length as "found" with a truthful "expected".
Confidence: HIGH — Verification: read.

### F-003 [S4]
Where: crates/codecs/src/audio/alac/decoder.rs:139-146
Code:
```rust
        if buf.len() >= 12 && &buf[4..8] == b"frma" {
            buf = &buf[12..];
            continue;
        }
        if buf.len() >= 12 && &buf[4..8] == b"alac" {
            buf = &buf[12..];
            continue;
        }
```
What is wrong: both strips are fourcc-match-only — the atom's own size word (bytes 0-4) is never validated (contrast `find_alac_atom`, which does check `size >= 12 && at + size == buf.len()`), despite the doc's "never a blind skip" claim (lines 134-136) which is scoped to the sample-entry case only.
Why it matters: a hostile cookie can chain 12-byte strips through crafted fourccs; all outcomes still land in the typed 24/48 length gate (no panic, bounded by input length), so the blast radius is diagnostic confusion at worst.
What correct looks like: validate `be32(0) >= 12` (and `== buf.len()` where the shape demands it) before each strip, matching `find_alac_atom`'s discipline.
Confidence: HIGH — Verification: read + hand-derivation of termination (each iteration removes ≥ 12 bytes or jumps `at ≥ 1`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module doc claims | 1-95 | REAL | — | Engine-bug-not-reproduced narrative (63-87) is an honest re-derivation record; differentials pass in baseline. |
| `AlacConfig` (struct) | 112-128 | REAL | S3 | `max_frame_bytes` dead + misparsed (F-001); all other fields consumed (frame_length→max_output_len, bit_depth→width, channels/rate→format+gates). |
| `skip_cookie_prefixes` | 137-153 | REAL | S4 | Terminates; unvalidated size word on frma/alac strips (F-003). |
| `find_alac_atom` | 160-172 | REAL | — | Bounds-safe (`scan = min(len−8, 40)`); size self-delimitation checked; 40-byte cap documented. |
| `parse_alac_config` | 183-185 | REAL | — | Public wrapper; used by tests/fuzz + contract configure. |
| `parse_alac_config_inner` | 189-250 | REAL | S3 | Typed gates (version/bit_depth/channels/frame_length/rate) all sound; frame_length gate mirrors the engine's 4096×16 DoS bound; F-001 offset bug. |
| `output_sample_format` | 256-262 | REAL | — | ≤16→S16, 17-24→S24, else S32; test-pinned (`output_width_follows_the_bit_depth`, baseline:313). |
| `output_format` | 266-274 | REAL | — | Interleaved, cookie rate; test-pinned (`output_format_matches_the_decode_mapping`, baseline:320). |
| `bytes_per` / `max_output_len` | 277-287 | REAL | — | Slot sizing from frame_length×channels×width — correct maximum under the engine's frame_length respect (see open question). |
| `DecodedAlac` (struct) | 292-301 | REAL | — | Free-surface carrier. |
| `engine_for` | 310-323 | REAL | — | Strips prefixes (engine rejects the MP4 sample-entry header — documented workaround, no engine patching); free surface/fuzz only. |
| `map_engine_error` | 327-378 | REAL | S4 | Full taxonomy incl. `#[non_exhaustive]` arm; string-match truncation tripwire documented against engine source; filler detail (F-002). |
| `BlockMeta` (struct) | 381-387 | REAL | — | Carrier. |
| `decode_into` (head; body continues in part 2) | 392-442 | REAL | — | Empty-packet typed gate; zero-samples typed gate; channels-vs-cookie drift gate (documented unreachable-by-construction guard); scratch grow-only, interleaved copy; narrowing loop is part 2's range. |

## 5. Test honesty
In-file tests for this range live past line 442 (part 2/3); baseline evidence: `cookie_parse_accepts_the_passthrough_shapes`, `cookie_damage_is_typed_per_rule`, `truncation_maps_to_the_typed_tripwire`, `output_width_follows_the_bit_depth` all pass (log:311-315), and the external `tests/alac_bitexact.rs::fixtures_bitexact_vs_ffmpeg` passes (log:825). Mutation check (reasoned, not run): flipping the version gate (`body[4] > 0` → `> 1`) would fail `cookie_damage_is_typed_per_rule`; breaking the width map would fail `output_width_follows_the_bit_depth`. No test asserts `max_frame_bytes` — consistent with F-001's deadness and why the wrong offset survived.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the SAME bytes are then handed to the engine untouched — we never re-serialize the cookie" (46-48) | module doc | REAL | `engine_for`:310-316 passes `skip_cookie_prefixes(cookie)` verbatim as `extra_data`. |
| "differentials below … are bit-exact" (77-79) | module doc | REAL | baseline `fixtures_bitexact_vs_ffmpeg ... ok` (log:825). |
| "locating the well-formed `alac` atom; never a blind skip" (134-136) | fn doc | SHALLOW | True for `find_alac_atom`; the `frma`/`alac` 12-byte strips in `skip_cookie_prefixes` ARE blind on the size word (F-003). |
| "Bounded work: one decode_ref call consumes at most the packet" (29-31) | module doc | REAL | engine's BitReaderLtr bounded by `packet.data`; fuzz survival passes (log:829-831). |

## 7. Dead / duplicated / unreachable code
- `AlacConfig::max_frame_bytes`: dead (rg over `crates`: zero readers) + misparsed (F-001).
- `rice_pb_mb_kb`: doc says "diagnostics; the engine's own copy governs decode" — read by tests only (acceptable, documented).

## 8. Dependencies & flags
- `symphonia-codec-alac` =0.6.1 behind `codec-alac` (default-on) — admission record `docs/dependencies/records/symphonia-alac-0.6.1.md` (R6 coverage verified against extracted source by the admitting session; not re-verifiable in this sandbox — registry cache holds only build artifacts).
- No unsafe; no unwrap/expect in this range; single `unreachable!`-free part.

## 9. Open questions for the next agent (part 2, lines 443-884)
- The narrowing loop (from line 442) writes into `dst` sized by `max_output_len` = cookie frame_length × channels × width. Is there a guard that the engine's per-packet `samples` ≤ `config.frame_length`? A hostile partial-frame header re-stating a LARGER sample count than the cookie's frameLength would overflow `dst` — check the loop bounds before concluding (the engine's own config parse may clamp it; if not, that is an S1 slice-overflow candidate spanning the part boundary).
- Confirm part 2's `AlacDecoder::decode` re-checks `drop_stale_packet` and whether re-`configure` rebuilds the engine from the retained cookie (the ReseedingDecoder wrapper carries the blob).
