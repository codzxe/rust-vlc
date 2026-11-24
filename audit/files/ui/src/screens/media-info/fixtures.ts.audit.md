# AUDIT: ui/src/screens/media-info/fixtures.ts
| Field | Value |
|---|---|
| Auditor | W15-A10 |
| Date | 2026-02-14 |
| Lines / functions | 159 / 3 exported fixtures |
| File verdict | REAL (test/demo fixture data — correctly labeled and contained) |
| Reachable from shipped entry point? | INDIRECT — test files (MediaInfoPanel.test, MediaInfoScreen.test) + client/demo-transport.ts:31,123,152 (the BADGED browser DemoTransport fallback; NOT the desktop TauriTransport path) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg consumers (2 test files + demo-transport only); rg MediaInfo type source |

## 1. One-paragraph verdict
Three fixture objects typed against the generated `MediaInfo` wire type
(compile-checked shape, not a hand-copied approximate): full multi-stream,
sparse-everything-unknown, live radio. They are consumed only by tests and by
the DemoTransport browser fallback — the desktop shell's real transport never
serves this data, and the demo is badged "Demo" in the chrome (main.tsx
comment). No mock-as-real leakage found.

## 2. What actually works (verified)
- Type-safety: `import type { MediaInfo } from "../../client/transport"` (line 6) — fixtures break at compile time if the wire shape changes.
- Wire units honored: duration `635_000_000` documented as µs (lines 10, 14) matching the w26.k17 µs contract.
- Consumer scan: only MediaInfoPanel.test.tsx, MediaInfoScreen.test.tsx, demo-transport.ts — zero production-screen imports.

## 3. Findings
None. Checked: mock-as-real leakage (none — demo-only + badged), shape drift
(compile-checked), duplication (three distinct cases, no copy-paste), honest-
unknown coverage (sparse case exercises null paths).

## 4. Per-function table (file >100 LOC; data constants)
| Export | Lines | Verdict | Notes |
|---|---|---|---|
| fullMediaInfo | 11-97 | REAL | 6 tracks, chapters, metadata, µs durations |
| sparseMediaInfo | 101-134 | REAL | honest-unknown case |
| liveAudioMediaInfo | 137-159 | REAL | infinite/live/unseekable case |

## 5. Test honesty
These fixtures are inputs to tests, not tests themselves. They exercise the
sparse/null rendering paths — the interesting cases for an honest-unknown UI.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "fixtures shaped EXACTLY like the `media.get-info` command result type" | fixtures.ts:1-3 | REAL | generated-type import, compile-checked |
| "Consumed by the panel/screen tests via FakeTransport" | fixtures.ts:4 | REAL | + additionally demo-transport.ts (undocumented consumer — see note) |

## 7. Dead / duplicated / unreachable code
None. Note: demo-transport.ts is an undocumented consumer of two fixtures —
harmless (badge covers it) but the header's consumer list is incomplete.

## 8. Dependencies & flags
Type-only import. No gates.

## 9. Open questions for the next agent
- demo-transport.ts auditor: confirm the "Demo" badge actually renders in
  the chrome when DemoTransport is active (claim from main.tsx comment).
