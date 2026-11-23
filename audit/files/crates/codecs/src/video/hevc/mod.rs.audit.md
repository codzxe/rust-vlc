# AUDIT: crates/codecs/src/video/hevc/mod.rs
| Field | Value |
|---|---|
| Auditor | W06-A07 |
| Date | 2026-11-23 |
| Lines / functions | 53 / 0 (module root: 4 `pub mod` + 3 re-exports; all substance in the doc claims) |
| File verdict | REAL (module root; doc claims verified against children) |
| Reachable from shipped entry point? | NO — HevcDecoder has no registry lane (rg pipeline_host: zero hevc references); baseline mp4-smoke: HEVC MP4 exits 65 "no decoder for `hevc`" — consistent |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg wiring (pipeline_host: no hevc); rg hevc.hw.required (decode.rs:434 exists); baseline hevc suites cited |

## 1. One-paragraph verdict
A module root whose value is its honesty boundary: no software HEVC decode, parser + hw seam only, typed `hevc.hw.required` refusal without an accelerator, plus a patent-note pointer (§14/ADR-010). Every claim checks out against the children: the refusal code exists (decode.rs:434), the parser/fuzz/hw/decode children exist, the hevc_hw_path integration suite passes 17/17 in baseline, and the shipped registry indeed has no hevc lane (the engine's "no decoder" exit is the honest outcome, not a silent skip). The only finding is the same one as h264: the whole tree is shipped-unreachable.

## 2. What actually works (verified)
— verified `rg "hevc.hw.required" crates/codecs/src/video/hevc/decode.rs` -> L434: the typed refusal the doc promises is implemented.
— verified baseline audit/runs/baseline/cargo-test-workspace.txt:1184-1206 -> hevc_hw_path 17/17 ok in 0.03s; hevc_parser_corpus runs at :1207.
— verified shipped status: no hevc lane in apps/cli/src/pipeline_host.rs (rg zero hits) — matches the doc's "hw-only, blocked software" posture and the smoke's exit 65.

## 3. Findings
F-001 [S4]
Where: crates/codecs/src/video/hevc/mod.rs:14-18 (whole-tree reachability)
Code: (verbatim)
```
//! impl under this tree ([`decode::HevcDecoder`]) feeds hardware
//! adapters only: with no accelerator negotiated it fails configure
//! with the typed `hevc.hw.required` `Unsupported` error — the
```
What is wrong (status note): the entire HEVC tree (parser + hw seam) is unreachable from shipped entry points — no registry lane exists, and unlike h264 (which at least has the sw lane shipped), hevc has no shipped consumer at all. The doc does not claim one, so this is documented-substrate status, not a false claim. Confidence: HIGH — Verification: rg (zero wiring hits) + baseline smoke.

## 4. Per-function table
N/A — module root (4 `pub mod`, 3 `pub use`); no functions. Under 100 LOC.

## 5. Test honesty
No tests in this file; the children's suites (hevc_hw_path 17/17, hevc_parser_corpus) provide coverage — baseline-cited.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "no software HEVC decode here — parses and classifies ... routes to a hardware session" | L8-10 | REAL | decode.rs refuses typed without accelerator (L434); no sw path in tree |
| "Software decode remains `blocked`/`hw-only` in the feature matrix" | L10-11 | REAL | registry has no hevc lane; smoke exits 65 |
| "Every parser entry point is total ... the fuzz-lite suites pin this" | L39-41 | REAL-UNPROVEN | fuzz.rs + hevc suites pass in baseline; not executed by me (not in assignment) |

## 7. Dead / duplicated / unreachable code
The whole tree is shipped-unreachable (F-001) — mirrored with h264's S2 (decode.rs audit) but with the difference that hevc's docs never promise activation (no from_setting settings knob exists for hevc — nothing is dead-WIRED, merely unwired).

## 8. Dependencies & flags
Children: codec_api, media_types, parser/hw submodules. No feature gates at this level.

## 9. Open questions for the next agent
- hevc/decode.rs, hw.rs, parser/ are NOT in my assignment — the 5.G2 mirror claims (§7.4.2.4.4 AU assembly) need their own audit; the h264 decode.rs findings (non-VCL attachment, dead ambiguity branch, POC gap) may have twins there.
