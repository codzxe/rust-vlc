# AUDIT: tools/vta/dnx/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 17 / 1 manifest ([workspace] isolation table, [package], one dependency) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO (standalone harness manifest; the DEPENDENCY it pins is shipped: codec-dnx default feature) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |

## 1. One-paragraph verdict
A correct standalone-harness manifest: the empty `[workspace]` table genuinely pins its own workspace root (proven: the committed tools/vta/dnx/Cargo.lock carries `vta-dnx` as the root package — a lock that could only exist with an isolated workspace), the candidate pin `=0.9.0` with `default-features = false` is EXACTLY the shipped pin's posture (crates/codecs/Cargo.toml:158 — same version operator, same features posture), `publish = false`, edition 2021. One staleness: the comment calls the pin "Proposed admission" while the admission has since landed (codec-dnx is a default feature in the root workspace).

## 2. What actually works (verified)
— `=0.9.0` exact pin (L17) == shipped pin at crates/codecs/Cargo.toml:158 (grep, ran in the main.rs audit).
— `default-features = false` == shipped posture (same line).
— Workspace isolation verified: tools/vta/dnx/Cargo.lock exists with `vta-dnx 0.1.0` as the lock's root package (read; the root workspace's lock does not contain vta-dnx).
- Candidate checksum binding: the committed lock pins filmcraft-dnx 0.9.0 ck `a3a59d55...` == root Cargo.lock entry (byte-compared, ran).

## 3. Findings

F-001 [S4]
Where: tools/vta/dnx/Cargo.toml:14
Code:
```
# Proposed admission pin: exact version, default-features OFF (drops the
```
What is wrong: "Proposed" is stale — filmcraft-dnx =0.9.0 is ADMITTED (codec-dnx in the root default feature set; root Cargo.lock carries it). The comment reads as pre-admission state. Confidence: HIGH — Verification: read + admission wiring verified by grep (crates/codecs/Cargo.toml:52/158, ran).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| manifest | 5-17 | REAL | S4 | F-001 comment staleness only |

## 5. Test honesty
N/A (manifest). The harness it configures has committed evidence (see the README claim ledger).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "NOT a workspace member (root members = crates/*, apps/*, tools/xtask)" | L1-3 | REAL | empty [workspace] + own lock with vta-dnx root (verified) |
| "single-threaded decode path is what we measure — DecodeOptions { threads: false } is the no-rayon path" | L14-16 | REAL-UNPROVEN | features posture matches (no rayon in the harness lock — the lock contains no rayon, verified by reading it); the DecodeOptions claim is the record's domain |

## 7. Dead / duplicated / unreachable code
None.

## 8. Dependencies & flags
filmcraft-dnx =0.9.0, default-features=false. No [dev-dependencies], no profile overrides.

## 9. Open questions for the next agent
1. None open — this manifest's claims are verified; the sibling-harness comparison (ruopus/vp9 add `demux`/`plugin-api` path deps; this one measures a raw-ES file so needs neither) explains the smaller dep set legitimately.
