# AUDIT: crates/codecs/src/video/hevc/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A04 |
| Date | 2026-11-21 |
| Lines / functions | 53 / 0 functions (module root: 1 doc block, 4 `pub mod`, 2 `pub use`) |
| File verdict | UNREACHABLE |
| Reachable from shipped entry point? | NO. Call-chain proof: `video/mod.rs:32` `pub mod hevc;` compiles the tree unconditionally, but `rg -n "HevcDecoder"` repo-wide → definition (hevc/decode.rs:159,537), this file's re-export (:52), and `crates/codecs/tests/hevc_hw_path.rs` (18 construction sites, tests) ONLY. `apps/cli/src/pipeline_host.rs` `decoder_for` (:236-346) has mjpeg/av1/vp9/dnxhd/h264 arms and no hevc arm — the fall-through (:375-377) prints `no decoder for \`hevc\` in this build (landed: …, h264)`. Baseline behavioral proof: `audit/runs/baseline/mp4-smoke-open-session-hevc.txt` — `sample-hevc.mp4` → `cli.codec_unavailable` → exit 65. |
| Findings | S0:0 S1:0 S2:0 S3:1 S4:3 |
| Commands run | Read full file (53 lines); `rg -n "hevc" crates/codecs/Cargo.toml` (no feature gate; comment at :26 confirms in-house modules unconditional); Read `video/mod.rs` (gate at :30-32); `rg -rn "HevcDecoder"` repo-wide (tests-only constructors — reachability proof); `rg -n "h264\|mjpeg\|vp9\|av1\|dnx\|decoder_for" apps/cli/src/pipeline_host.rs` (dispatch arms + landed-list); `rg -i "hevc\|hvc1\|hev1\|h265" apps/` (no production hevc consumer; ts.rs:968 `unreachable!("no hevc decoder exists in this build")`); `rg "impl Decoder for" hevc/` (exactly one, decode.rs:537 — doc claim check); `rg "Recovered\|hevc\.hw\." hevc/decode.rs` (:434,:570 hevc.hw.required; :716,:724 Recovered); Read `hevc/parser/mod.rs` (37 lines, ZERO re-exports — broken-link proof); Read `hevc/fuzz.rs` (slice surface excluded); `sed -n` quote verification on mod.rs:6-11/19-21/39-45/47-53, video/mod.rs:30-32, fuzz.rs:19-20, decode.rs:434/570, pipeline_host.rs:373-378; `sed -n '300,335p' docs/04-codec-format-roadmap.md` (§4.3 verbatim match); `sed -n '432,465p' docs/feature-matrix.toml` (hevc rows); grep MASTER-PROMPT §14, ADR-001 header, doc 04 section list, codec-api lib.rs:119; baseline: INDEX.md:120-127, test-workspace.log.txt:630-647 (fuzz+parser tests ok), :1086-1110 (hevc_hw_path 17 tests, hevc_parser_corpus), mp4-smoke-open-session-hevc.txt (full). |

## 1. One-paragraph verdict
This is the HEVC module root: 45 lines of architecture doc plus six lines of wiring (`pub mod decode/fuzz/hw/parser`, re-export of `HevcDecoder` and three `hw` seam types). The wiring is correct — every declared module exists as a file, both re-export targets exist (`decode::HevcDecoder` decode.rs:159; `hw::{HevcHwBridge, HevcHwDecoder, HevcHwPath}` hw.rs:290/248/335) — and the doc is unusually honest about the *software*-decode boundary: its doc-04 §4.3 quote is verbatim-accurate, the matrix software row really is `blocked` on all four platforms, and the typed `hevc.hw.required` refusal really is in the code (decode.rs:434,570). But the file's central narrative — "the decode-path integration … routes the parsed result to a hardware session" — describes an integration that no shipped entry point can reach and no adapter exists to serve: `HevcDecoder` is constructed only by tests, the CLI has no hevc dispatch arm, and the shipped binary answers an HEVC track with `cli.codec_unavailable` (baseline). The honesty boundary stops one sentence short: it never says the *hardware* lane itself is dormant scaffolding. Three smaller defects: a broken rustdoc link for the module's error type, an overbroad fuzz-pinning claim, and a dangling `§14` cross-reference.

## 2. What actually works (verified)
- All four submodule declarations resolve to real files (LS of `hevc/`: decode.rs, fuzz.rs, hw.rs, parser/{mod,nal,vps,sps,pps,slice}.rs) and both re-exports resolve — verified by rg of the definition sites.
- "The one `Decoder` impl under this tree" (:15): TRUE — `rg "impl Decoder for" hevc/` → decode.rs:537 only.
- "with no accelerator negotiated it fails configure with the typed `hevc.hw.required` `Unsupported` error" (:16-18): TRUE — decode.rs:434 and :570, sed-verified.
- "Software decode remains `blocked`/`hw-only` in the feature matrix" (:11-14): TRUE — feature-matrix.toml:445-452 `codec-video-hevc-software` status `blocked` ×4 platforms.
- The doc-04 §4.3 quotation ("ADR-001 hw-only, parser self-build M, software decode `blocked` with next-step recorded"): VERBATIM — sed of docs/04-codec-format-roadmap.md:308-310.
- `DecodeOutcome::Recovered` damage model (:42-45): TRUE at library level — decode.rs:716,724; `codec_api::DecodeOutcome` re-exported at codec-api/src/lib.rs:119.
- The re-exported API is exercised (as a library, not shipped): baseline test-workspace.log.txt:630-647 (hevc fuzz + parser unit tests ok), :1086-1088 `hevc_hw_path` "running 17 tests", :1109 `hevc_parser_corpus` — all ok.

## 3. Findings

### F-001 [S3]
Where: crates/codecs/src/video/hevc/mod.rs:8-10 (with :29-35)
Code:
```
//! There is **no software HEVC decode here** — the module parses and
//! classifies bitstream structure, and routes the parsed result to a
//! **hardware session** ([`hw`]). Software decode remains
```
What is wrong: the module root advertises a live "decode-path integration" ("routes the parsed result to a hardware session"; :29-35 "Card 5.G2 adds the consumption side … the consumption point") without disclosing that the entire consumption side is unreachable: `HevcDecoder` has zero production constructors (`rg -rn HevcDecoder` → definition, this re-export, and `crates/codecs/tests/hevc_hw_path.rs` only), no `HevcHwBridge` implementation exists outside test fakes (W06-A06, hw.rs), and no shipped host dispatches hevc (`pipeline_host.rs:375-377`: `no decoder for \`hevc\` in this build`; `containers/ts.rs:968`: `unreachable!("no hevc decoder exists in this build")`). The celebrated typed `hevc.hw.required` refusal (:16-18) is therefore unreachable by any user of a shipped binary — users instead get the blunter CLI-level `cli.codec_unavailable` (baseline mp4-smoke-open-session-hevc.txt, exit 65). product.toml:168 (`hw-decode = false`) and :191 (`hw_decode_policy = "software-only"`) keep the whole hw family off besides.
Why it matters: a developer reading the module root's "honesty boundary" concludes HEVC hardware decode is plumbed and merely adapter-gated; in fact it is compile-only scaffolding on every shipped platform. The doc's honesty stops at the software boundary and omits the hardware one — the exact "future-tense capability described as present" pattern this audit exists to catch.
What correct looks like: one added sentence at the root — "no shipped host constructs `HevcDecoder` yet; the hw lane activates with the Phase-7 adapter cards (7.B)" — or registration in the pipeline host so the typed refusal path actually fires.
Confidence: HIGH — Verification: rg callers (tests-only) + pipeline_host.rs read + baseline smoke log read. Sibling audits (W06-A05 nal.rs#part1, W06-A06 hw.rs, decode.rs audit) independently established the same zero-caller fact; I re-verified the rg myself.

### F-002 [S4]
Where: crates/codecs/src/video/hevc/mod.rs:40
Code:
```
//! a parsed structure or a typed [`parser::HevcError`] — no panics,
```
What is wrong: broken intra-doc link. `HevcError` is defined in `parser::nal` (nal.rs; vps.rs:10 imports it as `super::nal::{…, HevcError, …}`), and `hevc/parser/mod.rs` (37 lines, read in full) contains **zero** `pub use` statements — so the path `parser::HevcError` does not exist and rustdoc reports `broken_intra_doc_links` for the module root's central damage-model reference.
Why it matters: the module root's "total function, typed errors" claim points at a type path that does not resolve — doc rot exactly where a hostile reader checks the error story; every later doc that copies the path propagates it.
What correct looks like: `[`parser::nal::HevcError`]`, or add `pub use nal::HevcError;` to parser/mod.rs and keep the link.
Confidence: HIGH — Verification: full read of parser/mod.rs (no re-exports) + rg of HevcError definition/imports.

### F-003 [S4]
Where: crates/codecs/src/video/hevc/mod.rs:39-41
Code:
```
//! Every parser entry point is total: any byte string yields either
//! a parsed structure or a typed [`parser::HevcError`] — no panics,
//! no unbounded work (the fuzz-lite suites pin this).
```
What is wrong: overbroad pinning attribution. The fuzz-lite harness itself excludes the slice-segment surface — fuzz.rs:19-20 (sed-verified):
```
//! The slice-header seam needs active VPS/SPS/PPS contexts (its own
//! unit suites own that pass); every context-free surface is here.
```
So "every parser entry point … (the fuzz-lite suites pin this)" is true for 4 of 5 parser surfaces (Annex-B walk, VPS/SPS/PPS, hvcC, length-prefixed); the fifth (`SliceSegmentHeader::parse`) is pinned only by context-driven unit suites over crafted headers (baseline :642-647), not by any byte-string fuzz pass — and "any byte string" cannot even reach it without parameter-set contexts.
Why it matters: the module root's damage-model universal ("every entry point, any byte string, fuzz-pinned") overstates the evidence for the one surface where a hostile input actually needs a context to construct; a regression in slice.rs's totality would not be caught by the fuzz sweeps the doc relies on.
What correct looks like: "every context-free parser entry point is fuzz-pinned; the slice-segment header (which requires live VPS/SPS/PPS contexts) is pinned by its context-driven unit suites."
Confidence: HIGH — Verification: full read of fuzz.rs + baseline test lines.

### F-004 [S4]
Where: crates/codecs/src/video/hevc/mod.rs:19-21
Code:
```
//! Doc 04 §4.3 also carries the heavier-patent note: licensing
//! assessment for *distributing an HEVC decoder at all* is a §14
//! item (doc 11 / ADR-010 territory) — the hw-first posture keeps
```
What is wrong: dangling cross-reference. "a §14 item" is inherited from doc 04 §4.3's own wording, but doc 04 has exactly six sections (## 1 … ## 6, verified by grep of its headers) — the §14 anchor lives in MASTER-PROMPT.md:284 ("## 14. Known limits"), which this doc never names, and ADR-001's sections are unnumbered (Context/Decision/Consequences). A reader starting from this module doc cannot resolve "§14".
Why it matters: the patent/licensing pointer is the single highest-stakes reference in the file (distributing an HEVC decoder at all); sending the reader to a section number that exists in none of the cited documents is exactly how compliance pointers rot.
What correct looks like: "is a MASTER-PROMPT §14 (known-limits/matrix) item — doc 11 / ADR-010 territory".
Confidence: MEDIUM — Verification: doc 04 header grep (no §14), MASTER-PROMPT:284, ADR-001 header read; the anchor is inferable but never stated.

## 4. Per-function table (module root: wiring items, no functions)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| Module doc block (honesty boundary / structure / damage model) | 1-45 | REAL (claims) / UNREACHABLE (capability) | S3/S4 | F-001..F-004; doc-04 quote verbatim-accurate; matrix claims accurate |
| `pub mod decode;` | 47 | REAL | — | file exists; sole `Decoder` impl of the tree lives there (decode.rs:537) |
| `pub mod fuzz;` | 48 | REAL | — | in-repo deterministic sweeps, executed in baseline (:630-631) |
| `pub mod hw;` | 49 | REAL | — | adapter seam; zero non-test impls (W06-A06) |
| `pub mod parser;` | 50 | REAL | — | 5 submodules, all audited by W06-A05/A06/A07 |
| `pub use decode::HevcDecoder;` | 52 | UNREACHABLE | — | consumed only by crates/codecs/tests/hevc_hw_path.rs:25 |
| `pub use hw::{HevcHwBridge, HevcHwDecoder, HevcHwPath};` | 53 | UNREACHABLE | — | no production importer; tests import the rest via the `hw::` path directly |

## 5. Test honesty
No tests in this file (module root). The re-exports are exercised indirectly: baseline test-workspace.log.txt:1086-1088 — `hevc_hw_path` "running 17 tests" (would fail if the re-export paths were wrong: compile error, not silent pass). Content-sensitivity of those tests is owned by decode.rs/hw.rs audits.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no software HEVC decode here … routes the parsed result to a hardware session" | mod.rs:8-10 | REAL (library) / UNREACHABLE (shipped) | F-001 chain: zero production constructors; baseline hevc smoke → exit 65 |
| "with no accelerator negotiated it fails configure with the typed `hevc.hw.required` Unsupported error" | mod.rs:16-18 | REAL | decode.rs:434,570 (sed) |
| "Software decode remains `blocked`/`hw-only` in the feature matrix" | mod.rs:11-14 | REAL | feature-matrix.toml:445-452 (`blocked` ×4) |
| doc 04 §4.3 quote "ADR-001 hw-only, parser self-build M, software decode `blocked` with next-step recorded" | mod.rs:13-14 → doc 04 §4.3 | REAL | sed docs/04-codec-format-roadmap.md:308-310, verbatim |
| "patent/licensing … §14 item (doc 11 / ADR-010)" | mod.rs:19-22 → doc 04 §4.3 | REAL (substance) / dangling (anchor) | doc 04:306-307 carries the note; ADR-010 exists; §14 unresolvable from cited docs (F-004) |
| "Every parser entry point is total: any byte string … (the fuzz-lite suites pin this)" | mod.rs:39-41 | SHALLOW (overbroad) | fuzz.rs:19-20 excludes the slice surface (F-003) |
| "in-stream damage is skipped-and-counted recovery (`DecodeOutcome::Recovered`), never a panic" | mod.rs:42-45 | REAL (library) / UNREACHABLE (shipped) | decode.rs:716,724; codec-api lib.rs:119 |
| HEVC hw row "partial … hw seam delivered (5.G2), software decode absent" | feature-matrix.toml:432-443 | REAL (as stated) — note: `partial` describes a seam no shipped binary can reach | this audit's F-001 chain |

## 7. Dead / duplicated / unreachable code
The entire tree this root wires is production-dead in every shipped build (see F-001). This root's specific contribution: two `pub use` lines whose only consumer is a test file. The unconditional gate is `video/mod.rs:30-32` ("unconditional until the registration card's per-codec gates (doc 01 §2.5(a))") — an admitted, documented gate-deferral, not concealment. apps/web/Cargo.toml:92-96 claims the parsers "tree-shake out of an engine module that only dispatches decoders" — UNVERIFIED (would need a wasm size report; no cargo runs allowed this session).

## 8. Dependencies & flags
- No Cargo feature gates the hevc tree: codecs/Cargo.toml:26 comment + video/mod.rs:32 (unconditional `pub mod hevc;`) — the tree compiles into every shipped binary, including tauri desktop builds (apps/desktop-tauri/Cargo.lock carries the codecs crate).
- product.toml:168 `hw-decode = false` (ADR-001, Phase 7 pending), :191 `hw_decode_policy = "software-only"` — the hw lane the doc describes is doubly off: no decoder registration AND no accelerator negotiation.
- No external deps (in-house parser) — consistent with the crate's zero-dep claim for in-house modules.

## 9. Open questions for the next agent
- Wave-6 hostile question, answered for this tree: **H.264 decode IS reachable from the shipped build** — pipeline_host.rs:346 `"h264" | "avc1" => self.seeded(track, config, H264SwDecoder::new)` (import :65), the shipped binary's own refusal string lists `h264` among "landed" (:377), and baseline INDEX.md shows sample-420.mp4 **PLAYS**. HEVC, in contrast, has no lane at all (F-001). The asymmetry is honest in the matrix but not in this module doc.
- Does any wasm size report actually confirm the apps/web tree-shake claim for the hevc parser (§7)? docs/web/size-report.md owner should check.
- If a future card registers `HevcDecoder` in the CLI host, F-001 downgrades to a doc nit; the then-live refusal users would see is `hevc.hw.required`/`hevc.hw.no_bridge` — the es.rs test helper `decode_refusal` (apps/cli/src/containers/es.rs:669, `unreachable!("h264/hevc refuse decode")`) will need its h264 limb re-checked at that point (it already conflicts with the landed h264 sw lane — out of my file's scope, flagged for the es.rs owner).
