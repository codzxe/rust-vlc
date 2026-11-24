# AUDIT: scripts/packaging/sign-windows.sh
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Date | 2026-11-24 |
| Lines / functions | 150 / 5 (config_get, find_signtool, write_unsigned_sidecar, main unsigned path, main signed path) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — release lane: `.github/workflows/release.yml:237-238` runs build-msi.sh which "called inside: real signtool signing when the..." (sign-windows.sh); sibling lane zip/dmg use the same contract. Not part of the debug build lane; it is release-only tooling. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 3 |
| Commands run | bash -n -> OK; sh -n -> OK; grep wiring: release.yml:237-238, packaging.toml:139-142, packaging-env.py:324-325 |

## 1. One-paragraph verdict
A real, careful Authenticode driver. Both parsers accept it, all six env guards fail loud with actionable messages, the signtool-discovery walks Windows SDK version dirs newest-first with a documented reason (W24-R1 P3-2), base64 decode is strict (`validate=True`) with an empty-file check, the cert lands in a `mktemp -d` cleaned by an EXIT trap, and the signed path refuses to half-sign (verify after sign, `die` on either failure). The unsigned fallback (exit 0 + sidecar + `::warning::`) is a deliberate, documented, externally-blocked compromise — not a hidden soft-fail. Residual hygiene: the eval-based indirect read and the argv-visible PFX passphrase.

## 2. What actually works (verified)
— verified by `bash -n scripts/packaging/sign-windows.sh` -> exit 0; `sh -n` -> exit 0 (the header's "`bash -n` clean" claim L31).
— verified by grep chain: `packaging/packaging.toml:139-142` defines `[signing.windows]` (`pfx_b64_env = "WINDOWS_SIGNING_PFX_B64"`, `pfx_password_env`, `timestamp_url`), `scripts/packaging/packaging-env.py:324-325` exports those names, and the script reads exactly `PACKAGING_SIGNING_WINDOWS_PFX_B64_ENV`/`_PASSWORD_ENV`/`_TIMESTAMP_URL` (L35-40) — the "no secret-name literal in this script" claim (L9) is true: no `WINDOWS_SIGNING_PFX_B64` literal appears in the file.
— verified by `grep -n "sign-windows" .github/workflows/release.yml` -> :237-238 the MSI lane invokes it.

## 3. Findings
F-001 [S4]
Where: scripts/packaging/sign-windows.sh:46-52
Code:
```
# Read a variable whose NAME comes from config (POSIX indirect read).
config_get() { # $1 = env var name
    eval "printf '%s' \"\${$1-}\""
}
```
What is wrong: `eval` on a config-supplied name. The name arrives from `packaging.toml` (repo-committed, validated by packaging-env.py against packaging.schema.json), so the injection surface is repo-config only — not attacker-controlled input — but any future drift that lets a schema-pattern bypass slip a shell metacharacter into an env name executes here. What correct looks like: a `case`/regex whitelist on `$1` (e.g. `^[A-Z_][A-Z0-9_]*$`) before the eval. Confidence: HIGH — Verification: read.
F-002 [S4]
Where: scripts/packaging/sign-windows.sh:135-143
Code:
```
    "$SIGNTOOL_BIN" sign \
        /f "$work/cert.pfx" \
        /p "$pfx_password" \
```
What is wrong: the PFX passphrase rides `argv`, visible in the runner's process list for the lifetime of the signtool call. Standard signtool practice has no stdin option, so this is the ecosystem norm; on a single-tenant CI runner the exposure is bounded. What correct looks like: signtool's KSP/CNG `/csp` route or a temp-file+`/p` from stdin wrapper if the runner ever becomes shared. Confidence: HIGH — Verification: read.
F-003 [S4]
Where: scripts/packaging/sign-windows.sh:101-109
Code:
```
if [ -z "$pfx_b64" ]; then
    echo "::warning::Windows artifacts are UNSIGNED ..."
    for artifact in "$@"; do
        [ -f "$artifact" ] || die "artifact not found: $artifact"
        write_unsigned_sidecar "$artifact"
    done
    exit 0
```
What is wrong: nothing functionally — this is a documented cannot-fail path (exit 0 with no cert). It is the ONE soft-fail in the file, self-labeled (`::warning::`, sidecar with reason + next step + integrity pointer), justified by the user-owned external blocker (phase8-cards card 8.E1, "[VERIFY] — escalate if absent"). Per spec 6.6 a `|| true`-class step is flagged; this one is flagged and its blast radius is an honest unsigned artifact. Recorded so the next auditor does not mistake it for an accident. Confidence: HIGH — Verification: read.

## 4. Per-function table (mandatory >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| guards (6x `:?`) | 35-40 | REAL | — | fail-loud, names the fixer (packaging-env.py) |
| usage check | 42 | REAL | — | exit 2 with usage |
| `die()` | 44 | REAL | — | stderr + exit 1 |
| `config_get()` | 47-49 | REAL | S4 | eval indirection (F-001) |
| `find_signtool()` | 54-83 | REAL | — | $SIGNTOOL override -> PATH -> SDK glob newest-first (sort -Vr), x64/x86; return codes correct |
| `write_unsigned_sidecar()` | 85-99 | REAL | — | reason + owner + next step + integrity pointer; heredoc-safe |
| unsigned path | 101-109 | REAL | S4 | documented soft-fail (F-003) |
| signed path | 111-147 | REAL | — | refuses empty passphrase (L112), signtool discovery die (114), strict b64 decode (124-127), non-empty cert check (128), sign+verify per artifact with die (135-145) |
| trap/cleanup | 119-120 | REAL | — | rm -rf work on EXIT INT TERM |

## 5. Test honesty
No test file exercises this script in-repo (grep for tests referencing sign-windows: none found). The lane evidence lives in docs/packaging/evidence/* and the release CI run — runner-only. UNVERIFIED by execution: no Windows host, no signtool, no cert in sandbox. Syntax proven (bash -n / sh -n).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "this script carries no secret-name literal" | sign-windows.sh:9 | REAL | no `WINDOWS_SIGNING_*` literal in file; names flow packaging.toml:141-142 -> packaging-env.py:324-325 -> guards L35-40 |
| "every artifact gets a `<artifact>.UNSIGNED.txt` sidecar ... the script exits 0" | sign-windows.sh:19-26 vs :101-109 | REAL | code matches comment byte-for-intent |
| "any signing or verification failure is a HARD failure" | sign-windows.sh:27-28 | REAL | L143/L145 `|| die` on sign and verify |

## 7. Dead / duplicated / unreachable code
None. Sibling sign-macos.sh implements the same contract (sh -n OK); the pair shares structure by intent, not copy-paste rot.

## 8. Dependencies & flags
POSIX sh; python3/python for stdlib base64; signtool from SDK/PATH/$SIGNTOOL; mktemp. No bashisms (verified by sh -n).

## 9. Open questions for the next agent
- Was the signed path ever executed on a real runner (cert present)? Runner-only; UNVERIFIED (CI billing-blocked, no Windows host in sandbox).
- Does `ls` under `find_signtool` handle SDK version dirs with spaces? Windows Kits version dirs never contain spaces (x.y.z form); acceptable, noted for completeness.
