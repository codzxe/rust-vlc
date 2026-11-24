# AUDIT: ui/src/screens/player/variants/variantLayout.ts
| Field | Value |
|---|---|
| Auditor | W15-A05 |
| Date | 2026-06-05 |
| Lines / functions | 115 / 1 (resolveVariantLayout) + 2 TOML consts + 3 exported maps |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (PlayerScreen.tsx:133,630 `resolveVariantLayout(layoutSource, variant)` inside the /player route tree) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | rg consumers (PlayerScreen + variants/index); rg layoutSource production mounts (none pass it); sed quote verification |

## 1. One-paragraph verdict
Real layout data module: two layout.v1 TOML sources (miniplayer, music) parsed
at module load by C1's genuine parser (parseLayoutConfig), a per-variant
default map, and a resolver with documented precedence (custom source wins,
empty falls back to variant default, invalid throws LayoutParseError). The
region matrix in the header matches the TOML contents (music: album-art +
controls + left playlist; mini: thumbnail + controls). The custom-source seam
is never fed in production — App.tsx:203-209 mounts PlayerScreen without
layoutSource — so only the two embedded defaults ever resolve.

## 2. What actually works (verified)
— PlayerScreen.tsx:630 calls the resolver in a memo keyed on [layoutSource,
variant] — the variant switch re-resolves correctly.
— MUSIC_LAYOUT_TOML regions match the header matrix (music: no title-bar, no
video-canvas; controls-bar ungated; playlist visible_when music-mode).

## 3. Findings
F-001 [S4]
Where: ui/src/screens/player/variants/variantLayout.ts:107-115
Code:
```
export function resolveVariantLayout(
  source: string | null | undefined,
  variant: PlayerVariant,
): LayoutConfig {
  if (source === null || source === undefined || source.trim() === "") {
    return PLAYER_VARIANT_LAYOUTS[variant];
  }
  return resolveLayout(source);
}
```
What is wrong: The custom-source branch (doc 07 §7.1 "config owns
existence/order") has no production caller — rg shows the only mount
(App.tsx:203) passes no layoutSource. The seam is wire-ready but unwired;
config/layouts/*.toml "will be" (line 24) — future tense, still. Documented
[PROPOSAL]; not a defect, tracked debt.
Confidence: HIGH — Verification: read + rg mounts.

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| MINIPLAYER_LAYOUT_TOML / MUSIC_LAYOUT_TOML | 38-88 | REAL | - | parsed at load; throw on bad source (programmer error) |
| MINIPLAYER_LAYOUT / MUSIC_LAYOUT / PLAYER_VARIANT_LAYOUTS | 91-99 | REAL | - | full = DEFAULT_LAYOUT |
| resolveVariantLayout | 107-115 | REAL | F-001 | custom branch unreached in production |

## 5. Test honesty
No tests in file; layout tests live with C1's parser (another agent).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A present-but-invalid source throws LayoutParseError (C1 semantics)" | 105-106 | REAL-UNPROVEN | depends on resolveLayout (../layout, not audited here); module-load parse of the two good sources succeeds (app builds) |
| "config/layouts/*.toml will be" | 23-24 | PAPER-ONLY | no config/layouts directory consumed in production path |

## 7. Dead / duplicated / unreachable code
The custom-source branch is unreachable from shipped mounts (F-001) — kept as
the documented host seam; exports MINIPLAYER_LAYOUT_TOML/MUSIC_LAYOUT_TOML
re-exported for tests (variants/index.ts:16-19).

## 8. Dependencies & flags
../layout (parseLayoutConfig, resolveLayout, DEFAULT_LAYOUT), ./variant type.
No flags.

## 9. Open questions for the next agent
layout.ts (C1) owner: confirm resolveLayout's error behavior matches the
"throws LayoutParseError" contract claimed here.
