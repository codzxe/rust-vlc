# CLAIM AUDIT: docs/guides/README.md (credit: W19-A02, recovered by W19-R7)
| Field | Value |
|---|---|
| Auditor | W19-R7 (recovery) |
| Date | 2026-11-24 |
| Claims extracted | 10 |
| REAL 5 / REAL-UNPROVEN 1 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 3 / UNREACHABLE 0 / UNVERIFIED 1 |
| Internal contradictions found | 1 (README vs KNOWN-LIMITS.md desktop-shell claim) |

## 1. Blunt verdict
A user-facing index that is mostly honest — the "no sound, no window" admissions are real —
but it carries three stale FALSE claims: a version string that contradicts product.toml, a
desktop-shell description contradicted by KNOWN-LIMITS.md's 2026-11-15 strike note, and an
off-by-one format-row count. The "honest scope" section is itself outdated by 5 days of repo
history.

## 2. Claim ledger
| # | Claim (verbatim) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | guides getting-started/settings/shortcuts/troubleshooting exist | :5-8 | REAL | `ls docs/guides/` -> all 4 present |
| 2 | `display_version = "0.1.0-dev"` | :11 | FALSE | product.toml:27 `display_version = "0.1.3"` (sed verified) |
| 3 | CLI is real player surface: open/play/pause/seek/stop/info/get/set | :14-16 | REAL-UNPROVEN | docs/05-command-layer.md:495 §5.3 exists; baseline rc-smoke PASS for command-layer-play/headless-decode (audit/runs/baseline/rc-smoke.txt); not re-run by me |
| 4 | audio goes to the null device, video renders to no window | :17-18 | REAL | docs/KNOWN-LIMITS.md "Audio output: null backend only ... no sound on any OS yet"; ADR-005 exists |
| 5 | desktop shell is a std-only skeleton (ADR-004) | :19-20 | FALSE | KNOWN-LIMITS.md: "~~no tauri binary has ever been built or launched on any OS~~ Struck 2026-11-15 ... w28 built and shipped `lumen-tauri.exe` ... the real engine host behind `lumen_dispatch`" |
| 6 | React UI runs as dev-server app with settings/shortcuts/onboarding/player/playlist | :21-24 | REAL | `ls ui/src/screens/` -> onboarding, player, playlist, settings, shortcuts, ...; ui/package.json:8 `"dev": "vite"` |
| 7 | React UI does not yet play media through the engine | :23-24 | REAL-UNPROVEN | consistent with baseline rc-smoke (web-ui-build/browser-load, no engine playback lane); not re-executed |
| 8 | 37 of 73 format rows claimed (implemented or partial) on desktop | :26-27 | FALSE (off-by-one) | tomllib parse of docs/feature-matrix.toml: format rows = 73, desktop claimed = 38 (undercount) |
| 9 | zero [format rows] on web | :27 | REAL | tomllib: format rows web implemented/partial = 0 (non-format rows: 4 web-claimed — sentence scoped to formats) |
| 10 | sources: product.toml, schemas/settings.toml, keymaps/default.toml, locales/en-US.toml, docs/formats/ | :30-34 | REAL | `ls` each -> exists; `lumen --help` "generated from the settings schema" UNVERIFIED (not executed) |

## 3. Contradictions
| README says | Other source says | Which is true |
|---|---|---|
| "the desktop shell is a std-only skeleton (ADR-004)" (:19-20) | KNOWN-LIMITS.md (2026-11-15): "lumen-tauri.exe" shipped, "real engine host behind `lumen_dispatch`" | KNOWN-LIMITS is true (shipped artifacts v0.1.1/v0.1.2 cited); README is stale |

## 4. Fabrication / inflation indicators
Under-inflation, notably: claims 37 when 38 are claimed — a stale-count defect, not puffery.
The 0.1.0-dev version citation fabricates nothing but quotes a value product.toml no longer
holds. Quotes verified byte-verbatim via `sed -n '10,11p;18,20p;26,28p' docs/guides/README.md`.
