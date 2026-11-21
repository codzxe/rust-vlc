# AUDIT: ui/src/screens/player/dialogs/ToolsDialog.tsx
| Field | Value |
|---|---|
| Auditor | W15-A02 |
| Date | 2026-11-21 |
| Lines / functions | 169 / 6 |
| File verdict | SHALLOW (real composition + real trigger seam + strong tests — but every tool card it hosts is staged or local-only; the dialog configures nothing) |
| Reachable from shipped entry point? | YES (web app build): PlayerScreen.tsx:1203 mounts `<ToolsDialogHost />` (my part2 range), Ctrl+E opens it (PlayerScreen.tsx:808 `openToolsDialog("audio")`), ControlBar's gear/app-menu dispatch `lumen:open-tools`. Baseline #22 build OK, #23 vitest 1705 passed incl. ToolsDialog.test.tsx (17 cases). No GUI binary hosts it (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `Read` ToolsDialog.tsx 1-169 (complete); `rg -n "describe|it(" ToolsDialog.test.tsx` (17 cases incl. staged-marker assertions and the per-open host contract); `rg -n "export function useMsg" -A 6 messages.ts` (the `?? laneCatalog` fallback is sound — useMsg returns undefined for keys absent from both catalogs); `rg -ln "ToolsDialog" src --glob '*.test.tsx'` (also exercised via PlayerScreen.test.tsx's Ctrl+E suite); cross-refs from my own audits: PlayerScreen part2 (mount + Ctrl+E wiring), effects.tsx (`audio.set-eq` absent from catalog AND commands crate). |

## 1. One-paragraph verdict
A clean, well-tested composition shell: the decoupled `lumen:open-tools` trigger seam (mirroring `lumen:open-media`), a keyed per-open host, the Tabs primitive for the Audio/Video families, and 17 behavioral tests pinning chrome, tab behavior, tool mounts, staged markers, and fresh-state-per-open. What it composes, however, is nine staged tool cards: the Equalizer wrapper carries `data-pending-command="audio.set-eq"` — a command that exists in neither the generated catalog nor the commands crate (verified in my effects.tsx audit) — and the module header states the dynamics/visualizer/video cards carry their own staged markers with all state local. As VLC's "Adjustments and Effects" surface (Ctrl+E), the dialog adjusts and affects nothing; the honesty is thorough (markers, notes, tests), the function is absent. Minor seams: the `key as MessageKey` cast behind `useToolsMsg`, and the host's same-tab re-request does not remount.

## 2. What actually works (verified)
— Verified by baseline #23 (ui-vitest.log.txt, 1705 passed): all 17 ToolsDialog.test.tsx cases — titled shell + tab pair, initial focus on the tab (h19 P4), trap Tab-wrap, Escape close, both tool sets in frame order, collapsed-by-default cards, the Equalizer staged wrapper's `data-staged-tool`/`data-pending-command` attributes, the host's render-nothing-until-event / detail-preselects-tab / close-unmounts / `openToolsDialog` dispatches.
— Verified by read: `ToolsDialogHost`'s listener is add/removed correctly (158-159); the keyed remount on tab change (167) implements fresh tool state per open as documented; `asTab` normalizes unknown details to "audio" (defensive, honest default).
— Verified by read + messages.ts:3011-3016: `useToolsMsg`'s `m(key) ?? laneCatalog[key]` fallback is sound — `useMsg` returns `catalog[key] ?? messages[key]`, which is undefined only when the key is in neither catalog, exactly the lane-key case the fallback covers.
— Verified cross-file: PlayerScreen part2 (my audit) mounts the host and wires Ctrl+E; ControlBar's gear reaches the same seam.

## 3. Findings

### F-001 [S3]
Where: ui/src/screens/player/dialogs/ToolsDialog.tsx:116-131 (audio panel), 133-138 (video panel); module header 13-18
Code:
```tsx
          <div
            className="tools-dialog__staged-tool"
            data-staged-tool="equalizer"
            data-pending-command="audio.set-eq"
          >
            <EqualizerTool />
```
What is wrong: the dialog hosts nine tool cards and none of them writes to anything: the Equalizer awaits `audio.set-eq` (absent from ui/src/generated/commands.ts and crates/commands/src — verified in my effects.tsx audit; W7-A06: the engine's Eq3Band is constructed by nothing in any shipped build), and per the module header the dynamics/visualizer/video cards "carry their own honest staged markers" with "all state … local to the tools". Tool adjustments are discarded when the dialog closes (per-open fresh state is the tested contract).
Why it matters: as VLC-parity "Adjustments and Effects" (Ctrl+E), this is the product's effects surface in the player; every fader, slider, and toggle inside it is theater with disclosures. Rated S3 rather than S2 (effects.tsx's rating) because: the per-open ephemeral state is the DESIGNED and tested contract (VLC's own adjustments are session-scoped), the staged note renders on the expanded card the user must open, and the cards ship collapsed — the invitation-to-configure is weaker than the settings area's always-visible preset dropdown.
What correct looks like: the awaited commands (`audio.set-eq`, the dynamics family per my StereoWidenerTool audit to come) landing in the catalog + backend, at which point these wrappers' markers flip off; until then the dialog is a frame-accurate mock of VLC's tools window.
Confidence: HIGH — Verification: read (composition + markers traced; command absence established by rg in the effects.tsx audit; tests pin the staging).

### F-002 [S4]
Where: ui/src/screens/player/dialogs/ToolsDialog.tsx:78-81
Code:
```tsx
export function useToolsMsg() {
  const m = useMsg();
  return (key: ToolsDialogLabelKey): string => m(key as MessageKey) ?? laneCatalog[key];
}
```
What is wrong: the `key as MessageKey` cast lies to the type system — `ToolsDialogLabelKey` includes lane keys that are NOT `MessageKey`s; the runtime `??` fallback is what actually makes the seam total. A future refactor of `useMsg` to throw (or to a `Record<MessageKey, string>` lookup that TypeScript-narrows) breaks this silently at runtime, not at compile time.
Why it matters: type-safety hole in a seam pattern replicated across lanes (useEqMsg, useEffectsMsg, usePlaylistMsg all share the shape — makeAreaMsg centralizes it for the settings areas).
What correct looks like: `useMsg` typed `(key: string) => string | undefined` for seam consumers, or a `useMsgWith(laneCatalog)` helper that takes `string` keys and does the fallback internally.
Confidence: HIGH — Verification: read (both sides of the seam traced).

### F-003 [S4]
Where: ui/src/screens/player/dialogs/ToolsDialog.tsx:162-168
Code:
```tsx
  if (openTab === null) return null;
  // Keyed by the requested tab: a NEW open request (e.g. openToolsDialog
  // "video" while the audio tab is up) remounts the per-open dialog on the
  // requested tab — fresh tool state, the same per-open contract.
  return (
    <ToolsDialog key={openTab} open defaultTab={openTab} onClose={() => setOpenTab(null)} />
  );
```
What is wrong: a same-tab re-request (`openToolsDialog("audio")` while the audio tab is open) sets state to the same value → React bails out → the SAME key → no remount → tool state is NOT fresh for that re-open, deviating from the "per-open, fresh tool state" contract the comment and test (181) describe for the general case.
Why it matters: a user who expanded a card, changed local sliders, closed via Escape, and immediately re-opens with the same tab keeps stale local state in that edge path (dialog-unmount close resets it; event-while-open does not). Narrow edge, honest note in comment covers only the tab-change case.
What correct looks like: key on a monotonically increasing open sequence (`key={openSeq}`), keeping `defaultTab` for tab selection.
Confidence: HIGH — Verification: read (React key semantics traced; not executed).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `asTab` | 57 | REAL | — | Unknown → audio. Trivial, defensive. |
| `openToolsDialog` | 63-65 | REAL | — | The trigger-seam dispatch; consumed by PlayerScreen Ctrl+E + ControlBar; test-pinned. |
| `useToolsMsg` | 78-81 | REAL | F-002 | Sound runtime fallback; type-unsafe cast. |
| `ToolsDialog` | 99-142 | REAL (composition) | F-001 | Shell + Tabs + nine staged tool cards; Equalizer wrapper carries the staged markers. |
| `ToolsDialogHost` | 151-169 | REAL | F-003 | Event listener with cleanup; keyed per-open mount; same-tab re-open edge. |

## 5. Test honesty (ToolsDialog.test.tsx, 17 cases, ran in baseline #23)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| shell/tab/trap/focus cases (34-74) | YES | Focus-placement and wrap assertions (h19 P4 regression). |
| tool mount + staged marker cases (76-131) | YES | Frame-order mounts; the Equalizer wrapper's data-staged-tool/data-pending-command asserted exactly; sibling tools' own markers spot-checked. |
| video tab cases (133-160) | YES | Tab switch swaps the tool set; defaultTab preselects. |
| host seam cases (162-199) | YES | Nothing-until-event; detail preselects; close unmounts (fresh state next open); openToolsDialog dispatches. |
| NOT covered: same-tab re-request while open (F-003's edge) | — | No test re-dispatches the same tab while open. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "any surface asks for the tools dialog by calling `openToolsDialog(tab)` or dispatching `lumen:open-tools`" (20-23) | this file | REAL | PlayerScreen.tsx:808 (Ctrl+E) + ControlBar gear; host tests. |
| "The EqualizerTool (w26.c1) has no staged marker of its own — the wrapper below carries the same honest-unbound line … naming the awaited `audio.set-eq` command" (15-18) | this file | REAL | Wrapper at 117-126; test :108-115; the awaited command does not exist anywhere (effects.tsx audit rg evidence). |
| "Every card ships COLLAPSED per the frames" (13-14) | this file | REAL | Test :77 asserts collapsed mounts. |
| "all state is local to the tools" (14) | this file | REAL | Per-open fresh state is the tested contract (test :181) — and the substance of F-001. |

## 7. Dead / duplicated / unreachable code
- No dead code. `TOOLS_DIALOG_TABS` is exported and consumed by tests + render.
- Unreachable as a shipped binary (W14) — global UI condition.

## 8. Dependencies & flags
- Tabs/Tab/TabsList/TabsPanel (list primitives), ToolsDialogShell (w26.a18), nine tool cards (equalizer/dynamics/visualizer/video lanes), i18n lane seam. No feature flags.

## 9. Open questions for the next agent
- The eight sibling tool cards (Compressor/Spatializer/StereoWidener/Visualizer/Colors/Crop/Essential/Hdr) are other agents' files (StereoWidenerTool is mine — see its audit); the composite staging verdict (F-001) depends on their markers holding the same pattern.
- ToolsDialogShell.tsx (not mine): the h15 HDR dropdown-clipping repro that motivated Dropdown's clippingBounds came from this shell — verify its overflow contract still matches.
