# CLAIM AUDIT: design/figma/menu.svg
| Field | Value |
|---|---|
| Auditor | W19-A04 |
| Claims extracted | 4 |
| REAL 4 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
Valid SVG design asset. Its doc-08 claim is the strongest text claim of the figma set —
menu items plus keyboard accelerators — and every accelerator named in doc 08 actually
exists in the file as outlined-path ids.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Well-formed SVG | — | REAL | python3 ElementTree parse OK |
| 2 | Dimensions 375×585 | doc 08 Menus row | REAL | width="375" height="585", viewBox="0 0 375 585" |
| 3 | menu.svg contains: Open File(s) Ctrl+O, Open Folder Ctrl+F, Open Network Stream Ctrl+N, Open Disc Ctrl+D, Open Capture Device Ctrl+C, Convert Ctrl+R, Open Recent ▸, Keyboard Shortcuts, Customize Player, Settings Ctrl+P | docs/08-ui-figma-fidelity.md:143 | REAL | python3 over file content: path ids `Ctrl + O`, `Ctrl + F`, `Ctrl + N`, `Ctrl + D`, `Ctrl + C`, `Ctrl + R`, `Ctrl + P` all present (7 accelerators); strings `Keyboard Shortcuts`, `Customize Player`, `Open Recent` present — all ten items accounted for |
| 4 | Menus group membership (`menu.svg` etc.) | doc 08:166; design/README.md Menus row | REAL | listed in both |

## 3. Contradictions
None. Note: accelerator text is text-outlined (per design/README residual: font
identities [VERIFY]), and the "Ctrl + O" spacing variant vs doc-08's "Ctrl+O" is
formatting, not substance.
