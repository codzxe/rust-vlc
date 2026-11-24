# AUDIT: scripts/packaging/gate.sh
| Field | Value |
|---|---|
| Auditor | W17-A06 |
| Date | 2026-11-24 |
| Lines / functions | 233 / 0 functions (7 gate steps) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — dev/CI gate script; called by scripts/full_gate.sh step 11 (verified: audit/runs/baseline/full_gate.txt:86-88 shows it ran and PASSED) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 4 |
| Commands run | bash -n/sh -n over 14 scripts (fail=0); ast.parse on *.py (0 fail); ps1-sanity.py on *.ps1 (0 fail); `sh scripts/release/gen-wxs.sh --self-test` -> PASS rc=0; rg on packaging-env.py |

## 1. One-paragraph verdict
A genuinely honest POSIX packaging gate: config/schema validation via
packaging-env.py, dual-parser syntax checks on every packaging shell script,
AST parse of the python tools, a clearly-labeled NON-parse structural check
for .ps1 (no pwsh), real executions of the payload stager's loud-refusal
contract, the desktop-entry generator with content cross-checks against
packaging.toml, the honest-unsigned sidecar contracts, and the .wxs
generator's hostile-fixture self-test. I re-ran the cargo-free steps myself
(all pass) and the orchestrator baseline ran the whole script PASS. Only
hygiene-level defects found; the "never fakes" claims in the header are true
to the code.

## 2. What actually works (verified)
— `for f in scripts/packaging/*.sh; do bash -n "$f"; sh -n "$f"; done` ->
fail=0 over 14 scripts (my run, exit 0).
— `python3 -c 'ast.parse(...)'` per *.py -> 0 failures; `ps1-sanity.py` per
*.ps1 -> 0 failures.
— `sh scripts/release/gen-wxs.sh --self-test` -> "PASS (4 File rows
parsed; hostile basenames + identity round-trip; escape set pinned...)"
rc=0.
— Orchestrator: audit/runs/baseline/full_gate.txt:87 `gate.sh: PASS
(packaging config + scripts + self-tests + local-provable halves)`.

## 3. Findings
F-001 [S4]
Where: scripts/packaging/gate.sh:127,169,177,199
Code:
```
    stage=$(mktemp -d) || { bad "mktemp failed"; exit 1; }
```
What is wrong: no trap; `rm -rf "$stage"` only runs on the normal path, so
an interrupt (or the `exit 1` right after a failed mktemp is fine, but a
Ctrl-C mid-staging) leaks the throwaway tree. Cosmetic in CI.
Confidence: HIGH — Verification: read.
F-002 [S4]
Where: scripts/packaging/gate.sh:160
Code:
```
            grep -q "^Exec=${BIN_TAURI}$" "$stage/entry.desktop" ||
                bad "generated desktop entry Exec != BIN_TAURI (cargo metadata)"
```
What is wrong: on a tree whose bridge does not export BIN_TAURI (the
pre-w26.m.t1 case the script itself still describes at :72-73),
`set -u` makes this line abort the gate with an unbound-variable error
instead of a labeled `bad`. Latent only — packaging-env.py today supports
`--tauri-package` (packaging-env.py:220) so BIN_TAURI is exported.
Confidence: MEDIUM — Verification: read + rg (no such tree to run).
F-003 [S4]
Where: scripts/packaging/gate.sh:126-127 vs :206-208
Code:
```
if [ $fail -eq 0 ]; then
    stage=$(mktemp -d) || { bad "mktemp failed"; exit 1; }
```
vs
```
# ...independent of steps 1-6, so it runs even if they
# failed: more evidence, never less); the .ps1 twin runs only where
```
What is wrong: the "more evidence, never less" property is claimed for the
script's philosophy but steps 5 and 6 are skipped when any earlier step
fails; only step 7 truly runs regardless. Wording, not behavior — skips are
labeled honestly.
Confidence: HIGH — Verification: read.
F-004 [S4]
Where: scripts/packaging/gate.sh:144
Code:
```
                *"missing packaging input"*|*"BIN_TAURI not set"*) step "   stager refuses loudly without built binaries (as designed)" ;;
```
What is wrong: gate success is text-coupled to the stager's exact error
strings; a reworded stager message turns a correct loud refusal into
`bad "stager failed for an unexpected reason"`. Brittle contract, honest
failure mode (fails closed, not open).
Confidence: HIGH — Verification: read.

## 4. Per-step table (script sections)
| Step | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PATH/ROOT setup | 49-61 | REAL | — | `set -u` only (no -e) is correct for fail-accumulation gate; cd "$ROOT" guarded |
| tauri-bridge probe | 77-80 | REAL | — | probes `--help` for --tauri-package instead of assuming; honest |
| 1 config validation | 83-99 | REAL | — | packaging-env.py --print; could not run here (needs cargo) but PASS in baseline |
| 2 shell syntax | 102-107 | REAL | — | verified by my run: 14 scripts, bash -n + sh -n clean |
| 3 python syntax | 111-115 | REAL | — | verified: ast.parse clean |
| 4 ps1 structural | 118-122 | REAL | — | verified: ps1-sanity clean; labeled NOT-a-parse — honest |
| 5 stagers | 125-172 | REAL | — | loud-refusal test (TARGET_DIR=/nonexistent...), desktop-entry content cross-checked vs config |
| 6 unsigned fallbacks | 175-202 | REAL | — | stand-in artifacts; sidecar existence asserted |
| 7 wxs self-tests | 204-226 | REAL | — | verified: gen-wxs.sh --self-test PASS; pwsh absent -> honest skip label |
| summary/exit | 228-233 | REAL | — | exit $fail correct |

## 5. Test honesty
The script tests FAILURE paths on purpose (stager must refuse; signers must
write sidecars). Would it catch a broken stager? Yes — 5a inverts
expectation (success = `bad`). Honest by construction.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "This gate never fakes them" (wix/signtool/codesign/rpmbuild) | gate.sh:42-45 | REAL | no invocation of those tools anywhere in the script (rg) |
| "gate.sh — the packaging gate ... full_gate.sh step 11 calls THIS" | :2-3 | REAL | baseline full_gate.txt:86 section 11 header |
| PowerShell "structural check, not a PowerShell parse" | :15-20 | REAL | ps1-sanity.py is a balance/ASCII checker (read); label honest |

## 7. Dead / duplicated / unreachable code
None. Every step runs in the baseline gate run.

## 8. Dependencies & flags
python3, bash/sh, grep, mktemp; optional pwsh (probed, honestly skipped);
cargo metadata only via packaging-env.py (absent in this sandbox — step 1
environment-blocked, not a defect).

## 9. Open questions for the next agent
Does scripts/full_gate.sh still call this as step 11 (verify wiring),
and did the [VERIFY at first live run] pwsh/wix items ever discharge on a
Windows runner?
