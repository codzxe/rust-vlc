# CLAIM AUDIT: design/figma-renders/Add_Subtitles_to_Playlist-1.png (binary asset)
| Field | Value |
|---|---|
| Auditor | W19-A02 — RECOVERED by W19-R2 |
| Type | PNG image data, 1088 x 840, 8-bit/color RGB (no alpha), non-interlaced (`file`) |
| Size / sha256 | 71,160 B / 4d241476934e3e100ef049c78fb73dc1f644dcdcb2a5a8a83a5a0fc5eb00036b |
| Verdict | REAL (render; one of the 4-folder-state Add_Subtitles family) |

## 1. Blunt verdict
Genuine Figma render of the add-subtitles dialog flow. Notable verified property: this is
the ONLY 8-bit RGB (no-alpha) PNG in this Settings/dialog family batch (siblings are
RGBA) — a plain opaque export, 1088×840 (wider than the 900px Settings frames — dialog
popover geometry, not a settings window). Consumer w26.g19 marks it an "identical frame
state (SVG diff) — same capture" duplicate of its sibling with GAPS verdict.

## 2. Claim ledger
| # | Claim | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | One of the `Add_Subtitles_to_Playlist/` 4 folder-state renders | whole | REAL | docs/waves/w26/CONTRACT.md:56 lists "`Add_Subtitles_to_Playlist/` renders (4 folder states)"; `ls design/figma-renders | rg -c Add_Subtitles` -> 4 files in the family. |
| 2 | "identical frame state (SVG diff) — same capture" (w26.g19:88) | whole | REAL-UNPROVEN | w26.g19.md:88 says the -1 state is a duplicate per the doc's own SVG-diff method; not re-derived here (requires the sibling pair diff of pixel content — the 4 family PNGs are distinct files, sibling equality not re-measured). |

## 3. Contradictions
None. The consumer finding is a GAPS row (implementation deltas), the asset is a witness.
