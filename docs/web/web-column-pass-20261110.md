# Web feature-matrix column honesty pass (card 7.I1, issue #49)

Date: 2026-11-10 · Branch: `w24/a` · Verified: histogram + per-row reason
check over `docs/feature-matrix.toml`.

## Method

1. Every `status = { ... web = "<value>" }` cell was enumerated
   programmatically (Python, regex over the TOML source).
2. Each `blocked` / `unsupported` / `partial` web cell was checked for a
   reason on the row: `blocked_reason = "..."`, a nearby `reason`/`notes`
   field, or `evidence` + `notes` carrying the why.

## Result

| web value | rows | reason coverage |
|---|---|---|
| `not_started` | 206 | n/a (no claim made) |
| `blocked` | 27 | 27/27 carry `blocked_reason` (dependency-admission gates, ADR-010, conformance gates — e.g. codec-video-h264-software, codec-video-hevc) |
| `unsupported` | 15 | 15/15 carry row reasons |
| `partial` | 4 | 4/4 carry `evidence` + `notes` (playback-frame-step, playback-chapter-title, playback-bookmarks, playback-snapshot) |
| `implemented` (web) | 0 | none claimed — the conservative truth; the wasm module ships the engine core without builtin decoders (typed `web.decoders_absent` refusal, see `docs/web/browser-matrix-20261110.md`) |

Zero unexplained web cells. No web status was changed by this pass —
the cells already carried their reasons; this document records the
verification (the 7.I1 honesty-pass deliverable).

## Cross-references

- Browser matrix (the evidence behind the conservative web verdicts):
  `docs/web/browser-matrix-20261110.{md,json}`
- Size budgets + tree-shake evidence (7.K1): `docs/web/size-report.md`,
  `docs/web/size-record.toml`, `product.toml [web.size_budgets]`
- Gate: `cargo xtask web-budget --check` (full_gate step 10)
