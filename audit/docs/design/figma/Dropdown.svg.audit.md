# DATA AUDIT: design/figma/Dropdown.svg
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A07) |
| Type / viewBox | SVG ASCII, 406×369 artboard, `g id="Dropdown"` (file(1) verified) |
| Matrix evidence citations | NONE (feature-matrix.toml: 0 hits) |
| Consumers | ui/src/components/menu/Dropdown.tsx, menu.css:315, states.md, settings/setting-field.css, convert, shortcuts |

## Blunt verdict
Real shared-dropdown design frame; cited as the geometry source by the
implementation's own comments — menu.css:315 "Trigger: r6 + 1px border +
chevron (task W26; `Dropdown.svg` outer 364×48 ...". No matrix row cites the
asset; zero fabricated-evidence exposure. Note the CONTRACT components lane
lists the *render* `dropdown.png`, not this SVG — both exist as a pair.

## Checks run
- `file` -> SVG Scalable Vector Graphics image, ASCII text (exit 0)
- `head -2` -> width="406" height="369" viewBox="0 0 406 369", `<g id="Dropdown">`
- `rg 'Dropdown.svg' --glob '!audit'` -> 8 files (menu css/tsx/states.md,
  setting-field.css, convert css/tsx, shortcuts.css, w26.i17.md log);
  feature-matrix.toml: 0 hits.

Findings: none. Short data audit per template 12.2.
