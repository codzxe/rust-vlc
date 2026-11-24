# AUDIT: ui/src/screens/open-media/index.ts (short audit — 5 LOC barrel, spec §12/§13)
| Field | Value |
|---|---|
| Auditor | W15-A01 |
| Date | 2026-02-14 |
| Lines | 5 (1 barrel re-export) |
| File verdict | REAL (public surface of the decoupled-trigger open-media flow; consumers verified) |
| Reachable from shipped entry point? | YES — imported by DiscoveryScreen.tsx (the volume-row menu's one real action + the empty-state CTA), MiniPlayer.tsx, MediaInfoPanel.tsx and tests (rg census: 6 files) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `rg -ln "from \"../open-media\"\|from \"../../open-media\"" ui/src -g '!open-media/*'` -> DiscoveryScreen, MiniPlayer, MediaInfoPanel + 3 tests; read of full file |

## 1. Blunt verdict
A one-line barrel exporting OpenMediaDialog and OPEN_MEDIA_EVENT — the
event constant that the discovery screen's only enabled action and the
player surfaces dispatch to open the file dialog. The barrel is genuinely
the seam: every cross-screen consumer imports from here rather than the
dialog file, so the dialog's internals stay decoupled (the w26.b13/k16
design). Consumers verified in production files. No findings — this is
the wiring entry for one of the UI's few real user-action flows (which
then opens media through the client transport in the dialog itself,
another agent's file).

## 2. Findings
None. Checked: dead exports (none — both symbols consumed), circular
import risk (barrel imports only the dialog file; dialog does not import
the barrel — no cycle), naming (matches the flow's public-surface claim).
