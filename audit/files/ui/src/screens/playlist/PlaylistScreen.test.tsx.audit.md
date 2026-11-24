# AUDIT: ui/src/screens/playlist/PlaylistScreen.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-A04 |
| Date | 2026-11-24 |
| Lines / functions | 532 / 6 describe blocks, 20 it() + 11 nested = 31 tests, 3 helpers + 1 seam class |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (vitest suite; runs in `npm run test` lane — baseline ui-test.txt:35 "PlaylistScreen.test.tsx (31 tests)" ✓ PASS) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg suite name in audit/runs/baseline/ui-test.txt |

## 1. One-paragraph verdict
A genuinely hostile component test: it renders the real PlaylistScreen, drives real DOM events (clicks, keyDown, file input), and asserts semantic structure (roles, aria-activedescendant, data-active, classnames tied to shipped CSS), interaction outcomes (sort re-order, view switch, watch-toggle state, add-to-playlist store commits observed through a capturing persistence seam), and honest limitations (Play disabled with aria-disabled "true" — no catalog command; thumbnail picker disabled with a titled hint). No mock of the subject; the only double is CapturingSeam, a persistence-port implementation injected via the store's public seam — legitimate dependency injection, not a mock-of-subject.

## 2. What actually works (verified)
- Baseline ui-test.txt:35: `✓ src/screens/playlist/PlaylistScreen.test.tsx (31 tests) 1165ms` — green run cited (vitest not re-run here; npm-run forbidden by load discipline).
- Mutation reasoning applied throughout (below); no tautologies found: every assertion names concrete rendered content or state deltas.
## 3. Findings
F-001 [S4]
Where: PlaylistScreen.test.tsx:24-29 + playlistsStore export
Code:
```
import {
  resetPlaylistsForTests,
  type PlaylistsPersistence,
  type PlaylistsState,
  type SavedPlaylist,
} from "./playlistsStore";
```
What is wrong: `resetPlaylistsForTests` is a test-only reset hook exported from the PRODUCTION store module (ships in the module graph unless tree-shaken away; a test seam baked into prod code). It is the sanctioned DI pattern here (the seam also gives the suite hermetic state), so hygiene-level only. Confidence: HIGH — Verification: read.
## 4. Per-function table (test blocks)
| Block | Lines | Tests | Verdict | Notes |
|---|---|---|---|---|
| CapturingSeam + helpers | 32-71 | — | REAL | persistence port impl; load/save capture; not a subject mock |
| library panel | 73-118 | 4 | REAL | heading count, seeded rows, natural-sort order (10 vs 2 — would fail on lexical sort), kebab menu + disabled-honest Play + Remove deletes |
| opened-playlist browse | 120-286 | 10 | REAL | browse mount (4 items, real toolbar triggers), Grid View switch (cards=4, rows gone, honest "—" durations), Title Z-A re-order (exact order asserted), back-with-selection, empty state, inert rows + real file menu (12 frame rows exact), watched toggle, add-to-playlist store commit, Add-Subtitles dialog scoped, row-menu Open |
| Add-subtitles action | 288-308 | 2 | REAL | toolbar + row-menu entry both open the dialog; honest "No subtitle files found" (no folder-scan seam fabricated) |
| create-playlist dialog | 315-390 | 7 | REAL | Cancel/✕ close, focus-on-title-input (macrotask wait documented), Save gate until non-empty title, Save persists (seam commit asserted), thumbnail disabled-honest, drop zone = label for input |
| honest empty state | 392-405 | 1 | REAL | quiet state + teaching CTA opens dialog |
| dialog reset (w26.k14) | 410-434 | 1 | REAL | route-change shape (open=false without onClose) leaves clean sheet — rerender-driven, no stale title/files |
| arrow-key grid navigation | 442-532 | 6 | REAL | APG grid keyboard model: aria-activedescendant movement incl. clamping at ends, Enter/Space activation, no-active-row Enter no-op, kebab keys keep own behavior, media grid Enter opens file menu, real focus path |
## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| natural-sorts numbered titles | YES | exact array order ["Cool Playlist","Playlist 2","Playlist 10"] — a lexical sort fails it |
| View options → Grid View | YES | card count 4 + rows [] + four honest "—" — a list/grid mixup fails |
| Title Z-A re-orders | YES | exact 4-element order asserted |
| Mark as Watched toggles | YES | watched-class count 0 → >0 after toggle |
| Add to Playlist appends | YES | cross-playlist row visible after commit |
| Save persists through seam | YES | seam.saved.at(-1) contains the created playlist |
| arrow-key grid model | YES | aria-activedescendant ids asserted per step; a broken keydown handler fails step 2 |
| disabled-honest assertions | YES | aria-disabled="true" / .disabled — an over-eager enabled Play would fail the honesty pin |
## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "(31 tests)" green | audit/runs/baseline/ui-test.txt:35 | REAL | cited |
| frame-row order + copy pins (file context menu.svg) | test comments :205-227 | REAL-UNPROVEN | asserted in-test; visual frame SVGs not re-checked here (visual lane UNVERIFIED) |
## 7. Dead / duplicated / unreachable code
None. containerHasCards helper (:311-313) used once — fine.
## 8. Dependencies & flags
@testing-library/react + vitest; no engine/network mocks — the suite tests the UI with the local store only, honestly scoped ("no catalog command", "no folder-scan seam" comments match asserted DOM).
## 9. Open questions for the next agent
- The 12-row file-menu copy pins text content exactly — every future menu re-copy breaks the test loudly (by design). Confirm i18n run does not localize these labels in the shipped locale set (if localization lands, these pins need locale-scoped fixtures).
