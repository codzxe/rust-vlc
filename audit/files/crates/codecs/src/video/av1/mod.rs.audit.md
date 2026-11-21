# AUDIT: crates/codecs/src/video/av1/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A03 |
| Date | 2026-11-21 |
| Lines / functions | 86 / 3 module decls + 2 re-export groups + 1 doctest (module-root file: docs + wiring, no logic) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — `pipeline_host.rs:63` `use codecs::video::av1::Av1Decoder;` + dispatch `"av1" \| "av01" => { let mut decoder = Av1Decoder::new(); … }` (:306-310); containers feed it: IVF `AV01`→`av1` (`demux/src/ivf/mod.rs:206`), MKV `V_AV1`→`av1` (`demux/src/mkv/header.rs:807`), MP4 `b"av01"`→`av1` (`demux/src/mp4/mod.rs:446`). `codec-av1` is in the codecs default feature set (Cargo.toml `[features] default`), which `apps/cli/Cargo.toml:51` enables (no `default-features = false`), and release.yml:193 builds `-p lumen-cli` without feature overrides. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "av1" test-workspace.log.txt` (21 unit tests + conformance + differential, all `ok`); `sed -n '873,915p'` of same (conformance ran **14.05 s** — a real ffmpeg-matrix execution, not a skip); `rg -n "AV01\|av01\|V_AV1" crates/demux/src/{ivf,mkv,mp4}` (container mapping census); `rg -in "av1" crates/demux/src/ogg/` (empty — Ogg claim holds); rav1d pin cross-check vs `crates/codecs/Cargo.toml` |

## 1. One-paragraph verdict
The AV1 module root: a documentation-heavy wiring file over `pub mod decoder/obu/fuzz` and re-exports. Every load-bearing doc claim checks out against code and executed baseline evidence: the rav1d =1.1.0 asm-off pin is exactly what Cargo.toml declares; the descriptor gate is proven by the doctest and the decoder unit tests; the "differential against ffmpeg" claim is backed by a 14-second real conformance-matrix run plus 6/6 differential tests in the baseline; the "no Ogg AV1 mapping" claim holds (grep-empty). The decoder is genuinely reachable from the shipped CLI through three container lanes. One doc-drift defect: the module doc says Matroska/WebM and MP4 AV1 "follow in their container cards", but both mappings already landed — the doc under-claims shipped reality (the rare direction).

## 2. What actually works (verified)
— Executed AV1 evidence in baseline: `video::av1::*` 21 unit tests `ok` (:519-533, :571); `av1_conformance … finished in 14.05s` with `conformance_matrix_vs_ffmpeg_generated_set … ok` (:886-889) — 14 s of real decode+compare work (ffmpeg present; the file's own skip-mode would have finished in ~0 s, cf. the corpus-skip pattern proven in my slice.rs#part4 audit); `av1_differential` 6/6 `ok` in 0.43 s (:891-900), incl. `differential_pixel_md5_against_ffmpeg_dump` and `tenbit_fixture_is_a_typed_refusal_not_a_misdecode`.
— rav1d pin: `crates/codecs/Cargo.toml` `rav1d = { version = "=1.1.0", optional = true, default-features = false, features = ["bitdepth_8", "bitdepth_16"] }` — exactly the doc's claimed admission shape (mod.rs:9-10).
- Ogg claim: `rg -in "av1" crates/demux/src/ogg/` → empty — "no `Av1` in its codec table" (mod.rs:36-37) TRUE.

## 3. Findings
### F-001 [S4]
Where: crates/codecs/src/video/av1/mod.rs:40-42
Code:
```
//! proposal-class, not shipped), the same one-TU-per-packet contract
//! applies. Matroska/WebM `AV01` tracks and MP4 `av01` follow in
//! their container cards (the same AV1 decoder consumes their
```
What is wrong: The doc's future tense is stale: MKV already maps `V_AV1` → `av1` (`crates/demux/src/mkv/header.rs:807`, pinned by its own test at :982) and MP4 already maps `b"av01"` → `av1` (`crates/demux/src/mp4/mod.rs:446`). Both descriptors dispatch the shipped `Av1Decoder` arm in pipeline_host — the plumbing the doc defers to future "container cards" is wired today.
Why it matters: Doc/code drift in the module that is the project's "flagship software video codec" story; a reader would wrongly conclude mkv/mp4 AV1 open is unimplemented and skip testing it (or cite the doc as evidence of a gap that doesn't exist). Under-claiming is still drift.
What correct looks like: Update the doc to name the landed mappings and what remains unproven (end-to-end `lumen open` on an mkv/mp4 AV1 file — no baseline smoke covers that lane; the "re-framed TUs unchanged" contract is asserted, not demonstrated for those containers).
Confidence: HIGH — Verification: read + grep census of all three containers' mapping sites.

## 4. Per-function table (module-root items)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (admission/purity) | 1-15 | REAL | — | rav1d pin/asm-off claims verified vs Cargo.toml verbatim; the purity record `docs/dependencies/records/rav1d-1.1.0.md` exists (not re-audited here — dependency-record wave owns it). |
| module docs (structure) | 17-26 | REAL | — | `obu` + `decoder` module claims consistent with their files (obu tests :524-533; decoder.rs unsafe seam documented at its :4-10, matching "second sanctioned unsafe exception" — baseline hygiene notes confirm unsafe concentrated in av1/decoder.rs). |
| module docs (delivery) | 28-42 | REAL except F-001 | F-001 | IVF claim verified; Ogg no-mapping claim verified; MKV/MP4 stale (F-001). |
| module docs (v0 boundary) | 44-51 | REAL | — | 8-bit 4:2:0 only; 10-bit typed refusal proven by `tenbit_fixture_is_a_typed_refusal_not_a_misdecode … ok` (:899). |
| doctest | 55-74 | REAL (UNVERIFIED execution) | — | Content-asserting (vp9 descriptor → `refused.is_err()`); compiles as part of the crate; baseline contains NO `Doc-tests` sections at all (workspace test log has zero such headers; the run also aborted at apps/desktop) — doctest execution UNVERIFIED in baseline evidence. |
| `pub mod decoder/obu/fuzz` | 76-79 | REAL | — | Feature-gated: `#[cfg(feature = "codec-av1")]` on `pub mod av1` (video/mod.rs:27-29); `codec-av1` ∈ default → compiled+shipped. |
| re-exports (`Av1Decoder`, OBU vocabulary) | 81-86 | REAL | — | All names exist (crate compiles, baseline build #1 CLEAN); OBU constants consumed by obu tests. |

## 5. Test honesty (module's own doctest + the suites it references)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| mod.rs doctest | YES | Real `Av1Decoder::new()` + cross-codec configure refusal assert — a semantic gate, not a compile echo. Execution not evidenced in baseline (UNVERIFIED — no Doc-tests sections in the log). |
| `av1_bench::av1_decode_throughput_1080p_vs_ffmpeg_libdav1d` | N/A | `#[ignore]`d with an honest measurement label + exact rerun command (:876) — a disclosed measurement, not silent evasion. |
| conformance/differential suites | YES | 14.05 s conformance (real ffmpeg matrix), 6/6 differential incl. pixel-MD5 and typed-refusal lanes — executed in baseline. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| rav1d 1.1.0, asm OFF, bitdepth_8/16 only | mod.rs:7-11 | REAL | Cargo.toml rav1d entry verbatim match. |
| "differentially against ffmpeg" | mod.rs:50-51 | REAL | av1_conformance 14.05 s + av1_differential 6/6 in baseline (:886-900). |
| "no AV1 mapping exists there today" (Ogg) | mod.rs:36-37 | REAL | grep-empty over demux/ogg + apps/cli ogg container. |
| "Matroska/WebM AV01 tracks and MP4 av01 follow in their container cards" | mod.rs:40-42 | FALSE (stale) | Both mappings landed (F-001). |
| "8-bit 4:2:0 …; 10/12-bit, 4:2:2/4:4:4 and monochrome are typed Unsupported" | mod.rs:46-48 | REAL | `tenbit_fixture_is_a_typed_refusal_not_a_misdecode … ok` (:899); `av1.bitdepth`/`av1.layout` codes in decoder tests. |

## 7. Dead / duplicated / unreachable code
None in this file. (Cross-file: nothing dead; the module is fully wired and shipped.)

## 8. Dependencies & flags
`codec-av1` Cargo feature → `dep:rav1d`; in `default` → every default-feature consumer (lumen-cli, and transitively lumen-desktop/lumen-tauri via the lumen-cli lib) ships the AV1 decoder. Wasm builds exclude it (rav1d doesn't compile for wasm32 — codecs/Cargo.toml comment; consistent with parity 0/252 web column, xtask-parity-report.log.txt).

## 9. Open questions for the next agent
- W06-A02 (av1/fuzz.rs) / A05-A06 (av1/decoder.rs parts): does any suite prove the mkv/mp4 → AV1 handoff's "one TU per packet" contract end-to-end (a real mkv V_AV1 or mp4 av01 file decoded), or only IVF? If only IVF, the doc's "consumes their re-framed TUs unchanged" is an untested promise — worth a finding in those files.
- Baseline-evidence integrity (for the orchestrator): the baseline test log contains an anomalous interleaved non-test line inside the av1_conformance section (`test-workspace.log.txt:884-886` — "Enter command: <target>|all <time>|-1 … Parse error … 'e_agent_exit_code=$?'", between `running 1 test` and the test's `ok` at :887) — looks like capture-tool bleed, not repo content; someone should confirm the log's provenance chain.
