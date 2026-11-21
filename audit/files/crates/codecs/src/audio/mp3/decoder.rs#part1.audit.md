# AUDIT: crates/codecs/src/audio/mp3/decoder.rs#part1 (lines 1-428)
| Field | Value |
|---|---|
| Auditor | W07-A04 |
| Date | 2026-11-21 |
| Lines / functions | 428 (of 828; chunk part1 of 2 — boundary at `impl Mp3Decoder` line 429; part2 = 429-828, same agent) / 9 units: `DecodedMp3` (struct), `codec_id`, `LAYER3_PIPELINE_LATENCY` (const), `decode_one`, `decode_stream`, `facts` module (`encode`/`decode`), `Mp3Decoder` (struct), `Debug::fmt`, `SAMPLE_SLOTS`/`MAX_SAMPLES_PER_FRAME` (consts) |
| File verdict | REAL for the free surface (decode_stream executed: bitexact + corpus differentials + fuzz, all green in baseline); the contract `Decoder` impl (part2) carries two S2 defects — see part2 audit |
| Reachable from shipped entry point? | SPLIT. `decode_stream`/`facts`: NOT from playback — callers are tests (`tests/mp3*.rs`), the fuzz harness (`mp3/fuzz.rs:131`), a bench, and a doc example (verified by `grep -rln decode_stream`); the shipped lane reaches only `Mp3Decoder::{configure,decode}` (part2) via `apps/cli/src/pipeline_host.rs:261-268` `"mp3" \| "mp1"` / `:266 "mp2"` ← descriptors minted by `demux/src/es/mp3.rs` (raw .mp3), `demux/src/mp4/mod.rs` (`.mp3` stsd + esds OTI 0x6b quirk), `demux/src/ts/mod.rs` (0x03/0x04), `demux/src/mkv/header.rs` (A_MPEG/L2/L3). Executed in baseline: `es_mp3_family_lane ... ok`, `es_mp2_family_lane ... ok`, `mp3_start_time_watermark_is_exact ... ok` (orchrun-test-workspace-nofailfast.log.txt:2943, :2793; container_conformance.rs:700-744). |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:1 |
| Commands run | Read 1-428 (full) + 429-828 (part2 context); function map rg; Read symphonia-bundle-mp3-0.6.1/src/decoder.rs from the cargo registry cache (/tmp/my-project/.cargo/registry — MpaDecoder source: mid-stream spec change = typed decode_error, frame_size must match packet length); Read tests/mp3/mod.rs + mp3_corpus_differential.rs + reference data; baseline greps (:409-415 unit, :1159-1180 differential/fuzz, orchrun :2793,2943 lanes); `grep -rln Mp3Decoder` (users: mp2/mod.rs re-export, fuzz.rs, pipeline_host.rs, apps/web/src/host.rs); cross-read audit/files/.../mp3/xing.rs.audit.md (W07 sibling already proved the facts blob has no producer); sed verification of every quoted range. |

## 1. One-paragraph verdict
Part1 is the honest half: `decode_one` is a clean one-frame engine seam (typed refusals for reserved layer, codec switch, engine rejection; I verified against the actual symphonia 0.6.1 source that the wrapper's implicit "spec never changes mid-stream" assumption is engine-enforced, so the shared-scratch slicing cannot go out of bounds), and `decode_stream` is a real ID3→walk→Xing/VBRI→decode→gapless pipeline whose LAME window arithmetic I re-derived by hand and whose output is pinned bit-exact (±1 LSB) against ffmpeg by the recorded-window bitexact suite that ran green in baseline. The defects: VBRI's encoder-delay field is parsed into `VbriInfo` and then thrown away by the gapless window (LAME-only), and the two gaps already recorded by the xing.rs sibling audit (facts blob unwired in the shipped lane; VBRI truncation panic in xing.rs — not my file) mean everything good here runs only in test lanes.

## 2. What actually works (verified)
— `decode_one` (126-176): header parse → layer gate → codec-switch gate → engine decode → sized interleaved copy. Panic-safety of `scratch[..need]`: symphonia's MpaDecoder rejects any packet whose signal spec (rate+channels) differs from the first frame's (`if self.buf.spec() != &header.spec() { return decode_error(...) }`, symphonia decoder.rs:83-86) and requires exact frame-size packets (`header.frame_size != reader.bytes_available()` → error, :75-76) — so the in-house wrapper's assumptions hold by engine contract, verified in the vendored source.
— Gapless window arithmetic (310-325) re-derived: window raw = `[delay, total−padding)` (xing.rs `trim_window`), shifted `+529`, end clamped to decoded total; contract-path twin (part2) drops `padding−529` from the last frame — same math. Pinned by `gapless_window_applies_the_pipeline_latency_shift` (452 samples = [629,1081)) and `gapless_window_clamps_when_padding_undercuts_the_latency` (573 = [579,1152)), both ok in baseline (:413-414) — I re-computed both by hand.
— Bit-exactness evidence: `mp3_bitexact::fixtures_decode_the_recorded_window_exactly ... ok` (0.69 s real decode, baseline :1159-1164) and `tagged_art_is_cbr128_behind_a_tag ... ok`; `mp3_corpus_differential::corpus_and_fixture_differential_vs_ffmpeg ... ok` + `vbr_toc_seek_accuracy ... ok` (:1167-1173) — note the corpus suite finished in 0.06 s, consistent with either a warm-cache run or partial SKIP rows; the recorded-window bitexact run is the load-bearing evidence.
— Fuzz survival: `mp3_fuzz_survival` 2/2 ok (:1175-1180), and `mp3/fuzz.rs` drives BOTH `decode_stream` and the contract `Mp3Decoder::decode` (fuzz.rs:131-179) — panic-freedom exercised, values not (see part2 F-003).
— Xing consistency gate (238-252): claims-vs-walked count mismatch → typed `mp3.xing.frame_count_mismatch`; pinned by unit test (:415 ok).
— `facts::encode/decode` (350-368): trivial, round-trip + bad-length pinned (:412 ok).
— Contract lane executed end-to-end in the shipped binary: `es_mp3_family_lane`/`es_mp2_family_lane` ok (orchrun :2943, :2793) — full demux→pipeline_host→Mp3Decoder::decode chain.

## 3. Findings
### F-001 [S3]
Where: crates/codecs/src/audio/mp3/decoder.rs:314-325 (window source) + 96-97 (`vbri` surfaced) + xing.rs:103 (delay field)
Code:
```rust
    let gapless = xing.as_ref().and_then(|x| x.lame);
    if let Some(lame) = gapless {
```
What is wrong: the gapless window consults ONLY the LAME tag. A VBRI header (Fraunhofer encoders) carries its own encoder delay (`VbriInfo.delay`, parsed and returned in `DecodedMp3.vbri`), but a VBRI-bearing stream gets NO trim at all — not the delay, not the 529-sample shift — while a LAME-bearing stream gets both.
Why it matters: VBRI and LAME describe the same pipeline reality; a VBRI MP3 decoded by `decode_stream` keeps its encoder-delay silence AND the 529 priming samples, so the differential's "same window as ffmpeg" premise silently fails for VBRI files (none are in the fixture set — cbr128/vbr_q4/mono48/mpeg2_24k/mp2_48k/tagged_art are all Xing/LAME or plain). Blast radius: rare files, test/bench lane only (the shipped lane never applies ANY gapless facts — xing.rs.audit.md F-002).
What correct looks like: `let gapless = xing.and_then(lame).or_else(|| vbri.map(|v| LameTag { delay: v.delay, padding: 0, .. }))` (or an explicit documented refusal), plus one VBRI fixture in the differential.
Confidence: HIGH — Verification: read (caller/data-flow traced); no VBRI fixture exists to execute it.

### F-002 [S4]
Where: crates/codecs/src/audio/mp3/decoder.rs:199-207
Code:
```rust
            "mp3.stream.no_frames",
            media_types::error::CorruptDetail::Truncated {
                expected: 4,
                found: (data.len() - pos.min(data.len())) as u64,
            },
```
What is wrong: `found` clamps `pos` to `data.len()`, so when a corrupt ID3 size pushes `pos` past the end the reported `found` is 0 regardless of the real tail length; and `pos.min(...)` papers over `parse_id3v2` having accepted a tag whose declared length exceeds the buffer (if it does — id3.rs is a sibling file).
Why it matters: diagnostics-only imprecision in an error path; the guard arithmetic hides whether the demux-level invariant "tag fits the buffer" is actually upheld upstream.
What correct looks like: assert/propagate `pos <= data.len()` from the ID3 parse and report the true remaining-bytes figure.
Confidence: MEDIUM — Verification: read; id3.rs behavior on oversize tags not audited by me (sibling file).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `DecodedMp3` (struct) | 85-102 | REAL | — | plain result carrier; `frames` excludes the Xing frame (documented) |
| `codec_id` | 105-112 | REAL | — | L1/L2/L3 → engine ids; Reserved → None (gated at both call sites) |
| `LAYER3_PIPELINE_LATENCY` | 114-118 | REAL | — | empirically pinned constant (module doc); misapplication to L1/L2 is part2 F-001, not the constant's fault |
| `decode_one` | 126-176 | REAL | — | typed refusals (layer_reserved, codec_switch, engine rejection); `let _ = rate;` at 168 is safe — engine pins spec (verified in symphonia source); scratch reuse correct (resize-only-grow, exact prefix read) |
| `decode_stream` | 188-341 | REAL | S3 (F-001) | whole-file surface; typed gates (no_frames, header_frame_only, xing count mismatch, layer_reserved, engine init, no_decoded_samples); accumulates whole stream in memory — fine for its test/bench role, not shipped; VBRI delay ignored (F-001) |
| `facts::encode` | 350-356 | REAL | — | 12-byte LE triple; `#[must_use]` |
| `facts::decode` | 360-368 | REAL | — | exact-length gate, `None` on wrong length; round-trip pinned |
| `Mp3Decoder` (struct) | 381-407 | REAL | — | state fields all consumed by part2's impl |
| `Debug for Mp3Decoder` | 409-420 | REAL | — | honest `finish_non_exhaustive` (engine not Debug) |
| `SAMPLE_SLOTS` / `MAX_SAMPLES_PER_FRAME` | 423-427 | REAL | — | 1152 covers L1 384 / L2 1152 / L3 1152|576 — correct bound |

## 5. Test honesty (tests for this part live in part2's range and integration suites)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `gapless_window_applies_the_pipeline_latency_shift` / `..._clamps_...` (decoder.rs:794-827) | YES | exact sample-count expectations (452/573 per channel) hand-verified by me; any shift/trim regression trips them |
| `xing_frame_count_inconsistency_is_typed` | YES | code+kind+stage asserted on a constructed lying Xing header |
| `mp3_bitexact::fixtures_decode_the_recorded_window_exactly` | YES | recorded per-fixture MD5 + sample counts (reference file); decode actually ran (0.69 s) |
| `mp3_corpus_differential::corpus_and_fixture_differential_vs_ffmpeg` | PARTIALLY | live-oracle rows SKIP-with-message when corpus/ffmpeg absent (0.06 s runtime in baseline suggests partial SKIP — the recorded-window bitexact suite is the reliable evidence); md5-drift guard makes corpus rows loud, not silent |
| `mp3_fuzz_survival` (2 tests) | YES (for panics) | mutated fixtures + dirty pools through `decode_stream` and the contract path; asserts typed outcomes, not values |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "bit-exact to ±1 LSB once aligned (Layer II/MP2 … aligns at shift 0)" | decoder.rs:59-60 (module doc) | REAL for decode_stream (mp2_48k: 72576 = 63×1152 exact, reference line 86) — and the same doc is CONTRADICTED by part2's unconditional 529 trim for `mp2`/`mp1` (part2 F-001) |
| "differential tests … compare against ffmpeg on the SAME gapless window … see tests/mp3_bitexact.rs" | decoder.rs:40-46 | REAL | baseline :1159-1164 ok (recorded windows); live-oracle corpus row ok but possibly partially SKIPped (:1167-1173, 0.06 s) |
| "One packet = one complete frame … the engine hard-requires the match" | decoder.rs:20-22 | REAL | symphonia decoder.rs:75-76 (`header.frame_size != reader.bytes_available()` → decode_error) — verified in engine source |
| "Damage is never a panic" (decode_stream doc) | decoder.rs:185-186 | REAL-UNPROVEN for VBRI specifically | xing.rs F-001 (sibling audit) found a VBRI truncation panic reachable from `decode_stream`; the doc's promise is false until that lands — not my file, cross-cited |

## 7. Dead / duplicated / unreachable code
- `facts::encode`: zero non-test callers (proven by the xing.rs sibling audit) — the 12-byte facts blob has NO producer in the tree; the entire gapless-facts channel of the contract decoder is dead-in-practice (see part2).
- `DecodedMp3.vbri`/`VbriInfo.delay`: parsed, surfaced, never consumed by any trim decision (F-001).
- No duplicated logic beyond the deliberate free-surface/contract-path split (both call `decode_one`).

## 8. Dependencies & flags
- `symphonia-bundle-mp3` =0.6.1 + `symphonia-core` =0.6.1 behind `codec-mp3` (codecs/Cargo.toml:78), in `default` (:55) — shipped CLI builds it. Registry source inspected at /tmp/my-project/.cargo/registry (read-only) for the engine-contract verification in §2.
- `mp2` module re-uses this decoder via `codec-mp2 = ["codec-mp3"]` — one engine, two decoders, no duplication.

## 9. Open questions for the next agent
- part2 (my own, next): the two S2s — unconditional 529 trim for mp1/mp2 and the pts timeline hole.
- Tagless-MP3 alignment: RESOLVED by an in-session measurement (audit/runs/tmp-w07a04/EVIDENCE.md, ffmpeg 7.1.5): a tagless mp3 decodes under ffmpeg to exactly frames×1152 samples with the source's sample 0 at output index 1105 (576+529 delay head retained) — the reference applies NO head skip without a LAME tag, so `decode_stream`'s no-trim behavior is correct for tagless files and the contract decoder's unconditional 529 trim (part2 F-001) misaligns every shipped MP3/MP2/MP1 playback by 529 samples.
- id3.rs owner: does `parse_id3v2` accept a tag whose `total_len` exceeds the buffer? decode_stream feeds `pos = tag.total_len` into `walk_frames` unclamped (F-002).
