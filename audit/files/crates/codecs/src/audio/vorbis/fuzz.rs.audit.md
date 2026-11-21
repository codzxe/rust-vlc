# AUDIT: crates/codecs/src/audio/vorbis/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W07-A10 |
| Date | 2026-11-21 |
| Lines / functions | 174 / 4 prod items (harness fn, CountingSink, FrameSink impl) + 3 tests |
| File verdict | REAL (as a harness — with a FALSE central claim) |
| Reachable from shipped entry point? | NO (by design: fuzz infrastructure — the libFuzzer shim `fuzz/fuzz_targets/fuzz_codec_vorbis.rs` calls it in the separate nightly `lumen-fuzz` package; no shipped binary links it). The in-crate deterministic smoke tests DO run in the normal suite: baseline `test-workspace.log.txt:508-510` — all three `audio::vorbis::fuzz::tests` passed in run #3. |
| Findings | S0: 0 S1: 0 S2: 2 S3: 1 S4: 1 |
| Commands run | `od` hex-dump of both corpus seeds (both begin `OggS`); `sed -n` of `scripts/fuzz_registry_check.sh` (corpus gate = non-emptiness only) and `scripts/quality/25_fuzz_campaign.sh`; `ls fuzz/fuzz_targets fuzz/corpus fuzz/artifacts docs/audit/reports/fuzz` (35 targets, artifacts/report dirs ABSENT); `rg -n 'vorbis::fuzz' baseline log`; read of `decoder.rs:229-245,267-345` (bounds claim verification). |

## 1. One-paragraph verdict
The harness is well-structured (three genuinely different surfaces, bounded passes, saturating counters, honest swallow-errors-by-contract) and its deterministic smoke tests ran green in the baseline suite — but its central claim, "Never panics … by construction of the bounded parse/decode paths", is false: pass 1 feeds `VorbisHeaders::from_extra_data`, which panics on a valid ident+comment chain followed by a 1..6-byte `0x05`-prefixed remainder (headers.rs:237, cross-file S1 filed in my headers.rs audit). Worse, the committed seed corpus is format-mismatched (raw Ogg container files where the harness expects the joined header-chain blob), so every seed dies at the first byte of pass 1 and even a running libFuzzer campaign would start from zero parser coverage — which is exactly how the panic survived. No campaign-execution evidence exists anywhere in the tree.

## 2. What actually works (verified)
— verified by reading :89-130: `cut` is modulo-bounded (no overflow, `min` guard before slicing), the two contract passes reuse one decoder across clean+discontinuity (the documented overlap-reset target), all decode results are discarded by contract (`let _ =` is the harness's stated design, not a swallowed production error).
— verified by reading `decoder.rs:229-245` (`build_engine` refuses >`MAX_MAPPED_CHANNELS`) + `:343-345` (`max_output_len` = blocksize_long/2 × channels × 4): the module's memory-bound claim (:38-40) is arithmetically true for every pass that reaches allocation.
— verified by baseline `test-workspace.log.txt:508-510`: `empty_input_survives_all_passes`, `marker_only_and_short_inputs_survive`, `hostile_wide_inputs_survive` — all `ok` in run #3.
— verified by reading `fuzz/fuzz_targets/fuzz_codec_vorbis.rs`: the shim calls exactly this harness; the registry gate (baseline run #18, PASS, 35 targets, diff empty) is consistent with the tree.

## 3. Findings
### F-001 [S2]
Where: crates/codecs/src/audio/vorbis/fuzz.rs:67-69 (claim) and :74 (the panic-reaching call)
Code:
```
/// Run one fuzz pass over `data` (see the module docs). Never panics,
/// never hangs, never allocates beyond input-derived bounds — by
/// construction of the bounded parse/decode paths.
pub fn fuzz_codec_vorbis(data: &[u8]) {
```
What is wrong: The "never panics by construction" claim is false. Pass 1 calls `VorbisHeaders::from_extra_data(data)`; for a buffer of a valid 30-byte identification header + a valid comment header + a 1..6-byte remainder starting with `0x05`, that function panics at `&setup[1..7]` (headers.rs:237 — out-of-range slice; full analysis in my headers.rs audit, S1). The harness's own committed inputs never include that shape (short inputs all die earlier; `hostile_wide_inputs` appends 0xFF bytes that fail the comment magic), so the in-crate "proof" (:30-31 "the harness logic is proven by the in-crate deterministic smoke tests") does not cover the window.
Why it matters: This file is the panic-detection instrument for the Vorbis parser; its doc certifies a property (panic-free) that its own surface violates. Anyone trusting the module docs — or the "panic-free by construction" note repeated in the shim and `fuzz/Cargo.toml` — believes the Vorbis chain parse is crash-safe when a ~50-byte crafted blob kills the process through the shipped configure path.
What correct looks like: Fix headers.rs:237 (length guard) and add the regression input here: `ident(2,44100,8,11) ‖ minimal_comment() ‖ [0x05]` as a fourth deterministic test.
Confidence: HIGH — Verification: read (cross-file index arithmetic; panic not executed — no rustc in sandbox).

### F-002 [S2]
Where: fuzz/corpus/fuzz_codec_vorbis/ (seed files) vs crates/codecs/src/audio/vorbis/fuzz.rs:70-84
Code:
```
    let headers = match VorbisHeaders::from_extra_data(data) {
        Ok(headers) => headers,
        Err(_) => {
```
What is wrong: Both committed seeds (`seed_ogg_vorbis16k.ogg`, `seed_ogg_vorbis8k_st.ogg`) are raw Ogg CONTAINER files — hex-dump: `4f 67 67 53` (`OggS`) at offset 0. The harness expects the codec's joined header-chain blob (`\x01vorbis…`, the `extra_data` shape). `parse_identification` rejects `data[0]='O'` at the type-byte check, so every seed returns at pass 1 entry: zero coverage of the header split, the comment parser, the block decode, and the contract path. The registry "coverage gate" (`fuzz_registry_check.sh:64-73`) only verifies the corpus directory is non-empty — a format-mismatched corpus passes vacuously.
Why it matters: The nightly campaign (`audit.yml:325-356`, 180 s/target across 5 shards) would burn its budget rediscovering the `\x01vorbis` magic instead of mutating deep parser state; combined with F-001 this is precisely why a trivially-reachable panic is still in the tree. The same mistake likely repeats in sibling codec corpora that take container files where chain blobs are expected (each corpus owner should re-check).
What correct looks like: Seed with the extracted chains (identification ‖ comment ‖ setup blobs — e.g. what `containers/ogg.rs:202-208` joins from the committed fixtures), plus a truncated-setup seed; and/or make the registry gate validate that at least one seed penetrates pass 1 (a one-line harness call in the gate).
Confidence: HIGH — Verification: read + od (both seeds dumped; both `OggS`-led).

### F-003 [S3]
Where: crates/codecs/src/audio/vorbis/fuzz.rs:23-34 (wiring/execution claims) vs repo state
Code:
```
//! campaign itself is owed CI wiring (the same status as
//! `fuzz_codec_flac` / `fuzz_codec_mjpeg` / `fuzz_codec_pcm`).
```
What is wrong: No evidence exists that any fuzz campaign has ever run: `fuzz/artifacts/` is absent, `docs/audit/reports/fuzz/` is absent, and no campaign summary is committed anywhere (the nightly `audit.yml` fuzz tier uploads artifacts only `if: always()` to GitHub, retention 30 days — nothing durable in-repo). The doc's surviving claim "the harness logic is proven by … the committed-fixture + seeded-mutation survival run (`crates/codecs/tests/vorbis_fuzz_survival.rs`)" is real (that suite passed in baseline run #3), but it is a seeded-mutation sweep, not a coverage-guided campaign — it cannot speak for the input space the way "fuzz" implies.
Why it matters: Fuzz posture is presented as stronger than any run demonstrates; with F-001/F-002 the gap is not theoretical — a live panic sits inside the never-campaigned surface.
What correct looks like: Commit one campaign summary per nightly run (the script already writes `$OUT/fuzz/campaign/summary.txt`) and state in the module doc which date last ran clean.
Confidence: HIGH — Verification: read (tree listings; no artifact dirs; baseline has registry check only).

### F-004 [S4]
Where: crates/codecs/src/audio/vorbis/fuzz.rs:25-29
Code:
```
//! The cargo-fuzz target file is proposed at
//! `fuzz/fuzz_targets/fuzz_codec_vorbis.rs` exactly as doc 04 §8.5
//! registers it. Building it needs the `libfuzzer-sys` dev-dependency
//! plus the `fuzz/Cargo.toml` scaffolding — both Lead-owned dependency
//! decisions (AGENTS.md §3.2, C5) — so until they land the
```
What is wrong: Stale "Wiring status (honest)" section: the scaffolding HAS landed — `fuzz/Cargo.toml` exists with `libfuzzer-sys = "0.4.13"`, the target file exists, the corpus directories exist, and the registry gate (baseline #18) reports 35 targets clean. The doc says the target is "proposed" and the deps "until they land".
Why it matters: A "wiring status (honest)" header that contradicts the tree trains readers to skip it; the next honest-status reader gets wrong facts.
What correct looks like: Update to "scaffold landed (fuzz/Cargo.toml + shim + corpus); nightly campaign tier defined in .github/workflows/audit.yml; first campaign execution not yet recorded" — and fix the same paragraph in the sibling fuzz modules (flac/mjpeg/pcm at minimum).
Confidence: HIGH — Verification: read (file listings + Cargo.toml contents).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| CountingSink + FrameSink::accept | 55-65 | REAL | — | Saturating counter; correct. |
| fuzz_codec_vorbis | 70-131 | REAL (claim FALSE) | S2 | Pass structure sound; pass 1 reaches the headers.rs panic (F-001); `cut` bounded; both contract passes legal; errors swallowed by documented contract. |
| tests: empty_input_survives_all_passes | 137-140 | REAL test | — | Fails only on panic — the contract. |
| tests: marker_only_and_short_inputs_survive | 142-151 | REAL test | — | 1..40-byte sweeps; does not reach F-001's window. |
| tests: hostile_wide_inputs_survive | 153-173 | REAL test | — | Legal-ident + 0xFF tails; dies at comment magic — no truncated-setup coverage (F-001). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| All three | Only on panic/hang — which is the harness contract | But none constructs ident+comment+short-0x05-remainder, so the F-001 panic class passes silently: the deterministic "proof" has a hole exactly where the defect is. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Never panics … by construction of the bounded parse/decode paths" | fuzz.rs:67-69 | FALSE | headers.rs:237 panic via :74 (F-001). |
| "Memory is bounded … at most blocksize_long (≤8192)/2 × channels (≤8) × 4" | fuzz.rs:36-44 | REAL | `build_engine` channel gate (decoder.rs:231) + `max_output_len` (decoder.rs:343-345) — arithmetic verified. |
| "a lying length buys a typed truncation, not an allocation" | fuzz.rs:42-43 | REAL | comment length-walk bounds-checked (comments.rs:111-140, verified in my headers.rs audit). |
| "harness logic is proven by the in-crate deterministic smoke tests" | fuzz.rs:30-31 | SHALLOW | Tests exist and pass (baseline :508-510) but do not cover the panic window (F-001). |
| "the cargo-fuzz target file is proposed … until they land" | fuzz.rs:25-29 | FALSE (stale) | Scaffold landed (F-004). |

## 7. Dead / duplicated / unreachable code
- The harness is unreachable from shipped binaries (by design — fuzz infra); noted for the reachability ledger, not a defect.
- No duplication within the file; the shim in `fuzz/fuzz_targets/` is the single caller.

## 8. Dependencies & flags
- `pub mod fuzz` compiled unconditionally in the codecs crate (vorbis/mod.rs:66) — the harness body ships in every build of `codecs` (dead weight in release binaries, tiny; consistent with the 35-target registry design).
- Uses codec-api prelude + media-types only; the libfuzzer-sys edge lives in the separate `lumen-fuzz` package (own workspace root), so the shipped workspace stays clean.

## 9. Open questions for the next agent
- Corpus owners of the other 34 targets: are YOUR seeds format-matched to the harness input? (F-002's pattern — container file vs parsed-substrate — is easy to repeat; the registry gate will not catch it.)
- Who owns fixing headers.rs:237 — this file's F-001 and the headers.rs S1 are the same defect; coordinate one fix + one regression test.
