# DATA AUDIT: tools/bench-vs-vlc/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W19-R6 (credit A06) |
| Type | Cargo manifest (56 lines) for the 3.E3 differential-vs-VLC harness |
| Parsed with tomllib (display-safe) | all structural claims verified |
| Reachable from shipped entry point? | NO — dev tool crate, invoked explicitly (not in root members) |

## 1. Blunt verdict
Every manifest claim in the header comments verifies mechanically:
standalone workspace (empty `[workspace]`, NOT in root members), 7 path-only
dependencies (zero external crates — std-only holds), input's `contract`
feature pin, edition 2024, lint policy mirrors root's deny set, and the MD5
implementation really is in-repo (src/md5.rs).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Deliberately a STANDALONE workspace (empty [workspace] table below), NOT a member of the root workspace" | :4-5 | REAL | root Cargo.toml members array (single-line) excludes bench-vs-vlc (python check: False); tools/bench-vs-vlc/Cargo.toml:17 `[workspace]` bare |
| 2 | "Std-only, zero external dependencies" | :13 | REAL | tomllib: 7 deps, all `{path: ...}` — no version-req crates; src/ = lib.rs + md5.rs only |
| 3 | "MD5 is implemented in-repo from RFC 1321" | :14 | REAL | tools/bench-vs-vlc/src/md5.rs on disk |
| 4 | "Path dependencies ... resolve their own `[lints] workspace = true` ... this crate mirrors the root lint policy locally" | :8-11 | REAL | `[lints] workspace = true` at :41-42; deny set (unsafe_code/unwrap_used/todo/...) matches root Cargo.toml's deny entries |
| 5 | "`input = { path = ..., features = [\"contract\"] }` — the lumen_pcm test drives SyntheticStream" | :48-52 | REAL | tomllib: input features = ['contract'] exactly |
| 6 | "edition = \"2024\"" / package name bench-vs-vlc v0.1.0, publish=false | :34-39 | REAL | tomllib parse: edition 2024, name/version/publish match |
| 7 | Card ownership "OWNED by 3.E3 (tools/bench-vs-vlc/**)" | :1-2 | REAL | doc 13 card 3.E3 scope (corroborated by the README audits already in-tree) |

## 3. Contradictions
None.

## 4. Fabrication / inflation indicators
None — the header's stated membership workaround (why it is standalone) is
accurate against the root manifest's own comments. Findings: none.
