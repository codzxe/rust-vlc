# AUDIT: tools/vta/dnx/Cargo.lock
| Field | Value |
|---|---|
| Auditor | W17-A09 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Lines / functions | 81 / n/a (generated lockfile, 9 packages) |
| File verdict | REAL (data: internally consistent, triple-checksum-corroborated, cannot be re-generated here — no cargo) |
| Reachable from shipped entry point? | NO — by design: tools/vta/dnx is a standalone VTA spike workspace (empty `[workspace]` table, tools/vta/Cargo.toml:1-5); NOT a member of the root workspace (rg vta in root Cargo.toml/Cargo.lock -> no hits) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 (zero findings — checked list in §2) |
| Commands run | rg checksum/rayon/vta greps (all clean/verified); ls/find of the spike dir; sed quote verifications; wc counts (8 checksums / 9 packages) |

## 1. One-paragraph verdict
A correct, self-consistent generated lockfile for the filmcraft-dnx VTA
differential spike: 9 packages, 8 registry entries all carrying 64-hex
checksums, the local harness (vta-dnx 0.1.0, no checksum — correct for a
path package), and a dependency graph that exactly matches the manifest's
promise — filmcraft-dnx "=0.9.0" with default-features=false yields deps of
ONLY filmcraft-bitstream + thiserror, and NO rayon anywhere in the lock,
which is the no-`threads`-feature claim proven at lock level. The two
filmcraft checksums byte-match BOTH the root workspace Cargo.lock (the
admitted copy) AND the dependency record's independently downloaded-tarball
sha256 — a three-way match that makes this the best-corroborated lock data
in my batch. Zero findings.

## 2. What actually works (verified) — the zero-findings checklist
Exactly what was checked and how:
1. Structure: 9 `name =` entries (rg -c), 8 `checksum = "[0-9a-f]{64}"`
   entries (rg -o | wc -l) — the 9th (vta-dnx, :77-81) is the local
   path package, correctly checksum-less.
2. Manifest↔lock consistency: Cargo.toml:17 `filmcraft-dnx = { version =
   "=0.9.0", default-features = false }` ↔ lock filmcraft-dnx 0.9.0 with
   deps exactly `[filmcraft-bitstream, thiserror]` (:11-19); vta-dnx deps
   exactly `[filmcraft-dnx]` (:76-81).
3. No-rayon proof: rg rayon tools/vta/dnx/Cargo.lock -> NO-RAYON-IN-SPIKE-
   LOCK — the default-features=false claim is enforced by the lock's
   membership (features only materialize as lock entries).
4. Triple checksum corroboration:
   - filmcraft-dnx: `a3a59d559caf74b9281578f06015702bd402192198f71f5d41e7c6a9ec3592ab` == root Cargo.lock:220 == record tarball sha256 (docs/dependencies/records/filmcraft-dnx-0.9.0.md:138-139).
   - filmcraft-bitstream: `08f8be9fae696d78e233591b4d57871c39f15ac0ac7445168356cda705efa499` == root Cargo.lock:214.
5. Isolation: no vta entries in root Cargo.toml/Cargo.lock — the spike
   cannot pollute the shipped build; conversely the spike lock carries none
   of the root workspace's 150+ crates.
6. Transitive closure sanity: thiserror 2.0.21 → thiserror-impl →
   proc-macro2/quote/syn/unicode-ident — the full closure for the
   manifest's declared deps, nothing extra.
7. Evidence neighbors exist: evidence/compare.py, compare-results.txt,
   lane-md5s.txt, ffprobe-facts.txt, hostile-results.txt all present
   (find) — the README's committed-evidence list checks out.

## 3. Findings
None (S0–S4 = 0). Nothing in this lockfile is wrong, stale, or
inconsistent with its manifest, its sibling evidence, the root lock, or
the dependency record.

## 4. Per-function table
N/A — generated data file, no functions.

## 5. Test honesty
N/A — no tests in a lockfile. Execution of the harness itself is recorded
in the record (dnx_differential 6/6) and evidence files; rebuilding it here
is UNVERIFIED (no cargo in sandbox; cargo-build-budget).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Proposed admission pin: exact version, default-features OFF (drops the threads feature and with it rayon)" | tools/vta/dnx/Cargo.toml:14-17 | TRUE | lock has filmcraft-dnx =0.9.0 with deps [bitstream, thiserror] and zero rayon entries |
| "Tarball fetched ... downloaded-file sha256 `a3a59d55...`" | docs/dependencies/records/filmcraft-dnx-0.9.0.md:138-139 | TRUE | byte-equal to this lock's checksum and the root lock's |

## 7. Dead / duplicated / unreachable code
Unreachable from the shipped entry point BY DESIGN (validate-then-admit
spike, excluded workspace). Not dead: it is the input contract for
re-running the differential (README:7-36) and its record cites it.

## 8. Dependencies & flags
Graph (from the lock): vta-dnx → filmcraft-dnx(0.9.0) → {filmcraft-bitstream
(0.9.0), thiserror (2.0.21 → thiserror-impl → syn 3.0.9/quote 1.0.47/
proc-macro2 1.0.107/unicode-ident 1.0.27)}. No feature flags in a v4 lock.

## 9. Open questions for the next agent
- The spike lock's transitive versions (syn 3.0.9, proc-macro2 1.0.107)
  are VTA-era resolutions; if the harness is re-run after a `cargo update`,
  the differential evidence (compare-results.txt) should be regenerated —
  the lock as committed is the reproducibility anchor.
