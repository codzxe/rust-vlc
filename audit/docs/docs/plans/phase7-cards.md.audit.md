# CLAIM AUDIT: docs/plans/phase7-cards.md
| Field | Value |
|---|---|
| Auditor | W19-A03 |
| Date | 2026-11-14 |
| Claims extracted | 6 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIF | 4 / 0 / 0 / 0 / 1 / 0 / 1 |
| Internal contradictions found | 1 (dated by construction — plan predates landings) |

## 1. Blunt verdict
A planning doc whose de-risk claims verify (TLS landed in crates/streaming
via ureq+rustls with Range fetch; ADR-002 wasmi exists; ADR-013 exists as
proposed exactly as stated; phase6-exit.md exists). Its "still-missing
targets" list is TRUE AT DATE and stale TODAY — every named missing crate
(platform-*, device-discovery, transcode, plugin-host-wasm,
control-protocol, apps/web, apps/control-server) now exists in-tree: the
plan's Phase 7 was subsequently executed.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "TLS is landed and activated … crates/streaming now has a proven https + Range-fetch path" | :14-17 | REAL | crates/streaming/Cargo.toml:43 ureq 3.5.0 admitted; src/{client,fetch}.rs range paths exist |
| 2 | "ADR-002 is accepted (wasmi …) — resolved: wasmi" | :21 | REAL | docs/adr/ADR-002-wasm-runtime-wasmi.md + evidence file exist |
| 3 | "ADR-013 … drafted, status proposed" | :23 | REAL | ADR-013-va-api-hw-decoder.md:3 "**Status:** **proposed** (drafted 2026-11-09" verbatim |
| 4 | "Phase 6 gate is done (docs/gates/phase6-exit.md)" | :25 | REAL | file exists in docs/gates/ |
| 5 | "Confirmed still-missing targets: crates platform-{…}, …, crates/codecs/src/video/h264/hw/" | :27-29 | FALSE (stale today) | `ls crates/` → platform-{windows,linux,macos}, device-discovery, transcode, plugin-host-wasm, control-protocol ALL exist; h264/hw/ exists (VT+VA-API landed 2026-11-10); apps/web + apps/control-server exist |
| 6 | "The Lead has converted this table into GitHub issues #40–#51" | :6 | UNVERIFIED | GitHub state not inspectable from sandbox |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| crates "still-missing" (2026-11-09) | ls crates/ today shows them | both at their dates; plan is a historical snapshot (doc purpose: dependency-ordered work items, PAPER-ONLY by genre) |

## 4. Fabrication / inflation indicators
None — the plan self-labels deltas as de-risks, credits the source doc 13
§10 as scope truth, and invents no scope.

Findings: F-001 [S4] plan is fully stale on its missing-crates list; a
reader today would wrongly conclude Phase 7 is unstarted.
Commands: ls crates/ apps/ docs/gates/, rg streaming Cargo.toml (exit 0).
