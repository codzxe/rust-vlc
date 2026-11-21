# AUDIT: crates/codecs/src/video/hevc/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-21 |
| Lines / functions | 93 / 3 production items (`FUZZ_NAL_BUDGET`, `fuzz_codec_hevc`, module contract) + 2 tests |
| File verdict | UNREACHABLE (the guarded subject — the HEVC parser — has no shipped consumer; the harness itself is dev/fuzz infra by design) |
| Reachable from shipped entry point? | NO, twice over. (a) The harness: only callers are `fuzz/fuzz_targets/fuzz_codec_hevc.rs:19` (cargo-fuzz target, dev tooling) and the in-file tests. (b) The subject: `rg '"hevc"\|"h265"' apps/cli/src/pipeline_host.rs crates/playback-engine/src/` → zero dispatch arms; pipeline_host's fall-through (:375-380) lists landed codecs WITHOUT hevc; `hevc::parser` importers outside the hevc module are tests only (`crates/codecs/tests/hevc_parser_corpus.rs`, `hevc_hw_path.rs`). Baseline smoke confirms: `sample-hevc.mp4` → Opening → **Error** (no HEVC decoder) (INDEX.md corrected matrix). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `cat fuzz/fuzz_targets/fuzz_codec_hevc.rs` (shim wiring); `rg -n "fuzz_codec_hevc" fuzz/ scripts/ baseline` (registry rows); `rg -n "video::hevc::fuzz" test-workspace.log.txt` (2/2 `ok`, :630-631); `head -40 scripts/fuzz_registry_check.sh` (what the CLEAN gate actually proves); `rg -n -A8 "pub fn annexb_nals" hevc/parser/nal.rs` (collection semantics) |

## 1. One-paragraph verdict
A compact, honest fuzz harness: one call drives every context-free HEVC parser surface (Annex-B walk, VPS/SPS/PPS parses with a reused RBSP buffer, hvcC record walk, length-prefixed walk at all three legal length sizes), swallowing typed errors per the documented absence-of-panic-only contract. The wiring is real and registry-gated (target file, Cargo.toml `[[bin]]`, README row, and harness fn all cross-verified by `fuzz_registry_check.sh`, which passed CLEAN with 35 targets in baseline full-gate), and both deterministic smoke tests pass. But the fuzzing itself has never happened: no libFuzzer campaign evidence exists anywhere in the baseline, the registry gate proves only name-list consistency (it does not compile or execute), and the harness's panic-freedom claim rests on 7 hand-picked inputs. All of this is disclosed in the file's own "wiring status (honest)" section — the gap is real but not a lie. The subject it guards is itself unreachable in shipped builds (no HEVC dispatch arm exists).

## 2. What actually works (verified)
— Both tests `ok` in baseline (test-workspace.log.txt:630-631): empty input, bare start codes, VPS-typed header, 4096×`0x00`, 4096×`0xff`, and a 256×8-byte start-code flood with hostile VPS/SPS/PPS headers.
— Registry consistency: `fuzz/Cargo.toml:237-238` (`fuzz_codec_hevc`), `fuzz/fuzz_targets/fuzz_codec_hevc.rs:19` (calls `codecs::video::hevc::fuzz::fuzz_codec_hevc`), `fuzz/README.md:80` (registry row) — and baseline `full-gate.log.txt:47` `fuzz-registry: CLEAN — 35 targets, diff empty, corpora present`.
— Bounds shape: `annexb_nals` returns `Vec<&[u8]>` of zero-copy views (nal.rs:390-392) — allocation is a linear function of start-code count (≤ input_len/4 × 16 B/view); `parse_with_buffer` reuses `rbsp` across NALs (48-53); pass 4 walks `LengthPrefixedNals` to exhaustion, bounded by input length.

## 3. Findings
### F-001 [S4]
Where: crates/codecs/src/video/hevc/fuzz.rs:29-31
Code:
```
//! The cargo-fuzz target is `fuzz/fuzz_targets/fuzz_codec_hevc.rs`
//! (Lead-owned nightly wiring, see `fuzz/README.md`); the harness
//! logic is meanwhile proven on stable by the deterministic sweeps
//! below and the crate's parser conformance suites.
```
What is wrong: "Proven on stable by the deterministic sweeps" overstates seven fixed inputs (2 tests × ~4 shapes). No fuzz campaign has ever run: the baseline contains no libFuzzer execution evidence, `fuzz_registry_check.sh` (read in full) verifies only manifest/file/README/harness-name consistency plus non-empty seed corpora — it neither compiles `fuzz/` nor executes a single fuzz iteration. The comment's own claim that the fuzz package "compiles on stable" (`cargo check` in `fuzz/`) is likewise unevidenced in baseline (fuzz/ is not a workspace member; workspace build #1 does not cover it).
Why it matters: The HEVC parser is a hostile-input surface by definition; its panic-freedom currently rests on 7 inputs + review. Mitigations: the parser's own unit suites, and — decisively for today — the surface is unreachable from any shipped entry point (no hevc dispatch arm), so no user input reaches it. Hence S4, not S3.
What correct looks like: One recorded libFuzzer run (even 60 s × target) in the baseline evidence, or the doc downgraded to "smoke-swept only, campaign pending nightly wiring".
Confidence: HIGH — Verification: read + registry script read + baseline grep (no campaign evidence); UNVERIFIED only for the fuzz package's stable compile claim.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module contract docs | 1-36 | REAL | F-001 | Honest scope statements (absence-of-panic-only; damage swallowed); bounds claim (35-36) holds: views zero-copy, buffer reused, 4 passes. |
| `FUZZ_NAL_BUDGET` | 38-40 | REAL | — | 4096; bounds the per-NAL parse loop only — `annexb_nals` still collects every NAL view first (nal.rs:390-392 `Vec<&[u8]>`), so inputs with >4096 NALs pay full walk cost (still input-linear — within the stated contract). |
| `fuzz_codec_hevc` | 45-67 | REAL | — | Passes 1-2: Annex-B walk + VPS/SPS/PPS `parse_with_buffer` (buffer reused, `let _ =` swallows typed errors per contract); pass 3: `HvcCConfig::parse`; pass 4: `LengthPrefixedNals` at length sizes 1/2/4 walked to exhaustion. No panics on the tested shapes; would surface any parser panic as a test failure. |
| test `empty_and_marker_inputs_survive` | 73-79 | REAL | — | Empty, bare start code, VPS-typed header. Bites: a panic on any of these fails the test (executed in baseline). |
| test `hostile_wide_inputs_survive` | 81-93 | REAL | — | 0x00/0xff floods + structured start-code flood with hostile parameter-set headers. Shallow by fuzzing standards, honest by its own naming ("sweeps"). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| both tests | YES, for the shapes they cover | Direct execution of the harness; any parser panic/index error on these exact inputs propagates. Coverage is 7 inputs — the gap is F-001, not dishonesty. |
| Run status | 2/2 `ok`, baseline :630-631; none ignored. | |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One call = one untrusted byte buffer through every parser surface" | fuzz.rs:5-17 | REAL | The four passes (47-66) match the enumerated surfaces; slice-header exemption (19-20) is stated and true (needs active contexts). |
| "Never panics, never hangs, never allocates beyond input-derived bounds" | fuzz.rs:42-44 | REAL-UNPROVEN | Holds on the 7 tested inputs; no campaign evidence for the general claim (F-001); subject unreachable in shipped builds today. |
| "the harness logic is meanwhile proven on stable by the deterministic sweeps" | fuzz.rs:29-31 | SHALLOW | 7 fixed inputs ≠ proof; see F-001. |
| Registry name `fuzz_codec_hevc` "(parser self-build)" (doc 04 §8.5) | fuzz.rs:1-3 | REAL | Target + registry + gate CLEAN (full-gate.log.txt:47). |

## 7. Dead / duplicated / unreachable code
- The entire HEVC parse surface this harness guards is unreachable from shipped entry points (no hevc codec dispatch; importers are tests + the equally unreachable hevc decode path). Same family as my sps.rs#part1 finding: the in-house HEVC parser (~2000+ LOC) is maintained dead weight until an HEVC lane ships.

## 8. Dependencies & flags
In-crate module (`crates/codecs/src/video/hevc/mod.rs:48 pub mod fuzz`), unconditional (no feature gate; hevc is an in-house module). The cargo-fuzz side (`fuzz/`, libfuzzer-sys, nightly) is outside the workspace and outside baseline build evidence. No unsafe in this file.

## 9. Open questions for the next agent
- HEVC parser owners (W06-A02 hevc/parser/slice.rs etc.): the harness deliberately excludes the slice-header seam ("needs active VPS/SPS/PPS contexts, its own unit suites own that pass") — do those unit suites feed *hostile* contexts, or only well-formed ones? If only well-formed, the context-requiring parse paths are fuzzed nowhere.
- REQUEST-ORCH-RUN (low priority, see log): `cargo +nightly fuzz run fuzz_codec_hevc -max_total_time=60` once nightly wiring exists, to convert F-001's gap into evidence.
