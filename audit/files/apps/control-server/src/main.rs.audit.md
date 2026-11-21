# AUDIT: apps/control-server/src/main.rs (whole file)
| Field | Value |
|---|---|
| Auditor | W13-A07 |
| Date | 2026-11-21 |
| Lines / functions | 270 / 9 items (3 consts + 2 types + 7 fns; no tests in this file) |
| File verdict | UNREACHABLE (real, coherent entry-point code; the `lumen-control-server` binary is built by the workspace but excluded from the packaging payload and disabled by the shipped product flag) |
| Reachable from shipped entry point? | NO (packaging stages only `BIN_CLI` + `BIN_TAURI` — scripts/packaging/stage-linux-payload.sh:7–8,71–72; product.toml:167 ships `control-server = false`, so the default run of this binary exits 2 `FeatureDisabled` at `resolve_server_config`; wave-2 SUMMARY already recorded the same unshipped-consumer verdict for 5 control-protocol files) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 3 |
| Commands run | `rg -n "lumen-control-server|CARGO_BIN_EXE"` (no test spawns the binary; Cargo.toml:37 names it); read apps/control-server/src/config.rs:29–119 (AppError → exit-code map 2/3); read apps/control-server/tests/e2e.rs structure (in-process TestClient, no argv coverage); sed on baseline test-workspace.log.txt:1510–1517 (main.rs "running 0 tests" verbatim); `sed -n` on every quoted range |

## 1. One-paragraph verdict
A clean, honest argv/config/bind/serve entry point: strict flag parsing with
typed usage errors, the product-schema pipeline, missing-config-equals-strict
defaults, the F2 feature gate, loopback-only bind, token-on-stderr with a
weak-entropy warning, and the documented 0/2/3/4 exit contract wired
correctly to `AppError::exit_code` (config.rs:72–79). But none of it ships:
the binary is excluded from the release payload and the product model keeps
`control-server = false`, so the surface this file mounts is unreachable from
any shipped entry point and refuses even a manual default run — the same
verdict wave-2 gave its sibling crate. The argv layer additionally has zero
tests (baseline log: "running 0 tests" for this target) and the config file
is read twice.

## 2. What actually works (verified)
- The lib stack this main wires is executed and green in the baseline:
  test-workspace.log.txt:1495–1528 — control-server lib 11/11 ok (incl.
  `config::tests::repo_product_today_refuses_the_feature_gate`, which proves
  the F2 refusal path run() depends on, and
  `run::tests::the_cap_stops_the_loop_after_serving_that_many_sessions`) plus
  e2e 6/6 ok through the app stack.
- Exit-code wiring traced end-to-end: usage→2 (169), product/config errors→
  `err.exit_code()`→2, bind→3 (config.rs:79), serve-loop failure→4 (259),
  `--help`/cap→0 — consistent with the module-doc contract (3–9).
- `main` exits with the code only when non-zero (264–270) — no
  `process::exit` on the happy path, as the doc comment promises.

## 3. Findings

### F-001 [S3] — The whole surface is unshipped: excluded from the payload and disabled by the product flag
Where: apps/control-server/src/main.rs:1 (the binary), vs scripts/packaging/stage-linux-payload.sh:7–8 and product.toml:167
Code:
```rust
//! `lumen-control-server` — the binary (card 7.J1).
```
```text
#   <dest>/$PACKAGING_LINUX_BINDIR/$BIN_CLI            0755
#   <dest>/$PACKAGING_LINUX_BINDIR/$BIN_TAURI          0755
```
```toml
control-server = false          # Phase 5+ [PROPOSAL]
```
What is wrong: the binary is a workspace member (baseline build #1 compiled
it clean) but the release payload stages only the CLI and Tauri binaries, and
the shipped product model keeps the feature off — `resolve_server_config`
answers `FeatureDisabled` (config.rs:148–155) and `run` exits 2 before
binding. No shipped entry point reaches this file; no CI or script runs the
binary (rg: zero `CARGO_BIN_EXE`/spawn references).
Why it matters: 270 lines here (plus the crate it fronts) are maintained,
linted, and built into every workspace build with zero product reach — dead
weight in the release pipeline until the Phase-5 card lands. Wave-2 already
carried this verdict for the protocol crate; this is the same conclusion at
the entry point.
What correct looks like: either the Phase-5 card that flips product.toml and
adds the binary to the payload, or move the crate behind a workspace
optional-member/dev-only boundary until then.
Confidence: HIGH — Verification: read (packaging script + product.toml +
config.rs gate) + baseline (build compiled it; tests green in-process; no
executed evidence of the binary itself).

### F-002 [S4] — The operator config file is read twice (discarded first read, TOCTOU window)
Where: apps/control-server/src/main.rs:182–189
Code:
```rust
    let file = match std::fs::read_to_string(&args.config) {
        Ok(_) => match load_config_file(&args.config) {
            Ok(file) => file,
```
What is wrong: `read_to_string`'s result is thrown away — it exists only to
distinguish NotFound from other errors — and `load_config_file` then re-opens
and re-reads the path. If the file is deleted or becomes unreadable between
the two reads, the second read surfaces as `ConfigRead` (a hard exit-2
error) instead of the missing-file strict-defaults path; between the same
two reads a file swap means validation and parse can see different bytes.
Why it matters: duplicated I/O plus a small race on a security-relevant
input (the policy file).
What correct looks like: read once into a String, branch on the error kind,
pass the text into the parser.
Confidence: HIGH — Verification: read (load_config_file re-reads internally,
config.rs:127–139).

### F-003 [S4] — Usage errors exit 2 but the documented exit-2 bucket omits them; no `--` separator, no `--version`
Where: apps/control-server/src/main.rs:3–9 vs 167–170
Code:
```rust
//! * exit 2 — config/product trouble (unreadable/unparsable config,
//!   bad policy grants, product.toml diagnostics, feature flag off);
```
```rust
        Parsed::Usage(detail) => {
            eprintln!("usage error: {detail}\n\n{}", usage());
            return 2;
        }
```
What is wrong: the process contract enumerates four exit meanings; a usage
error (unknown flag, bad `--sessions`, missing value) also answers 2 without
being listed in the contract. Additionally argv has no `--` end-of-flags
form (a path beginning with `-` is unexpressible) and no `--version` (the
CLI sibling has one).
Why it matters: script authors branching on exit codes per the module doc
cannot distinguish "my flags were wrong" from "config trouble".
What correct looks like: document the usage-error→2 mapping (or give usage
its own code), add `--` and `--version`.
Confidence: HIGH — Verification: read.

### F-004 [S4] — The argv layer has zero tests; the documented process contract is unpinned
Where: apps/control-server/src/main.rs (no `#[cfg(test)]`); baseline evidence
Code (baseline log, audit/runs/baseline/test-workspace.log.txt:1512–1516):
```text
     Running unittests src/main.rs (/home/z/my-project/target/release/deps/lumen_control_server-7fa7a7bfd5e6f131)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```
What is wrong: `parse_args` (69–105), `usage` (108–129), and `run`'s
help/usage/token/startup-line branches have no tests in this file, and
nothing in the repo spawns the binary (rg: no `CARGO_BIN_EXE_lumen_control_server`).
The e2e suite drives the LIB in-process (TestClient) — argv parsing, the
exit codes, the stdout startup line, and the stderr token line are all
unverified by any executed run.
Why it matters: the file's entire user-visible contract (the module doc's
first 16 lines) rests on reading, not evidence; a regression in `parse_args`
(e.g. the `value_after` index arithmetic) ships invisibly.
What correct looks like: unit tests for parse_args/usage (pure functions,
trivial to test) plus one spawned-binary smoke asserting the startup line
and exit 0 on `--help`.
Confidence: HIGH — Verification: read + baseline log verbatim.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts `DEFAULT_CONFIG`/`DEFAULT_PRODUCT`/`DEFAULT_SCHEMA` | 33–39 | REAL | — | relative paths (cwd-dependent — documented as "repo root / product working dir"); allow-literal annotated |
| `struct Args` | 42–47 | REAL | — | 3 paths + optional cap |
| `enum Parsed` | 50–57 | REAL | — | Help/Args/Usage(String) |
| `value_after` | 60–66 | REAL | — | advances index; typed "needs a value" error; clones the value (argv-sized, fine) |
| `parse_args` | 69–105 | REAL | S4 | strict unknown-flag errors; `--help` wins wherever it appears; `--sessions` u32 parse with typed error; no `--`/`--version` (F-003); `--sessions 0` accepted — "serve at most 0" → immediate exit 0 (serve_loop semantics, lib.rs, pinned by the lib cap test) |
| `usage` | 108–129 | REAL | — | text matches the flags actually parsed (both reviewed line by line); allow-literal annotated |
| `load_product` | 132–156 | REAL | — | schema read→parse→product load, every failure a typed AppError with path context |
| `run` | 161–262 | REAL | S3/S4 | F-001 (unshipped), F-002 (double read); ordering product→config→resolve→bind→addr→token→startup→serve matches the doc; NotFound config → strict defaults; token only on stderr with weak-entropy warning; `engine_info` cloned once and shared by dispatch+welcome (no-drift comment verified against CatalogDispatch::new at 246) |
| `main` | 264–270 | REAL | — | collects argv, exits only on non-zero |

## 5. Test honesty
No tests exist in this file. Nearest coverage (all lib-level, in-process):
| Test (elsewhere) | Would it fail if this file's code were broken? | Reason |
|---|---|---|
| control-server lib tests (11, baseline ok) | NO | they exercise config/dispatch/logging/run::serve_loop — none of them call main.rs's parse_args/usage/run |
| tests/e2e.rs (6, baseline ok) | NO | TestClient drives the lib stack in-process; no argv, no exit codes, no spawned binary |
| (none) | — | F-004: the argv layer and process contract have zero executed evidence (baseline "running 0 tests" verbatim) |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "exit 0/2/3/4" process contract | module doc 3–9 | REAL-UNPROVEN | traced consistent (run 165–260 + config.rs:72–79 map); no executed evidence of the binary (F-004); usage-error→2 undocumented (F-003) |
| "Startup prints the bound ws:// address on STDOUT" | module doc 11 | REAL-UNPROVEN | run.rs:244 `println!("lumen-control/{PROTOCOL_VERSION} listening on ws://{address}")`; never executed in evidence |
| "the token on STDERR (the single sanctioned reader path)" | module doc 12–14 | REAL-UNPROVEN | 231–242; structured sink is JsonStderr (separate handle) — code-verified, run-unverified. Note: stderr is the controlling terminal only when not redirected |
| "missing file = strict defaults" | module doc 33 + usage text | REAL | code 190–192 + lib test `a_missing_config_file_is_the_strict_default_not_an_error` (baseline ok) |
| "No signal handling (std-only): SIGTERM/SIGINT end the process" | module doc 15–16 | REAL | no signal code (imports std + workspace crates only) |
| product.toml feature exposure for control-server | product.toml:167 | honest FALSE flag (feature off) | `control-server = false # Phase 5+ [PROPOSAL]`; `repo_product_today_refuses_the_feature_gate` lib test proves the refusal — no FALSE shipping claim exists for this surface (docs/feature-matrix.toml has no control-server row, rg-verified) |

## 7. Dead / duplicated / unreachable code
- The entire file is unreachable from shipped entry points (F-001) — the
  binary is built, never shipped, and self-refuses by product flag.
- No internal dead code: every function is on the run() path.

## 8. Dependencies & flags
- Consumes: `control_protocol` (ControlServer, TokenSource, policy,
  token::EntropySource), `control_server` (the lib: AppError,
  CatalogDispatch, JsonStderr, load_config_file, resolve_server_config,
  serve_loop), `product_config`, std only otherwise.
- Two `lumen-allow-literal` annotations (34, 243) + one in usage (110) —
  the annotated identity-literal posture, consistent with the baseline
  lint-literals gate.
- No feature gates in this file; the gate that matters is the product-model
  flag (off in the shipped model).

## 9. Open questions for the next agent
- A05 (config.rs, this wave): F-002's fix point (pass text into the parser)
  and the `FeatureDisabled` message quality both live in your file.
- A01 (run.rs, this wave): the `--sessions 0` boundary (cap 0 → immediate
  exit) — your `the_cap_stops_the_loop_after_serving_that_many_sessions`
  covers N≥1; is 0 pinned?
- Wave lead: F-001 is a product decision (ship the surface or fence the
  crate) — same call wave-2 flagged for control-protocol.
