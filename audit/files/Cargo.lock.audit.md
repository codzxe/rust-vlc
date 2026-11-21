# AUDIT: Cargo.lock
| Field | Value |
|---|---|
| Auditor | W01-A04 |
| Date | 2026-11-21 |
| Lines / packages | 1441 / 158 `[[package]]` entries (39 workspace, 119 registry) |
| File verdict | REAL |
| Reachable from shipped entry point? | N/A (build input, not code) — consumed by every `cargo --locked` run (baseline #1/#3/#4, all executed against this exact file) and by the `xtask sbom` drift gate (baseline #15) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 2 S4: 2 |
| Commands run | `sed -n` (quote verification, all findings); `awk`/`sort`/`uniq` state-machine parse of all 158 package blocks; `rg -n '^source|^dependencies|^name' Cargo.lock`; `rg -n '^\[(patch\|replace)' --glob '**/Cargo.toml'`; `jq` over docs/security/sbom/cyclonedx.json (components/dependencies/hashes); python3 edge-set reconstruction lock↔SBOM; `git log/diff --stat` (timeline, worktree purity). No cargo/npm run. |

## 1. One-paragraph verdict
The root lockfile itself is healthy: format v4, all 39 workspace members present with matching
name+version, the excluded `apps/desktop-tauri` correctly absent, every registry dep pinned to
crates.io with a checksum, zero git deps, zero `[patch]`/`[replace]` sections, zero exotic
sources, zero dangling dependency references, and only 3 crates held at two versions each —
and baseline run #1 (`cargo build --release --locked --workspace`, exit 0) executed against it.
The damage is in what surrounds it: the committed SBOM (docs/security/sbom/cyclonedx.json) is
stale against this lockfile by exactly one dependency edge (quantified in F-002 — the baseline
#15 DRIFT), the SBOM's "product subject Lumen 0.1.3" omits the entire 493-package shipped
desktop-tauri closure (F-001), and the SBOM presents dev tools and the `_template` scaffold as
"required" product components (F-003).

## 2. What actually works (verified)
— Lockfile↔members consistency: parsed all 39 member manifests (`crates/*`, `apps/*`,
`tools/xtask|mem-audit|sweep`, `tests/integration/first-vertical`, minus excluded
`apps/desktop-tauri`) and diffed name+version against the 39 source-less lockfile packages →
`MEMBERS_MATCH_LOCKFILE_39_of_39` (empty diff). Excluded `lumen-desktop-tauri 0.1.3` absent. ✔
— Lockfile builds: baseline INDEX #1 `cargo build --release --locked --workspace` → exit 0
(3m46s) at e83e5bd; `git diff --stat e83e5bd3..HEAD` shows only `audit/` files changed, so the
run is valid for HEAD. ✔
— Checksum coverage: 119/119 registry packages carry `checksum = "…"`; awk state machine found
zero registry packages without one. ✔
— Source hygiene: `rg '^source' Cargo.lock | sort | uniq -c` → one line: `119 …
registry+https://github.com/rust-lang/crates.io-index`; `rg 'git\+' Cargo.lock` → 0;
`rg '^\[(patch|replace)'` across every `Cargo.toml` → no matches. ✔
— Edge resolution: python3 reconstruction of all 368 `dependencies = [...]` edges → every
target resolves to a package in this lockfile (0 dangling). ✔

## 3. Findings

### F-001 [S2] Product SBOM omits the entire shipped desktop-tauri dependency closure
Where: docs/security/sbom/cyclonedx.json:10-13 (subject), Cargo.toml:51 (exclusion),
docs/security/sbom/README.md:55-57 (known-limits register)
Code:
```
      "bom-ref": "Lumen@0.1.3",
      "name": "Lumen",
      "version": "0.1.3",
      "publisher": "Lumen Contributors"
```
```
exclude = ["apps/desktop-tauri"]
```
```
- Covers the Rust/Cargo graph only. The wasm build graph (doc 09) and
  the npm UI graph (doc 11 §3.4 `sbom-ui.json` [PROPOSAL]) are separate
  future SBOMs; per-platform finalization is card 8.E3.
```
What is wrong: The SBOM declares the *product* "Lumen@0.1.3" as subject, but the generator
reads only `<root>/Cargo.lock` (tools/xtask/src/cmd/sbom.rs:157 default). The workspace
excludes `apps/desktop-tauri` (ADR-004, own lockfile), so the shipped GUI app's closure —
493 packages / 472 registry deps in `apps/desktop-tauri/Cargo.lock`, including the entire
tauri/wry/webkit2gtk attack surface — is invisible: `rg -c tauri docs/security/sbom/cyclonedx.json`
→ 0. The README's "known limits (honest register)" lists the wasm and npm graph gaps but not
this one.
Why it matters: A CycloneDX SBOM exists so vulnerability scanners can match advisories against
what ships. The installable desktop binary (MSI/installer lanes per commits 4ee9fb33/53a106fb)
ships the 493-package closure; a CVE in tauri/wry/webkit2gtk-sys would never be flagged by
scanning this artifact. Silent omission in a security artifact = wrong result in the artifact's
one realistic use case.
What correct looks like: Either a second committed SBOM generated with `--lock
apps/desktop-tauri/Cargo.lock` (the flag already exists), or the README known-limits register
explicitly naming the desktop-tauri closure as uncovered, and the subject renamed so it cannot
be read as the whole product.
Confidence: HIGH — Verification: read (rg/jq counts above; `wc -l apps/desktop-tauri/Cargo.lock`
→ 5138 lines, 493 `[[package]]`, 472 registry sources, 0 git).

### F-002 [S3] Committed SBOM is stale vs this lockfile by exactly ONE dependency edge (the baseline #15 DRIFT, quantified)
Where: docs/security/sbom/cyclonedx.json:3828-3833 vs Cargo.lock:190-196
Code (SBOM):
```
      "ref": "device-discovery@0.1.0",
      "dependsOn": [
        "media-types@0.1.0",
        "plugin-api@0.1.0",
        "testkit@0.1.0"
      ]
```
Code (lockfile):
```
[[package]]
name = "device-discovery"
version = "0.1.0"
dependencies = [
 "media-types",
 "testkit",
]
```
What is wrong: Quantification of the baseline INDEX #15 DRIFT (exit 1): component sets are
IDENTICAL — 158 components in the committed SBOM = 158 `[[package]]` rows here, same
name@version pairs (set-diff empty); all 119 SHA-256 `hashes` match the lockfile `checksum`
lines byte-for-byte (diff empty); SBOM dependency edges 408 = 39 synthetic root edges
(subject → every path package, by README design) + 369 component edges vs the lockfile's 368
component edges. The delta is exactly one stale edge: `device-discovery@0.1.0 →
plugin-api@0.1.0`, present in the committed SBOM, absent from this lockfile. Timeline (git
log): release commit 53a106fb regenerated the SBOM ("158 components … no drift") while
device-discovery still depended on plugin-api; audit-T5 commit 0e4bc2cc removed the dead dep
from `crates/device-discovery/Cargo.toml` (its own NOTE comment says so) and 5f5e34f9 refreshed
this lockfile — neither ran `cargo xtask sbom`.
Why it matters: The SBOM at HEAD misstates the resolved graph; the drift gate
(docs/security/sbom/README.md:13-15 — "Any change to `Cargo.lock` … must be followed by
`cargo xtask sbom` + commit, or the merge gate fails") is failing at HEAD (baseline #15 exit 1),
i.e. the repo's own security lane is red and the project's own rule was violated by its audit
commits. Blast radius is one edge in a doc artifact, so S3 not S2.
What correct looks like: Run `cargo xtask sbom` and commit; going forward, the T5-style
"remove dead dep + refresh lockfile" commits must include the SBOM regen the README mandates
(or the gate should be wired into the same commit hook).
Confidence: HIGH — Verification: read + python3/jq set-diff executed; baseline #15 cited for the
gate failure.

### F-003 [S3] SBOM lists dev tools, the test suite, and the `_template` scaffold as "required" components of the product subject
Where: docs/security/sbom/cyclonedx.json:3635-3638 & 3670-3675 (root node; components
`xtask@0.1.0`, `lumen-mem-audit@0.1.0`, `lumen-sweep@0.1.0`, `first-vertical@0.1.0`,
`template@0.1.0` all `"scope": "required"`)
Code (verbatim, elided middle marked):
```
      "ref": "Lumen@0.1.3",
      "dependsOn": [
        "audio-engine@0.1.0",
        "audio-output@0.1.0",
```
```
        "template@0.1.0",
        "testkit@0.1.0",
        "theme-tokens@0.1.0",
        "transcode@0.1.0",
        "video-engine@0.1.0",
        "xtask@0.1.0"
```
What is wrong: The subject "Lumen@0.1.3" (the product) is asserted to *require* the build
tooling (xtask, mem-audit, sweep), the integration test crate (first-vertical), and the crate
scaffold (`template`, i.e. `crates/_template`, which this lockfile correctly shows with zero
dependents). README.md:41-44 admits "Shipped-app vs dev-tool split is NOT encoded in
Cargo.lock" — but the artifact still emits `scope: required` for all of them.
Why it matters: A machine consumer (scanner/license tool) reads this as the shipped product's
bill of materials and will misattribute advisories/licenses to the product from tools that
never leave the build host. Misleading security-artifact content, documented only in a
human-read sidecar.
What correct looks like: Emit `scope: optional`/`excluded` (CycloneDX supports both) for
dev-only path packages, or split dev tooling out of the product SBOM.
Confidence: HIGH — Verification: read (jq over components/dependencies; `template` dependents
count in lockfile = 0 via rg).
Note: root cause is the generator/README design, not this lockfile — cross-ref to whoever owns
tools/xtask/src/cmd/sbom.rs and docs/security/sbom/README.md.

### F-004 [S4] Three crates pinned at two versions simultaneously
Where: Cargo.lock:247-253 & 258-261 (getrandom), 269-275 & 278-281 (hashbrown), 1086-1092 & 1097-1100 (syn)
Code:
```
[[package]]
name = "getrandom"
version = "0.2.17"
```
```
[[package]]
name = "getrandom"
version = "0.4.3"
```
(and likewise `hashbrown` 0.15.5 @270 / 0.17.1 @279, `syn` 2.0.119 @1087 / 3.0.9 @1098)
What is wrong: 158 packages resolve from 155 unique names — getrandom, hashbrown and syn each
appear twice. Dependents (awk over `dependencies` arrays): getrandom 0.2.17 ← ring only;
getrandom 0.4.3 ← jobserver; hashbrown 0.15.5 ← string-interner; hashbrown 0.17.1 ← indexmap;
syn 2.0.119 ← atomig-macro, strum_macros, zerocopy-derive; syn 3.0.9 ← ref-cast-impl,
schemars_derive, serde_derive, serde_derive_internals, thiserror-impl. No same-name-same-version
duplicates; all other 152 names single-version.
Why it matters: Pure transitive duplication — compile-time cost and dependency-graph noise, no
runtime type conflicts (dupes are isolated behind unrelated parents). At 3/155 names this is a
healthy graph, not dependency bloat.
What correct looks like: Nothing required; if desired, bump ring (pulls getrandom 0.2) and
string-interner (hashbrown 0.15) when upstreams allow.
Confidence: HIGH — Verification: read (awk/uniq parse of all 158 blocks).

### F-005 [S4] Stale count in the SBOM generator's doc comment
Where: tools/xtask/src/cmd/sbom.rs:7-8
Code:
```
//! of the **resolved Cargo.lock graph** (all 156 lockfile packages:
//! registry dependencies and workspace members alike), with the product
```
What is wrong: The comment hard-codes "all 156 lockfile packages"; this lockfile has 158, and
the committed SBOM (also 158) contradicts the comment its own generator carries. The count is
unstable by nature (every dep change moves it).
Why it matters: Doc comment lies about the artifact it documents; trivial but exactly the kind
of unanchored number a hostile audit exists to catch.
What correct looks like: Drop the literal count from the comment ("all lockfile packages").
Confidence: HIGH — Verification: read (sed -n '5,10p'); cross-ref to owner of sbom.rs.
(Recorded here because the task assigned the lockfile↔SBOM relationship to this audit.)

## 4. Integrity-check table (lockfile equivalent of the per-function table)
| Check | Method | Result |
|---|---|---|
| Lock format version | sed -n '1,4p' | `version = 4`; matches SBOM property `lumen:cargo-lock-version: v4` and baseline #15 note "lock v4" |
| Members ↔ packages | 39 member manifests parsed, diffed vs 39 source-less blocks | EXACT match 39/39; excluded `lumen-desktop-tauri` absent (correct per Cargo.toml:51) |
| Duplicate versions | awk name/version parse, uniq -c | 3 names × 2 versions (F-004); 0 same-version dupes |
| Registry sources | rg '^source' \| sort \| uniq -c | 119/119 `registry+https://github.com/rust-lang/crates.io-index`; nothing else |
| Git deps | rg 'git\+' Cargo.lock (and tauri lock) | 0 |
| Patched crates | rg '^\[(patch\|replace)' all Cargo.toml | 0 sections anywhere |
| Checksums | awk state machine | 119/119 registry packages have SHA-256 checksum; 0 missing |
| Dangling dep refs | python3 edge resolution (368 edges) | 0 unresolved targets |
| `template` crate usage | rg '"template' Cargo.lock | own entry only (name line 1109), 0 dependents — matches root manifest's "no crate may ever depend on it" |
| Lock↔manifest freshness | baseline #1 `--locked` build exit 0 at e83e5bd; git diff e83e5bd3..HEAD = audit/ only | Consistent at HEAD |
| SBOM component set | jq components vs lockfile packages | 158 = 158, identical name@version sets |
| SBOM hashes | jq hashes vs lockfile checksums | 119/119 byte-identical |
| SBOM edges | python3 set-diff | 1 stale edge (F-002): device-discovery→plugin-api in SBOM only |

## 5. Test honesty
No tests target Cargo.lock directly. The de-facto tests are the gates: baseline #1/#3/#4
(`--locked` build/tests) prove the lockfile resolves and is fresh; baseline #15
(`xtask sbom --check`, exit 1) proves the drift gate works and correctly flags the committed
SBOM. These would fail if the lockfile were broken — they are honest evidence.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "SBOM … for the resolved Cargo.lock graph (registry dependencies + workspace members)" | docs/security/sbom/README.md:3-4 | REAL | jq: 158 components = 158 packages, sets identical |
| "Any change to Cargo.lock … must be followed by `cargo xtask sbom` + commit, or the merge gate fails" | README.md:13-15 | REAL (rule enforced; currently violated at HEAD) | baseline #15 exit 1 DRIFT after commits 0e4bc2cc/5f5e34f9 skipped regen |
| "dependencies — the lockfile dependency graph … plus the subject depending on every path-source package" | README.md:41-44 | REAL | jq: 39 root edges to all 39 path packages |
| "all 156 lockfile packages" | tools/xtask/src/cmd/sbom.rs:7 | FALSE | lockfile has 158 (F-005) |
| Known limits list wasm + npm gaps | README.md:55-57 | REAL but INCOMPLETE | tauri-closure gap missing (F-001) |
| "(158 components, lock v4)" drift note | audit/runs/baseline/INDEX.md #15 | REAL — quantified | drift = exactly 1 edge (F-002), not a component-count delta |
| Membership policy comments (39 members, tauri excluded, template member) | Cargo.toml:1-51 | REAL | diff members vs lockfile: 39/39; tauri absent; template present with 0 dependents |

## 7. Dead / duplicated / unreachable code
Not applicable to a lockfile. Nearest equivalents: the `template` scaffold package rides the
workspace graph (by documented design, 0 dependents); duplicate versions covered in F-004.

## 8. Dependencies & flags
No feature flags live in the lockfile. Workspace lints/policies are manifest-side (Cargo.toml:53-67).
Yanked-registry status of the 119 pinned versions is UNVERIFIED — cannot be checked offline
without cargo/network; baseline #1 built them successfully from this exact pin set, which is
the strongest available offline evidence.

## 9. Open questions for the next agent
1. Who owns tools/xtask/src/cmd/sbom.rs — fix F-005 (stale "156"), and consider emitting
   scope=optional/excluded for dev-tool path packages (F-003), plus a tauri-closure SBOM or an
   explicit known-limits entry (F-001).
2. apps/desktop-tauri/Cargo.lock (493 packages) got only a light integrity pass here (sources
   clean, 0 git deps) because it is outside my assignment row — it deserves its own lockfile
   row if the orchestrator wants the GUI closure audited to the same depth.
3. Whether `cargo xtask sbom` regeneration at HEAD produces a byte-identical file except the
   one stale edge (serialNumber is content-derived per README:45-48, so it should also change)
   — REQUEST-ORCH-RUN: `cargo xtask sbom --check` (expect: prints the one-edge diff, exit 1);
   confirms my offline quantification.
