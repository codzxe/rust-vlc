# AUDIT: crates/codecs/src/audio/opus/decoder.rs
| Field | Value |
|---|---|
| Auditor | W07-A07 |
| Date | 2026-11-16 |
| Lines / functions | 852 / 17 (+8 tests, 3 test helpers) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `lumen-cli`: `apps/cli/src/pipeline_host.rs:281 self.seeded(track, config, OpusDecoder::new)` (pipeline_host is `pub mod` of apps/cli/src/lib.rs:76, the CLI's decode pipeline); also `apps/web/src/host.rs:647 codecs::audio::opus::OpusDecoder::new()` |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `rg -n "OpusDecoder" crates apps` (exit 0); `rg -n "ruopus" Cargo.lock` (exit 0, lock:763-765 v0.1.2); `rg -n opus audit/runs/baseline/cargo-test-workspace.txt` (exit 0, 5 unit tests ok + corpus differential "2 passed ... 0.01s"); `test -d /tmp/my-project/corpus-v4` -> ABSENT; sed quote verification (all quotes re-printed) |

## 1. One-paragraph verdict
This is a REAL decoder: the Opus engine is **`ruopus` 0.1.2** (NOT symphonia — symphonia indeed has no Opus; `crates/codecs/Cargo.toml` gates it as `codec-opus = ["dep:ruopus"]`, default-on, exact-pinned, `default-features = false, features = ["std"]`, Cargo.lock:763-765). `configure` parses/validates the OpusHead before any engine is built (panic window closed), `decode` runs the real engine, applies the RFC 7845 pre-skip/end-trim window, the seek re-anchor rule, the channel permutation and the Q7.8 output gain, and writes into a pooled 4-slot rotation. Unit tests ran green in the orchestrator baseline. The one real defect is inherited from its sibling `order.rs`: the 4-channel Vorbis→house permutation row contradicts RFC 7845 §5.1.1 and swaps FR↔BL on 4-channel (quadraphonic) family-1 streams — a lane no test covers, in a file whose own header admits the 3/4/5/7/8 rows are untested.

## 2. What actually works (verified)
- Verified by `rg -n "opus" audit/runs/baseline/cargo-test-workspace.txt` -> all 5 decoder unit tests `ok` (incl. `seek_landing_is_missing_until_the_granule_reanchors`, `hostile_packets_never_panic_and_isolate_to_the_packet`), plus head/order/error unit tests ok (lines 543-558), plus a doc-test at decoder.rs:165 ok (line 1409).
- Verified by reading `ruopus-0.1.2/src/multistream.rs:64-106` in the cargo registry cache: `decode_packet` parses self-delimited elementary streams, checks per-stream duration equality, and routes via the mapping table — the output order is the container's Vorbis order exactly as this wrapper assumes.
- Verified the Ogg seam: `crates/demux/src/ogg/mod.rs:204-207` builds `Clock::Opus48000 => TimeBase { num: 1, den: 48_000 }`, and `TimeBase::new(1, 48_000)` reduces by gcd=1 to the identical struct (media-types/src/time.rs:77-89), so decoder.rs:342's `ts.base == base` granule match genuinely fires for Ogg Opus packets.

## 3. Findings
F-001 [S2]
Where: crates/codecs/src/audio/opus/decoder.rs:440-443 (consumer), crates/codecs/src/audio/opus/order.rs:40-41 (defect)
Code:
```
                    let source = match &permutation {
                        Some(perm) => usize::from(perm[channel]),
                        None => channel,
                    };
```
with the defective row it consumes (order.rs:40-41, verbatim):
```
        // 4: Vorbis FL FC FR BC → house FL FR FC BC.
        4 => Some(&[0, 2, 1, 3]),
```
What is wrong: RFC 7845 §5.1.1 (by reference to the Vorbis I spec) defines the 4-channel Vorbis order as **quadraphonic FL FR BL BR**, not "FL FC FR BC". The house/WAV 4-channel order (WAVEFORMATEXTENSIBLE QUAD, ffmpeg's native default) is also FL FR BL BR — so the correct permutation is the identity `[0,1,2,3]`; the shipped `[0,2,1,3]` emits `out FR = in BL` and `out BL = in FR`: front-right and back-left audio are swapped on every 4-channel mapping-family-1 stream. Rows 3, 5, 6, 7, 8 check out against the spec (I recomputed all of them); only the 4-row is wrong. Why it matters: quadraphonic Opus is a legal, encodable configuration (any `-af aformat=channel_layouts=quad` ffmpeg encode); the output is silently channel-scrambled with no error. What correct looks like: `4 => Some(&[0, 1, 2, 3])` (or `[0,1,2,3]` with a comment naming quadraphonic). Confidence: HIGH — Verification: read (spec table + ruopus multistream mapping semantics confirmed order passthrough); UNVERIFIED: no runnable 4-channel corpus lane exists (F-003) to execute the swap end-to-end.
Note for orchestrator: the defect lives in order.rs (not in my assignment list); decoder.rs inherits it at its output seam, hence logged here.

F-002 [S3]
Where: crates/codecs/src/audio/opus/decoder.rs:833-835
Code:
```
            match outcome {
                Ok(_) | Err(_) => {}
            }
```
What is wrong: this match accepts every possible outcome — it is dead code asserting nothing. The hostile-barrage test's only real checks are (a) `catch_unwind` no-panic and (b) the final post-barrage decode `is_ok()`. The name promises "isolate to the packet": if all six hostile cases returned typed Corrupt, or if the decoder had emitted garbage frames instead, the loop would still pass. A wrong-but-non-panicking hostile handling cannot fail this test. What correct looks like: assert each case is `Ok(NeedMoreInput) | Ok(Emitted..) | Err(Decoder-corrupt)` explicitly, and assert the post-barrage output is the expected DTX concealment (window > 0, non-trivial samples), not merely `is_ok`. Confidence: HIGH — Verification: read; test ran ok in baseline (cargo-test-workspace.txt:546).

F-003 [S3]
Where: crates/codecs/tests/opus_corpus_differential.rs:49-52, 95-98 (evidence path for this decoder)
Code:
```
    let Some(root) = corpus_dir() else {
        eprintln!("SKIP {lane}: no corpus directory (LUMEN_CORPUS unset, /tmp default absent)");
        return;
    };
```
What is wrong: the entire differential suite (geometry pins, pre-skip 312 / end-trim 648 exactness, the 5.1 channel-order assertion that order.rs's 6-row "corpus-verified" claim rests on) silently skips when the corpus is absent — and it IS absent: `test -d /tmp/my-project/corpus-v4` -> ABSENT, `LUMEN_CORPUS` unset. The orchestrator baseline shows `opus_corpus_differential ... 2 passed; ... finished in 0.01s` — 0.01s is the SKIP path (201 packets × ogg-demux + engine decode + ffmpeg oracle cannot run in 10ms); the "pass" proves nothing about decode output in this environment. The claimed evidence (max|Δ| 4.381e-06, 0/1,152,000) is recorded from a machine that no longer has the corpus. What correct looks like: distinguish SKIP from PASS (`assert!(corpus_dir().is_some(), "corpus required")` or a counted skip) so the baseline cannot record vacuous greens. Confidence: HIGH — Verification: ran `test -d` + read baseline log lines 1290-1296.

F-004 [S4]
Where: crates/codecs/src/audio/opus/decoder.rs:328-331 and 425
Code:
```
        let head = match self.head.clone() {
            Some(head) => head,
            None => return Err(OpusError::BadState.into_engine_error()),
        };
```
(and `let permutation = self.permutation.clone();` at 425)
What is wrong: per-packet clones. `OpusHead` carries `mapping: Vec<u8>` and the permutation is a `Vec<u8>` — every decoded packet performs one or two tiny heap allocations for family≥1 streams (family 0's empty Vecs avoid the alloc). Hot path, per frame, forever. What correct looks like: borrow via `as_ref()` lifetime split or store the permutation as `Option<[u8; 8]> + len`. Confidence: HIGH — Verification: read. Severity minor: allocation is small; measured impact UNVERIFIED (cargo-build-budget).

F-005 [S4]
Where: crates/codecs/src/audio/opus/decoder.rs:56-61 (right formula) vs crates/codecs/src/audio/opus/head.rs:50-52 (wrong formula)
Code (head.rs:50-52, verbatim):
```
    /// Output gain, Q7.8 dB-scaled linear gain (RFC 7845
    /// §4.2): the decoder applies `sample × gain / 256`. Every corpus lane
```
What is wrong: head.rs documents `sample × gain / 256` — a linear fractional reading that contradicts both RFC 7845 §4.2 (output gain is Q7.8 **dB**) and what decoder.rs:307 actually implements (`10.0f32.powf(f32::from(head.output_gain) / 5120.0)`). The implementation is correct; the head.rs comment is false and would mislead a maintainer into "fixing" the decoder to match it. The gain≠0 branch itself is admittedly untested (decoder.rs:59-61: "recorded as an untested honest gap"). What correct looks like: head.rs comment should read `sample × 10^(q/5120)`. Confidence: HIGH — Verification: read both sites.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Engine::decode` | 110-120 | REAL | — | thin dispatch Mono/Multi -> ruopus `decode_packet` |
| `build_engine` | 124-143 | REAL | — | family 0 -> `RuopusMono::with_rate(48000, ch)`; else multistream with head's streams/coupled/mapping; panic window pre-closed by parse_head (baseline: head tests ok) |
| `max_output_len` | 147-149 | REAL | — | 5760 × channels × 4; RFC 6716 120 ms ceiling; correct |
| `OpusDecoder::new` | 231-233 | REAL | — | Default derive |
| `OpusDecoder::accepts` | 237-239 | REAL | — | strict `"opus"` |
| `output_format` | 244-251 | REAL | — | F32/Interleaved/48 kHz/channels |
| `pre_skip` | 256-258 | REAL | — | 0 before configure |
| `decoded_total` | 263-265 | REAL | — | raw counter accessor |
| `Decoder::configure` | 269-314 | REAL | — | head gate + engine build + permutation + gain + state reset; baseline-tested |
| `Decoder::decode` | 316-488 | REAL | S2 via F-001 | granule/anchor/window math recomputed correct; permutation row 4 defective (cross-file); stale guard via codec-api `drop_stale_packet` |
| `Decoder::flush` | 490-497 | REAL | — | Drained, nothing buffered — correct for Opus (independently coded packets) |
| `PooledWindow` | 502-504 | REAL | — | newtype |
| `pooled_sample_slot` | 510-547 | REAL | — | 4-slot rotation, exclusive-ownership reuse, typed slot-size guard (431-437) — sound |
| test `descriptor_gate...` | 582-588 | REAL | — | would fail on regression |
| test `seek_landing...` | 653-723 | REAL | — | pins the anchor rule numerically (granule−window−312); ran ok in baseline |
| test `configure_requires...` | 725-753 | REAL | — | 3-channel family-0 refusal + format negotiation |
| test `decode_before_configure...` | 755-783 | REAL | — | typed BadState |
| test `hostile_packets...` | 785-851 | SHALLOW | S3 | no-panic only; outcome unchecked (F-002) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| seek_landing_is_missing_until_the_granule_reanchors | YES | exact pts arithmetic pinned (granule−window−312); mutation of window/anchor math moves pts |
| configure_requires_and_validates_the_head | YES | error codes + negotiated format asserted |
| decode_before_configure_is_typed_bad_state | YES | asserts code + ErrorKind |
| hostile_packets_never_panic_and_isolate_to_the_packet | NO (partially) | catches panics, but any Ok/Err mix passes the loop (F-002); final assertion is `is_ok()` only |
| corpus_0047_mono / corpus_0077_surround_51 (tests/opus_corpus_differential.rs) | NOT IN THIS ENV | corpus absent → silent SKIP; baseline green is vacuous (F-003) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Opus decode engine = ruopus 0.1.2, default-features off, std only" | decoder.rs:7-8; codecs/Cargo.toml `ruopus = { version = "=0.1.2", ... }` | REAL | Cargo.lock:763-765 `name = "ruopus" / version = "0.1.2"`; registry src read |
| "measured 312 [pre-skip] on every corpus lane" | decoder.rs:29-31 | REAL-UNPROVEN (here) | corpus ABSENT; numbers only re-runnable with /tmp/my-project/corpus-v4 restored |
| "Mapping family 1 output is permuted from ruopus's RFC 7845 Vorbis order into the house WAV order" | decoder.rs:47-50 | FALSE for 4 channels | order.rs:41 `[0,2,1,3]` ≠ RFC 7845 §5.1.1 quad FL FR BL BR → identity (F-001) |
| "Every corpus lane measures gain = 0 (unity)" | decoder.rs:59-60 | REAL-UNPROVEN (here) | corpus absent; gain≠0 path untested by own admission |
| HEVC-variant error text claims "landed: ... opus ..." (engine-info says decoders: mjpeg, pcm_s16le) | baseline mp4-smoke.md | registry-side issue | this decoder is real and wired (pipeline_host.rs:281), so the CLI's registry assembly, not this file, is where the claim diverges — flagged to the registry owner (W-something) |

## 7. Dead / duplicated / unreachable code
- `match outcome { Ok(_) | Err(_) => {} }` (833-835) — dead assertion (F-002).
- `TimeBase::new(1, DECODE_RATE).map_err(...)` (332-335) — unreachable error arm by the map's own comment; harmless, honestly documented.
- No `#[ignore]`, no feature-gated code beyond the crate-level `codec-opus` gate (default-on; release lane enables it — reachable).

## 8. Dependencies & flags
- `ruopus =0.1.2` (optional, `codec-opus` feature, in `default` → shipped). Lock-pinned. No symphonia involvement — the hostile-focus hypothesis "is it a symphonia wrapper?" is answered: no, ruopus.
- `codec-api` (Decoder, drop_stale_packet), `media-types` (frame/packet/time). No unused imports observed.

## 9. Open questions for the next agent
- order.rs owner: fix row 4 to identity (or run a real 4.0 corpus lane to falsify me).
- head.rs owner: the `sample × gain / 256` comment (head.rs:50-52) is false vs the implemented `10^(q/5120)` — fix the doc.
- Whoever owns the corpus: restore /tmp/my-project/corpus-v4 or make the differential tests fail loudly when absent — the baseline's vacuous green is an evidence hazard for every codec lane, not just Opus.
