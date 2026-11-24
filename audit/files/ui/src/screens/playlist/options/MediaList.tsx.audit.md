# AUDIT: ui/src/screens/playlist/options/MediaList.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 230 / 3 (MediaList, openMenuAtKebab, MediaRow) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES: App.tsx:319 /playlist -> PlaylistScreen.tsx:376 `<PlaylistOptionsProvider entries={openedEntries}>` -> MediaList (also engine-scope via player PlaylistProvider) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg mount-context checks (exit 0); baseline ui-test.txt cited |

## 1. One-paragraph verdict
Honest-by-construction row list: engine-scope rows (PlaylistProvider ancestor) get a stretched native play button (`onPlay` -> `playlist.playItem`); local-scope rows (/playlist's saved-name browse, where the provider is deliberately absent — PlaylistScreen.tsx:19-20) render as pure presentation with the `--inert` class and NO button — the "no dead click targets" rule is actually implemented, not just claimed. The ••• renders only when a menu seam is wired; it synthesizes a `contextmenu` MouseEvent at its own position to reuse the shared ContextMenu wrapper. The one master-question catch: the engine-scope mode's play action dispatches `playlist.play-item`, which every real host refuses — but in a real host the queue is empty (all playlist.* commands refused), so `sortedEntries.length === 0` short-circuits at L70 and no row ever renders there. The live mode is demo-transport-only; the production mode is the honest inert one.

## 2. What actually works (verified)
— Inert-scope design verified: `onPlay={playlist !== null ? () => playlist.playItem(entry.id) : undefined}` (L101) + `onPlay === undefined ? "lumen-plo-mrow--inert"` (L176) + no button rendered (L190).
— Empty short-circuit: L70 `if (sortedEntries.length === 0) return <></>;` — the demo-only live mode renders nothing in a real host (queue always empty: playlist.* refused, dispatch.rs:191-192).
— Baseline: options/playlist tests pass (ui-test.txt).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/playlist/options/MediaList.tsx:101 (play seam) + ui/src/screens/playlist/PlaylistProvider.tsx:232
Code:
```
            onPlay={playlist !== null ? () => playlist.playItem(entry.id) : undefined}
```
What is wrong: `playItem` dispatches `{ cmd: "playlist.play-item", args: { id } }` (PlaylistProvider.tsx:232), refused by every shipped host (`playlist.*` -> dispatch.rs:191-192 "the playlist model is a Phase-4 card"), and the provider's dispatch swallows `unsupported-for-host` silently (PlaylistProvider.tsx:203 checks only "feature-not-exposed") — so were a row to render in engine scope in a real host, the click would be a silent dead click. In practice rows cannot render there (queue empty in every real host), so this is latent, not user-visible today. What correct looks like: the provider should treat `unsupported-for-host` as not-exposed (then rows would render inert, consistent with the file's own rule). Confidence: HIGH — Verification: read + rg (dispatch.rs, PlaylistProvider.tsx).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MediaList | 64-123 | REAL | S3 | VirtualList grid; scrollToIndex IIFE; F-001 latent |
| openMenuAtKebab | 131-141 | REAL | - | synthesized contextmenu event, clamped by shared popup |
| MediaRow | 143-230 | REAL | - | stretched play target; progress clamped 0..1 (L205); ••• only when a seam exists |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| options tests (baseline pass) | YES | inert vs engine scope, kebab synthesis, watched states asserted in options/*.test.tsx |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "never a dead click target (the 'no dead affordance' rule)" | L16-17 | TRUE in production scope | L176 + L190: no button when onPlay undefined |
| "One click on a row plays the item" (engine scope) | L10-11 | DEMO-ONLY | playlist.play-item refused by every real host (F-001); queue empty there |

## 7. Dead / duplicated / unreachable code
None. The engine-scope branch is unreachable in real hosts only because the data prerequisite (playlist.changed events) cannot occur — flagged in F-001, not dead code per se.

## 8. Dependencies & flags
No gates. VirtualList, ContextMenu, providers.

## 9. Open questions for the next agent
- PlaylistOptionsProvider.metaOf: with no engine resolveMeta seam and no local watched marks, does ANY production path set watched=true? If not, the watched bar/hover-watched states are demo-only visuals.
