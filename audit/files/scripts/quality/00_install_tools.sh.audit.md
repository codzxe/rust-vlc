# AUDIT: scripts/quality/00_install_tools.sh
| Field | Value |
|---|---|
| Auditor | W17-A08 (recovered by W17-R1) |
| Date | 2026-11-24 |
| Lines / functions | 61 / 0 named (two sequential sections) |
| File verdict | REAL (executed in sandbox: honest-failure recording works; 1 real S3 defect in binary-name extraction) |
| Reachable from shipped entry point? | NO (dev tooling), but WIRED in CI: .github/workflows/audit.yml:71-77 and :116-122 run it (static/quality tiers) + its hash is the audit-tools cache key |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | bash -n (0); sh -n (fails — bash-only arrays, shebang is bash); FULL sandbox execution via copied {common.sh,00_install_tools.sh} pair in audit/scratch (repo untouched): exit 0 with all tools absent; tool-versions.txt output pasted below; rg callers/pins greps |

## 1. One-paragraph verdict
A genuinely pinned installer: all 13 entries carry @version (nothing floats on
the install path), cargo-deny@0.20.2 byte-matches ci.yml's own pin, and the
record-don't-gate design WORKS — I executed it in a sandbox with every tool
absent and it recorded "NOT INSTALLED" per tool into tool-versions.txt and
exited 0, exactly the common.sh:5 contract ("never abort the whole sweep
because one tool failed - record and continue"). The one real defect is the
binary-name extraction `bin="${p%%@*}"`: for typos-cli the suite's own
consumers use the binary `typos` (common.sh:38 need / 08_typos.sh:6/:9), so
`command -v typos-cli` can never hit — the install is re-attempted on every
sweep and tool-versions.txt forever records "typos-cli NOT INSTALLED" even
after a successful install (a false record from the file whose purpose is
honest records). Three smaller items: the script can never fail CI by design,
rustc/cargo rows record empty strings when absent, and the present-skip never
verifies the installed version against the pin.

## 2. What actually works (verified)
- `bash -n scripts/quality/00_install_tools.sh` -> 0 (bash-only arrays; sh -n
  fails as expected for a bash-shebang script).
- FULL sandbox execution (copied common.sh + script pair under
  audit/scratch, so OUT lands in scratch; repo untouched), no cargo/rustup/
  typos in sandbox:
  - rustup section: "rustup: command not found" x2, execution continues.
  - install loop: all 12 entries attempted, "cargo: command not found", continues.
  - versions section wrote (scratch) tool-versions.txt:
    ```
    generated: 2026-11-24T17:56:48Z
    rustc:
    cargo:
    cargo-machete          NOT INSTALLED
    ... (12 rows)
    typos-cli              NOT INSTALLED
    ast-grep               NOT INSTALLED
    ```
  - script exit 0 — the honest-record contract held end-to-end.
- Pins: 13/13 entries @version-pinned; `cargo-deny@0.20.2` matches ci.yml:95
  "cargo-deny 0.20.2 (SHA-256-verified release binary)" — the :19 claim TRUE.
- Wiring: audit.yml:77/:122 `run: bash scripts/quality/00_install_tools.sh`;
  cache key hashFiles this script (:71/:116).

## 3. Findings
F-001 [S3]
Where: scripts/quality/00_install_tools.sh:35, :52
Code:
```
  bin="${p%%@*}"
  if command -v "$bin" >/dev/null 2>&1; then
```
What is wrong: the pin list entry is `typos-cli@1.52.2` (:20) but the suite consumes the binary as `typos` — common.sh's `need` (via 08_typos.sh:6 `need typos`) and 08_typos.sh:9 `typos --format brief .` — so `command -v typos-cli` never succeeds even after `cargo install typos-cli` succeeds. Consequences: (a) the skip never triggers → `cargo install typos-cli@1.52.2` re-runs on EVERY sweep (network + full rebuild); (b) tool-versions.txt records "typos-cli NOT INSTALLED" forever while the `typos` binary the sweep uses is present — a false record from the tool whose job is recording the truth. Why it matters: the version-record file is the sweep's source of truth for drift; one row is structurally wrong. What correct looks like: a crate→binary map (or list binaries, not crates): "typos-cli:typos@1.52.2". Confidence: HIGH — Verification: in-tree consumer usage + sandbox execution of the NOT-INSTALLED path.
F-002 [S4]
Where: scripts/quality/00_install_tools.sh:4 (set -uo pipefail; no -e, by design)
What is wrong: the script exits 0 with EVERY tool missing (empirical) — audit.yml's install step can never fail; the only failure signal is inside tool-versions.txt, which no workflow step parses (audit.yml reads not-run.txt and the absent astgrep-summary.txt, not tool-versions.txt). Mitigation exists downstream (per-script `need` + not_run records), so record-don't-gate is defensible, but a step that cannot fail in CI is a gate-theatre risk. Confidence: HIGH — Verification: ran it.
F-003 [S4]
Where: scripts/quality/00_install_tools.sh:48-49
Code:
```
  echo "rustc: $(rustc --version)"
  echo "cargo: $(cargo --version)"
```
What is wrong: when rustc/cargo are absent the rows record EMPTY values ("rustc: " — empirical) instead of the NOT INSTALLED convention every other row uses; an empty row can be misread as "present but silent". Confidence: HIGH — Verification: ran it.
F-004 [S4]
Where: scripts/quality/00_install_tools.sh:36-39
Code:
```
  if command -v "$bin" >/dev/null 2>&1; then
    say "present: $p"
    continue
```
What is wrong: the present-skip never compares the installed version against the pin — a stale or newer binary already on PATH is reported "present: <pin>" (the pinned string is echoed regardless of what's actually there; only the versions file shows the real value). "Every version here is pinned; nothing floats into CI" (:3) is version-SELECTION pinning, not state pinning. Confidence: HIGH — Verification: read.

## 4. Per-section table
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| pin table | 7-27 | REAL | — | 13/13 pinned; cargo-deny matches ci.yml; EVAL-only list documented |
| rustup components | 29-31 | REAL | — | failures tolerated (empirical); output tail-1 |
| install loop | 33-42 | REAL | S3/S4 | F-001 (typos-cli bin name), F-004 (no version check on skip) |
| version record | 44-58 | REAL | S4 | honest NOT INSTALLED rows (empirical); F-003 empty rustc/cargo rows |
| output | 59-61 | REAL | — | cat + written message matched the run |

## 5. Test honesty
No tests. Mutation reasoning: the version-record loop would catch a broken
install (NOT INSTALLED row) but nothing verifies the RECORD itself — the
typos-cli false-negative has survived precisely because no consumer checks
tool-versions.txt.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Every version here is pinned; nothing floats into CI (AGENTS.md 3.2 / doc 11 3.7)" | 00_install_tools.sh:3 | TRUE-on-install-path / FALSE-on-skip-path | 13/13 @pins; but F-004: present binaries are never version-checked |
| "cargo-deny@0.20.2 # matches the pin already used by ci.yml" | :19 | TRUE | ci.yml:95 "cargo-deny 0.20.2 (SHA-256-verified release binary)" |
| "Pinned versions (resolved from crates.io on 2026-11-16, see 01-TOOL-INVENTORY.md)" | :7 | TRUE-consistent | version list dated; crates.io resolution not checkable offline (UNVERIFIED network detail) |

## 7. Dead / duplicated / unreachable code
None dead. The EVAL-only list (:23-27) is documentation, not code.

## 8. Dependencies & flags
Sources common.sh (OUT=$ROOT/docs/audit/reports, say/head1/stamp; common.sh:9).
Runs rustup, cargo install --locked, and each tool's --version. No feature
flags. Consumers: audit.yml static/quality tiers; every quality script's
`need` gate.

## 9. Open questions for the next agent
- Confirm crate-name→binary reality for ast-grep@0.51.3 offline-uncertain
  (the in-repo consumers check `ast-grep` and `sg` — common.sh:47-52 —
  consistent with the entry; the CRATE name on crates.io not verifiable here).
- If 14_crap.sh's cargo-crap or others share the typos-cli disease (crate
  name ≠ binary name), the same false-NOT-INSTALLED record spreads — audit
  each consumer's `need` name against this pin list.
