# AUDIT: tools/bench-vs-vlc/Cargo.lock
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Date | 2026-11-24 |
| Lines / functions | 689 / n/a (generated lockfile) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — standalone harness workspace, dev tool only |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | grep/awk over the lock: 78 packages, 70 registry sources, 0 git+ sources |

## 1. One-paragraph verdict
A genuine cargo-generated lockfile (version 4 header, checksums present on every registry package). Consistent with the standalone manifest: path deps on codec-api/codecs/demux/input/media-probe/media-types/plugin-api appear as the bench-vs-vlc package's direct dependencies; no git+ or patch sources; 70 registry crates, all symphonia-family + toolchain-adjacent. Crucially it contains `rust_h264 0.6.1` — the lock was regenerated after w27 admitted the codec, so this is NOT a repeat of the stale-subcrate-lock defect that broke the desktop release lane (worklog w28.normal-app, "PRE-EXISTING BREAKAGE FIXED").

## 2. What actually works (verified)
— verified by `grep -A2 'name = "rust_h264"'` -> present at lock with registry source; root Cargo.lock also has it (line 769) — no drift between the two locks on this crate.

## 3. Findings
None. Zero-findings justification: checked (a) generated header intact, (b) package count 78 = 70 registry + 8 path/engine crates, (c) no git/patch/local-override sources, (d) rust_h264 post-w27 refresh present, (e) checksums on all registry entries.

## 4. Per-function table
N/A (data file).

## 5. Test honesty
N/A.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Deliberately a STANDALONE workspace ... Std-only, zero external dependencies" | tools/bench-vs-vlc/Cargo.toml header | MOSTLY | "zero external dependencies" means zero *direct* external deps; the lock legitimately carries 70 transitive registry crates pulled by the engine path deps (symphonia, atomig, cc, syn...). The wording overstates purity; the intent (no new direct deps) holds. |

## 7. Dead / duplicated / unreachable code
N/A.

## 8. Dependencies & flags
See above; lints deny-set mirrors root workspace per Cargo.toml comment — verified present in Cargo.toml read.

## 9. Open questions for the next agent
Compare lock freshness vs root Cargo.lock for the *input* crate `contract` feature toggle if the engine crates gain new default deps later.
