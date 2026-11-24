# CLAIM AUDIT: docs/prototypes/video-in-webview.md
| Field | Value |
|---|---|
| Auditor | W19-A02 (recovered by W19-S2) |
| Date | 2026-11-24 |
| Claims extracted | 9 |
| REAL 6 / REAL-UNPROVEN 1 / FALSE 1 / TRUE-by-substance 1 | |
| Internal contradictions found | 1 (phantom harness vs "runs headless in CI") |

## 1. Blunt verdict
The prototype's measured substance is real and strong (asserted 11,666,667 ns exact
latency, 60-frame byte-exact round-trip test, golden-stream emitter, honest pending
matrix) — but its to-run list item #1 is broken in-tree: `apps/desktop/prototype/
harness.mjs` DOES NOT EXIST, so the documented command `node apps/desktop/prototype/
harness.mjs` is unrunnable, and the parenthetical "also runs headless in CI with
Node" is false (no CI step references it). A prior auditor (W13-A03) caught this,
built a substitute harness, and PASSED the conformance 8/8 — the substance is real,
the doc's execution path is not.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | apps/desktop crate: wire.rs, bridge.rs, mailbox.rs, transport.rs, host.rs, error.rs — 2,541 LOC, std-only | :12-17 | REAL (structure, grown) | ls apps/desktop/src -> all 6 files present (+8 later files); wc -l the 6 = 2766 today (record wrote 2541; crate grew post-record). |
| 2 | prototype/lumen-frames.mjs — dependency-free canvas-side reference, byte-for-byte mirror of wire.rs | :20-23 | REAL-UNPROVEN (mirror) | File exists (15840 B). "Byte-for-byte mirror" across languages is not literally checkable; the conformance substance was executed by W13-A03's substitute (below). |
| 3 | examples/emit_stream.rs emits a golden LUF1 stream (conformance corpus) | :24-25 | REAL | File exists; consumed by audit/runs/proto-test1/stream.bin + expected.json (W13-A03 golden pairs). |
| 4 | apps/desktop/tests suites (32 tests) | :29 | REAL (drift) | Census today: 10 `#[test]` in tests/ (pipeline 4, protocol 4, zero_alloc_transport 2) + 65 in src/ inline mods — the 32 figure is a historical count of the then-suite. |
| 5 | End-to-end pipeline latency: every frame min 11,666,667 ns, exact and reproducible | :35-39 | REAL | apps/desktop/tests/pipeline.rs:187-189 `assert_eq!(latency.min_ns, Some(11_666_667));` (+max/mean same) — the exact figure is asserted in a test, not narrated. |
| 6 | 60 frames through bridge→mailbox→transport with per-frame payload hash equality | :31-33 | REAL | apps/desktop/tests/protocol.rs:25 `bridge_stream_round_trips_sixty_frames_varied_sizes` asserts records.len()==60, monotonic ids, byte-exact payload per frame, repeats==8. |
| 7 | `node apps/desktop/prototype/harness.mjs` — the JS parser conformance (also runs headless in CI with Node) | :52-53 | FALSE | ls apps/desktop/prototype -> ONLY lumen-frames.mjs (no harness.mjs); no CI workflow references it (W13-A03 log:33 "harness.mjs DOES NOT EXIST anywhere ... 'runs headless in CI' — false"). Conformance SUBSTANCE true-by-execution: W13-A03 substitute harness decoded stream.bin via lumen-frames.mjs FrameDecoder — all 90 headers + FNV digests + head/tail windows match expected.json (8/8, exit 0; audit/runs/W13-A03.log.md:41). |
| 8 | Compatibility matrix: Node row proven (this sandbox), WebView2/WKWebView/WebKitGTK honestly "to-run [VERIFY]" | :61-68 | REAL | The matrix's only green row is the parser-only one; all per-OS rows carry [VERIFY] pending markers — matches the headless-sandbox reality (AGENT-SPEC §13: desktop-tauri build fails at gdk-sys, no webview runtime). |
| 9 | PROVISIONAL-B stands; per-OS verdicts remain ADR-003's own gates | :70-77 | REAL | docs/adr/ADR-003-video-in-webview.md exists; nothing in the record claims a per-OS verdict. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "node apps/desktop/prototype/harness.mjs ... also runs headless in CI with Node" (:52-53) | apps/desktop/examples/emit_stream.rs:14 and apps/desktop/README.md:36 give the same command — but the file is absent everywhere | The command is unrunnable in-tree: `ls apps/desktop/prototype/` shows only lumen-frames.mjs. All three citations reference a phantom. |

## 4. Fabrication / inflation indicators
One: the "runs headless in CI" parenthetical asserts an execution that does not exist
(anywhere: no workflow file references harness.mjs). Mitigated in substance: the
cross-language conformance itself was later executed successfully by an independent
auditor's substitute harness. The rest of the record is unusually honest (pending
matrix, exact to-run commands, "cannot run here" disclosure).

Commands run: ls/wc of apps/desktop/src, prototype, examples; rg -c '#\[test\]' (10 tests/ + 65 src); rg 11_666_667 + bridge_stream_round_trips in tests; ls prototype (no harness.mjs); cross-read audit/runs/W13-A03.log.md:32-45 + proto-test1/2 artifacts; ls docs/adr/ADR-003*.
