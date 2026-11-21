# AUDIT: ui/src/screens/playlist/options/SortingOptionsMenu.tsx
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-21 |
| Lines / functions | 41 / 1 (SortingOptionsMenu) + the sorting model it renders (sorting.ts, 199 lines, read in full) and the provider state it drives (PlaylistOptionsProvider.tsx, 163 lines) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: /playlist route → PlaylistScreen.tsx:376-384 `<PlaylistOptionsProvider entries={openedEntries}><PlaylistBrowse …/>` → PlaylistOptionsToolbar.tsx:32 `<SortingOptionsMenu align={align} />` — the browse view's options row) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | Read full file + sorting.ts + PlaylistOptionsProvider.tsx + OptionsMenus.test.tsx (162 lines) · `rg -n "PlaylistOptionsProvider\|resolveMeta=" ui/src` (production mounts: PlaylistScreen.tsx:376 ONLY — no resolveMeta; harness/test mounts wire it) · `rg -n "SortingOptionsMenu" ui/src` (toolbar + harness + tests) · `sed -n` on every quoted range |

## 1. One-paragraph verdict
The menu itself is 41 lines of clean wiring over an unusually well-built sorting model: 16 frame rows in frame order, deterministic total stability (ties break by queue index — never engine-order roulette), natural-order title/URI comparison, and honest degradation as a design principle — unknown metadata never reorders, and the 10 metadata-dependent rows visibly say "Needs media metadata" while nothing can influence them (test-pinned at exact row indices, and the hint correctly disappears both when a resolver is wired and when a local Mark-as-Watched override exists). The honest-degradation claim survives the production trace: the ONLY production mount (PlaylistScreen.tsx:376) wires NO resolveMeta, so the shipped menu really does show its own limits. The one real defect lives in the state beneath it: the provider is component state with no persistence, mounted inside the route — the user's Mark-as-Watched marks, sort selection, and view mode are all silently discarded the moment they navigate away from the browse screen, a fact disclosed in a provider code comment but nowhere at the interaction point where the user creates the marks.

## 2. What actually works (verified)
— Frame data, test-pinned: all 16 rows in `Sorting Options.svg` order with frame copy, "Unwatched First" the single aria-checked default, plain rows with no icons/shortcuts (OptionsMenus.test.tsx:30-58).
— Selection is real end-to-end: clicking "Title A-Z" re-sorts the provider's entries through the actual view rows (test :60-77 asserts the reordered row text).
— The honest hint mechanism is exact: 10 of 16 rows hint when unwired; title/uri rows never hint; newest/oldest correctly stay unhinted (they ride the honest add-order proxy — queue position IS add order); hints drop with a wired resolver OR one local watched mark (tests :80-125).
— The sorting model (sorting.ts): unknown-first comparators, natural compare, total stability via index tiebreak, `sortPlaylistEntries` pure and non-mutating, presentation-order-only (no `playlist.move` dispatched — header :179-180 verified).
— Production reachability: the /playlist browse screen mounts the whole stack (PlaylistScreen.tsx:376).

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/playlist/options/PlaylistOptionsProvider.tsx:55-60, 85-90 (manifests through SortingOptionsMenu's watched-first/unwatched-first rows); mount scope PlaylistScreen.tsx:376
Code:
```
  /** w26.gfix-E — Mark as Watched: a client-local watched override merged
   * over `resolveMeta` (the file context menu's row 2). No engine command
   * exists (doc 05 §2.9 has no watched command) and nothing persists — the
   * row's watched state + the watched-first sorts are the real, visible
   * effect. */
```
```
        <PlaylistOptionsProvider entries={openedEntries}>
```
What is wrong: all user-created state in the options stack — Mark-as-Watched overrides, the selected sort key, the view mode — is `useState` inside a provider that mounts inside the /playlist route. Navigating away (or switching the browsed playlist) unmounts it and silently discards everything: the user's watched marks vanish, the sort resets to the frame default, the view resets to list. The disclosure exists only in this provider comment; the file-row context menu that creates a watched mark gives no hint the mark is session-scoped.
Why it matters: a user who marks episodes watched (the one action that makes the default "Unwatched First" sort meaningful — with no marks it is a stable no-op) loses the work on the next navigation and cannot know why; "watched" is a persistence-shaped concept in every media library. This is the W11 settings-never-persist pattern in miniature, here at least honestly documented in code — but the documentation is invisible to the user.
What correct looks like: persist `watchedOverrides` (and arguably sortKey/viewMode) to localStorage under a `lumen.*` key the way playlistsStore/profilesStore do, or label the Mark-as-Watched action as session-only in its menu copy.
Confidence: HIGH — Verification: read of provider + mount site; no persistence path exists in the module (no localStorage import); the reset-on-unmount behavior follows from React state semantics.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| SortingOptionsMenu | 18-41 | REAL | — | pure wiring over SORT_KEYS/sortLabelKey/sortUsesMeta; the `id as PlaylistSortKey` cast is safe (ids are SORT_KEYS); hint expression implements the honest-degradation contract exactly |
| — dependency: sortPlaylistEntries / compareDecorated (sorting.ts:119-198) | REAL | — | deterministic, stable, non-mutating; unknown-first; natural compare |
| — dependency: PlaylistOptionsProvider state (85-135) | REAL | S3 | F-001 ephemeral watched/sort/view state |

## 5. Test honesty (if tests exist in/for this file)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| OptionsMenus.test.tsx:30 16 rows in frame order with frame copy | YES | exact labels at exact indices |
| :45 default checked row is Unwatched First | YES | single aria-checked assertion |
| :53 rows carry no icons | YES | absence assertion |
| :60 selecting Title A-Z re-sorts entries | YES | end-to-end through the real view rows |
| :81 unwired → 10 hints at exact indices, real sorts unhinted | YES | the honest-degradation contract, per-row |
| :99 wired resolver → no hints | YES | the frame-pure state |
| :107 local watched mark also drops hints | YES | the metaSeamWired OR-condition |
All green: baseline ui-vitest.log.txt (1705 passed). GAP: no test covers state persistence across provider unmount/remount — F-001's behavior is unpinned.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "the frame's 16 rows, no icons, no shortcuts, row 1 ('Unwatched First') checked by default" | header :1-3 | REAL | SORT_KEYS (16); DEFAULT_SORT_KEY; tests :30-58 |
| "metadata-dependent rows render a quiet honest-degradation hint … while nothing can influence their order — no engine resolveMeta seam AND no local watched marks" | :5-8 | REAL | test :81 pins 10 hints; production mount wires no resolver (PlaylistScreen.tsx:376) |
| "the hint appears only in the unwired state (harness/frame mounts wire a resolver, so comparison shots stay frame-pure)" | :9-10 | REAL | harness.tsx:57 wires HARNESS_META; tests :99 |
| (sorting.ts) "unknown metadata never reorders the queue (it sorts last, stably) instead of pretending a value" | sorting.ts:7-9 | REAL | knownFirst + index tiebreaks, read line by line |
| (provider) "No engine command exists … and nothing persists" | provider :56-58 | REAL — and is F-001's defect | no persistence path in the module |

## 7. Dead / duplicated / unreachable code
- None in this file. (sortLabelKey's template type guarantees the i18n key shape at compile time — good hygiene.)

## 8. Dependencies & flags
- PlaylistOptionsMenu (the generic radio menu), sorting.ts, PlaylistOptionsProvider, useOptionsMsg lane seam. No engine commands fired by sorting (presentation-order-only, verified). No feature flags.

## 9. Open questions for the next agent
- PlaylistOptionsMenu.tsx (the generic menu shell rendering the hint span `.lumen-plo__hint`) is not in my file set — its auditor should verify the hint is announced accessibly (it currently rides inside the menuitem's content).
- fileRowMenu.ts ("Mark as Watched", row 2) — the interaction point F-001 says should disclose session scope.
- i18n: all 16 `playlist.options.sort.*` keys + `needs-meta` — spot-checked registered (test uses real copy through the lane seam); full l10n coverage is the catalog check's job.
