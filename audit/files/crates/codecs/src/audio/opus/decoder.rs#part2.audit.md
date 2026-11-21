# AUDIT: crates/codecs/src/audio/opus/decoder.rs#part2 (lines 427-853)
| Field | Value |
|---|---|
| Auditor | W07-A07 |
| Date | 2026-11-21 |
| Lines / functions | 427-853 of 853 / 15 items (decode() tail incl. fill closure + emission, flush, PooledWindow, pooled_sample_slot, 11 test items) |
| Chunk boundary | part2 = 427-853 (midpoint 426.5). `decode()` spans 316-488: its first half (granule capture, anchor arithmetic, window math, lines 341-426) is W07-A01's part1; I read the whole function for context, verified the arithmetic my half depends on, and attribute first-half findings to A01. Boundary note recorded for cross-agent dedup. |
| File verdict | REAL (part2 code executed in baseline via 5 real unit tests) |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → apps/cli/src/pipeline_host.rs:276-282 `HostDecoderProvider::decoder_for` `"opus" => self.seeded(track, config, OpusDecoder::new)`; opus tracks minted by demux/src/ogg/mod.rs:153, demux/src/mkv/header.rs:810, demux/src/mp4/mod.rs:453). No baseline smoke run opens an opus sample (baseline samples: MP4/AVI only) → shipped-binary lane UNPROVEN; unit lane REAL (baseline test log lines 446-449, 514: 5/5 opus decoder tests ok). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 2 |
| Commands run | `sed -n '131,430p'`+`431,853p` opus/decoder.rs (full read) · `sed -n '1,92p'` opus/order.rs (permutation contract `out[i] = decoded[PERM[ch][i]]`) · `rg -n '"opus"' crates/demux/src` (track sources) · `sed -n '1,140p'` tests/opus_corpus_differential.rs + `sed -n '28,35p'` tests/opus/mod.rs (corpus_dir: `LUMEN_CORPUS` else `/tmp/my-project/corpus-v4`, else SKIP) · baseline test-workspace.log.txt:1192-1197 (corpus tests "2 passed ... finished in 0.01s" — vacuous-skip timing proof, see F-001) + :446-449,:514 (unit tests ok) · `grep -n 'corpus\|SKIP'` baseline log (no SKIP lines captured; libtest swallows stdout on pass) · `which ffmpeg` → /usr/bin/ffmpeg (present) — used only for the timing argument |

## 1. One-paragraph verdict
The decode tail is correct and fast: the window copy applies the channel permutation in the right direction (hand-verified against order.rs's `out[i] = decoded[PERM[ch][i]]` contract for the 5.1 row), the output-gain formula `10^(q/5120)` is right, all indexing is provably in-bounds (window ≤ packet_samples ≤ slot), and the pooled-slot rotation is the proven Vorbis pattern. The unit tests are real — the seek-reanchor test decodes a verbatim 95-byte fixture packet and pins exact pts arithmetic, and the hostile barrage uses `catch_unwind` with 6 cases; all 5 ran green in the baseline. The one substantive defect is evidentiary: the matrix row's only differential evidence ("opus_corpus_differential 2/2 pass") is a **vacuous pass** — the corpus tests SKIP silently when `/tmp/my-project/corpus-v4` is absent, the corpus media is not committed to the repo, and the baseline log's 0.01s wall time for both tests (with ffmpeg demonstrably present, making a real differential ≥100× slower) proves they took the skip path. The "corpus-verified" pre-skip/end-trim/5.1-order claims are therefore unauditable from the repo by any third party.

## 2. What actually works (verified)
- 5/5 opus decoder unit tests executed in baseline — `audit/runs/baseline/test-workspace.log.txt` lines 446-449 and 514 (`configure_requires_and_validates_the_head`, `decode_before_configure_is_typed_bad_state`, `descriptor_gate_accepts_only_opus`, `hostile_packets_never_panic_and_isolate_to_the_packet`, `seek_landing_is_missing_until_the_granule_reanchors` — all `ok`).
- Permutation direction at my call site (lines 440-451) — hand-checked against order.rs:29-30 contract: `dst[frame,target] = decoded[perm[target]]`; for 5.1 perm `[0,2,1,5,3,4]`: out FR = decoded[2] (Vorbis FR), out LFE = decoded[5] (Vorbis LFE) ✓ — the inverse-direction trap is not present.
- Gain formula (part1 line 307, consumed by my fill at 444-448): `10.0f32.powf(q / 5120.0)` = 10^(q/(20·256)) = Q7.8 dB → linear ✓; `gain == 1.0` exact short-circuit is safe (powf(10,0)=1.0 exactly).
- Bounds safety of the copy loop: `scratch_offset + frame*channels + source` with `scratch_offset/window ≤ packet_samples`, `source < channels` (bijection per order.rs test) — no panic path; slot-size runtime check (431-437) backs the `debug_assert`.
- `flush` (490-497): immediately `Drained` — correct for per-packet-coded Opus (no concealment tail; the valid tail was already granule-trimmed in decode).

## 3. Findings

### F-001 [S2]
Where: evidence defect — tests/opus_corpus_differential.rs:49-52 (skip path) + docs/feature-matrix.toml:747 (evidence string); touches decoder.rs module docs (lines 30-31, 59-61, part1) and opus/order.rs:22-26
Code:
```rust
    let Some(root) = corpus_dir() else {
        eprintln!("SKIP {lane}: no corpus directory (LUMEN_CORPUS unset, /tmp default absent)");
        return;
    };
```
What is wrong: the corpus differential tests — the ONLY end-to-end ogg→OpusDecoder composition and differential assertions (geometry: pre-skip 312 + end-trim 648 exactness; channel order via the 5.1 tolerance assertion; s16-LSB equivalence) — silently return before every assertion when the corpus directory is absent. The corpus media (`0047-audio-only.ogg`, `0077-audio-only.ogg`) is **not committed to the repo** (it is `/tmp`-resident per tests/opus/mod.rs:25-27). In the committed baseline run the two tests report `ok` in a combined **0.01s** (test-workspace.log.txt:1192-1197) — with ffmpeg present (a real differential spawns ≥2 ffmpeg subprocesses and compares 768k samples; two 201-packet decodes alone approach the whole budget), proving both took the skip path. The matrix row `codec-audio-opus` cites exactly this as its evidence: "opus_corpus_differential 2/2 pass in the codecs suite".
Why it matters: a "2/2 pass" that a fresh `cargo test` reproduces with a **broken** Opus decoder (the skip is unconditional on corpus absence) is evidence that proves nothing — and it is the string backing the `implemented` status of a P0 row. Every "corpus lane measured" claim in the module docs (pre-skip 312, end-trim 648, gain 0, 5.1 order) is unauditable from the repo. Notably, the Vorbis suite names its equivalent test `corpus_0085_differential_or_skip` (honest naming); the Opus tests carry no skip hint in their names.
What correct looks like: commit the two corpus lanes as fixtures (they are ~200 KB of ogg), or gate the matrix evidence on a reproducible run; at minimum rename the tests `_or_skip` and change the evidence string to name the environment the 2/2 ran in.
Confidence: HIGH for the vacuous baseline pass (timing + path-presence proof); UNVERIFIED whether the admission-time run (2026-11-09, the numbers in the record) had the corpus present — the recorded maxΔ values are specific and plausible, but no committed artifact can re-derive them.

### F-002 [S4]
Where: crates/codecs/src/audio/opus/decoder.rs:833-835
Code:
```rust
            match outcome {
                Ok(_) | Err(_) => {}
            }
```
What is wrong: this match accepts every possible outcome and does nothing — it is dead assertion code dressed as a check. The test's doc comment claims "every case is an Ok (consumed, possibly zero-window) or a typed Corrupt", but no assertion distinguishes those (a wrong-but-non-panicking outcome — e.g. a successful decode of garbage into silence, or an error of the wrong taxonomy — passes).
Why it matters: the line reads as if outcomes were validated; a reader trusts more than was tested. The test's real assertions (catch_unwind + post-barrage survival) are sound.
What correct looks like: assert `outcome.is_ok() || matches!(outcome, Err(ref e) if e.kind is the typed Corrupt)` with the error code, or delete the match.
Confidence: HIGH — Verification: read; baseline log shows the test passing but cannot validate what it does not assert.

### F-003 [S4]
Where: crates/codecs/src/audio/opus/decoder.rs:347-358 (empty-packet path, part1 lines but the contrast is with my chunk's suite) vs vorbis/decoder.rs:754-756
Code (vorbis, the contrast):
```rust
        if packet.data.is_empty() {
            return Ok(DecodeOutcome::NeedMoreInput);
        }
```
What is wrong: Opus `decode` has no empty-packet guard — a zero-length packet (legal in Ogg per RFC 3533 lacing) goes straight to `engine.decode(&[])`. Whether that is a typed `PacketError` (→ typed Corrupt, counted as damage by the pipeline's bounded recovery) or an Ok-with-zero-samples (→ window 0 → `NeedMoreInput`) depends on ruopus's empty-input behavior, which I could not verify without running the engine. Vorbis treats empties as graceful `NeedMoreInput`.
Why it matters: same-container, same-situation, two decoders answer differently — one counts damage for a legal packet shape. If ruopus errors on empty, an Ogg Opus stream using zero-length cadence packets accrues spurious damage accounting (bounded, non-fatal).
What correct looks like: an explicit `if packet.data.is_empty() { return Ok(DecodeOutcome::NeedMoreInput); }` matching the Vorbis precedent (or a test pinning whichever behavior ruopus has).
Confidence: LOW on the actual behavior — UNVERIFIED (ruopus is a crates.io dep; no vendored source to read, no run allowed); HIGH that the asymmetry exists.

## 4. Per-function table (part2 items)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `decode` tail: fill closure | 428-454 | REAL | — | window copy + permutation + gain; bounds verified (see §2); `debug_assert` + runtime slot check both present |
| `decode` tail: pooled call + exact | 455-457 | REAL | — | `exact` recomputed identically to fill's — duplication is deliberate (slot vs slice) |
| `decode` tail: pts_stamp | 458-468 | REAL | — | anchored → `Present(start − pre_skip)` (saturating, `unwrap_or(i64::MAX)`); unanchored → `Missing` (the honest rule, pinned by unit test) |
| `decode` tail: frame build/accept | 469-487 | REAL | — | duration in ns via u128; one frame per packet |
| `flush` | 490-497 | REAL | — | BadState guard + immediate Drained — correct for Opus |
| `PooledWindow` | 502-504 | REAL | — | thin result |
| `pooled_sample_slot` | 510-547 | REAL | — | Vorbis pattern verbatim; `fill`'s Ok value discarded at 523 (caller recomputes) — harmless; fresh-slot not-exclusive arm is provably unreachable but typed loudly |
| test `extra_v0` | 555-564 | REAL | — | minimal head builder; comment tail honestly "unvalidated" |
| test `config` | 566-580 | REAL | — | config factory |
| test `descriptor_gate_accepts_only_opus` | 582-588 | REAL | — | case-sensitive gate |
| test `Seen`/`Capture` | 592-612 | REAL | — | capture sink |
| test `FIXTURE_PACKET` | 618-626 | REAL | — | verbatim 95-byte real packet (provenance documented) — no filesystem dependency |
| test `packet` | 629-643 | REAL | — | packet factory |
| test `seek_landing_is_missing_until_the_granule_reanchors` | 653-723 | REAL | — | would fail on any anchor regression: pins head-pts 0, Missing prefix, Missing second unstamped, re-anchor at `granule − window − pre_skip` exactly |
| test `configure_requires_and_validates_the_head` | 726-753 | REAL | — | pins `opus.head.missing`, `opus.head.family0_channels` (the pre-constructor panic gate), 48k stereo F32, pre_skip 312 |
| test `decode_before_configure_is_typed_bad_state` | 756-783 | REAL | — | typed BadState |
| test `hostile_packets_never_panic_and_isolate_to_the_packet` | 786-851 | SHALLOW (one arm) | S4 | no-panic + survival are asserted (F-002's no-op match weakens the claim) |

## 5. Test honesty (part2's tests)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| seek_landing_is_missing_until_the_granule_reanchors | YES | exact pts equality on 3 anchor states + re-anchor arithmetic, over a real packet |
| configure_requires_and_validates_the_head | YES | typed codes + negotiated format equality |
| decode_before_configure_is_typed_bad_state | YES | error code equality |
| descriptor_gate_accepts_only_opus | YES | 4-case boolean |
| hostile_packets_never_panic… | PARTIALLY | no-panic + post-barrage survival YES; outcome classification NO (F-002) |
| (out of chunk) corpus_0047/0077 | NO (in baseline) | vacuous skip — F-001 |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "opus_corpus_differential 2/2 pass in the codecs suite" as the row's evidence | docs/feature-matrix.toml:747 | PAPER-ONLY (in committed baseline) | F-001: 0.01s for both = skip path; corpus media not in repo |
| "pre-skip … measured 312 on every corpus lane", "end-trim, measured 648 on every corpus lane" | decoder.rs:30-31 (module doc, part1) | UNVERIFIED | same corpus dependency; unit test pins pre_skip 312 on ONE fixture only |
| "Every corpus lane measures gain = 0 (unity) — the multiplication is a no-op there" | decoder.rs:59-61 | UNVERIFIED | no committed lane; gain≠0 honestly recorded untested in matrix notes |
| "The 6-channel (5.1) row is corpus-verified (0077's differential …)" | opus/order.rs:22-24 | UNVERIFIED | order.rs's OWN test only asserts the row equals itself (`assert_eq!(perm, &[0,2,1,5,3,4])`) — a tautology vs its comment; the real verification is the skipped corpus lane |
| "- status = **blocked** (decode…)" | docs/formats/audio-opus.md:9 | FALSE (stale) | the decoder is implemented, admitted (ruopus =0.1.2 in Cargo.toml:137), unit-tested and dispatched by the shipped provider — the doc records the pre-admission decision (2026-10-24) with no update pointer |

## 7. Dead / duplicated / unreachable code
- `match outcome { Ok(_) | Err(_) => {} }` (833-835) — dead (F-002).
- `pooled_sample_slot` is a near-verbatim copy of vorbis/decoder.rs:1009-1057 (and flac's equivalent) — third duplication of the same rotation; a shared util would own it (S4-adjacent; noted, not counted separately — cross-file duplication belongs to the mod.rs owner).

## 8. Dependencies & flags
- `ruopus =0.1.2` exact pin, default-features off, `features = ["std"]` (codecs/Cargo.toml:137) — the admission record's binding condition 1 honored (verified in Cargo.toml). `codec-opus` in default set → shipped build compiles it (baseline #1 CLEAN).
- No `unsafe` in this file; no unwrap/expect on input paths (test code excepted, where fixtures are constants).

## 9. Open questions for the next agent
- W07-A01 (part1): the granule/anchor arithmetic (383-399) — a regressed mid-stream granule (damaged page) with `discontinuity=false` keeps the monotonic raw counter but `end = min(raw_after, granule)` can zero the window — dropped output vs damage accounting; please assess the blast radius.
- W07-A05 (order.rs): the 3/4/5/7/8 rows are doc-table-only (your file admits it); the 5.1 row's only live verification is the skipped corpus lane (F-001).
- REQUEST-ORCH-RUN: none for my chunk; if the orchestrator wants to settle F-001/F-003 empirically: `LUMEN_CORPUS=<corpus-v4 root> cargo test -p codecs --test opus_corpus_differential -- --nocapture` and a one-off rustc snippet calling `ruopus::OpusDecoder::decode_packet(&[])`.
