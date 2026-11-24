# AUDIT: docs/security/sbom/cyclonedx.json (data audit, chunks L1-4780)
| Field | Value |
|---|---|
| Auditor | W17-A02 |
| Date | 2026-11-24 |
| Lines / components | 4780 / 158 (+1 synthetic root `Lumen@0.1.3`) |
| File verdict | SHALLOW (data artifact: inventory REAL and exact vs Cargo.lock; dependency graph stale by 1 edge; no timestamp, blanket `required` scope) |
| Reachable from shipped entry point? | N/A (docs artifact). Consumed by `scripts/full_gate.sh` §9 sbom --check (baseline: FAILED/DRIFT) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 1 |
| Commands run | python3 cross-check vs Cargo.lock (components/edges/checksums) -> exact match except 1 edge; grep/sed quotes verified; grep device-discovery Cargo.lock |

## 1. One-paragraph verdict
The component inventory is genuine and byte-faithful to Cargo.lock: all 158
packages match name@version, all 119 registry checksums match, no duplicates,
no phantom crates. But the `dependencies` graph is stale by exactly one edge
(`device-discovery@0.1.0 -> plugin-api@0.1.0`), which was deleted from
device-discovery's Cargo.toml (and Cargo.lock) on 2026-11-17 and never removed
here. The repo's own drift gate is red about it and the stale SBOM is shipped
anyway. The file also omits `metadata.timestamp` entirely, so the artifact
carries zero generation-freshness evidence, and labels all 158 components
`scope:"required"` including dev-only crates like `assert_matches`.

## 2. What actually works (verified)
- python3 full-file parse: 158 components == 158 Cargo.lock packages; set difference empty both directions; 119 SHA-256 contents identical to Cargo.lock `checksum` fields; 39 path crates correctly carry no hash/purl; source property distribution {registry:119, path:39}; no duplicate bom-refs; every dependency ref resolves.
- Root `Lumen@0.1.3` dependsOn 39 == exact workspace membership (31 crates/* + 4 apps (desktop-tauri excluded) + xtask + mem-audit + sweep + first-vertical).
- JSON parses clean; CycloneDX 1.5, serialNumber present.

## 3. Findings
F-001 [S2] — SBOM dependency graph stale by one edge (Wave 1's "stale by one edge" confirmed and pinpointed)
Where: docs/security/sbom/cyclonedx.json:3828-3833 vs Cargo.lock:191-196
Code (SBOM):
```
      "ref": "device-discovery@0.1.0",
      "dependsOn": [
        "media-types@0.1.0",
        "plugin-api@0.1.0",
        "testkit@0.1.0"
      ]
```
Code (Cargo.lock):
```
name = "device-discovery"
version = "0.1.0"
dependencies = [
 "media-types",
 "testkit",
]
```
What is wrong: the SBOM asserts a device-discovery->plugin-api edge that
Cargo.lock does not have; crates/device-discovery/Cargo.toml removed plugin-api
("NOTE (audit T5, 2026-11-17): the registry row once listed plugin-api here
... the declared dep was dead weight"). Why it matters: a supply-chain
consumer doing impact analysis (e.g. a plugin-api advisory) gets a wrong
dependency graph from the "authoritative" SBOM. The repo knows: baseline
full_gate §9 `sbom: FAILED ... result: DRIFT — regenerate with cargo xtask
sbom and commit` (audit/runs/baseline/full_gate.txt:69-75). What correct
looks like: regenerated file without line 3831.
Confidence: HIGH — Verification: ran (python3 edge diff: exactly this edge in
SBOM minus lock; all other 367 edges equal).

F-002 [S3] — No `metadata.timestamp` anywhere
Where: docs/security/sbom/cyclonedx.json:7-14
Code:
```
  "metadata": {
    "component": {
      "type": "application",
      "bom-ref": "Lumen@0.1.3",
      "name": "Lumen",
      "version": "0.1.3",
```
What is wrong: metadata has only component+properties; no timestamp field
(top-level keys are only $schema/bomFormat/specVersion/serialNumber/version/
metadata/components/dependencies — python3 `list(sbom.keys())`). serialNumber
is a fixed UUID, so nothing in the file distinguishes a fresh SBOM from one
rotting for months. Combined with F-001 the artifact's age is undetectable
in-band. Why it matters: SBOM consumers cannot judge freshness; NTIA minimum
elements call for a timestamp.
Confidence: HIGH — Verification: ran (json key dump).

F-003 [S3] — Blanket `scope: "required"` mislabels dev-only dependencies
Where: docs/security/sbom/cyclonedx.json:36 (scope field on every component; 158/158 `required` per python3 scope distribution)
Code:
```
      "type": "library",
      "bom-ref": "pkg:cargo/assert_matches@1.5.0",
      "name": "assert_matches",
      "version": "1.5.0",
      "scope": "required",
```
What is wrong: assert_matches is a dev-dependency of rav1d (Cargo.lock:679
lists it only inside rav1d's deps; upstream it is test-only), yet it is
declared `required` like every other component. Cargo.lock cannot express
dev/prod, and the generator (property `xtask sbom (std-only; ...)`) made no
attempt — so dev-only crates are indistinguishable from shipping deps. Same
applies to e.g. `nasm-rs`/`jobserver` (build-time). Why it matters: CycloneDX
`scope` is exactly the field for this; ignoring it inflates the runtime
dependency set a consumer must track/vuln-scan. What correct looks like:
`optional`/dev distinction from Cargo.toml manifests, or at minimum honest
`scope` from xtask's knowledge of the release lane.
Confidence: MEDIUM (dev-dep status of assert_matches is from upstream rav1d
metadata — UNVERIFIED locally beyond Cargo.lock's rav1d-only reference).

F-004 [S4] — Serial number is a frozen constant
Where: docs/security/sbom/cyclonedx.json:5 `"serialNumber": "urn:uuid:57dc9149-8ff3-89ca-8883-92b89fc20cec"`
What is wrong: deterministic/constant serial means every regenerated SBOM
claims to be the "same" BOM; CycloneDX intends serialNumber to be unique per
BOM instance. Cosmetic given no timestamp either, but it cements the
"regeneration is invisible" problem from F-002.
Confidence: HIGH — Verification: read.

## 4. Chunk coverage (functions N/A — data file)
- part1 L1-795: $schema/metadata + components assert_matches..bytes — read; inventory spot-checked, hashes match lock.
- part2 L796-1595: cc..(registry crates) — covered by programmatic full-file parse (checksums all verified).
- part3 L1596-2395: workspace path crates — verified 39 path components have no hash/purl.
- part4 L2396-3194: rav1d..symphonia — rav1d edge set matches lock exactly.
- part5 L3195-3994: tail components + start of dependencies — device-discovery edge F-001 lives here (L3828-3833).
- part6 L3995-4780: dependencies graph tail — all 159 entries resolve; 367/368 edges match lock.

## 5. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| generator is xtask sbom, std-only, from Cargo.lock+product.toml | L17-18 property | REAL-UNPROVEN | generator code owned by tools/xtask/src/cmd/sbom.rs (audited by another wave); lock fidelity verified here |
| Cargo.lock version v4 | L21-22 | REAL | Cargo.lock:3 `version = 4` |
| license fields omitted; cargo-deny owns policy | L25-27 | REAL-UNPROVEN | Cargo.lock carries no license fields (grep: none); cargo-deny execution UNVERIFIED here (quality/07_supply_chain.sh not in my lane) |
| SBOM is current/committed intentionally | implicit | FALSE in effect | drift gate FAILED in baseline full_gate.txt:69-75 |

## 6. Dead / duplicated / unreachable code
N/A (data). Note: root `Lumen@0.1.3` is synthetic (no such Cargo.lock package);
its 39 dependsOn exactly equal workspace membership — verified.

## 7. Dependencies & flags
SBOM covers ONLY the root workspace lock. It does NOT cover fuzz/Cargo.lock,
tests/compat/Cargo.lock, apps/desktop-tauri/Cargo.lock (excluded member with
its own lock), or tools/vta/* (standalone, non-member crates). If any doc
claims "SBOM covers everything shipped", that claim is false for those lanes.

## 8. Open questions for the next agent
- Does `tools/xtask/src/cmd/sbom.rs` regenerate deterministically (constant serial) — i.e., does a commit-time hook exist that would have caught F-001? full_gate has the check but gate is red and artifacts shipped anyway (orchestrator-level process finding).
- ffprobe-facts.txt (my next file) is evidence for tools/vta/dnx — a NON-member crate outside this SBOM's coverage.
