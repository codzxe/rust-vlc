# AUDIT: tools/vta/ac3/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W17-A03 |
| Date | 2026-10-30 |
| Lines / functions | 14 / n/a (manifest) |
| File verdict | REAL — consistent manifest, lock, record, README, and source imports all agree |
| Reachable from shipped entry point? | NO — by design: standalone `[workspace]`, not a member of the Lumen workspace, not a dependency of any workspace crate; invoked only manually as differential evidence (consumers: docs/dependencies/records/oxideav-ac3-0.0.11.md, worklog archive) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg` over Cargo.lock, root Cargo.toml, record, README, main.rs (all exit 0); `ls tools/vta` |

## 1. One-paragraph verdict
A five-line manifest for a harness-only tool that does exactly what its dependency record says: pins oxideav-ac3 0.0.11 (caret on 0.0.x = exact), pulls oxideav-core for the Packet/Frame types, and is sealed off from the Lumen workspace by its own `[workspace]` table. Every claim cross-checks: the lock pins oxideav-ac3 0.0.11 + oxideav-core 0.1.42; the record documents that same graph edge and the ARCHIVED-NOT-ADMITTED verdict; the README states the differential gate FAILED (maxΔ 316 LSB) and forbids wiring the crate in; main.rs imports the exact symbols the manifest comment promises. The repo's dependency-discipline story holds for this crate.

## 2. What actually works (verified)
- Lock pins: tools/vta/ac3/Cargo.lock:24-25 `name = "oxideav-ac3" / version = "0.0.11"`; :33-34 `name = "oxideav-core" / version = "0.1.42"` — matching the record's "oxideav-ac3 0.0.11 → oxideav-core 0.1.42 (non-optional, normal)" (records/oxideav-ac3-0.0.11.md:39).
- The manifest comment's claim about Packet/Frame: main.rs:30 `use oxideav_core::{CodecId, CodecParameters, Frame, Packet, TimeBase};` — the types are actually used.
- Workspace isolation: root Cargo.toml `exclude = ["apps/desktop-tauri"]` (:51) does not list vta tools — they self-exclude via their own `[workspace]` table, exactly as main.rs:4-5 documents ("Not a workspace member (own `[workspace]` table in Cargo.toml)").
- README:4-6 "STATUS: ARCHIVED — NOT ADMITTED. The differential gate FAILED … `oxideav-ac3` is NOT a dependency of any Lumen workspace crate." — the not-admitted claim verified: no workspace crate references oxideav-ac3 (rg over docs shows only the record + worklog; crates/ has no reference).

## 3. Findings
F-001 [S4]
Where: tools/vta/ac3/Cargo.toml:6
Code:
```
edition = "2021"
```
What is wrong: every workspace tool (xtask, sweep, mem-audit, bench-vs-vlc) is edition 2024; the five vta harnesses are 2021. Not a correctness bug (harness-only code, compiles on 2021), but a consistency split that makes the vta tree look like foreign grafts. Confidence: HIGH — Verification: `rg -n "^edition" tools/*/Cargo.toml tools/vta/*/Cargo.toml` (output above). No round trip to check the LICENSE field — this package is publish = false and private, so no license key needed (none declared — correct).

F-002 [S4]
Where: tools/vta/ac3/Cargo.toml:11-13
Code:
```
# already in oxideav-ac3's graph — declared directly for the Packet/Frame
# types the Decoder trait exchanges (no additional packages fetched)
oxideav-core = "0.1"
```
What is wrong: "no additional packages fetched" is a property of the CURRENT resolution (lock: oxideav-core 0.1.42 present in both graphs), not of the manifest — `0.1` caret allows 0.1.x while oxideav-ac3 could later require a different core line; a fresh resolve could then fetch a version oxideav-ac3's graph wouldn't have had. True today (record + lock agree), not guaranteed by the declaration. Confidence: HIGH — Verification: read + lock/record cross-check.

## 4. Per-function table
N/A (manifest; 3 sections audited: workspace/package/dependencies — all verified above).

## 5. Test honesty
The harness itself (tools/vta/ac3/src/main.rs, 323 lines) is another agent's file; this audit covers only the manifest and its cross-references.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "harness-only AC-3 … oxideav-ac3 **0.0.11**" | records/oxideav-ac3-0.0.11.md:8 | REAL | manifest :10 + lock :24-25 |
| "oxideav-ac3 0.0.11 → oxideav-core 0.1.42 (non-optional, normal)" | records/…:39 | REAL | lock :33-34 |
| "ARCHIVED — NOT ADMITTED … NOT a dependency of any Lumen workspace crate" | README:4-6 | REAL | no references from crates/ or apps/ (rg); standalone [workspace] confirmed |
| "unsafe: 0 in oxideav-ac3" | records/…:29 | UNVERIFIED (record's claim about the crate's source; tarball not inspected in this audit) |

## 7. Dead / duplicated / unreachable code
None in the manifest. The whole tool is deliberately unreachable from production (that is its documented purpose — differential evidence only).

## 8. Dependencies & flags
oxideav-ac3 ^0.0.11 (effectively exact), oxideav-core ^0.1, both network crates, committed lock pinning both. publish = false. No features, no lints (outside the workspace deny-lint regime — harness code is exempt from the unwrap/panic bans the workspace enforces).

## 9. Open questions for next agent
- The differential failure (maxΔ 316 LSB) that archived this crate — is the ffmpeg-oracle lane itself verified anywhere (the harness trusts ffmpeg as ground truth; if the ffmpeg build differs, so does the verdict)? Check the record's R6.
- tools/vta/dnx carries an evidence/ directory with compare results — the ac3 tool has none; its numbers live only in the record.
