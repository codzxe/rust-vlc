# Wave 26 Program — 13 waves × 20 agents (260) + Lead integration

Lead-run program (2026-11-11). Every agent reads `docs/waves/w26/CONTRACT.md`
first. Task IDs: `w26.<wave><nn>` (wave letters a–m). After each wave the Lead
builds, tests, integrates, commits and pushes to `main`.

| Wave | Focus | Lanes (20 each) |
|---|---|---|
| a | Foundation + Player + Onboarding | tokens, theme css, button, slider, menu, form, icon, surface, list, appshell, app-flow, onboarding, player core/controls/states/variants/customize/dialogs, menu analysis, tools analysis |
| b | Settings + Playlists + working screens | settings shell, 13 areas (seed + hand), appearance/swatches, playlist core, sorting/view, subtitle flows, thumbnail/media, discovery, convert, media-info, network-stream, shortcuts dialog, not-found |
| c | Audio/video tools + menus + dialog polish | EQ, EQ presets, compressor, spatializer, widener, visualizer, audio value, colors, crop, essential, HDR, context menus ×4, dropdown/profiles/presets, control parts, origin/target/play/volume |
| d | Flow integration | routing, keyboard nav, focus/a11y, responsive, light mode, hide-UI, miniplayer flow, i18n registration, test updates, css lint sweep |
| e | Verification #1 (20 verifiers, all families + flow + code) | files findings in docs/waves/w26/findings/ |
| f | Fixers #1 | one finding-cluster per lane |
| g | Verification #2 (loop until clean) | same as e |
| h | Harsh senior-UI ratings (20 raters) | docs/waves/w26/ratings/ |
| i | Fixers #2 (ratings) | |
| j | Harsh UI code review (20 reviewers) | docs/waves/w26/reviews/ |
| k | Fixers #3 (code review) | |
| l | Final verification + polish + baselines | |
| m | Releases: Windows/Linux/macOS/web + GitHub Release + docs | |

Total: 260 agents. Verification loop e→f→g repeats (g restarts f) until the
verifiers return clean. The Lead re-seeds visual baselines only after wave d
and wave l (runner-rendered truth per W25 spec §4).
