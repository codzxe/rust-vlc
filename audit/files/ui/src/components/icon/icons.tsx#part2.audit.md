# AUDIT: ui/src/components/icon/icons.tsx#part2
| Field | Value |
|---|---|
| Auditor | W15-A09 |
| Date | 2026-11-21 |
| Lines / functions | 452 (453–904 of 905) / 4 exports + 3 structures (46 glyph entries of the `glyphs` record, `iconNames`, `getIconGlyph`, `twoToneGlyphs` record + folder alias mutation, `getTwoToneGlyph`, `hasTwoToneVariant`) |
| Chunk boundary | part2 = 453–904 (assignments.tsv even split; part1 = 1–452 owned by W15-A08 — IconName union, header, first 24 glyph entries incl. the `folderGlyph`/`infoGlyph`/`personGlyph`/`equalizerGlyph` shared consts). The boundary cuts the `glyphs` record mid-table (mine runs "artist" → "hide-ui", closes the record at 822). Shared consts defined in part1 are consumed in my range — read for context, owned by A08. |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (Icon.tsx:2 imports getIconGlyph/getTwoToneGlyph → `<Icon>` used across every screen; my range's glyphs render wherever their consumers mount — verified consumer lists below). Product caveat: no desktop binary (W14-A01). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | usage loop `rg -l "\"$i\"" src --glob '*.tsx' --glob '*.ts' | rg -v icon/icons.tsx` for all 46 glyph names in my range → every one has ≥1 consumer (single-use consumers located: save→PlaylistPanel, plugins/wrench→SettingsScreen:95-104, capture-device→ControlBar:985, skip-forward-10→customize/panels.ts, artist→sorting.ts+panels.ts); Read Icon.tsx (consumer: twoTone→getTwoToneGlyph, else getIconGlyph; title→role=img, else aria-hidden); `rg -n 'two-tone\|__detail' icon.css` (53: `.lumen-icon--two-tone .lumen-icon__detail { color: var(--color-accent-strong) }` — detail markers verifiably inert in mono mode as the header claims; no base __detail color rule exists); `rg -n 'wrench\|performance' SettingsScreen.tsx` (103: `performance: "wrench"`); Read Icon.test.tsx:44-95 (registry pinned at exactly 76 names, uniqueness asserted) + 310-380 (two-tone fingerprint tests, folder-alias identity, hasTwoToneVariant dedicated-set assertion); `rg -ln hasTwoToneVariant src` (test-only, documented as a test hook). Baseline: ui-vitest.log.txt 136/136 files 1705 passed; ui build OK (#22). |

## 1. One-paragraph verdict
This half of the icon registry is honest mined data with real consumers and real pinning tests: all 46 glyph entries in my range are referenced by at least one non-registry source file (verified name-by-name), the registry is compile-time total (`Record<IconName, ReactElement>` — a missing entry fails tsc, and the build passed baseline), Icon.test.tsx pins the registry at exactly 76 unique names and regression-guards the mined path geometry, and the two-tone system's core claim (detail markers inert in mono mode) is verified in icon.css — no rule colors `.lumen-icon__detail` outside `.lumen-icon--two-tone`. The findings are hygiene only: a stale comment mapping `wrench` to a nonexistent "Settings → General" area (it serves Performance), a post-declaration mutation to alias the folder two-tone face instead of the shared-const pattern the mono registry uses two files up, and four skip-glyphs that each re-embed a ~600-char duplicated numeral outline with coordinate jitter.

## 2. What actually works (verified)
— verified by the 46-name usage loop (Commands run) → zero dead glyphs in my range; single-use icons located to real surfaces (PlaylistPanel "save", SettingsScreen AREA_ICONS "plugins"/"wrench"/"tv", ControlBar "capture-device", customize/panels.ts seek tiles for the skip family).
— verified by Read icon.css:46-68 → `.lumen-icon--two-tone .lumen-icon__detail` is the ONLY rule coloring detail elements (accent-strong; `--bright` variant accent-icon): the 30+ `className="lumen-icon__detail"` markers in my range are provably inert in mono rendering.
— verified by Icon.test.tsx:47-94 (registry = exactly 76, unique, arrayContaining the doc-named set) and :135 (every glyph inside the 24×24 grid) and :198-265 (mined-geometry regression pins + alias identity folder/info) — executed in baseline (136/136 files, 1705 passed).
— verified by Icon.tsx:60-79 → both my exports (`getIconGlyph` via the default path, `getTwoToneGlyph` via twoTone) render into the shared svg shell; `hasTwoToneVariant` is consumed only by Icon.test.tsx (documented test hook at 900-901).

## 3. Findings
### F-001 [S4]
Where: ui/src/components/icon/icons.tsx:805-807
Code:
```tsx
  // Icon.svg "name=wrench" (Frame_98) — wrench (Settings → General); needle keeps
  // its source stroke-width 1.28571
  "wrench": <path d="M16.0875 3.2715H14.2265C13.9995 3.3615 13.7815 3.5225 13.6205 3.5225L11.5505 5.5915C11.3905 5.7525 11.2995 5.9705 11.2995 6.1975V9.7735C11.2995 10.0005 11.2095 10.2185 11.0485 10.3795L3.9405 17.4875C3.2715 18.1575 3.2715 19.2425 3.9405 19.9125L4.0875 20.0595C4.7575 20.7285 5.8425 20.7285 6.5125 20.0595L13.6205 12.9515C13.7815 12.7905 13.9995 12.7005 14.2265 12.7005H17.8025C18.0295 12.7005 18.2475 12.6095 18.4085 12.4495L20.4775 10.3795C20.6385 10.2185 20.7285 10.0005 20.7285 9.7735V7.9125C20.7285 7.1485 19.8055 6.7665 19.2655 7.3065L17.5505 9.0205C17.3905 9.1815 17.1725 9.2715 16.9445 9.2715H15.5855C15.1125 9.2715 14.7285 8.8875 14.7285 8.4145V7.0555C14.7285 6.8275 14.8185 6.6095 14.9795 6.4495L16.6935 4.7345C17.2335 4.1945 16.8515 3.2715 16.0875 3.2715Z" strokeWidth={1.28571} />,
```
What is wrong: Two comment defects on one entry: (a) "wrench (Settings → General)" — there is no General settings area (the 13 areas are global/interface/privacy/playback/video/audio/subtitles/controls/library/network/performance/recording/extensions, App.tsx:235-249); the glyph is mapped to `performance` (SettingsScreen.tsx:103 `performance: "wrench"`); (b) "needle keeps its source stroke-width 1.28571" — the wrench has no needle; the needle wording is copy-paste rot from the playback-speed entry (533-534) where it IS a needle.
Why it matters: Comment rot in a registry whose comments are the only record of frame provenance (every other entry cites its sheet source precisely) — a future icon-sweep auditing "General" against the frames will chase a nonexistent mapping.
What correct looks like: `— wrench (Settings → Performance); keeps its source stroke-width 1.28571`.
Confidence: HIGH — Verification: read the entry + AREA_ICONS map (SettingsScreen.tsx:95-104) + App.tsx area list.
### F-002 [S4]
Where: ui/src/components/icon/icons.tsx:888-889
Code:
```tsx
/** The folder alias shares the open-folder two-tone variant (same glyph). */
twoToneGlyphs.folder = twoToneGlyphs["open-folder"];
```
What is wrong: The two-tone folder alias is created by mutating the record after its declaration, while the mono registry solves the identical alias with a shared const (`const folderGlyph = …` at 192-194, consumed by both "folder" (424) and "open-folder" (646)). Two patterns for the same problem in one file.
Why it matters: The mutation is invisible to the record's type (a `Partial<Record>` key assignment after initialization), so a future reader scanning the literal sees only 4 two-tone variants when there are effectively 5; the mono registry's const pattern keeps the alias countable at the declaration site.
What correct looks like: A shared const (like `folderGlyph`) referenced by both literal keys, or at minimum the mutation co-located with the other alias documentation.
Confidence: HIGH — Verification: read both patterns (192-194/424/646 vs 888-889).
### F-003 [S4]
Where: ui/src/components/icon/icons.tsx:505, 513, 521, 529
Code:
```tsx
      <path d="M11.459 12.167V19.44H10.358V13.268H10.315L8.575 14.404V13.353L10.39 12.167H11.459ZM15.958 19.561C15.397 19.558 14.918 19.41 14.52 19.117C14.122 18.823 13.818 18.396 13.607 17.835C13.397 17.274 13.291 16.598 13.291 15.807C13.291 15.019 13.397 14.345 13.607 13.787C13.82 13.228 14.126 12.802 14.523 12.508C14.924 12.215 15.402 12.068 15.958 12.068C16.514 12.068 16.991 12.216 17.389 12.512C17.787 12.805 18.091 13.231 18.302 13.79C18.515 14.346 18.621 15.019 18.621 15.807C18.621 16.6 18.516 17.277 18.305 17.838C18.095 18.397 17.79 18.824 17.393 19.12C16.995 19.414 16.517 19.561 15.958 19.561ZM15.958 18.613C16.451 18.613 16.835 18.372 17.112 17.892C17.392 17.411 17.531 16.716 17.531 15.807C17.531 15.203 17.467 14.693 17.339 14.277C17.214 13.858 17.033 13.54 16.796 13.325C16.562 13.107 16.282 12.998 15.958 12.998C15.468 12.998 15.083 13.24 14.804 13.723C14.525 14.206 14.384 14.9 14.381 15.807C14.381 16.413 14.444 16.926 14.57 17.345C14.697 17.761 14.879 18.078 15.113 18.293C15.347 18.506 15.629 18.613 15.958 18.613Z" fill="currentColor" stroke="none" />
```
What is wrong: The four skip-glyph entries (skip-back-10 :502-507, skip-forward-10 :510-515, skip-back-15 :518-523, skip-forward-15 :526-531) each embed their own ~600-character copy of the "1" and "0"/"5" numeral outlines, differing only by a global coordinate offset of 0.011/3 units of jitter (e.g. `M11.459 12.167V19.44` vs `M11.459 12.178V19.451`) and the arc glyph. Four copies of the same digit geometry.
Why it matters: Pure data, so nothing breaks today, but the file's own convention (shared consts for repeated geometry — folderGlyph, infoGlyph, personGlyph, equalizerGlyph) exists precisely to keep repeated paths single-sourced; these four dodge it, and a future numeral refinement must be applied four times.
What correct looks like: `const tenGlyph`/`const fifteenGlyph` (or per-digit consts) consumed by all four entries, mirroring folderGlyph.
Confidence: HIGH — Verification: read all four entries; diffed by eye — same outline, offset jitter only.

## 4. Per-function table (my range)
| Entry/Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| glyph "artist" | 453-461 | REAL | — | Person + eighth note; consumers: options/sorting.ts, customize/panels.ts. |
| glyph "favorite" | 463-467 | REAL | — | Full-accent heart; 5 consumers. |
| glyph "equalizer" (=equalizerGlyph const) | 469-470 | REAL | — | Alias of part1's const; 11 consumers. |
| glyph "audio" | 472-484 | REAL | — | 5-bar two-tone (bars 2/4 accent); 18 consumers. |
| glyph "subtitles" | 486-496 | REAL | — | CC card; 18 consumers. |
| glyph "info" (=infoGlyph) | 498-499 | REAL | — | Documented alias of about. |
| glyphs "skip-back/forward-10/15" | 501-531 | REAL | S4 | F-003 duplicated numeral outlines; consumers: customize/panels.ts seek tiles (real -10_000/+10_000 seek actions). |
| glyph "playback-speed" | 533-539 | REAL | — | Speedometer; consumer panels.ts. |
| glyph "repeat" | 541-547 | REAL | — | 4 consumers. |
| glyph "shuffle" | 549-557 | REAL | — | Two-tone crossings; 7 consumers. |
| glyph "record" | 559-567 | REAL | — | Ring+dot full accent; 4 consumers incl. SettingsScreen recording area. |
| glyph "snapshot" | 569-575 | REAL | — | Focus frame; 2 consumers. |
| glyph "pip" | 577-587 | REAL | — | Carries the only `--bright` detail in the registry (documented: the frame's single bright icon detail). |
| glyph "hdr" | 589-595 | REAL | — | Outlined HDR letters; 5 consumers. |
| glyph "zoom" | 597-604 | REAL | — | Magnifier+plus; 3 consumers. |
| glyph "right-sidebar" | 606-612 | REAL | — | 3 consumers. |
| glyph "quit" | 614-622 | REAL | — | Two-tone bracket+arrow; 9 consumers. |
| glyph "settings" | 624-632 | REAL | — | Gear with accent centre dot; iconOf() fallback too. |
| glyph "go-home" | 634-640 | REAL | — | 4 consumers. |
| glyph "open-file" | 642-643 | REAL | — | Mono page; two-tone variant adds folded corner (842-852) — the documented added-geometry pattern, fingerprint-pinned by Icon.test.tsx:322. |
| glyph "open-folder" (=folderGlyph) | 645-646 | REAL | — | Shared-const alias done right. |
| glyph "network-stream" | 648-658 | REAL | — | Two-tone arcs; consumers incl. SettingsScreen network area. |
| glyph "open-disc" | 660-670 | REAL | — | 4 consumers (incl. inert app-menu row — the icon is real even where the action is honestly disabled). |
| glyph "capture-device" | 672-680 | REAL | — | Consumer: ControlBar app-menu inert row. |
| glyph "convert" | 682-690 | REAL | — | Two-tone swap arrows. |
| glyph "keyboard-shortcuts" | 692-708 | REAL | — | 9 accent key marks + body. |
| glyph "customize-player" (=wandGlyph) | 710-711 | REAL | — | Shared const. |
| glyph "about" (=infoGlyph) | 713-714 | REAL | — | Alias pair complete. |
| glyph "recent" | 716-727 | REAL | — | Clock+rewind; hands accent. |
| glyph "refresh" | 729-737 | REAL | — | Decorative-in-context (ControlBar output-device header) — the glyph itself is real. |
| glyph "save" | 739-748 | REAL | — | Floppy; consumer PlaylistPanel. |
| glyph "edit" | 750-756 | REAL | — | Consumers: shortcuts pencil etc. |
| glyph "remove" | 758-759 | REAL | — | Minus bar. |
| glyph "music" | 761-762 | REAL | — | 11 consumers. |
| glyph "chapters" | 764-772 | REAL | — | 7 consumers. |
| glyph "bookmark" | 774-781 | REAL | — | 6 consumers. |
| glyph "interface" | 783-784 | REAL | — | 9 consumers. |
| glyph "privacy" | 786-787 | REAL | — | Shield; 9 consumers. |
| glyph "playback" | 789-791 | REAL | — | Outlined triangle; two-tone variant adds screen rect (870-875). |
| glyph "plugins" | 793-794 | REAL | — | Puzzle; extensions area. |
| glyph "tv" | 796-803 | REAL | — | Video area + ControlBar video tab. |
| glyph "wrench" | 805-807 | REAL | S4 | F-001 comment rot (General→Performance; needle wording). |
| glyph "divider" | 809-810 | REAL | — | 3 consumers. |
| glyph "hide-ui" | 812-821 | REAL | — | 2 consumers. |
| `iconNames` | 824 | REAL | — | Runtime-derived key list; pinned at 76 unique by test. |
| `getIconGlyph` | 826-828 | REAL | — | Total lookup (compile-enforced record); consumed by Icon.tsx:60. |
| `twoToneGlyphs` + folder alias | 838-889 | REAL | S4 | 4 dedicated variants (open-file/open-folder/playback/check — each the documented added-geometry case) + F-002 mutation alias. |
| `getTwoToneGlyph` | 896-898 | REAL | — | Fallback to shared glyph; fallback branch pinned by Icon.test.tsx:365. |
| `hasTwoToneVariant` | 902-904 | REAL | — | Test-only hook, documented; asserted by Icon.test.tsx:378. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| Icon.test.tsx:38 "every registry name renders a glyph" | YES | Iterates iconNames, asserts non-empty rendered svg children. |
| :47 "registry holds exactly the doc-named icons" | YES | Exact count 76 + uniqueness + containment; adding/removing an icon fails. |
| :135 "keeps every glyph inside the 24×24 grid" | YES | Bounds check over rendered paths. |
| :198/:222 "pins mined sheet geometry — regression guard" | YES | Path `d` strings asserted verbatim (covers my range's a07 additions and skip family). |
| :322 "dedicated two-tone variants carry the frame's ADDED geometry (fingerprints)" | YES | Asserts the two-tone entries include geometry the mono face lacks. |
| :355 "the folder alias shares the open-folder two-tone face" | YES | Pins the F-002 mutation's behavior (identical element). |
| :378 "hasTwoToneVariant reports exactly the dedicated set" | YES | Dedicated set = {open-file, open-folder, playback, check, folder}. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Tone splits below are frame-measured … per-glyph comments cite the source" | icons.tsx:88-91 header (part1, governs my entries) | REAL (one entry rotting — F-001) | Every entry in my range cites a sheet/frame source except the wrench comment's wrong target. |
| "detail markers are inert in mono mode (icon.css colors them only under `.lumen-icon--two-tone`)" | icons.tsx:834-836 | REAL | icon.css:53 is the only __detail color rule; no base rule exists. |
| "32 frozen (wave-1) + 44 frame-mined w26.a07 additions = 76; the 9 dead names (j04-4) were pruned … after an exhaustive usage grep" | Icon.test.tsx:91-93 | REAL | 76 pinned by the test; my range's independent usage loop found 0 dead names (the prune held). |
| two-tone dedicated variants exist only where "the frame variant ADDS geometry" | icons.tsx:830-837 | REAL | Exactly 4 dedicated entries, each documented as added-geometry (folded corner, front pocket, screen rect, open box) + the folder alias. |

## 7. Dead / duplicated / unreachable code
Dead: none (all 46 range entries consumed; hasTwoToneVariant is test-only but documented as such — not dead). Duplicated: F-003 (4× numeral outlines); the F-002 alias-mutation pattern duplicates the mono registry's const-alias intent. Unreachable: nothing.

## 8. Dependencies & flags
No imports (pure data module + three lookup functions) besides React types. Consumed by Icon.tsx (both getters), Icon.test.tsx (iconNames, hasTwoToneVariant). The `.lumen-icon__detail--bright` modifier (pip, 585) depends on icon.css:68. No feature flags.

## 9. Open questions for the next agent
- A08's part1 contains "menu" (318-327) and "lists" (372-381) with IDENTICAL path data but no documented alias relationship (unlike folder/info) — worth their check; if the frames really carry two names for one glyph, it should use the shared-const alias pattern.
- The customize-panel catalog consumes 12 of my range's glyphs (skip family, playback-speed, repeat, shuffle, record, snapshot, pip, hdr, zoom, right-sidebar, quit, divider, hide-ui) — whether those BUTTONS do anything is CustomizePanels.tsx's audit (my part1, next file).
