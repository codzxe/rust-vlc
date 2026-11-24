# AUDIT: ui/src/screens/player/variants/variants.css
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 722 / 71 rule blocks (CSS) |
| File verdict | REAL (one dead selector) |
| Reachable from shipped entry point? | YES: imported by ui/src/screens/player/variants/index.ts (side-effect css import chain) -> PlayerScreen.tsx:127-136 -> /player route |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg/awk/python class cross-reference (exit 0); sed quote checks (exit 0) |

## 1. One-paragraph verdict
Real, disciplined styling: 70 semantic-token usages, raw hex only in two places each guarded by a stylelint-disable comment naming the frame and the "picture-frame rule" (L73-74, L455-457), every structural px annotated `frame:` with the measured source artboard. One selector is dead: `.lumen-music .lumen-icon-button` requires an ancestor carrying class `lumen-music`, and no element in the tree ever does (MusicMode's root is `lumen-music__art`, the screen root is `.lumen-player[data-variant="music"]`) — the "frame-exact rx4" icon rounding it promises never applies in music mode. Everything else cross-references to live class names (verified by extracting all 51 class tokens and matching them against string literals in ui/src non-CSS sources; only `lumen-music` and `lumen-vlist` came back unmatched, and `lumen-vlist` appears only inside a comment).

## 2. What actually works (verified)
— python cross-reference: 51 classes in sheet; 49 used literally in TSX (e.g. `lumen-mini` at MiniPlayer.tsx:151, `lumen-music__art` at MusicMode.tsx:113); `lumen-vlist` is comment-only (L398 references list.css, not a selector here).
— `data-active` attribute selector (L300) matches MiniPlayer.tsx:299 `data-active={queueOpen ? "" : undefined}`.

## 3. Findings
F-001 [S3]
Where: ui/src/screens/player/variants/variants.css:40-43
Code:
```
.lumen-music .lumen-icon-button,
.lumen-mini .lumen-icon-button {
  border-radius: var(--radius-inset); /* frame: Icon Wrapper.svg rx4 */
}
```
What is wrong: the `.lumen-music .lumen-icon-button` half is dead — no element in any rendered tree carries a bare `lumen-music` class (rg over ui/src non-CSS: zero matches; MusicMode.tsx:113 root is `lumen-music__art`; the screen root is `.lumen-player[data-variant="music"]`). The rx4 icon-wrapper rounding claimed "frame-exact" never applies to music mode's playlist-toggle IconButton. The `.lumen-mini` half works (MiniPlayer.tsx:151). Why it matters: silent style drift from the frame; dead code. What correct looks like: `[data-variant="music"] .lumen-icon-button` or add the class to MusicMode's root. Confidence: HIGH — Verification: read + rg cross-reference.

## 4. Per-function table (selector groups; CSS has no functions)
| Selector group | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `.lumen-music .lumen-icon-button` | 40-43 | DEAD (music half) | S3 | F-001 |
| `.lumen-mini` card | 45-60 | REAL | - | 391px frame-measured; tokens |
| `.lumen-mini__video` (+light-theme override) | 62-90 | REAL | - | justified raw hex #2b2b2b w/ stylelint-disable + reason |
| mini seek/title/meta/transport/controls | 91-299 | REAL | - | all classes matched to MiniPlayer.tsx |
| `.lumen-mini__queue-toggle[data-active]` | 300-302 | REAL | - | matches MiniPlayer.tsx:299 |
| `.lumen-mini__queue*` panel | 305-397 | REAL | - | frame-measured pitch/rows |
| `.app-shell__frame:has(...miniplayer...) > .app-shell__sidebar` | 420-424 | REAL-UNPROVEN | - | `:has()` support in the Tauri webview UNVERIFIED (no browser here); appshell.css owns the base class |
| `.lumen-music__art/stage/artwork/caption/title` | 426-640 | REAL | - | dark-in-both-themes override justified; ambient radial uses accent token, pointer-events:none |
| `.app-shell__strip > .lumen-player__controls[data-variant="music"]` | 641-646 | REAL | - | strip portal companion (PlayerScreen.tsx:1177-1199) |

## 5. Test honesty
No tests for CSS itself; variants.test.tsx covers DOM behavior of the components this sheet styles (baseline ui-test.txt: 1705 passed). Style claims (frame measurements) are untestable here — UNVERIFIED against actual .svg artboards (no renderer in sandbox).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every color below is a semantic token; raw px appears solely for frame-measured structural dimensions" | L9-11 | TRUE (2 documented exceptions) | rg: raw hex only at L74, L457, both stylelint-disabled with reasons |
| "scoped here so this lane's surfaces are frame-exact" (rx4) | L36-39 | FALSE for music mode | F-001: selector dead |

## 7. Dead / duplicated / unreachable code
`.lumen-music .lumen-icon-button` (F-001). No duplication found.

## 8. Dependencies & flags
Pure CSS, token-driven; no feature flags. Depends on tokens (ui/src/tokens) and appshell.css classes (cross-file selectors, both verified to exist).

## 9. Open questions for the next agent
- Does the Tauri production webview (webkitgtk on Linux) support `:has()`? If not, L423 rule is dead at runtime too.
