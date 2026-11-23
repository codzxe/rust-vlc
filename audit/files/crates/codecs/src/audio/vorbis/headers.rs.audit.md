# AUDIT: crates/codecs/src/audio/vorbis/headers.rs
| Field | Value |
|---|---|
| Auditor | W07-A07 |
| Date | 2026-11-16 |
| Lines / functions | 400 / 5 (+4 tests, 2 fixtures) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen-cli`: apps/cli/src/pipeline_host.rs:274 `"vorbis" => self.seeded(track, config, VorbisDecoder::new)` → `VorbisDecoder::configure` → `VorbisHeaders::from_extra_data` (vorbis/decoder.rs:714); also apps/web/src/host.rs:641 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n "from_packets\|from_extra_data\|is_vorbis_ident\|parse_identification" crates apps` (exit 0); `rg -n "headers::tests" audit/runs/baseline/cargo-test-workspace.txt` (exit 0, 4/4 ok lines 608-611); `rg -n "bitrate_\*\|blocksize_short" --type rust` (exit 0 — readers census); sed quote verification |

## 1. One-paragraph verdict
A small, tight, hostile-input-safe header parser. Every Vorbis I §4.2.2 field of the identification header is validated with a stable typed error code (length, magic, version, channels, rate, blocksize exponents, framing bit); the three-header chain split is driven by a measured (not trusted) comment-header length — `comments::header_len` bounds-walks the actual buffer with a 16 MiB per-field ceiling and a lying-count clamp, so no over-read, no unbounded loop, no panic. All four unit tests assert typed rejections and field round-trips and pass in the baseline; the fuzz target (vorbis/fuzz.rs:74-81) exercises both entry points. Only hygiene-level complaints: the bitrate hints and short blocksize are parsed into public fields nothing in the tree reads, and an oversized identification packet (trailing bytes past 30) is silently tolerated in standalone parse.

## 2. What actually works (verified)
— verified by `rg -n "headers::tests" audit/runs/baseline/cargo-test-workspace.txt` -> `identification_roundtrips_every_field`, `bad_magic_is_the_typed_unsupported_refusal`, `truncation_version_geometry_and_framing_are_all_typed`, `header_chain_split_rejects_short_and_foreign_remainders` all `ok` (lines 608-611).
— verified by reading the chain-split safety: `header_len` (comments.rs:108-146) returns `7 + pos + 1` only after every `read_slice` stayed inside `data`, and `from_extra_data` slices `data[IDENT_LEN..comment_end]` with `comment_end ≤ data.len()` by construction — no index panic is reachable on crafted input.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/audio/vorbis/headers.rs:91-101 and 157-165
Code:
```
    if data.len() < IDENT_LEN {
        return Err(corrupt(
            "vorbis.ident.truncated",
```
(only a lower bound; bytes beyond 30 are never checked)
What is wrong: Vorbis I §4.2.2 fixes the identification header at exactly 30 bytes; `parse_identification` accepts a 40-byte "identification header" with 10 trailing bytes (a standalone oversized packet is parsed, Ok, trailing bytes ignored). In the `from_extra_data` chain the garbage shifts the comment split and dies as a downstream `vorbis.comment.magic` — honest, but the standalone surface is lenient where the module's own doctrine is "damaged chain dies as OUR typed error". What correct looks like: reject `data.len() > IDENT_LEN` in the standalone entry (or at least in `from_packets`, where packet boundaries are known and an ident packet ≠ 30 bytes is malformed per spec). Confidence: HIGH — Verification: read. Blast radius: probe/diagnostic paths only.
F-002 [S4]
Where: crates/codecs/src/audio/vorbis/headers.rs:64-69 and 60-63
Code:
```
    /// Optional maximum bitrate hint (bits/s), `None` when 0.
    pub bitrate_max: Option<i32>,
```
What is wrong: `bitrate_max`/`bitrate_nominal`/`bitrate_min` (and `blocksize_short`) are parsed into public fields that no production code reads (rg census: the only consumer of `Identification` fields is decoder.rs:244/344/352, using `blocksize_long` and `channels` alone). Parsed-but-unread surface — informational API, zero behavior. What correct looks like: either surface the bitrate hints in engine-info/metadata output or drop the fields until a reader exists. Confidence: HIGH — Verification: read + rg.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `is_vorbis_ident` | 76-78 | REAL | — | cheap probe; bounds-checked; re-exported but no non-test caller (probe surface for future demuxers) |
| `parse_identification` | 90-166 | REAL | S4 (F-001) | all §4.2.2 fields typed; LE decode verified against test fixture; `1 << bs0` with bs0 ≤ 13 safe |
| `VorbisHeaders::from_packets` | 189-211 | REAL | — | count gate = 3 (spec-exact); joins then re-splits — one extra copy, configure-time only |
| `VorbisHeaders::from_extra_data` | 220-253 | REAL | — | measured comment split; setup magic/truncation typed; clones verbatim packets (configure-time) |
| `engine_extra_data` | 259-264 | REAL | — | ident ‖ setup; consumed by decoder.rs `build_engine` (symphonia non-laced branch) |
| tests (4) | 288-399 | REAL | — | rejection-path complete; would fail on regression |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| identification_roundtrips_every_field | YES | every field asserted including None-vs-Some bitrate mapping |
| bad_magic... | YES | asserts code `vorbis.ident.magic` + probe false |
| truncation_version_geometry_and_framing... | YES | 5 distinct typed codes exercised (truncated/version/rate/blocksize-inverted/framing) |
| header_chain_split... | YES | 4 typed outcomes: comment magic, setup truncated, setup magic, headers count |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Nothing here trusts the engine for header validation ... a damaged chain dies as OUR typed error" | headers.rs:25-28 | REAL | the 4 rejection tests + fuzz entry (fuzz.rs:74-81) |
| "the comment header is length-prefixed to its last byte ... the chain is self-delimiting" | headers.rs:11-14 | REAL | header_len's bounds-walk (comments.rs:108-146) |
| "channels (1–255; symphonia maps up to 8 positions — above that configure is a typed refusal)" | headers.rs:55-57 | REAL (claim belongs to decoder) | vorbis/decoder.rs enforce path — outside this file; UNVERIFIED here, decoder.rs audited by another agent |

## 7. Dead / duplicated / unreachable code
`bitrate_max/nominal/min`, `blocksize_short` — parsed, published, never read (F-002). `is_vorbis_ident` — no production caller (re-exported probe surface). No dead branches.

## 8. Dependencies & flags
`super::comments` (header_len/parse), `super::error` (corrupt/unsupported), `media_types::EngineError`. No external deps. No feature gate on this path (codec-vorbis default-on). No unsafe. No allocation beyond configure-time packet clones.

## 9. Open questions for the next agent
- vorbis/decoder.rs owner: confirm the >8-channel typed refusal actually exists at configure (headers.rs:55-57 defers to it; I did not audit decoder.rs — 1118 lines, another agent's row).
- If a WAV/FLAC VORBIS_COMMENT reuse lands (comments.rs:6-9 proposal), `parse_comment_body` is the seam — its lying-count clamp (`count.min(remaining/4 + 1)`) deserves its own test there; here it is exercised only via header_len.
