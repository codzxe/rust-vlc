# AUDIT: ui/scripts/lint-css.mjs
| Field | Value |
|---|---|
| Auditor | W15-A03 |
| Date | 2026-11-21 |
| Lines / functions | 65 / 2 (canRun, top-level launcher flow) |
| File verdict | REAL-UNPROVEN (logic traced sound and CI-wired, but no baseline run executed `lint:css`, and this checkout has no ui/node_modules to run it — execution unproven) |
| Reachable from shipped entry point? | N/A (dev/CI tooling) — `npm run lint:css` (ui/package.json:12) → this script → stylelint; CI runs it in .github/workflows/ui.yml:112 with an honest provenance comment (the fonts.css slip that proved "a CSS gate that CI never runs cannot gate"). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | `ls ui/node_modules/.bin/stylelint` → absent (this checkout has no node_modules — cannot execute, environment artifact); `rg -n "lint:css|stylelint" ui/package.json` → :12 script + :52-53 devDeps (stylelint `^18.0.0`, config-standard `41.0.0`); `rg '"node_modules/stylelint"' -A4 ui/package-lock.json` → locked 18.0.0 + 41.0.0 exactly; `sed -n '104,120p' .github/workflows/ui.yml` → CI step 2 chains lint + lint:css; `ls audit/runs/baseline/ \| rg -i lint` → only gate-lint_literals.log.txt (a RUST xtask gate, not this); read of the whole file. |

## 1. One-paragraph verdict
A correct, small, no-false-PASS launcher: prefer the local stylelint binary, fall back to the historical `npx -p stylelint@18.0.0` form with a loud stderr notice, forward argv verbatim, propagate the child's exit status and convert launch failure/signal-kill to exit 2. The escape-hatch design is documented and the CI wiring carries a candid history (the gate was local-only until a real slip proved the gap). Three S4 warts: no baseline evidence run exists for this gate (its PASS is asserted only by CI wiring); the header's "exact versions the npx form pinned, from package.json devDependencies" is only true via the lockfile — the devDep itself is a caret range, so a plain `npm install` can make path 1 run a different 18.x than the fallback pins; and the X_OK probe + extensionless spawn is POSIX-shaped, degrading Windows to the network fallback or a loud failure.

## 2. What actually works (verified)
— verified by reading .github/workflows/ui.yml:104-112: CI step 2 runs `npm run lint` then `npm run lint:css` (chained, with the fonts.css-slip rationale in comments).
— verified by read: exit-code discipline — `child.error` → exit 2 (:61-64); `child.status ?? 2` (:65) turns signal-kill (null status) into failure; a passing run requires stylelint itself to exit 0. No path silently passes.
— verified by `rg` in package-lock.json:3626-3630, 3693-3697: stylelint 18.0.0 and stylelint-config-standard 41.0.0 are locked exactly, so under the documented `npm ci` flow (baseline #21) path 1 and the npx fallback run identical versions.
— verified by read: the fallback prints a loud notice to stderr before falling back (:45-48) — the degraded path is never silent.

## 3. Findings
### F-001 [S4]
Where: ui/scripts/lint-css.mjs:1-19 (+ audit/runs/baseline/)
Code:
```
//   1. ui/node_modules/.bin/stylelint exists → run it (offline, canonical);
//   2. otherwise → fall back to the historical npx form with a loud notice
```
What is wrong: Evidence gap, not a code bug: the baseline run set (audit/runs/baseline/, INDEX.md rows) contains no `lint:css` execution — the only lint-flavored log is gate-lint_literals.log.txt (`cargo xtask lint-literals`, a Rust-side gate). So the ui stylelint gate's current PASS state is proven only by CI wiring, never by a recorded run I can cite.
Why it matters: An audit of CSS discipline (e.g., my variants.css/setting-field.css findings) cannot cite a green stylelint run as evidence; "CI runs it" is a claim about workflow shape, not about today's tree.
What correct looks like: one baseline run `npm run lint:css` → log under audit/runs/baseline/.
Confidence: HIGH — Verification: baseline dir listing + INDEX read; ui-vitest/ui-build logs contain no stylelint output.
Severity: S4 (process/evidence debt; REQUEST-ORCH-RUN logged).

### F-002 [S4]
Where: ui/scripts/lint-css.mjs:10-11 (+ ui/package.json:52)
Code:
```
// stylelint. ui/node_modules now carries the real install (stylelint 18.0.0
// + config-standard 41.0.0 — the exact versions the npx form pinned, from
```
What is wrong: The devDependency is `"stylelint": "^18.0.0"` — a range, not the exact pin the comment implies. Exactness comes from package-lock.json (18.0.0). A plain `npm install` (not `ci`) can bump within 18.x, after which path 1 runs a newer stylelint than the npx fallback's hard-pinned 18.0.0 — the two paths diverge in behavior.
Why it matters: The whole point of the launcher is that both paths are equivalent; the equivalence silently depends on lockfile discipline that the comment doesn't mention.
What correct looks like: pin `"stylelint": "18.0.0"` in devDependencies (config-standard already is exact) or note the lockfile dependency in the comment.
Confidence: HIGH — Verification: package.json:52-53 and lockfile entries read.

### F-003 [S4]
Where: ui/scripts/lint-css.mjs:30-37, 41-42, 60
Code:
```
const canRun = (path) => {
  try {
    accessSync(path, constants.X_OK);
```
What is wrong: POSIX shaping: on Windows the local install produces `stylelint.cmd`/`.ps1` shims; `accessSync(localBin, X_OK)` on the extensionless sh script is unreliable, and even if it passes, `spawnSync(localBin, …)` without `shell: true` cannot execute it — so Windows lands on the network-touching npx fallback or a loud exit 2. No silent-pass path either way.
Why it matters: The "offline, canonical" path 1 is Windows-degraded in a cross-platform desktop project (tauri targets Windows); contributors on Windows pay the network/notice tax on every run.
What correct looks like: probe `stylelint.cmd` too (process.platform check) or spawn via `npm exec --offline`.
Confidence: MEDIUM — Verification: Node spawn semantics knowledge + shim layout; no Windows environment available to demonstrate (UNVERIFIED empirically).
Severity: S4 (dev tooling, loud failure modes only).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| canRun | 30-37 | REAL | S4 | F-003 X_OK/POSIX wart; total (throws→false), no false positive path beyond Windows shim case |
| launcher flow | 39-58 | REAL | S4 | F-002 version-equivalence claim; fallback notice is loud; argv forwarded verbatim (:28,43,56) |
| spawn + exit mapping | 60-65 | REAL | — | stdio inherit; error→2, signal→2, status forwarded — no false PASS |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| no self-tests for lint-css.mjs | N/A | a 65-line launcher with 3 branches and zero tests; the branches (local/fallback/error) are exactly what a 3-case unit test would pin — absent |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "real installs and CI always take path 1" | :15 | REAL | npm ci installs devDeps (baseline #21 OK); CI step runs after install |
| "path 2 exists so the gate keeps working in a degraded checkout instead of silently passing" | :15-16 | REAL | fallback runs real stylelint; loud notice; exit codes forwarded |
| "the exact versions the npx form pinned, from package.json devDependencies" | :10-11 | PARTIAL | exact only via lockfile; devDep is ^18.0.0 (F-002) |
| "That path hits the NETWORK on a cold npx cache — verboten (contract §4.1)" | :8-9 | REAL (disclosed) | npx -p fetches from registry on cold cache — the fallback knowingly violates the no-network contract in degraded checkouts, with a warning |

## 7. Dead / duplicated / unreachable code
None. Both branches are live by design.

## 8. Dependencies & flags
node: child_process/fs/path/url. No deps. Args after argv[2] forwarded verbatim (package.json passes the glob `'src/**/*.css'`). Cross-file note: the stylelint token-discipline claim is weakened by 44 inline `stylelint-disable` comments counted in my crates/theme-tokens/src/lint.rs audit (booked there) — this launcher faithfully runs a gate that the css partially opts out of.

## 9. Open questions for the next agent
- REQUEST-ORCH-RUN (logged): `cd ui && npm ci && npm run lint:css` to produce the missing baseline evidence (F-001).
- Does e2e/ or the release workflow run lint:css anywhere else? (Only ui.yml found in this audit.)
