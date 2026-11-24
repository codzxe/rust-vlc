# AUDIT: scripts/packaging/sign-macos.sh
| Field | Value |
|---|---|
| Auditor | W17-A10 |
| Date | 2026-02-20 |
| Lines / functions | 280 / 5 (die, config_get, write_sidecar, unsigned_blocker_prose, resolve_identity) + main |
| File verdict | REAL-UNPROVEN (unsigned path EXECUTED and green in orchestrator baseline; signed/notarized path macOS-only, never run anywhere — by R-35 design) |
| Reachable from shipped entry point? | N/A — release infra; invoked by build-dmg.sh flow / release.yml macos lanes (release.yml:79-80,501-508,654) and locally by gate.sh:191 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | sh -n OK; bash -n OK; empirical sidecar-quoting test (/bin/sh); gate.sh run (my env fails step 1 on cargo-PATH); baseline full_gate.txt:88 `packaging: PASS` cited |

## 1. One-paragraph verdict
A serious, defensively-written codesign/notarytool driver with a genuinely
honest unsigned-fallback contract: no credentials -> `.UNSIGNED.txt` sidecar +
`::warning::` + exit 0 (executed and passing in the orchestrator baseline's
packaging gate); identity-without-notary-creds -> signed + `.NOT-NOTARIZED.txt`
sidecar; ANY step failure with credentials present -> hard `die` exit 1 —
"never silently pretend" is actually implemented. `set -eu`, `:?` guards on all
8 config envs, EXIT/INT/TERM trap cleanup with if-guards that preserve the exit
status (the W24-R1 lesson is coded, not just cited), and the resolve_identity
rc-0/rc-2 contract with an explicit refuse-to-fallback branch. Two real defects
sit in the never-executed signed path: the Apple CLI subcommand is written
`security list-keychain` (singular — Apple's CLI is `list-keychains`) on both
the setter and getter, so the keychain search-list prepend will likely die at
the FIRST live signing run; and `tr -d '\n'` concatenates multiple existing
keychain names with NO separator, corrupting the search list on hosts with >1
keychain. Both are UNVERIFIABLE on Linux (no macOS) — but they are in the exact
block whose own header says "[VERIFY at first live run]".

## 2. What actually works (verified)
- `sh -n` / `bash -n` on sign-macos.sh -> both clean (exit 0).
- Unsigned path: baseline audit/runs/baseline/full_gate.txt:88 `packaging: PASS` (gate.sh step 6 runs `sh scripts/packaging/sign-macos.sh --app "$stage/standin.app"` and asserts the `.UNSIGNED.txt` sidecar exists — gate.sh:191-196). My local gate.sh run confirmed steps 2-4 pass; step 6 skipped only because my shell lacks cargo on PATH (environment artifact, not a repo defect).
- Sidecar quoting: empirical /bin/sh test of `write_sidecar "$t" ".UNSIGNED" "$prose" "$(unsigned_blocker_prose)"` -> multi-line prose reproduced LINE-PER-LINE correctly (the quoted command substitution is one argument; embedded newlines survive). No word-splitting mangling.
- `resolve_identity` runs in the main shell (not `$(...)` subshell) — read; the comment's subshell-swallow rationale is technically correct for `set -eu` + command substitution.
- Exit-status-preserving trap: `if [ -n "$work" ]; then rm -rf "$work"; fi` — last command in the trap is the `fi`/test structure; verified shape by reading + my /tmp reproduction of the pattern.

## 3. Findings
F-001 [S3]
Where: scripts/packaging/sign-macos.sh:177-179
Code:
```
    security list-keychain -d user -s "$scratch_keychain" \
        "$(security list-keychain -d user | sed 's/^[[:space:]]*"\([^"]*\)".*$/\1/' | tr -d '\n')" ||
        die "security list-keychain (search-list prepend) failed"
```
What is wrong: the subcommand is `list-keychain` (singular) in BOTH the getter
and setter; Apple's security CLI subcommand is `list-keychains` (plural). If
correct, the getter errors and the setter dies at the first live signing run —
loudly (die), not silently, but the scratch-keychain search-list prepend can
never succeed as written. Why it matters: this block is the "standard CI dance"
the header sells; a guaranteed-failing step in it blocks the signed path the
script exists to provide. What correct looks like: `security list-keychains -d
user -s "$scratch" existing...`. Confidence: MEDIUM-HIGH — Verification:
UNVERIFIED (macOS-only; no `security` binary in sandbox; no in-tree run log of
the signed path — R-35 means it has never run anywhere).

F-002 [S3]
Where: scripts/packaging/sign-macos.sh:178
Code:
```
        "$(security list-keychain -d user | sed 's/^[[:space:]]*"\([^"]*\)".*$/\1/' | tr -d '\n')" ||
```
What is wrong: independent of F-001: `tr -d '\n'` JOINS multiple keychain
names with NO separator — two existing keychains `login.keychain-db` +
`System.keychain` become the single garbage name `login.keychain-dbSystem.keychain`,
which is then passed as ONE search-list entry. On any host with >1 keychain the
search list is corrupted for the rest of the runner session. Confidence: HIGH
(POSIX semantics verified by reading; impact on macOS hosts UNVERIFIED).
What correct looks like: preserve one-name-per-argument quoting (`while read`
loop or an array-shaped equivalent).

F-003 [S4]
Where: scripts/packaging/sign-macos.sh:68-70
Code:
```
config_get() { # $1 = env var name
    eval "printf '%s' \"\${$1-}\""
}
```
What is wrong: eval over a config-supplied NAME — a product.toml/
packaging.toml value like `X; rm -rf ~` in [signing.macos] identity_env would
execute. Config is committed (Lead-controlled) today, so this is hygiene, not a
live hole. Confidence: HIGH — Verification: read.

F-004 [S4]
Where: scripts/packaging/sign-macos.sh:217-219
Code:
```
        codesign --force --sign "$signing_identity" \
            --timestamp --options runtime "$target" ||
```
What is wrong: no `--deep` (or per-nested-code signing): a .app that ever gains
embedded frameworks/helpers would sign only the outer bundle and notarytool
would then reject it. Today's Tauri bundle (WKWebView is a system framework)
likely has no nested code, so latent. Confidence: MEDIUM — Verification: read.

## 4. Per-function table (mandatory; file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| die | 55 | REAL | — | stderr + exit 1 |
| config_get | 68-70 | REAL | S4 | eval name-indirection (F-003); used 6x |
| write_sidecar | 79-88 | REAL (empirically reproduced) | — | quoted-multiline prose proven correct in /bin/sh test |
| unsigned_blocker_prose | 90-103 | REAL | — | R-35 prose; checksum-manifest pointer |
| resolve_identity | 139-187 | REAL-UNPROVEN | S3 | p12 import dance; F-001/F-002 in the tail; rc contract 0/2 |
| main flow | 57-64,105-123,189-281 | REAL (unsigned path) / REAL-UNPROVEN (signed path) | — | modes app/dmg; codesign+verify+spctl; notarytool submit --wait; stapler; validate |

## 5. Test honesty
gate.sh step 6 (executed in baseline, PASS) proves the unsigned contract for
REAL: exit 0 + sidecar present. Mutation reasoning: if write_sidecar broke,
gate.sh:192-193 `bad` fires. The signed path has NO test anywhere (R-35: no
account) — the script's own header says so. Honest boundary documented at
lines 40-42.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the unsigned path (sidecar + warning) is exercised locally in gate.sh" | sign-macos.sh:41-42 | TRUE | gate.sh:191-196; baseline full_gate.txt:88 packaging: PASS |
| "Never silently pretend, never ship half-done" | sign-macos.sh:27-28 | TRUE for unsigned path (executed); UNVERIFIED for signed path | die() on every step; rc contract line 196-199 |
| "any credential present but a step FAILS -> HARD exit" | sign-macos.sh:27-28 | TRUE (structurally) | `||` die on every security/codesign/xcrun call — read |
| "POSIX sh; `bash -n` clean" | sign-macos.sh:37-38 | TRUE | sh -n + bash -n both exit 0 (ran) |

## 7. Dead / duplicated / unreachable code
`[ -n "$work" ] || work=$(mktemp -d)` (line 257) — the notary path reuses the
identity-resolution work dir or creates one if identity came from a preinstalled
keychain; not dead. No duplication found.

## 8. Dependencies & flags
Config env names come from packaging.toml [signing.macos] (identity_env /
p12_b64_env / p12_password_env / notary key id+b64+issuer) — mapped by
release.yml:501-508. External CLIs: codesign, security, spctl, xcrun
(notarytool/stapler), uuidgen, python3. All absent on this sandbox (Linux) —
hence REAL-UNPROVEN for that half.

## 9. Open questions for the next agent
- F-001 (list-keychain vs list-keychains): confirm against `security(1)` on a
  macOS host at first live run — this is the single most likely first-run
  breaker in the whole signed path.
- release-checklist.md / packaging README: do they claim macOS artifacts are
  "signed" today? If so that's FALSE (R-35, unsigned by design) — check when
  auditing those docs.
