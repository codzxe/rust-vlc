# AUDIT: ui/src/components/menu/MenuBar.test.tsx
| Field | Value |
|---|---|
| Auditor | W16-RR3 (recovery run; file originally assigned to W16-A01) |
| Date | 2026-11-24 |
| Lines / functions | 289 / 19 tests + fixture + helpers |
| File verdict | REAL (tests) — SUBJECT UNREACHABLE |
| Reachable from shipped entry point? | Test lane YES (baseline ui-test.txt:59, 19 tests pass). Component under test: NO — `MenuBar` is exported and never mounted by any non-test module |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | rg -n "MenuBar" src (import scan via python3 — no non-test importer), sed -n '250,254p' + '1,6p' index.ts (quotes verified), rg -c "it(" -> 19 |

## 1. One-paragraph verdict
The suite itself is real and hostile in the right ways: 19 tests covering the full WAI-ARIA
menubar keyboard model (roving tabindex, ArrowUp→LAST item, submenu hover-intent with fake timers
around the real SUBMENU_OPEN_DELAY_MS constant, focus trap/restore, disabled inertness, density
classes), all executed and passing in the orchestrator baseline (ui-test.txt:59). The damning
fact is what it tests: MenuBar (168 LOC) is dead in the shipped app — exported from the barrel,
imported by nobody; every screen that needs a menu uses Dropdown/ContextMenu/MenuPopup instead.
A thorough test suite for a component no route can render.

## 2. What actually works (verified)
— verified by rg -c "it(" (19 = baseline count) + citation audit/runs/baseline/ui-test.txt:59
`✓ src/components/menu/MenuBar.test.tsx (19 tests) 448ms` (part of the 1705-passed run).

## 3. Findings
F-001 [S3]
Where: ui/src/components/menu/index.ts:2 (and absent usage anywhere)
Code:
/** Card 4.B3 — menu primitives barrel. */
export { MenuBar } from "./MenuBar";
export type { MenuBarProps, MenuBarMenu } from "./MenuBar";
What is wrong: `MenuBar` is exported but no non-test file imports or mounts it (python3 scan of
src/**/*.tsx|ts for import/mount: zero hits; consumers import only Dropdown/ContextMenu/MenuPopup/
MenuEntry/identityResolveLabel). The only mentions are comments in ControlBar.tsx:882
("f08/AppShell may relocate the trigger into the shell title bar (MenuBar)") — a maybe-future
note, not a call. Why it matters: 168 LOC component + 289 LOC test suite = dead-code mass; the
19 passing tests verify code the shipped desktop app can never reach. What correct looks like:
either mount it (AppShell title bar per f08) or delete both files.
Confidence: HIGH — Verification: read (rg scan ran).
F-002 [S4]
Where: ui/src/components/menu/MenuBar.test.tsx:250-254
Code:
  it("renders accelerator strings from keymap data verbatim", () => {
    render(<MenuBar menus={menus} resolveLabel={resolve} />);
    openFileMenu();
    expect(screen.getByText("Ctrl+O")).toBeInTheDocument();
  });
What is wrong: the name says "from keymap data" but `Ctrl+O` is defined in THIS file's fixture
(menus line 36), not any keymap module; the test cannot distinguish prop-passed strings from
keymap-sourced ones. Name promises more than the body proves (checklist 6.4).
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Test (line) | Verdict | Severity | Notes |
|---|---|---|---|
| labels via resolveLabel seam (74) | REAL | — | menubar + closed state |
| haspopup/expanded state (83) | REAL | — | toggles on click |
| ArrowDown opens+focuses first (92) | REAL | — | tabindex model |
| ArrowUp opens+focuses LAST (103) | REAL | — | APG menubar; menu-switch reset |
| arrows skip separators (125) | REAL | — | |
| Home/End (136) | REAL | — | |
| Enter activates+restores (145) | REAL | — | onSelect once |
| Escape restores trigger (154) | REAL | — | |
| submenu Right/Left (163) | REAL | — | aria-expanded |
| Escape in submenu (180) | REAL | — | only submenu closes |
| hover-intent delay (192) | REAL | — | fake timers vs real constant, off-by-1ms boundary |
| trigger arrows switch menu (215) | REAL | — | |
| Tab trap wraps (229) | REAL | — | |
| disabled inert (239) | REAL | — | no onSelect |
| accelerator "keymap data" (250) | REAL | S4 | F-002 name inflation |
| checkbox role (256) | REAL | — | aria-checked |
| separator role (263) | REAL | — | |
| density comfortable (269) | REAL | — | |
| density compact propagates (275) | REAL | — | into submenu |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| all 19 | YES | assertions bind to roles/aria/focus/callbacks produced by the real component; hover test pins the real SUBMENU_OPEN_DELAY_MS boundary (line 203-208) |
Mocking posture: none of the subject is mocked; resolveLabel seam is a legitimate stand-in (comment lines 10-11). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "nothing is hardcoded in the component" (labels) | header 10-11 | REAL | labels resolve from dict via resolve prop |
| WAI-ARIA menubar model coverage | header 8-9 | REAL | 19 tests as tabled |

## 7. Dead / duplicated / unreachable code
F-001 (subject dead). Test file itself is live in the vitest lane only.

## 8. Dependencies & flags
In-repo imports only (MenuBar, timings). No deps added, no gates.

## 9. Open questions for the next agent
Is MenuBar's death intentional (superseded by MenuPopup pattern) or an unwired f08 AppShell
integration? If the former, both files should be deleted, not tested.
