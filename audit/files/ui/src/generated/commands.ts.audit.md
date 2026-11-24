# AUDIT: ui/src/generated/commands.ts
| Field | Value |
|---|---|
| Auditor | W15-A04 |
| Date | 2026-11-24 |
| Lines / functions | 123 / 45 exported types |
| File verdict | REAL (generated, drift-gated, catalog-faithful) |
| Reachable from shipped entry point? | YES: imported by client/transport.ts, every screen's command dispatch (type-level reachability: the whole UI's command surface) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python cmd-set comparison vs crates/commands/src/catalog/ids.rs (exit 0); rg gate verification in tools/xtask + .github/workflows/ci.yml (exit 0) |

## 1. One-paragraph verdict
The UI's command vocabulary, machine-generated from the Rust catalog (tools/xtask/src/cmd/codegen.rs:16-20 writes this file plus `schemas/generated/commands.checksum.txt`; CI runs `cargo run -p xtask --locked -- codegen --check`, .github/workflows/ci.yml:279-280 — the header's "checksum gate fails on drift" claim is REAL). I diffed the 65 `cmd` literals against `crates/commands/src/catalog/ids.rs`: zero mismatches (the 27 ids.rs entries absent from CommandRequest are event ids, present in the Event union, plus the special `wire.id`). The file itself is clean. The master-question story it tells by existing: the UI can TYPE 65 commands, the shipped desktop host serves 15 of them (dispatch.rs:81-153: session.open/close, playback.play/pause/toggle/stop/seek, media.get-info/current-time/duration, tracks.list, system.get-engine-info/list-commands, config.get/set-setting), and the browser demo transport serves 41 (python count over demo-transport.ts case labels) — including every command behind volume, playlists, track selection, bookmarks, and chapters. The fake host implements 2.7x the real host's surface; that asymmetry is why the dev shell demos features the product cannot perform.

## 2. What actually works (verified)
— python diff: 65/65 catalog commands present, no extras, no missing; event union cross-checked.
— Drift gate exists and runs in CI (codegen.rs:16-20; ci.yml:279-280).
— Types are the compile-time wall the rest of the UI dispatches against (transport.ts, context.tsx, PlaylistProvider, BookmarksDialog, ConvertScreen probe all cite these types).

## 3. Findings
None in this file. (The 15-vs-41 host asymmetry is a repo-level finding recorded in PlayerScreen/BookmarksDialog/ConvertScreen audits and the final report; the generated file faithfully mirrors the catalog — the catalog's breadth is the design, not this file's defect.)

## 4. Per-function table (types are the units)
| Type | Lines | Verdict | Notes |
|---|---|---|---|
| CommandRequest (65-cmd union) | 24 | REAL | diffed vs ids.rs: exact |
| CommandResponse / ResultWire / CommandOutcome | 22,26,78 | REAL | envelope shapes |
| CommandResult (31-kind union) | 28 | REAL | matches catalog result kinds |
| Event (30-kind union) | 46 | REAL | event ids cross-checked |
| ErrorCode incl. unsupported-for-host | 42 | REAL | the honest typed refusal the hosts use |
| wire data types (Timestamp, Duration, SeekTarget, TrackInfo, StateSnapshot, ...) | 6-120 | REAL | µs Timestamp contract documented in consumers |
| CatalogDocument | 123 | REAL | schema root |

## 5. Test honesty
transport.test.ts and per-screen tests type-check against these types (baseline pass). The generation gate is CI-enforced, not just claimed.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NEVER hand-edit; the catalog checksum gate fails on drift" | L2-4 | TRUE | ci.yml:279-280 + codegen.rs:17-20 |
| "Source of truth: crates/commands/src/catalog" | L3-4 | TRUE | python diff exact |

## 7. Dead / duplicated / unreachable code
None in-file. Type-level note: commands like `audio.list-devices`, `video.take-screenshot`, `subtitle.set-sync` have UI types but NO UI sender anywhere (rg shows no dispatch sites) and no host — catalog breadth without consumers.

## 8. Dependencies & flags
None; pure generated types.

## 9. Open questions for the next agent
- Repo-level: should the catalog shrink to served reality, or should hosts grow? The 65:15:41 (catalog:CLI:demo) ratio is the root cause of every "works in demo, refused in production" finding in this wave.
