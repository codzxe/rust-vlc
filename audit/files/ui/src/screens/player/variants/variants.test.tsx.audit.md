# AUDIT: ui/src/screens/player/variants/variants.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 596 / 25 tests across 7 describes |
| File verdict | REAL |
| Reachable from shipped entry point? | YES as test-infra for shipped UI — the tested components (PlayerScreen, MiniPlayer, variants model) are the shipped player; test itself runs under vitest (baseline ui-test.txt L32) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `rg -n "variants.test.tsx" audit/runs/baseline/ui-test.txt` → `32: ✓ src/screens/player/variants/variants.test.tsx (33 tests) 663ms`; baseline totals L178-179: 136 files / 1705 passed 1 skipped; `sed -n` quote verification |

## 1. One-paragraph verdict
The best test file audited in this assignment: 33 tests (all green in the committed vitest baseline) that render the real PlayerScreen through a mocked TRANSPORT (not a mocked subject), assert honest empty states, drive real keyboard paths, and verify real commands issued (playlist.enqueue/play-item/move asserted against transport.sent), plus layout-schema conditions, persistence round-trips, and CSS regression pins read from the real variants.css file. The no-echo limitation of MockTransport (rows cannot grow in jsdom) is documented in-test instead of hidden. The one defect is the cssRule helper's prefix-shadowing footgun — acknowledged and worked around in-test, but the helper itself still invites mis-pinned selectors.

## 2. What actually works (verified)
- Committed run: baseline ui-test.txt L32 `✓ src/screens/player/variants/variants.test.tsx (33 tests) 663ms`; file totals 136 passed / 1705 passed 1 skipped (L178-179).
- Real seams exercised: parsePlayerVariant rejects unknown values (L72-79); persistence round-trip via real localStorage (L81-87); enqueue issues the real command with exact source object (L272-276: `transport.sent.some((request) => request.cmd === "playlist.enqueue")` + args matchObject); keyboard activation issues playlist.play-item for ids [2,3] (L319-326); Alt+arrow issues playlist.move with exact toIndex sequence and edge no-op pin (L330-351).
- Honest-state assertions: "Nothing playing" (L230), "Nothing in the queue" (L247), disabled prev/next "— no media open" (L188-192), placeholder art without <img> and data-placeholder="artwork-engine-data-unwired" (L356-362).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/variants/variants.test.tsx:39-42
Code (verbatim, verified `sed -n '39,42p'`):
```
const cssRule = (selector: string): string => {
  const escaped = selector.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  return variantsCss.match(new RegExp(`\\n${escaped}[^{]*\\{([^}]*)\\}`))?.[1] ?? "";
};
```
What is wrong: `[^{]*` after the escaped selector lets `cssRule(".lumen-mini")` match the FIRST rule whose selector merely STARTS with `.lumen-mini` (e.g. `.lumen-mini__queue {` earlier in the file) — the prefix-shadowing the comment claims to prevent ("anchored so prefixed selectors cannot shadow each other", L35-38) is NOT what this regex does; the anchor prevents selectors ending-with shadowing, not prefix continuation. The file knows: L549-551 works around it by matching the bare card selector with a dedicated regex (`variantsCss.match(/\n\.lumen-mini \{([^}]*)\}/)`) and comments why. A future pin added with cssRule(".lumen-mini") would silently assert the WRONG rule's body (or "" → a false pass on `toContain` failure... actually empty string makes toContain FAIL, so wrong-rule match = either false fail or false pin). Why it matters: CSS pins are regression guards; a guard that can read the wrong rule is a guard that lies about which rule it guards. What correct looks like: terminate the selector at whitespace/comma in the regex (`(?:[ ,\n][^{]*)?\{`).
Confidence: HIGH — Verification: read; regex reasoning (mutation: rename the bare `.lumen-mini {}` rule and L552 pin fails while cssRule(".lumen-mini") pins elsewhere would keep passing on a different rule).
No other findings. Zero-finding checklist: tautology scan — every `expect` asserts observable DOM/transport/CSS state produced by the real component under test; no mocks of the subject (MockTransport mocks the backend seam and the tests assert what was SENT through it, the honest direction); no skipped/ignored tests in this file (the 1 skipped in the run is elsewhere — UNVERIFIED which file); names match bodies (checked each describe/it against its assertions).

## 4. Per-function table (file >100 LOC; test-file form)
| Test group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| variant model | 71-88 | REAL | — | parse/reject/persist round-trip |
| variant-scoped layout data | 90-128 | REAL | — | region ids, anchors, schema, custom-layout override, condition tokens |
| full variant | 130-162 | REAL | — | data-variant, no variant chrome, openPanels seam both directions |
| miniplayer variant | 164-352 | REAL | — | card anatomy, C2 state mapping reuse, expand seam, sr-only h1, queue panel DOM order, enqueue command, keyboard listbox + Alt reorder + edge no-ops |
| music variant | 355-436 | REAL | — | honest art placeholder, left dock, no-column state reachability both directions, live playlists-browser rows |
| keymap switching | 439-475 | REAL | — | B/G toggles, onVariantChange callbacks, persistence across remount |
| w26.gfix-F CSS pins | 482-539 | REAL | — | band color-mix/border/blur, tick field alive under portal, 44px pitch, radius pins, stage clamp math |
| w26.i04 CSS pins | 546-595 | REAL | S4 | cssRule footgun (F-001) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 33 (grouped above) | YES | assertions bind to rendered DOM roles/attributes, issued command payloads, and exact CSS substrings from the real stylesheet; e.g. breaking enqueue wiring fails L272-274; breaking Alt+move reorder fails L332-351; dropping a CSS pin fails the corresponding toContain. Verified green in baseline (L32), not re-run here (no npm per load discipline). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "anchored so prefixed selectors cannot shadow each other" | comment L35-38 | FALSE (anchor prevents suffix-shadowing only) | F-001 regex analysis + in-file workaround L549-551 |
| "MockTransport never echoes playlist.changed, so the popup path cannot grow rows in jsdom" | comment L279-282 | REAL-UNPROVEN | MockTransport impl not my file; consistent with the observed one-shot enqueue assertions |

## 7. Dead / duplicated / unreachable code
None. The helper was deduplicated to module scope (comment L36-38 cites j08 finding #9) — verified single definition.

## 8. Dependencies & flags
@testing-library/react, vitest, node:fs readFileSync for CSS pins, ClientProvider/MockTransport (client seam), real PlayerScreen/MiniPlayer/layout/variant modules. No env-dependent behavior beyond jsdom localStorage/fullscreen mocks (restored? requestFullscreen mock persists across file — configurable:true, cosmetic).

## 9. Open questions for the next agent
- Which file owns the run's 1 skipped test (baseline L179)? Not this file.
