# AUDIT: crates/settings-schema/src/bin/schema_gen.rs
| Field | Value |
|---|---|
| Auditor | W11-A03 |
| Date | 2026-11-13 |
| Lines / functions | 331 / 10 (main, run, usage_error, parse_args, value_of, output_paths, find_workspace_root, failure, drift, write_file) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO as a runtime binary — it is a dev codegen entry: `cargo xtask schema-gen` (tools/xtask/src/cmd/schema_gen.rs:66 spawns `cargo run -p settings-schema --bin schema-gen`). Its three committed outputs are the deliverable; `schemas/generated/settings-help.txt` is the CLI `--help` golden (G2) and `settings-docs.md` the reference (G3). Not linked into lumen-cli/desktop release binaries. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `./target/debug/schema-gen --check` → exit 0 "no drift — outputs match … (121 entries)"; drift probe (`--check --ts-out <bogus file under audit/runs/>`) → exit 1 + line diagnostic; `--bogus` → exit 2; `./target/debug/catalog-gen --check` → exit 0; `rg` cross-refs (exit 0) |

## 1. One-paragraph verdict
A small, clean CLI wrapper and it is honest: I executed all three documented exit lanes — `--check` (exit 0, "no drift — outputs match /home/z/.../schemas/settings.toml (121 entries)"), forced drift (exit 1, per-file "differs … at line 1" diagnostic), and a usage error (exit 2, "unexpected argument `--bogus` — see `schema-gen --help`") — matching the header contract "0 success / no drift, 1 drift, 2 usage, I/O or validation error" (L19-21). The "no panics, no unwraps" claim (L22) is true (rg: zero unwrap/expect/panic; only `unwrap_or_else` on Options). Real work is delegated to the library (`settings_schema::load` + `generate_str`, L102-107). The two defects found are S4 CLI-robustness nits: flag values are not validated against looking like flags, and the workspace-root walk-up matches any manifest containing the literal `[workspace]` — which tools/bench-vs-vlc/Cargo.toml actually contains (its own standalone workspace), so running the tool from that directory silently picks the WRONG root for default output paths.

## 2. What actually works (verified)
- `./target/debug/schema-gen --check` (exit 0): `schema-gen: no drift — outputs match …/schemas/settings.toml (121 entries)` + `121 entries across 11 screen(s) of 14 known areas` — the committed G1/G2/G3 outputs match fresh regeneration. This also settles the drift half of the hostile focus for schemas/generated/settings.d.ts (audited separately): no drift.
- Drift path: probe file written under audit/runs/, `--check --ts-out <probe>` → stderr "differs from the regenerated output at line 1" + "output drift — run `cargo xtask schema-gen`…" and exit 1. Note the non-short-circuiting `drift(a)? | drift(b)? | drift(c)?` (L110-111) still evaluates all three `?`s in order, so every drifted file gets its own diagnostic — correct.
- Usage path: `--bogus` → "unexpected argument `--bogus` — see `schema-gen --help`", exit 2.
- Sibling pipeline cross-check: `./target/debug/catalog-gen --check` → exit 0 "no drift — outputs match the catalog (65 commands / 26 events)" (commands codegen in sync too).
- USAGE text (L34-58) lists exactly the flags parse_args implements (--source/--product/--ts-out/--help-out/--docs-out/--out-dir/--check/-h/--help) — read side-by-side, no lies. `--out-dir` conflict guard at L203-209 works as documented.

## 3. Findings
F-001 [S4]
Where: crates/settings-schema/src/bin/schema_gen.rs:213-218 (value_of) and 184-196 (parse_args)
Code: (verbatim)
```
fn value_of<'a>(args: &'a [String], index: usize, flag: &str) -> Result<&'a str, Failure> {
    args.get(index + 1)
        .map(String::as_str)
        .ok_or_else(|| usage_error(format!("missing value for {flag}")))
}
```
What is wrong: the token after a value-flag is taken verbatim as the value; `schema-gen --source --check` sets `source="--check"` and leaves `--check` unprocessed-as-flag (it is consumed as the value, so the check mode is silently NOT enabled — the tool then regenerates and WRITES outputs reading from a nonexistent path named `--check`, failing at load with a confusing "reading `--check`" error instead of "missing value for --source").
Why it matters: hand-rolled arg parsing that cannot distinguish values from flags produces misleading diagnostics; every other flag parser in this repo's tools faces the same pattern, but here the failure mode inverts the tool's mode (check→write).
What correct looks like: reject a following token that starts with `--` as a value (or require `--source=value` syntax).
Confidence: HIGH — Verification: read (control-flow traced; not executed — mutation reasoning only)

F-002 [S4]
Where: crates/settings-schema/src/bin/schema_gen.rs:251-276 (find_workspace_root)
Code: (verbatim, L258-264)
```
    loop {
        let manifest = dir.join("Cargo.toml");
        if let Ok(text) = fs::read_to_string(&manifest)
            && text.contains("[workspace]")
        {
            return Ok(dir);
        }
```
What is wrong: substring match on `[workspace]`. tools/bench-vs-vlc/Cargo.toml contains a literal standalone `[workspace]` table (verified: `[workspace]` on its own line, "Deliberately a STANDALONE workspace" per its header comment), so running schema-gen with cwd inside tools/bench-vs-vlc/ resolves the WRONG workspace root; default outputs then target `tools/bench-vs-vlc/schemas/generated/*` and `write_file` (L318-326) will `create_dir_all` and write them into an unrelated crate.
Why it matters: silent wrong-location writes for anyone who runs the tool from the one directory in this repo that trips the heuristic; the error path is only avoided because the default source also misses there (load error exit 2) — but explicit `--source`/`--product` from that cwd still writes outputs in the wrong tree.
What correct looks like: parse the manifest (or match `^\[workspace\]` as a line) and prefer the FIRST manifest walking up that also contains `[package]`-absence, or simply require the root marker file the other tools use.
Confidence: HIGH — Verification: read + `rg -l "\[workspace\]" crates/*/Cargo.toml tools/*/Cargo.toml` → tools/bench-vs-vlc/Cargo.toml (only hit); not executed from that cwd (cargo-build-budget)

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| main | 73-81 | REAL | — | maps Failure to ExitCode::from(code); executed (all three exit codes observed) |
| run | 83-154 | REAL | — | orchestrates load→generate→check/write; executed via --check and drift probe |
| usage_error | 66-71 | REAL | — | appends "see `schema-gen --help`"; observed on stderr |
| parse_args | 167-211 | REAL | S4 | F-001 (flag-as-value); conflict guard for --out-dir works |
| value_of | 213-218 | REAL | S4 | F-001 |
| output_paths | 226-249 | REAL | — | --out-dir vs per-file defaults correct; default names match doc |
| find_workspace_root | 253-276 | REAL | S4 | F-002 (substring [workspace] false-positive, bench-vs-vlc) |
| failure | 280-285 | REAL | — | LoadError→Failure code 2 |
| drift | 289-316 | REAL | — | missing file counts as drift (L292-298); first-diff-line loop correct incl. prefix case; executed |
| write_file | 318-331 | REAL | — | create_dir_all for parents; typed error mapping |

## 5. Test honesty
No tests in this file. Downstream: crates/settings-schema/tests/integration.rs spawns the binary and asserts exit 0 + "no drift" + an entry count DERIVED from the seed, "never pinned" (integration.rs:117-125 comment) — that is an honest design (a pinned literal went stale once, c71cd5a 27→32, and they fixed the pattern). Would fail if the committed outputs drifted: yes — the same `--check` I ran is what the test asserts.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Exit codes: 0 success (or no drift with --check), 1 drift with --check, 2 usage, I/O or validation error" | L19-21, L58 | REAL | executed: 0 (--check clean), 1 (drift probe), 2 (--bogus) |
| "All errors are typed and actionable on stderr; no panics, no unwraps" | L21-22 | REAL | rg found no unwrap/expect/panic; stderr messages observed actionable |
| "The workspace root is found by walking up … looking for a Cargo.toml containing a [workspace] table" | L56-57 | REAL (but fragile) | F-002: one repo manifest (tools/bench-vs-vlc) is a false positive |

## 7. Dead / duplicated / unreachable code
None in this file. The subprocess-bridge duplication with tools/xtask/src/cmd/codegen.rs and tokens cmd is a deliberate pattern (frozen xtask manifest cannot path-dep on the crates) — documented at L12-13.

## 8. Dependencies & flags
Depends only on the settings-schema lib (GenTarget, LoadError, generate_str) + std. No features. Binary built in target/debug (pre-existing) — no cargo build run by me.

## 9. Open questions for the next agent
- Who consumes `schemas/generated/settings-help.txt` at RUNTIME (lumen-cli --help) vs only as a golden? (W11-A04 owns that file; CLI wiring is theirs to settle.)
- settings.d.ts consumer question is settled in my audit of that file: zero importers.
