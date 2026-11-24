# AUDIT: ui/src/screens/playlist/enqueueSources.ts
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2025-06-27 |
| Lines / functions | 67 / 2 (useEnqueueSources, enqueueEntries) |
| File verdict | SHALLOW (real wiring, demo-only data; permanently empty in the shipped desktop shell) |
| Reachable from shipped entry point? | YES — consumed by PlaylistPanel.tsx:110/129 and MiniPlayer.tsx:354/389 (the "+" add-to-queue menus) -> player screen |
| Findings | S0: 0 S1: 0 S2: 1 S3: 0 S4: 0 |
| Commands run | `rg -n "isDemoTransport\|DEMO_CLIPS\|recentOpenings\|isDesktopShell"` over this file + OpenMediaDialog.tsx:124-130; `sed -n '105,150p' PlaylistPanel.tsx` |

## 1. One-paragraph verdict
The enqueue entries, when they exist, are genuinely wired: each source maps
to `playlist.playLater` (real `playlist.enqueue` via PlaylistProvider). But
the SOURCE LIST is demo-only: `isDesktopShell()` excludes DEMO_CLIPS and
`isDemoTransport(client)` excludes recentOpenings — in the shipped desktop
shell BOTH rows are empty, so `useEnqueueSources()` returns [] and
`enqueueEntries` early-returns a single DISABLED "empty" entry, dropping even
the "Open media" fallback. The "+" queue button in the real app opens a dead
menu; everything pickable exists only in the browser simulation.

## 2. What actually works (verified)
— Code read: dedupe by file path/URL (L38-43) is correct; playLater closure
wiring (L62) -> PlaylistProvider.playLater -> `playlist.enqueue` (verified in
my PlaylistProvider audit). — OpenMediaDialog.tsx:124-130 uses the same
demo-gated listing (recent = demo-only, clips = demo-only) — the comment "the
same clips + remembered openings the open-media dialog lists" is accurate.
— PlaylistPanel.test.tsx covers the entries (not run here; baseline cited).

## 3. Findings
F-001 [S2]
Where: ui/src/screens/playlist/enqueueSources.ts:24-37 and 56-57
Code:
```
  const rows = [
    ...(isDesktopShell()
      ? []
      : DEMO_CLIPS.map((clip) => ({
```
and
```
  return sources.length === 0
    ? [{ id: "add-empty", labelKey: emptyLabel, disabled: true }]
```
What is wrong: in the desktop shell (the shipped product), both source rows
are gated off — demo clips by `isDesktopShell()`, remembered openings by
`isDemoTransport(client)` (the real TauriTransport is not a DemoTransport).
The result is always []; the "+" menu renders one disabled "empty" row and the
early return drops the `add-open-media` fallback entry that the demo path
gets. Desktop users get a permanently dead menu from a live-looking button.
Why it matters: realistic, shipped-host behavior — a primary queue-affordance
("add something") whose menu can never offer anything; the only working data
lives in the honest simulation.
What correct looks like: a desktop recents source (engine/config-backed), or
always include the add-open-media entry so the menu offers the manual path.
Confidence: HIGH — Verification: read + rg (both gates traced; transport type
check isDemoTransport at demo-transport.ts:990).

## 4. Per-function table
N/A (<100 LOC — compact audit). useEnqueueSources (L19-45) SHALLOW (demo-only
data); enqueueEntries (L47-67) REAL logic, S2 via the empty-case early return.

## 5. Test honesty
PlaylistPanel.test.tsx / MiniPlayer tests exercise the entries (browser-demo
path). Whether any test pins the DESKTOP-empty case: UNVERIFIED (those suites
are other agents' scope); the S2 path is likely untested.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the pickable enqueue sources — the same clips + remembered openings the open-media dialog lists (never fabricated)" | enqueueSources.ts:1-4 | REAL (and damming) | same demo gates in OpenMediaDialog.tsx:124-130 |
| "each wired to `playlist.playLater` (playlist.enqueue)" | enqueueSources.ts:4-5 | REAL | L62 + PlaylistProvider audit |

## 7. Dead / duplicated / unreachable code
None in-file; the desktop-empty branch (L56-57) is the shipped reality (F-001).

## 8. Dependencies & flags
components/menu MenuEntry, useClient, demo-transport (DEMO_CLIPS,
isDemoTransport), app-transport (isDesktopShell), useMsg. No feature gates
beyond the runtime host probes.

## 9. Open questions for the next agent
- Is there an engine/config-backed "recent openings" store that could feed
  this in the desktop shell (open-path.ts classifies, but who records)? If
  nothing records openings in the real host, the recents concept is
  demo-only repo-wide — worth a wave-level finding.
