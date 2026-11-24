# AUDIT: scripts/packaging/build-targz.sh
| Field | Value |
|---|---|
| Auditor | W17-A02 |
| Date | 2026-11-24 |
| Lines / functions | 76 / 2 (main + die()) |
| File verdict | REAL (no placeholder artifacts; all failure paths loud) |
| Reachable from shipped entry point? | YES (release lane): verify-linux-artifacts.sh:115 executes it; docs/release-checklist.md:131 archive lane; docs/packaging/README.md:64 "CI Linux jobs; any host" PROVEN with evidence log |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `bash -n` -> OK; `sh scripts/packaging/build-targz.sh` (no env) -> first-guard die, exit 2; grep callers -> verify-linux-artifacts.sh:115; sed quotes verified |

## 1. One-paragraph verdict
The flat-archive builder is the cleanest packaging script audited so far:
`set -eu`, seven `:?` env guards, hard `die` on missing CLI *and* tauri
binaries (no stub shipped — L6-9 records that the exit-86 skeleton was
removed in w30.8), shipped-docs existence re-asserted, staging in `mktemp -d`
with a trap cleanup, explicit install modes (0755/0644), POSIX-safe argv
accumulation (`set --`) for the tar members, and a final non-empty-output
check. I executed its guard path (dies immediately without env). The only
defect is byte-nondeterminism of the archive (no --mtime/--owner), which
matters only because the release lane publishes checksums of these archives.

## 2. What actually works (verified)
- `bash -n scripts/packaging/build-targz.sh` -> exit 0.
- Ran with empty environment: `PRODUCT_LOWER: build-targz.sh: ... not set`
  printed, exit 2 — guards live.
- Both binaries guarded (L40-47), docs guarded (L51-53), output guarded
  (L74 `[ -s "$out" ] || die`).
- Caller chain: scripts/packaging/verify-linux-artifacts.sh:115 runs this
  script as artifact #1 of the Linux lane verify.

## 3. Findings
F-001 [S4] — Archive bytes are not reproducible
Where: scripts/packaging/build-targz.sh:73-74
Code:
```
tar -czf "$out" -C "$stage" "$@"
[ -s "$out" ] || die "tar.gz is missing or empty: $out"
```
What is wrong: plain `tar -czf` embeds the staged files' current mtimes
(installed seconds earlier) and uid/gid; gzip adds nothing stable either.
Two builds of the same commit yield different bytes. Why it matters: the
release lane publishes sha256 checksums (verify-linux-artifacts.sh), so a
user re-building from source can never match the published checksum —
the checksum identifies the build, not the source. No repo doc audited so
far claims reproducibility for this lane, so this stays hygiene. What
correct looks like: `tar --mtime=@0 --owner=0 --group=0 --sort=name` (GNU)
plus `GZIP=-n`/`gzip -n` style control.
Confidence: HIGH — Verification: read (tar semantics); not re-executed
(no release binaries present; cargo build banned by load discipline).
UNVERIFIED: the exact downstream checksum-publishing claim (release.yml is
another agent's file).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main | 20-76 | REAL | — | guards/installs/tar all verified; see F-001 for the one nit |
| die() | 35 | REAL | — | stderr + exit 1 |

## 5. Test honesty
Not a test file. docs/packaging/evidence/w26m-t2-linux-payload.md "Proof 1"
shows `bash -n`/`sh -n` over this script and gate.sh PASS (historical);
baseline packaging gate PASS is the orchestrator's citation for current state.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "EXECUTED for real in the local 8.E3 proof ... on every release-lane OS" (L18-19) | header | REAL-UNPROVEN | evidence logs exist (w26m/w24c); current-tree execution UNVERIFIED (would need release binaries) |
| output name `<slug>-<version>-<arch>-<os>.tar.gz` (L17) | header | REAL | L66-67 `base="${PRODUCT_LOWER}-${PRODUCT_VERSION}-${ARCH_ID}-${OS_ID}"` matches (PRODUCT_LOWER as slug) |
| "the two shipped binaries ... + the docs list ... at the archive root" (L5-11) | header | REAL | L60-73: exactly BIN_CLI, BIN_TAURI, each doc; nothing else staged |
| w26m evidence row "set -- ... $BIN_CLI $BIN_DESKTOP $BIN_TAURI" | docs/packaging/evidence/w26m-t2-linux-payload.md:23 | STALE-DOC (historical) | current L69 has two members only ($BIN_DESKTOP removed w30.8) — evidence predates the removal |

## 7. Dead / duplicated / unreachable code
None. `OS_ID`/`ARCH_ID` used only in the output name — both consumed.

## 8. Dependencies & flags
tar+gzip (host native), install, mktemp, packaging-env.py exports. No cargo,
no network, no python needed. POSIX sh, `bash -n` clean — verified.

## 9. Open questions for the next agent
- Does release.yml's published-checksum step assume stability across
  rebuilds? (release lane auditor: check what the checksums are claimed to prove.)
- build-zip.sh (not my file) shares this shape — same determinism question.
