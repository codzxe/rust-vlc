# AUDIT: crates/codecs/src/video/dnx/error.rs
| Field | Value |
|---|---|
| Auditor | W06-A05 |
| Date | 2026-11-21 |
| Lines / functions | 271 / 9 (enum + `into_engine_error` + `From` impl, `Detail` carrier + 2 impls, 3 constructors/mappers `unsupported`/`corrupt`/`engine_error`, 2 tests) |
| File verdict | UNREACHABLE (compiled + wired, but no shipped entry point can produce a `dnxhd` track) |
| Reachable from shipped entry point? | NO — `DnxError` exists only inside `DnxDecoder`, which IS compiled (default features: `codec-dnx` in codecs/Cargo.toml default list) and IS wired into the dispatch table (apps/cli/src/pipeline_host.rs:326 `"dnxhd" => … DnxDecoder::new()`), but **zero** demuxer/container/probe code mints the `dnxhd` descriptor (rg over crates/demux, apps/cli/src/containers, media-probe: no hits) and the CLI `open` takes no codec override (parse.rs: `open <path> [--start-time/--start-paused/--no-video/--no-audio]` only). The pipeline_host comment admits it (317-324: "No container lane mints the `dnxhd` descriptor yet"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | full `Read` (271 lines); `rg -n "dnxhd"` over crates/demux/src, apps/cli/src/containers, crates/media-probe/src (zero hits), apps/cli/src/pipeline_host.rs:326 (the wired arm), decoder.rs:265-270 (accepts only `dnxhd`); `rg -n "dnx"` audit/runs/baseline/test-workspace.log.txt (14 hits — unit tests + the differential suite `tests/dnx_differential.rs` 6/6 ok, log line 934-943) |

## 1. One-paragraph verdict
A small, disciplined error taxonomy: three internal variants mapping one-to-one onto public `EngineError` kinds, every engine error re-labeled with a stable dotted code, no panics, and two tests that assert the exact rendered strings and codes (mutation-sensitive, ran in baseline). Nothing here is technically wrong. The problem is upstream of the file: the decoder this taxonomy serves is compiled and dispatch-wired but no shipped code path can ever name it — no container, demuxer, or probe produces the `dnxhd` descriptor, and the CLI offers no way to force one — so every line of this file is dead weight in the shipped binary, kept honest only by the crate's own differential suite. One cosmetic defect: engine-reported truncations render as "expected 0 bytes" because the engine does not signal the required size and the mapper hard-codes 0.

## 2. What actually works (verified)
— `into_engine_error` (69-101): the 3-variant → 3-kind mapping is exact and pinned by `taxonomy_maps_each_variant_to_its_public_kind` (asserts `engine.to_string()`, `.stage`, and the source-chain text for all three variants) — ran ok in baseline (test-workspace.log.txt:547).
— `engine_error` (149-185): total over all five `filmcraft_dnx::Error` variants, each to a stable `codecs.dnx.*` code; the "encoder-side, unreachable from decode" `Input` arm is mapped defensively and documented as such (231-233); pinned by `engine_error_maps_every_engine_variant` incl. kind classification (`Truncated{expected:0, found:10}`, `Unsupported` for unknown CID) — ran ok (log line 546).
— The decoder behind it is differentially proven: `tests/dnx_differential.rs` ran 6 lanes (L1 1080p 175Mb, L2 36M, L3 145M, L4 720p 90M, L5 1080p10 185M + a real-frame truncation refusal), 6/6 ok (log 934-943).

## 3. Findings
### F-001 [S3] The entire taxonomy is unreachable: the `dnxhd` descriptor is minted by nothing in the shipped tree
Where: crates/codecs/src/video/dnx/error.rs:1-271 (the surface) and apps/cli/src/pipeline_host.rs:317-330 (the wired-but-triggerless arm)
Code (pipeline_host.rs:319-324):
```rust
            // headers, no `extra_data` seeding (the ivf/webm lanes;
            // the ogg/opus seeding precedent does not apply).
            // ...
            // No container lane
            // mints the `dnxhd` descriptor yet (the mkv V_MS/FOURCC
            // and ASF futures own that) — the codecs-crate
            // differential is the lane; the decoder's `configure`
```
What is wrong: `DnxDecoder::accepts` is `descriptor == "dnxhd"` (decoder.rs:269-270); the pipeline host arm exists (326-330); but rg finds zero producers of that string in `crates/demux/src`, `apps/cli/src/containers`, and `crates/media-probe/src` — no MKV V_MS/FOURCC mapping, no ASF, no raw-ES `.dnxhd` lane (the ES module handles pcm/mp3/aac/annexb only), and the CLI's `open` has no codec override. So the honest classification is: real, tested, compiled, wired — and dead in every shipped build until a container lane lands.
Why it matters: The binary carries a full VC-3 decoder (engine + wrapper + taxonomy) that no user input can reach; the risk is silent bit-rot of the descriptor contract (e.g. a future demuxer minting `dnxhr` per ffmpeg's newer naming, silently missing this arm's `dnxhd`-only match).
What correct looks like: Either land the smallest container lane (MKV `V_MS/FOURCC` mapping) or gate the dnx module behind a non-default feature until a lane exists, so `--no-default-features` builds measure what ships.
Confidence: HIGH — Verification: read + exhaustive rg (commands above); differential execution cited from baseline.
### F-002 [S4] Engine truncations render "expected 0 bytes" — a misleading diagnostic number
Where: crates/codecs/src/video/dnx/error.rs:151-163
Code:
```rust
        filmcraft_dnx::Error::Truncated => corrupt(
            "codecs.dnx.frame.truncated",
            CorruptDetail::Truncated {
                // The engine reports truncation without the required
                // byte count; 0 = "not signalled by the engine" and
```
What is wrong: `CorruptDetail::Truncated` renders as "truncated data: expected {expected} bytes, found {found}" (proven by the test's exact string at 208), so an engine-side truncation surfaces to users as "expected 0 bytes, found N" — which reads as nonsense ("needed zero bytes") rather than "required size unknown". The in-code comment explains the convention, but the comment does not ship to the user.
Why it matters: Diagnostics that look like bugs generate bug reports; the number 0 has a meaning here that only the source code knows.
What correct looks like: Either `expected: Option<u64>` in `CorruptDetail::Truncated` (rendering "required size unknown"), or a distinct dotted code that renders without the expected clause for the engine arm.
Confidence: HIGH — Verification: read; render format pinned by the sibling test assertion (test-workspace.log.txt:547 ok).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DnxError` enum (3 variants) | 41-65 | REAL (unreachable) | — | doc table maps variants→public kinds; matches `into_engine_error` exactly |
| `DnxError::into_engine_error` | 69-101 | REAL (unreachable) | — | stage Decode constant; FormatInfo name "dnxhd" consistent with the single-descriptor vocabulary (decoder.rs:265-268: ffmpeg's `codec_name=dnxhd` covers DNxHR too); source chain carries `detail` |
| `From<DnxError> for EngineError` | 104-108 | REAL (unreachable) | — | trivial delegation |
| `Detail` + `Display` + `Error` impls | 112-121 | REAL (unreachable) | — | chain-root carrier, 2.B4 pattern |
| `unsupported` | 124-129 | REAL (unreachable) | — | trivial constructor |
| `corrupt` | 132-142 | REAL (unreachable) | — | trivial constructor |
| `engine_error` | 149-185 | REAL (unreachable) | S4 | total over all 5 engine variants; F-002 expected:0; `Input` arm documented unreachable-from-decode |
| `taxonomy_maps_each_variant_to_its_public_kind` | 191-226 | REAL | — | asserts exact Display strings, stage, and source text — mutation-sensitive |
| `engine_error_maps_every_engine_variant` | 228-270 | REAL | — | all 5 engine variants → codes; kind classification pinned (Unsupported for CID, Corrupt::Truncated for truncation) |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `taxonomy_maps_each_variant_to_its_public_kind` (192) | YES | exact rendered strings ("[decode] truncated data: expected 917504 bytes, found 640 (codecs.dnx.frame.short)") + stage + source text — any mapping/Display drift fails |
| `engine_error_maps_every_engine_variant` (229) | YES | every engine variant's code + kind asserted; a missing arm fails compilation (non-exhaustive match) and a wrong code fails the zip |

Both ran in baseline: test-workspace.log.txt:546-547 `ok`.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Nothing panics; every constructor is total; no `unwrap` on any input path" | error.rs:25-26 | REAL | read of all 9 functions — zero unwrap/expect/panic tokens |
| "a damaged packet poisons at most itself (the MJPEG statelessness guarantee; the admission record's hostile battery measured zero panics)" | error.rs:18-23 | REAL-UNPROVEN here | the battery is the admission record's (docs/dependencies/records/filmcraft-dnx-0.9.0.md) — not re-run in baseline; the differential's truncation lane (6/6 ok) covers one slice of it |
| matrix row for DNxHD decode (`codec-video-dnxhd*`) | docs/feature-matrix.toml | not re-checked by me (other agent's row range) | — |

## 7. Dead / duplicated / unreachable code
- The whole file is unreachable in shipped builds (F-001) — kept honest by tests only.
- No duplication: the taxonomy follows the 6.M-opus/mjpeg `FeatureDetail` pattern consistently (stated at 110-111; consistent with mjpeg's error module which I audited as part of decoder.rs context).

## 8. Dependencies & flags
- `filmcraft-dnx = { version = "=0.9.0", optional = true, default-features = false }` (codecs/Cargo.toml:158) behind `codec-dnx`, which IS in `default` (line 56 area) → compiled into every shipped build; the pin's `default-features = false` keeps the graph rayon-free (record's binding condition — verified against the Cargo.toml audit's lock check).
- Wave-6 hostile question: dnx is the counter-example to the H.264 story — a decoder that IS default-enabled and dispatch-wired yet unreachable for lack of a descriptor producer. H.264 sw decode is reachable (pipeline_host.rs:346 + containers minting `h264`/`avc1`); dnx is not (no producer). Full H.264 trace: mjpeg/decoder.rs#part2.audit.md §8.

## 9. Open questions for the next agent
- decoder.rs owner (W06-a?? has dnx/decoder.rs#part1/2): the CBR gate referenced at 156-159 ("the wrapper's CBR gate covers the declared-size window with exact numbers") — confirm the gate's expected-size arithmetic (SMPTE ST 2019-1 CID table lookups) is engine-independent.
- Whoever lands the MKV/ASF lane: the descriptor minting must use exactly `dnxhd` (single spelling) or this arm stays dead.
