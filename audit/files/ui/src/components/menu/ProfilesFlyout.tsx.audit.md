# AUDIT: ui/src/components/menu/ProfilesFlyout.tsx
| Field | Value |
|---|---|
| Auditor | W15-A08 |
| Date | 2026-11-24 |
| Lines / functions | 214 / 10 (persistence pair, hydrate/coerceProfile, store trio, selectProfile, createProfile, nextId, buildProfileFlyout, resetProfilesForTests) |
| File verdict | REAL (honest local-only surface) |
| Reachable from shipped entry point? | YES — ControlBar.tsx:903,1016-1020 (player app-menu Profile submenu) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg consumers (ControlBar only); read of store logic; edge walk (corrupt store, empty list, SSR guard) |

## 1. One-paragraph verdict
An honestly-labeled local-only feature: the header states "there is no engine profile command in the catalog, so profiles are LOCAL device data... no row claims engine capability", and the code matches — select/create persist to localStorage `lumen.profiles.v1` with correct corrupt-store fold-back, an SSR import guard, seed-once semantics, and collision-safe quick-create naming. The dishonesty is one level up: the Profile menu implies a media-library-profile feature (Movies/Shows/Music/Work) that affects nothing anywhere — no consumer reads `selectedId` beyond the flyout's own check mark.

## 2. What actually works (verified)
— verified by read + rg: store round-trip (hydrate/commit/subscribe via useSyncExternalStore), seed never resurrects (empty list stays empty), unknown-id select is a no-op, `+ New` creates `Profile N` with taken-name skipping, ControlBar mounts the flyout with real handlers.
— The single consumer census (rg): only ControlBar.tsx — selection affects no other surface.

## 3. Findings
F-001 [S3]
Where: ui/src/components/menu/ProfilesFlyout.tsx:113-117 (SEED) + ControlBar.tsx:1016-1021 (mount)
Code:
```
const SEED: readonly MediaProfile[] = [
  { id: "movies", name: "Movies" },
  { id: "shows", name: "Shows" },
```
What is wrong: a first-run user sees Movies/Shows/Music/Work profiles — fixture rows from Profiles.svg — that gate nothing, filter nothing, and select nothing; switching profile changes exactly one check mark in one menu. The module admits local-only honestly, but the shipped UX (a player app menu named Profile with four named collections) implies capability. Why it matters: dead-surface theater at the app-menu level; the seed rows are the frame's art, not user data. What correct looks like: either hide the submenu until a consumer exists, or wire selection to a real library filter. Confidence: HIGH — Verification: read + rg (no other consumer).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| localStorage/memory persistence | 51-77 | REAL | - | corrupt-store + quota guards |
| coerceProfile/hydrate | 80-103 | REAL | - | malformed rows dropped; empty-list-is-valid |
| store (commit/subscribe/getSnapshot) | 105-135 | REAL | - | SSR window guard |
| useMediaProfiles/getProfilesState | 137-145 | REAL | - | |
| selectProfile | 147-151 | REAL | - | no-op guards |
| nextId/createProfile | 153-174 | REAL | - | collision-safe naming |
| buildProfileFlyout | 186-214 | REAL | - | menu data builder (selected-first? no — order kept, `selected` flag drives the face) |
| resetProfilesForTests | 176-182 | REAL | - | test-only, named as such |

## 5. Test honesty
Tests in the menu lane's files (not in my list); the store logic is the kind that the referenced playlistsStore precedent tests cover. UNVERIFIED.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "there is no engine profile command in the catalog, so profiles are LOCAL device data" | header L11-13 | REAL | rg: no profile command in the 65-cmd union |
| "no row claims engine capability" | header L15-16 | REAL | rows are plain labels + onSelect persistence |
| "[VERIFY] via the §5 vision loop; g20/h18/a19 all transcribed the same" | header L8-9 | UNVERIFIED | vision-loop transcription not re-run |

## 7. Dead / duplicated / unreachable code
None (memoryProfilesPersistence is a named test double).

## 8. Dependencies & flags
MenuPopup machinery via MenuEntry data; localStorage. No env vars.

## 9. Open questions for the next agent
- Is a "media library profile" on the engine roadmap (doc 05/07)? If not, recommend hiding the submenu.
