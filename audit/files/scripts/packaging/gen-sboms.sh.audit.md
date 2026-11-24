# AUDIT: scripts/packaging/gen-sboms.sh
| Field | Value |
|---|---|
| Auditor | W17-A02 |
| Date | 2026-11-24 |
| Lines / functions | 53 / 2 (main + die()) |
| File verdict | REAL (writer loop with genuine config-vs-output cross-check) |
| Reachable from shipped entry point? | YES (release lane): release.yml:399 (Windows zip+msi), release.yml:571 and :709 (macOS dmg+zip) call it directly; Linux lane via verify-linux-artifacts.sh:308 (step 8) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | `bash -n` -> OK; `sh scripts/packaging/gen-sboms.sh` (no env) -> guard die, exit 2; grep callers -> release.yml/verify-linux-artifacts.sh hits above; sed sbom.rs arg parse ( `--out` supported); grep packaging.toml [sbom] |

## 1. One-paragraph verdict
A real, small, honest wrapper: env guards, artifact existence check, the
release SBOMs are freshly regenerated (`cargo run -p xtask --locked -- sbom
--out <artifact>.sbom.cdx.json`), output non-emptiness is asserted, and — the
genuinely good part — the emitted `specVersion` is grepped against
packaging.toml's `sbom.spec_version` so generator/config drift is a hard
error. The one design flaw is inherited from the generator's scope: every
"per-artifact" SBOM is the byte-identical whole-workspace lockfile graph, so
the SBOM sitting beside a macOS .dmg lists windows_x86_64_gnu crates. The
header discloses this ("self-containment, not divergence") — disclosed, but
it still means the artifact-level SBOM is not an inventory of that artifact.

## 2. What actually works (verified)
- `bash -n` -> exit 0. No-env execution dies on first `:?` guard (exit 2).
- xtask `--out` support verified by reading tools/xtask/src/cmd/sbom.rs
  arg parsing (`"--root" | "--lock" | "--out"` consume a path).
- Caller claims verified verbatim: release.yml:396-399, :568-571, :706-709
  ("Per-artifact SBOM (gen-sboms.sh ...)"), verify-linux-artifacts.sh:54,308.
- packaging.toml:37-39 `[sbom] spec_version = "1.5" / artifact_suffix =
  "sbom.cdx.json"` — both consumed via env, no literals in the script.
- specVersion cross-check regex `"specVersion": *"${VER}"` accepts both
  pretty and compact JSON spacing — read, correct.

## 3. Findings
F-001 [S3] — "Per-artifact" SBOMs are identical workspace graphs, not
artifact inventories
Where: scripts/packaging/gen-sboms.sh:5-9 (design), :44 (single writer call)
Code:
```
# Wraps `cargo run -p xtask --locked -- sbom` (the wave-23 std-only
# CycloneDX generator) once per release artifact, so every download
# set carries its own SBOM and every release asset is self-contained.
# Content is lockfile-derived, hence byte-identical across platforms;
# the per-artifact copies exist for self-containment, not divergence.
```
What is wrong: the same 158-component workspace graph (with
windows_aarch64_*, wasi, redox_syscall, etc.) is written beside every
artifact on every platform. A consumer doing per-artifact analysis of the
macOS .zip SBOM gets Windows-only crates as declared dependencies of "the
artifact". Why it matters: the suffix `artifact.sbom.cdx.json` implies
artifact-scoped BOM; content is workspace-scoped. Disclosure in the header
keeps this at S3 (quality defect, limited blast radius). What correct looks
like: filter components by the artifact's platform/contents, or name the
sidecar `<product>.sbom.cdx.json` (product-scoped, no per-artifact claim).
Confidence: HIGH — Verification: read + my cyclonedx.json audit (component
set is the full lock, all platforms).
Corollary worth recording: per-release SBOMs are FRESH at generation time
(built from the current Cargo.lock), unlike the committed
docs/security/sbom/cyclonedx.json which is stale by one edge — so the
release artifacts and the repo's committed SBOM can disagree within one
release.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main | 29-53 | REAL | — | loop: existence, generate, -s check, specVersion grep |
| die() | 37 | REAL | — | stderr + exit 1 |

## 5. Test honesty
Not a test file. "EXECUTED for real against the committed linux-lane
artifacts (r3 log)" (L27-28) — docs/packaging/evidence/w24c-r3.log exists;
current-tree re-execution UNVERIFIED (needs cargo build of xtask — load
discipline; cargo-build-budget).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "release workflow calls this directly on the Windows/macOS jobs; the Linux lane ... verify-linux-artifacts.sh step 8" (L19-21) | header | REAL | release.yml:399/571/709; verify-linux-artifacts.sh:308 |
| "specVersion ... asserted against packaging.toml [sbom] spec_version ... hard error" (L14-17) | header | REAL | script L50-51 grep || die |
| "byte-identical across platforms" (L8) | header | REAL-UNPROVEN | follows from deterministic generator + identical lock; byte-diff not run (would need two platform builds) |
| "one writer, two callers" (L21) | header | REAL | callers listed above (plus release.yml:837 comment) |

## 7. Dead / duplicated / unreachable code
None. `PACKAGING_SBOM_SPEC_VERSION` guard (L32) is consumed at L50-51; both
env vars live.

## 8. Dependencies & flags
cargo + tools/xtask (buildable in release jobs), packaging-env.py exports,
POSIX sh. `cargo run --locked` — reproducible-ish toolchain use. No network.

## 9. Open questions for the next agent
- VERIFIED (this agent, follow-up): release.yml's `linux-arm64` job (which
  runs the same lane incl. per-artifact SBOMs) carries job-level
  `continue-on-error: true` (release.yml:871-876, "best-effort ... the
  release still ships the other five artifact sets"). So on aarch64 the
  per-artifact SBOM generation is cannot-block-the-release; only the other
  platform lanes' SBOM steps are blocking. Confirmed for release.yml's
  Windows/macOS SBOM steps: plain `run:` steps, no continue-on-error
  (release.yml:396-399).
