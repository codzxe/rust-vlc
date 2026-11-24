# AUDIT: ui/src/screens/playlist/options/PlaylistOptionsProvider.tsx
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-24 |
| Lines / functions | 163 / 1 provider + 1 hook (+4 inner callbacks/memos) |
| File verdict | REAL state container; the engine-metadata seam it exists to carry is wired ONLY by test harnesses — never in production |
| Reachable from shipped entry point? | YES (PlaylistScreen.tsx:376 mounts it with `entries` = local saved-playlist scope) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | rg `PlaylistOptionsProvider|resolveMeta=` src -g '!*.test.*' (exit 0 — production mounts pass no resolveMeta; only harness.tsx/optionsTestSupport.tsx do); read full file |

## 1. One-paragraph verdict
Clean, honest-degradation state container for sort key, view mode, sorted (presentation-only) entries, and the metadata seam. Two product-level gaps: (1) the `resolveMeta` engine seam — the entire reason artist/album/track/disc/watched metadata exists — is passed ONLY by test harnesses (rg: harness.tsx:57/80/118, optionsTestSupport.tsx:70/99; PlaylistScreen.tsx:376 passes none), so `metaSeamWired` is false in every shipped build until the user hand-marks something watched; (2) `setWatched` overrides live in a bare `useState` Map — the file itself says "nothing persists" — so Mark-as-Watched marks evaporate on reload AND on provider unmount (leaving /playlist), while being presented as a real content-state feature. The disclosure comments are accurate; the shipped behavior is a metadata-optional shell whose persistent-looking marks are ephemeral.

## 2. What actually works (verified)
— verified by read: sorting is presentation-only (L108-117, never mutates the engine queue — matches sorting.ts contract); local-scope override (entries prop) cleanly wins over the optional engine scope (L110-113); the watched-override merge is pure and override-wins (L96-106).

## 3. Findings
F-001 [S3]
Where: ui/src/screens/playlist/options/PlaylistOptionsProvider.tsx:71-72, 135 (vs PlaylistScreen.tsx:376)
Code:
```
  /** Engine metadata seam — wire engine-reported values here (handoff). */
  resolveMeta,
```
What is wrong: no production call site wires `resolveMeta` (rg across non-test sources: zero `resolveMeta=` outside harness/test-support). The "handoff" has not landed, so in the shipped app the metadata half of the sorting feature (10/16 keys per sorting.ts F-001) is inert and the "Needs media metadata" hint is the permanent state. The provider is architecture-for-a-future-backend shipped today. Confidence: HIGH — Verification: ran rg (exit 0).
F-002 [S3]
Where: ui/src/screens/playlist/options/PlaylistOptionsProvider.tsx:55-60, 88-90, 124-130
Code:
```
  /** w26.gfix-E — Mark as Watched: a client-local watched override merged
   * over `resolveMeta` ... No engine command
   * exists (doc 05 §2.9 has no watched command) and nothing persists — the
   * row's watched state + the watched-first sorts are the real, visible
   * effect. */
```
What is wrong: the feature exists only as component-lifetime React state: navigating away from /playlist (provider unmount) or reloading destroys every watched mark. A user who marks a series watched, browses elsewhere, returns — marks gone. Disclosed in a comment the user never sees; the UI affordance (context-menu row) presents as durable content state. Confidence: HIGH — Verification: read (no persistence layer anywhere in the module).
F-003 [S4]
Where: ui/src/screens/playlist/options/PlaylistOptionsProvider.tsx:46-48, 119-122
Code:
```
  readonly resolveMeta: ResolveEntryMeta | undefined;
  ...
  metaOf(entry: PlaylistEntry): PlaylistEntryMeta | undefined;
```
What is wrong: `metaOf` is a trivial pass-through wrapper of `resolveMetaEffective` and BOTH are published on the context (L147-148) — two public names for one lookup, kept for history (j15 P3-3). Consumers can take either; drift risk if one ever diverges. Confidence: HIGH — Verification: read.
F-004 [S4]
Where: ui/src/screens/playlist/options/PlaylistOptionsProvider.tsx:85-86
Code:
```
  const [sortKey, setSortKey] = useState<PlaylistSortKey>(initialSortKey);
  const [viewMode, setViewMode] = useState<PlaylistViewMode>(initialViewMode);
```
What is wrong: sort key and view mode are component state only — not persisted (contrast: player variant persists via localStorage, App.tsx:601-604 region). User's chosen sort/view resets on every /playlist remount. VLC-parity behavior would persist. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| DEFAULT_VIEW_MODE | 32 | REAL | — | "list" frame default |
| PlaylistOptionsProvider | 65-156 | REAL | S3 | seam unwired in product (F-001) |
| resolveMetaEffective | 96-106 | REAL | — | pure merge, override wins |
| sourceEntries / sortedEntries | 110-117 | REAL | — | presentation-only ordering |
| metaOf | 119-122 | REAL | S4 | duplicate public lookup (F-003) |
| setWatched | 124-130 | REAL / ephemeral | S3 | session-only marks (F-002) |
| metaSeamWired | 135 | REAL | — | honest flag; false in product until user marks |
| usePlaylistOptions | 158-162 | REAL | — | throws without provider |

## 5. Test honesty
optionsTestSupport/harness wire resolveMeta — meaning every metadata test runs against a seam NO production mount provides. Tests prove the container logic, not the product behavior. Would fail if merge logic broke — honest about the container, silent about the unwired seam. Baseline vitest covers.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "watched bars, artist/album/track/disc sorts and add-date ordering light up when the app shell wires the seam" (L7-10) | header | REAL as promise / unwired | rg: no production wiring (F-001) |
| "nothing persists — the row's watched state ... are the real, visible effect" (L57-59) | comment | REAL | accurate disclosure of F-002 |
| "No engine PlaylistProvider is required in that mode" (L75-76) | comment | REAL | usePlaylistOptional returns null cleanly |

## 7. Dead / duplicated / unreachable code
`resolveMeta` prop is production-dead (test-only consumers). `metaOf`/`resolveMeta` duplication (F-003).

## 8. Dependencies & flags
PlaylistProvider (optional read), sorting.ts. No transport, no feature flags — deliberately UI-state only.

## 9. Open questions for the next agent
- Should watched overrides join `playlistsStore`-style localStorage instead of useState? (F-002 — one-line-ish fix with an existing persistence pattern in the same directory.)
- When the engine metadata seam lands, PlaylistScreen.tsx:376 must pass it — add a drift test asserting a production mount wires resolveMeta (today none exists).
