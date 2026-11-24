# AUDIT: ui/src/screens/discovery/DiscoveryScreen.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | 343 / 4 describe blocks, 16 tests, EmitterTransport double + 2 helpers |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (vitest suite; baseline ui-test.txt:50 "DiscoveryScreen.test.tsx (16 tests)" ✓ PASS) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg suite in audit/runs/baseline/ui-test.txt |

## 1. One-paragraph verdict
A hostile capability-honesty test: it pins the honest EMPTY initial state (no fabricated volumes, zero commands sent — asserted via the transport double's sent log), folds real catalog events through the REAL subscribe seam, tests the pure reducer (re-add-replaces / remove-drops / unknown-id no-op / insertion order), pins the XSS-safety of untrusted labels (markup must render as text — no img element), and covers the full WAI-ARIA context-menu keyboard path (ContextMenu key, Shift+F10, plain F10 no-op, Escape restores focus to opener). The only double is EmitterTransport — a transport-port implementation (the sanctioned seam; the subject under test is the real screen component).

## 2. What actually works (verified)
- Baseline ui-test.txt:50: `✓ src/screens/discovery/DiscoveryScreen.test.tsx (16 tests) 337ms`.
## 3. Findings
Zero findings. Checked for: tautologies (none — every assertion names concrete rows/labels/ids/counts or event arrays), mocks-of-subject (none — the screen renders real; EmitterTransport mocks the dependency port only), ignored tests (none), vacuous greens (none — the honest-state test asserts transport.sent has length 0, an anti-fabrication pin).
## 4. Per-function table (test blocks)
| Block | Lines | Tests | Verdict | Notes |
|---|---|---|---|---|
| EmitterTransport + renderScreen | 34-68 | — | REAL | subscribe/send port impl; send returns a canned "completed unit" — only used for command-log assertions; screen never fabricates volumes from it |
| applyVolumeEvent (pure fold) | 74-109 | 3 | REAL | replace/remove/no-op/order semantics asserted exactly |
| DiscoveryScreen (/discovery) | 111-238 | 7 | REAL | no-backdrop lifecycle, W26 structure, honest empty state + honest network verdict text, Add-file CTA fires shell open-media event (window listener array), volume-added folding (labels/kinds/ids/count chip/empty-state gone), removal + unrelated-event isolation, XSS label safety |
| volume-row context menu | 240-343 | 6 | REAL | 6 pinned rows in order, focus-on-open, Add file fires the seam + dismiss, five no-seam rows aria-disabled, Escape closes, ContextMenu key path + focus restore, Shift+F10 vs plain F10 |
## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| honest initial state / sends nothing | YES | transport.sent length 0 — any fabricated snapshot command fails it |
| folds volume-added into rows | YES | row count + label + kind text + count chip all asserted |
| removal; other events ignored | YES | playback.rate-changed must not add rows — a naive event fold fails |
| untrusted label as text (V7) | YES | document.querySelector("img") must be null — an innerHTML sink fails |
| keyboard context menu | YES | focus assertions on opener restore; plain F10 must be a no-op |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the Solstice data-backdrop veil is GONE" (W25 retirement) | test :3-5 | REAL | asserted both mount and unmount (:112-117) |
| "no fabricated volumes, no commands sent" | test :8-9 | REAL | :146-147 queryAllByTestId length 0 + transport.sent length 0 |
| "SSDP/UPnP search is implemented but not reachable" honest verdict | test :136 | REAL-UNPROVEN | text asserted in DOM; the underlying engine capability claim belongs to the discovery/UPnP row audit |
## 7. Dead / duplicated / unreachable code
None.
## 8. Dependencies & flags
@testing-library/react, react-router MemoryRouter, vitest. The transport double implements the client transport port — no network, no engine. No feature flags.
## 9. Open questions for the next agent
- The canned send response `{ completed: { kind: "unit" } }` is never exercised by the screen (no command sent) — dead-ish branch in the double, harmless.
