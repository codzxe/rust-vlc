# AUDIT: tools/vta/vp9/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W17-A10 (recovered by W17-R2) |
| Date | 2026-11-24 |
| Lines / functions | 15 / 1 manifest (empty [workspace] + [package] + 3 [dependencies]) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — standalone non-workspace harness crate; the DECODER it pins (rusty_vp9 0.1.3) IS shipped via codecs/Cargo.toml:145 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | lock cross-read (rusty_vp9 entry L76-79); codecs/Cargo.toml:81/138/145 read; record rusty_vp9-0.1.3.md:18/140/238 read; sed quote verification |

## 1. One-paragraph verdict
A clean standalone-spike manifest, and the best-behaved of the vta family: the candidate is EXACT-pinned (`=0.1.3`, matching the shipped codecs pin and the record's admission form), the committed lock carries the same checksum as the root lock, both path deps resolve, and the empty `[workspace]` genuinely isolates it. The only defect is the same stale root-members comment its ruopus sibling carries.

## 2. What actually works (verified)
— verified: `rusty_vp9 = "=0.1.3"` (:13) — EXACT pin, same form as the shipped admission (crates/codecs/Cargo.toml:145 `rusty_vp9 = { version = "=0.1.3", optional = true }`, feature `codec-vp9 = ["dep:rusty_vp9"]` at :81, default per the record) and as the record's own statements (rusty_vp9-0.1.3.md:18 "Admission shape: `rusty_vp9 = { version = \"=0.1.3\" }`", :238 "Cargo.toml pin (Lead admission): `rusty_vp9 = \"=0.1.3\"`"). No lock-float risk (unlike the ruopus sibling's caret pin).
— verified: tools/vta/vp9/Cargo.lock:76-79 pins rusty_vp9 0.1.3 checksum `9bf32821db20be4ea56ce1f8feb757bf55f146f2685ba98822c3bd2d43a33889` — byte-identical to the root lock (W17-A07's lock audit, verified REAL) — harness candidate == shipped decoder bytes; the record's differential (:140 "BIT-EXACT against" the ffmpeg reference) covers those exact bytes.
— verified: path deps `demux`/`plugin-api` (:14-15) resolve to real crates; `[workspace]` empty table (:4) isolates the harness (root members list has no tools/vta glob — root Cargo.toml:41).
— verified: `publish = false` (:10).

## 3. Findings
F-001 [S4]
Where: tools/vta/vp9/Cargo.toml:2-3
Code: `# NOT a workspace member (root members = crates/*, apps/*, tools/xtask) —
# the empty [workspace] table below pins it to its own workspace root.`
What is wrong: same comment rot as the ruopus sibling — the parenthetical names 3 of the root's 6 members (root Cargo.toml:41 also has `tests/integration/first-vertical`, `tools/mem-audit`, `tools/sweep`). The substantive claim (not a member) is TRUE.
Confidence: HIGH — Verification: read.

## 4. Zero-findings justification for the rest (what was checked and how)
Checked and clean: exact-pin form (three-way match manifest/lock/codecs/record, quoted above); checksum byte-identity (lock L79 vs root lock via W17-A07 + my read); path-dep resolution (both crates exist); workspace isolation (empty [workspace], no tools/vta in root members); publish=false; no [features]/[lints] sections (standalone crate outside workspace lint inheritance — same posture as ruopus). No other defects found.

## 5. Test honesty
No tests in this manifest; the harness binary (tools/vta/vp9/src/main.rs) audited by W17-A09 (REAL harness; its F-001 S3 hostile()-panics-on-empty-IVF finding is a main.rs defect, not a manifest one).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NOT a workspace member" | :2 | REAL | root members list (Cargo.toml:41) |
| "the empty [workspace] table below pins it to its own workspace root" | :3 | REAL | :4 + own Cargo.lock |
| (record) "pin `rusty_vp9 = \"=0.1.3\"`" | rusty_vp9-0.1.3.md:238 | REAL | :13 byte-identical form |

## 7. Dead / duplicated / unreachable code
None. edition 2021 vs workspace 2024 — era marker only.

## 8. Dependencies & flags
rusty_vp9 =0.1.3 (registry, default features ON — same posture as the shipped pin), demux + plugin-api (path). No optional features of its own.

## 9. Open questions for the next agent
- None.
