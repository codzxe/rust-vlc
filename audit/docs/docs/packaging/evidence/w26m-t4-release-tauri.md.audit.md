# CLAIM AUDIT: docs/packaging/evidence/w26m-t4-release-tauri.md
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Claims extracted | 11 |
| REAL: 9 REAL-UNPROVEN: 2 SHALLOW: 0 STUB: 0 PAPER-ONLY: 0 FALSE: 0 UNREACHABLE: 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A static-validation evidence record that does not overclaim: it states up
front "no cargo, static validation ONLY" and "this workflow has NEVER
executed (CI billing blocked)". I re-ran its two proofs against the current
release.yml and they reproduce — job ids, step counts, the 5×5×5 grep
counts, the frontendDist ordering rationale, the webkit dep list, and the
linux-arm64-only job-level continue-on-error. One drift: the windows job has
since gained a 15th step, so the recorded "14 steps, 11 named" snapshot is
stale. The tauri bridge arg names it depends on are now real in
packaging-env.py.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "yaml ok" (release.yml parses) | :126-128 | REAL | my run: `yaml.safe_load` -> ok, job ids printed |
| 2 | "job ids: ['windows','macos-arm64','macos-x64','linux-x64','linux-arm64','release']" + step counts | :133-140 | REAL (5/6 jobs exact) | my python3 run matches all jobs EXCEPT windows: doc "14 steps, 11 named" vs actual 15 steps/12 named — one step ("Launch smoke — the SHIPPED zip's lumen-tauri.exe...") added after this evidence was written |
| 3 | "5 × the exact recipe command cargo build --release --locked --manifest-path apps/desktop-tauri/Cargo.toml" | :149-150 | REAL | grep -c -> 5; --tauri-package -> 5; --tauri-manifest -> 5 |
| 4 | "tauri.conf.json frontendDist is ../../ui/dist" (order is load-bearing) | :43-45 | REAL | apps/desktop-tauri/tauri.conf.json:7 `"frontendDist": "../../ui/dist"` |
| 5 | "linux-arm64 ... job-level continue-on-error: true is UNTOUCHED; NO step-level continue-on-error" | :92-97 | REAL | yaml: linux-arm64 coe=True, others None; only one `continue-on-error: true` in file (L876) |
| 6 | "the exact package list ... libwebkit2gtk-4.1-dev ... pkg-config --exists webkit2gtk-4.1" | :66-72 | REAL | grep: webkit2gtk-4.1-dev in 2 linux lanes; pkg-config assertion at :777/:925 |
| 7 | "Mirrors the proven ci.yml tauri-check lane's invocation exactly (same flags)" | :45-46 | REAL | ci.yml:721 `npm --prefix ui ci --no-audit --no-fund` == release.yml's 5 lanes' L202/483/636/787/935 |
| 8 | "packaging-env.py does NOT yet accept the --tauri-* args (argparse exits 2)" [at base] | :166-171 | REAL (period) | today packaging-env.py:220 DOES accept --tauri-package — T1 landed after; doc's fail-loud merge-order note was accurate for its base |
| 9 | "This workflow has NEVER executed (CI billing blocked)" | :186-188 | REAL-UNPROVEN | consistent with baseline xtask-parity-report (0/252 verified, CI never run); I cannot observe CI history (no git/network) |
| 10 | "The tauri binary rides INSIDE the existing installer payloads" (no new artifact kind) | :101-111 | REAL-UNPROVEN | read-only reasoning over a workflow that has never run; the payload-staging scripts are other lanes' files |
| 11 | "node/npm are claimed preinstalled ... [VERIFY]" | :191-194 | REAL | honest unverified marker — nothing fabricated |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| windows: "14 steps, 11 named" | current release.yml windows job: 15 steps/12 named (my yaml run; extra step = Launch smoke) | current file — evidence snapshot is stale by one later-added step |

## 4. Fabrication / inflation indicators
None. Every unverifiable item carries an explicit [VERIFY at first live run]
marker; the doc never claims the workflow ran. The only weakness is the
usual one for this repo: evidence snapshots are not revisited when the
workflow evolves (drift in claim 2's numbers).

## 5. Matrix rows
N/A.

## 6. What actually works (verified)
— python3 yaml parse + per-job step dump (job ids/counts/coe/needs all
reproduced); grep counts 5/5/5; tauri.conf.json frontendDist; ci.yml
tauri-check flags.
