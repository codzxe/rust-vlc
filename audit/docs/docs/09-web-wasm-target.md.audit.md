# CLAIM AUDIT: docs/09-web-wasm-target.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 10 (spec provisions sampled from 337 lines) |
| REAL 4 / REAL-UNPROVEN 3 / PAPER-ONLY 1 / FALSE 0 / UNVERIFIED 2 (external BCD/browser facts) | |

## 1. Blunt verdict
A Wave-1 planning specification for the web target. As a spec it is disciplined: every provision is traced to MASTER-PROMPT sections, alternatives (wasm32-wasip1/threads/p2, JSPI) are evaluated with dated sources, and unknowns carry [VERIFY]. Its provisions were later genuinely landed in-tree (apps/web worker host, wasm32-unknown-unknown composition root) and verified in the baseline rc-smoke (browser-load PASS; wasm build passes with the wasm32 rust-std component installed). The one labeling defect: "Status: implemented (awaiting Lead review)" — "implemented" is the FEATURE-STATUS vocabulary applied to a planning document; a fresh reader cannot tell whether the doc describes shipped behavior (much of §2-§7 was unimplemented at write time; the header itself says "planning document — specification, no implementation").

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "The engine compiles to `wasm32-unknown-unknown`" | §2.1 | REAL | apps/web/Cargo.toml:20,46 — "all build on wasm32-unknown-unknown", cdylib note; baseline rc-smoke: wasm build passes with wasm32 rust-std installed |
| 2 | "`apps/web` is a static site: no server-side code" | §2.3 | REAL | apps/web = Cargo.toml, README, worker.js, audio-worklet.js, src — static assets only (ls) |
| 3 | "web-budget --check (issue #49 / doc 09 §2.4 — wasm size-budget + growth gate)" wired into full gate | §2.4 | REAL | audit/runs/baseline/full_gate.txt:76-82 — step 10 web-budget exists; FAILED in sandbox only because wasm32 target absent (E0463 "target may not be installed") — environment limit, recorded |
| 4 | "Status: implemented (awaiting Lead review)" (header) | :4 | PAPER-ONLY-as-status | the doc is a spec ("planning document — specification, no implementation" in its own Task row) — the "implemented" label misuses the matrix status vocabulary for a document state |
| 5 | "engine crates must not call `std::thread::spawn`, file, socket or clock APIs outside the adapter traits; ... wasm-lint rule" | §2.1 | REAL-UNPROVEN | check-architecture exists (baseline full_gate arch PASS); wasm-lint specifics unverified by me |
| 6 | "BCD-verified 2026-10-19 ... Wasm threads 74+/68+ ... WebGPU 113+" | §8.1 | UNVERIFIED | external browser facts; offline sandbox; the doc honestly dates the verification |
| 7 | "JSPI shipped Chrome/Edge 137, Firefox 153, Safari 27 [verified 2026-10-19]" | §2.2 | UNVERIFIED | same |
| 8 | "The `<video>`/MediaSource path is explicitly rejected as the primary pipeline" | §1 | REAL-UNPROVEN | engine-in-Worker decode path exists (w31.1 frame channel; rc-smoke browser-load PASS); but the UI mounts a `<video>` surface for URL sources (ui-test baseline: "url sources carry REAL bytes") — whether that violates "primary pipeline" is unresolved; open question logged |
| 9 | "web column ... `unsupported`/`blocked` require `blocked_reason`, `partial` requires `notes`" | §1 pillar 3 | REAL | observed in matrix rows 203/208 (my chunk): skins2 unsupported carries blocked_reason; av-tools partial carries notes |
| 10 | "Panic/abort messages surface as a typed fatal `playback.error { stage: host }` event — never a silent dead worker" | §2.3 | REAL-UNPROVEN | plausible vs the crash-face work (w30.6); not executed by me |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "It is not a `<video>`-tag player" (§1) | ui-test baseline: "a session mounts the `<video>` surface; url sources carry REAL bytes" | Both can hold (video element as URL-substrate vs primary pipeline) — unresolved; flagged |
| (doc dates BCD verification 2026-10-19; "re-verified per release" §8.1) | No re-verification record found in docs/web/ | UNVERIFIED — stale-by-3-weeks browser facts at audit time |

## 4. Fabrication / inflation indicators
The "Status: implemented" header row is the only inflation-adjacent item (ledger #4). Everything else is deferral-honest ([VERIFY] markers, [PROPOSAL] labels on every suggestion).

## 5. Matrix rows
Not a matrix chunk; doc 09 defines the web column discipline (pillar 3) that my rows-196-208 chunk complies with.
