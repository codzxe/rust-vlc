# AUDIT: docs/gates/logs/phase7-exit-deny.log (DATA AUDIT — log, 2 chunks = one file)
| Field | Value |
|---|---|
| Auditor | W17-A08 |
| Date | 2026-11-24 |
| Lines | 1011 |
| File verdict | REAL-UNPROVEN (authentic cargo-deny output; internally consistent; exit code asserted, not executed here) |
| Reachable from shipped entry point? | N/A (gate log; cited as evidence by docs/gates/phase7-exit.md:58) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | rg -c 'warning\[wildcard\]' -> 30; rg -c 'warning\[duplicate\]' -> 2; sed -n quotes; rg Cargo.lock for getrandom/syn |

## 1. One-paragraph verdict
Genuine-looking `cargo deny --workspace check licenses sources bans advisories`
capture from worktree w24f @ a4e35e7 (2026-11-10): 30 `warning[wildcard]` blocks,
2 `warning[duplicate]` blocks (getrandom 0.2.17/0.4.3, syn 2.0.119/3.0.9), zero
errors, footer `advisories ok, bans ok, licenses ok, sources ok` + `DENY_EXIT=0`.
The two lockfile duplicates named in the log exist in the current repo Cargo.lock,
so the run reflects real dependency state. The consumer claim (phase7-exit.md:58
"PASS (exit 0 ... 30 non-blocking wildcard path-dep warnings)") matches the log
byte-for-byte on every checkable fact. I could not execute cargo-deny myself
(load discipline), so exit-0 is REAL-UNPROVEN on my own authority, corroborated by
internal consistency + deny.toml policy.

## 2. What actually works (verified)
- `rg -c 'warning\[wildcard\]'` -> 30; `rg -c 'warning\[duplicate\]'` -> 2. Matches the "30" recounted in docs/gates/phase7-exit.md:58 and worklog.md:957.
- `rg -n 'getrandom 0\.2\.17|getrandom 0\.4\.3' Cargo.lock` -> both versions present (lines 337, 756); `rg -n 'syn 2\.0\.119|syn 3\.0\.9'` -> both present (28, 729...). The two duplicate warnings are real dependency facts.
- `sed -n '1010,1011p' docs/gates/logs/phase7-exit-deny.log` -> `advisories ok, bans ok, licenses ok, sources ok` / `DENY_EXIT=0`.
- `.github/workflows/ci.yml:490-519` runs the identical command with pinned cargo-deny 0.20.2 (SHA-256 9f12ed4c...c8d5f) — log header's binary claim matches the CI pin.

## 3. Findings
F-001 [S3]
Where: docs/gates/logs/phase7-exit-deny.log:1010-1011 vs deny.toml:154-167
Code (log):
```
advisories ok, bans ok, licenses ok, sources ok
DENY_EXIT=0
```
Code (deny.toml:162-165):
```
# the only two expected warnings are the two workspace path edges (codec-api→media-types, media-types→testkit);
# any THIRD wildcard warning is a real `*` version requirement in a
# manifest and is a review stopper.
```
What is wrong: the log contains 30 wildcard warnings, all of them intra-workspace
`path =` deps (e.g. `media-types = { path = "../media-types" }` at log:10), i.e.
28 "third" warnings by deny.toml's own rule, yet the gate passed with exit 0 and
phase7-exit.md calls them non-blocking. deny.toml's stated review-stopper rule was
not applied. Why it matters: the gate's written policy says the run should have been
stopped; instead the policy comment is dead text. What correct looks like: deny.toml
comment corrected to match actual cargo-deny 0.20.2 behavior (fires on every
version-less path dep), or the gate doc explaining the accepted deviation.
Confidence: HIGH — Verification: read + counted.

F-002 [S4]
Where: docs/gates/logs/phase7-exit-deny.log:2-4
Code:
```
# Worktree: /home/z/my-project/wt/w24f (branch w24/f @ a4e35e7)
# Date: 2026-11-10T16:57:54Z
# Binary: cargo-deny 0.20.2 (pinned tarball, SHA-256 verified == ci.yml pin)
```
What is wrong: the SHA-256 verification is asserted in a comment only; no checksum
output is committed anywhere in the log, and the referenced worktree/commit cannot
be re-inspected (agents cannot run git). Why it matters: "verified" claims without
committed artifacts are unverifiable. What correct looks like: append the raw
`sha256sum -c` output line to the log. Confidence: HIGH — Verification: read.
UNVERIFIED: sha256 equality, worktree commit a4e35e7 state.

F-003 [S4]
Where: docs/gates/logs/phase7-exit-deny.log:255,853 vs repo Cargo.lock
Code:
```
   ┌─ /home/z/my-project/wt/w24f/Cargo.lock:28:1
```
What is wrong: log cites Cargo.lock line 28 for the getrandom duplicate and 109 for
syn; in the current root Cargo.lock the getrandom name entries are at 248/259 and
syn at 1087/1098. Line numbers are snapshot-relative (w24f) and do not match the
tree that ships this log. Why it matters: byte-precision citations rot; a re-run
today would produce different line numbers, and the gate doc's "pin == ci.yml:469"
style references already drift (pin is at ci.yml:505 now). What correct looks like:
cite crate/version pairs, not line numbers, in gate docs. Confidence: HIGH —
Verification: rg on Cargo.lock.

## 4. Data integrity checks
- Wildcard blocks: 30, each names a crate and lists the offending path deps — all
  intra-workspace `path` deps, no `*` version requirements found in the quoted
  manifests (spot-checked media-types:29, cli:48-67, transcode:56-66 — all `path`).
- Duplicate blocks: 2 (getrandom 0.2.17/0.4.3; syn 2.0.119/3.0.9) — both pairs
  exist in the current Cargo.lock. CONSISTENT.
- Warning counts vs current tree drift: log says lumen-cli 16 / lumen-web 12 /
  transcode 7 path wildcards; current Cargo.tomls have 19 / 13 / 7 `path =` deps —
  the tree has since gained 3 (cli) and 1 (web) path deps. Log is a w24f snapshot,
  not current HEAD; consumers must treat counts as stale. No contradiction with
  exit 0 (wildcards=warn, multiple-versions=warn in deny.toml:153,167).
- Exit code: DENY_EXIT=0 asserted at line 1011; consistent with warnings-only run.

## 5. Consumer claims
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "cargo-deny 0.20.2, CI command ... PASS (exit 0 ... 30 non-blocking wildcard path-dep warnings)" | docs/gates/phase7-exit.md:58 | REAL-UNPROVEN | log:1010-1011 (exit 0, all four checks ok), rg count 30; could not re-run cargo-deny |
| "SHA-256 verified == ci.yml pin" | log:4 (via phase7-exit.md:58) | UNVERIFIED | no committed sha256 output; ci.yml:505 pin text matches tool version only |
