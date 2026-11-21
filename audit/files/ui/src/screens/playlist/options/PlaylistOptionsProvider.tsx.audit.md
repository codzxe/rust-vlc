# AUDIT: ui/src/screens/playlist/options/PlaylistOptionsProvider.tsx
| Field | Value |
|---|---|
| Auditor | W15-A07 |
| Date | 2026-11-21 |
| Lines / functions | 162 / 4 (provider + hook + 2 types) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES in the UI bundle — mounted at PlaylistScreen.tsx:376 (`<PlaylistOptionsProvider entries={openedEntries}>`) around the /playlist library view; consumed by MediaList/ThumbnailGrid/SortingOptionsMenu/ViewOptionsMenu/fileRowMenu. W14 caveat: no shipped desktop binary (build-desktop-tauri.log.txt exit 101). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | Grep 'PlaylistOptionsProvider|resolveMeta=' src (production mounts: PlaylistScreen.tsx:376 ONLY, no resolveMeta — every other mount is harness/test); read sorting.ts header + metaSeamWired consumers (SortingOptionsMenu.tsx:8); state-derivation review of all useMemos/useCallbacks. |

## 1. One-paragraph verdict
A clean, well-memoized state container whose engine seam is deliberately unwired: the ONLY production mount (PlaylistScreen.tsx:376) passes local `entries` and no `resolveMeta`, so the entire metadata surface (engine-reported watched bars, artist/album/track/disc/add-date sorts) is inert in every shipped build — degraded honestly (the `metaSeamWired` flag drives a visible "needs metadata" hint, and the module header says the seam "lights up when the app shell wires" it). Mark-as-Watched works but is session-local React state that dies on navigation (documented in-file: "nothing persists — no engine command exists"). The provider logic itself is correct: overrides merge over engine meta, sorting is presentation-only, memo deps are complete.

## 2. What actually works (verified)
— Local-scope sorting verified by trace: entries ?? enginePlaylist ?? [] (110-113), sortPlaylistEntries pure call, value memo deps complete (137-153). Context guard throws with a precise message (158-162). Baseline: PlaylistOptionsProvider.test.tsx suites (defaults, state+sortedEntries, watched override, metaSeamWired, local scope) pass (ui-vitest.log.txt); `tsc --noEmit` clean (ui-build.log.txt).

## 3. Findings
### F-001 [S3]
Where: ui/src/screens/playlist/options/PlaylistOptionsProvider.tsx:71-72,84 (+ PlaylistScreen.tsx:376)
Code:
```
  /** Engine metadata seam — wire engine-reported values here (handoff). */
  resolveMeta,
```
```
  const enginePlaylist = usePlaylistOptional();
```
What is wrong: the engine metadata seam is wired by no production mount. PlaylistScreen.tsx:376 passes `entries={openedEntries}` only; every `resolveMeta` in the tree belongs to harness.tsx or tests. In every shipped build: engine meta is always undefined, watched bars never come from the engine, and artist/album/track/disc/add-date sorts are stable no-ops (the provider's own metaSeamWired=false path, hint shown).
Why it matters: a documented handoff that has not landed — the /playlist view's richer modes are demo/harness-only; the honest-degradation design (visible hint) keeps this S3 rather than S2.
What correct looks like: wire resolveMeta to engine track/metadata events (media.get-info results per entry) at the PlaylistScreen mount.
Confidence: HIGH — exhaustive mount grep.
### F-002 [S4]
Where: ui/src/screens/playlist/options/PlaylistOptionsProvider.tsx:100-103
Code:
```
      if (override === undefined) return base;
      return override
        ? { ...base, watched: true }
        : { ...base, watched: false, watchedProgress: 0 };
```
What is wrong: the unmark path (`override === false`) with NO engine meta (base === undefined) synthesizes a meta object from nothing: `{ ...undefined, watched: false, watchedProgress: 0 }` → `{ watched: false, watchedProgress: 0 }`. Marking a never-watched entry as unwatched invents a (truthful-by-default) meta record where "no data" existed.
Why it matters: the module's own honesty rule ("degrades honestly instead of inventing values", line 9-10) is bent on this branch; harmless in effect (false/0 IS the default reading) but inconsistent with the stated discipline.
What correct looks like: `return base === undefined ? undefined : { ...base, watched: false, watchedProgress: 0 }`.
Confidence: HIGH — JS spread semantics certain.
### F-003 [S4]
Where: ui/src/screens/playlist/options/PlaylistOptionsProvider.tsx:46-48,119-122
Code:
```
  readonly resolveMeta: ResolveEntryMeta | undefined;
  /** Meta lookup bound to entry identity — stable for consumers. */
  metaOf(entry: PlaylistEntry): PlaylistEntryMeta | undefined;
```
What is wrong: the context publishes the SAME function under two names (`resolveMeta: resolveMetaEffective` at 147, `metaOf` wrapping it at 119-122) — API duplication kept for k15 backward-compat, with the history documented.
Why it matters: two keys for one behavior invites future drift (a consumer mutates one meaning, not the other).
What correct looks like: deprecate `resolveMeta` on the context (keep the prop name), publish only metaOf.
Confidence: HIGH.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `PlaylistViewMode` / `DEFAULT_VIEW_MODE` | 29-32 | REAL | — | frame default "list" |
| `PlaylistOptionsValue` (interface) | 34-61 | REAL | S4 (F-003) | dual resolver keys |
| `PlaylistOptionsProvider` | 65-156 | REAL | S3 (F-001) | memo deps complete; local scope wins over engine queue |
| — `resolveMetaEffective` (inner) | 96-106 | REAL | S4 (F-002) | override merge; unmark-synthesizes branch |
| — `sourceEntries`/`sortedEntries` (inner memos) | 110-117 | REAL | — | presentation-only sort documented |
| — `metaOf` (inner) | 119-122 | REAL | S4 (F-003) | alias of resolveMetaEffective |
| — `setWatched` (inner) | 124-130 | REAL | — | session-only (documented: nothing persists) |
| — `metaSeamWired` (inner) | 135 | REAL | — | honest-degradation flag |
| `usePlaylistOptions` | 158-162 | REAL | — | guard throws precise error |

## 5. Test honesty
PlaylistOptionsProvider.test.tsx (other agent): defaults, sorting with resolveMeta fixtures, watched override merge, metaSeamWired transitions, local-scope entries, context guard — 6 suites that would fail on real mutations of this file's logic (the resolveMeta fixture path is the ONLY thing exercising the engine-meta merge — nothing production-side does, F-001).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "watched bars, artist/album/track/disc sorts and add-date ordering light up when the app shell wires the seam … until then the presentation degrades honestly" | PlaylistOptionsProvider.tsx:7-10 | REAL | no production mount wires resolveMeta (F-001); hint rendered via metaSeamWired |
| "No engine command exists (doc 05 §2.9 has no watched command) and nothing persists" | PlaylistOptionsProvider.tsx:55-59 | REAL | setWatched is React state only; catalog union has no watched command |
| "a nested engine PlaylistProvider would let playlist.changed events contaminate that view" | PlaylistOptionsProvider.tsx:12-16 | REAL-UNPROVEN | design rationale; contamination not reproduced here |

## 7. Dead / duplicated / unreachable code
F-003 (dual context keys). No unreachable code — the engine-meta branch is reachable only via tests/harness (F-001).

## 8. Dependencies & flags
PlaylistProvider (usePlaylistOptional), sorting.ts (DEFAULT_SORT_KEY, sortPlaylistEntries). Both exist; typecheck clean in baseline.

## 9. Open questions for the next agent
- sorting.ts owner: do the seam-dependent comparators document their no-op behavior when meta is undefined (the "stable no-ops" claim at 49-53)?
- PlaylistScreen.tsx owner: does openedEntries carry stable numeric ids across saved playlists (watchedOverrides is keyed by entry.id — cross-playlist id collisions would cross-contaminate marks)?
