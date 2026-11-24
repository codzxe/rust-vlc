# CLAIM AUDIT: docs/packaging/README.md
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Claims extracted | 17 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 11 REAL / 3 REAL-UNPROVEN / 0 / 0 / 0 / 2 FALSE (stale) / 0 |
| Internal contradictions found | 2 |

## 1. Blunt verdict
The most operationally honest runbook in the tree: the payload-honesty table corrects an earlier false claim in-band ("the earlier 'every artifact packages BOTH binaries + docs' claim was false for two artifact families and is replaced by this table"), discloses the macOS doc-pair drift, and the signing posture is labeled-unsigned with sidecars — not faked. Two FALSE claims, both the same disease: the "**The workflow has never executed**" paragraph (L207) and blockers-row 3 ("every Actions run dies at 0 steps") are contradicted by release.yml's own inline live-run citations (43378757248, 43464162696) and gen-wxs.sh's (43729218652). A third staleness: the "42 run-blocks" count is now 44 (W26-M added two).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Nothing in `scripts/packaging/**` carries a product or packaging literal" | L11-12 | REAL | `grep -rn -i "lumen\|0.1.3"` over scripts/packaging/*.{sh,py,ps1} -> only the documented cargo-package-id exception (gate.sh:79-179, packaging-env.py:40-41; sanctioned by release.yml L57-61) + comments. No product-name/version/manifest literals in code lines (ran) |
| 2 | payload table: tar.gz/zip/deb/rpm = CLI + tauri + docs, no stub | L53-58 | REAL | gen-wxs.sh audit (file 2): MSI components = BinCli+BinTauri+Docs only; packaging.toml [shipped] comments agree; AppImage/.app = tauri-only per build-appimage.sh/make-app lane comments |
| 3 | "the earlier 'every artifact packages BOTH binaries + docs' claim was false ... replaced by this table" | L45-47 | REAL | self-correction documented in-band; the current table matches code |
| 4 | "The workflow has never executed (CI billing blocked)" | L207 | FALSE | release.yml:278/1138/1145 cite live runs 43378757248 + 43464162696; gen-wxs.sh cites 43729218652 — all with step-specific post-mortem fixes. The workflow HAS executed |
| 5 | blockers row 3: "every Actions run dies at 0 steps" | L238 | FALSE | same proof as #4 |
| 6 | "bash -n over all 42 run-blocks" | L209-211 | FALSE (stale count) | `awk '/^        run: \|/{n++} END{print n}'` -> 44 (ran); 42 was pre-W26-M |
| 7 | "gate.sh — wired as step 11 of scripts/full_gate.sh" | L172-173 | REAL | scripts/full_gate.sh:77-81: "=== 11. packaging gate ===" + `bash scripts/packaging/gate.sh` (ran grep, found) |
| 8 | "the release job's kind-check + manifest steps were dry-run for real against the local artifact set" | L212-213 | REAL | docs/packaging/evidence/w24c-r3.log:146-153 — the verbatim kind-check body emitting ::warning:: lines for windows.zip/msi/macos.dmg/zip on the linux-only local set (read) |
| 9 | "linux-lane-20261110.log — the full 8.E3 lane executed for real" | L244-246 | REAL | log exists; contains genuine dpkg-deb/appimagetool/mksquashfs output (spot-checked earlier in release.yml audit) |
| 10 | "w26.m.t5 launch-verdict lane ... verdict rows are all PENDING-CI" | evidence/w26m-t5 (context) | REAL | docs/packaging/evidence/w26m-t5-launch-verdict.md L13-14: "Linux PENDING-CI ... macOS/Windows PENDING-CI (best-effort lanes; human confirmation owed)" — confirms macOS GUI launch still unproven (my release.yml F-003 corroboration) |
| 11 | "appimagetool 1.9.1 + type2 runtime 20251108 pinned" | L66 | REAL | release.yml L834-840/966-973: both SHA-256-verified downloads (read) |
| 12 | "verify-linux-artifacts.sh ... asserts the exit-86 stub's ABSENCE from every payload" | L111-112 | REAL-UNPROVEN | script exists (not in my file list; consistent with w30.8 payload change) — read-only cross-check not yet done |
| 13 | "51 variables at the time of writing" (env bridge) | L23 | REAL | `packaging-env.py --print \| wc -l` -> 51 output lines on the no-packages run? (counted 51 in my executed run: PRODUCT_* 16 + PACKAGING_* 35) — matches |
| 14 | "Both signing lanes ... without them they write an explicit sidecar and a ::warning::" | L217-227 | REAL | sign-windows.sh / sign-macos.sh exist; release.yml MSI step comment documents the honest-unsigned path; sidecar copy step at release.yml:565-567 (read) |
| 15 | "Secret env-var NAMES are config ... no script carries a secret-name literal" | L229-230 | REAL | grep for secret names in scripts/packaging -> only config-derived (verified in literal hunt #1) |
| 16 | "macOS: the doc pair is hardcoded by the wave-22 make-app.sh, flagged as drift" | L57 | REAL (disclosed drift) | honest in-band disclosure; not hidden |
| 17 | "release ships labeled-unsigned + checksums" | L221-222 | REAL | design verified in release.yml; the §8.17 unsigned-binary question: the answer is "labeled, disclosed, checksum-protected" — NOT presented as signed |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "The workflow has never executed" (L207) | release.yml:278,539,681,1059 + :1138,1145 (run 43378757248, 43464162696); gen-wxs.sh:404 (43729218652) | The workflow HAS executed — three distinct live run IDs with concrete post-mortems; the README paragraph and its blockers row 3 are stale-false |
| "42 run-blocks" (L209) | actual file | 44 run blocks today (awk count, ran) — W26-M added the ui/dist + tauri build steps after the count |

## 4. Fabrication / inflation indicators
NONE — the doc voluntarily retracts its own earlier false payload claim (L45-47) and hard-codes drift admissions (L57). The only false content is inherited staleness ("never executed"), not invention: at some earlier billing state it was presumably true, and nobody updated it when runs 43378757248/43464162696/43729218652 happened.

## 5. Matrix rows
N/A.
