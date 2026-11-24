# CLAIM AUDIT: plugins/extensions/README.md
| Field | Value |
|---|---|
| Auditor | W12-A03 |
| Date | 2025-06-27 |
| Claims extracted | 17 |
| REAL 12 / REAL-UNPROVEN 3 / PAPER-ONLY 1 / FALSE 1 / STUB 0 / UNREACHABLE 0 (context: the whole extension lane is unshipped) | 12 / 3 / 1 / 1 / 0 / 0 |
| Internal contradictions found | 0 (1 external: missing `scripts/cargo-env.sh`) |

## 1. Blunt verdict
This README is the rare honest one: the two committed `plugin.wasm` blobs hash to EXACTLY the digests printed in its output blocks (sha256 verified by me), the sizes match (764 bytes both), the memory-layout table matches the generator's constants line-for-line, and the test-output block matches the orchestrator baseline byte-for-byte (5 tests, same names, "finished in 0.01s"). One claim is false as written: the walkthrough's reproducibility preamble sources `scripts/cargo-env.sh`, a file that exists neither in the repo nor at the sandbox's scripts path. One implied claim is paper-only: "install is the app's user-consented flow" — no app code anywhere consumes `ExtensionStore` (plugin-host-wasm has zero external dependents; the repo's own gap test records "plugins.crashed not wired — handoff H-10 ... Phase 7").

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Two real, loadable example extensions" + id/kind/capability/permission table | README:3-11 | REAL | dirs + manifests read: `ext.example.metadata`/`metadata-provider`/`provides = ["tags"]`/`fs-sandboxed-read`, `ext.example.panel`/`ui-panel`/`surfaces = ["side-panel"]`/`ui-surface` all match; loading proven by baseline conformance 5/5 |
| 2 | "every output block below is real command output, not illustrative" | README:5-6 | REAL-UNPROVEN | 3 of 5 blocks byte-verified (digests/sizes via `sha256sum`, test block vs baseline); the two `lumenext-examples` console blocks not re-executed (cargo-run budget) |
| 3 | plugin.wasm "committed and pinned … drift gate … fails CI if a committed blob diverges" | README:21-24 | REAL | baseline `committed_example_modules_match_regeneration ... ok` (cargo-test-workspace.txt:4975); `--check` exit-1 path read at bin/lumenext-examples.rs:32-33,524,542 |
| 4 | provenance hash "ships the placeholder form `sha256:…(64 hex)`; packaging stamps the real digest" | README:18-21 | REAL | both manifests carry `hash = "sha256:…(64 hex)"` (sed); `stamp_manifest_hash` at crates/plugin-host-wasm/src/package.rs:359 |
| 5 | modules "emitted by … lumenext-examples.rs … std-only, no wat/wasm-tools" | README:27-33 | REAL | generator read (hand-encoded sections; offsets below); Cargo.toml:60-63 dev-deps note "no `wat`/`wast` dev-dependency" |
| 6 | "after `source scripts/cargo-env.sh` in this sandbox" | README:85-86 | FALSE | `test -f scripts/cargo-env.sh` → MISSING; not in repo `scripts/` nor `/home/z/my-project/scripts/` (which has `env.sh`) |
| 7 | guest memory layout table (0/1024/2048/2052/4096/8192+) | README:69-78 | REAL | generator consts MSG_AT=1024, HANDLE_AT=2048, OUT_LEN_AT=2052, RESULT_AT=4096 (lumenext-examples.rs:326-332); REQUEST_AT=8192 (plugin_examples_conformance.rs:151) |
| 8 | metadata protocol: fs_open → fs_read ≤4 KiB → fs_close; host reads len@2052, text@4096 | README:52-60 | REAL | conformance test asserts the jailed fs round trip + exact sidecar bytes back (plugin_examples_conformance.rs:231-262); READ_CAP=4096 |
| 9 | ui-panel: init registers descriptor via `ui_register_panel`; host surfaces `PanelRecord`; `on_command` acks via log | README:80-87 | REAL | `ui_panel_example_passes_conformance ... ok` (baseline:4977); `panels()` accessor + `PanelRecord` in instance.rs:150/state.rs |
| 10 | "lumenext … never installs and never executes guest code" | README:96-97 | REAL | bin/lumenext.rs:3-8 doc + main() read: only package/inspect subcommands; conformance does the install/spawn |
| 11 | package outputs: "764 bytes", digests `sha256:ed7ec5…`, `sha256:acfe36…` | README:104-115 | REAL | `sha256sum` on committed blobs → identical digests; `stat` → 764 bytes both |
| 12 | inspect outputs: "VALID … sha256 matches … signature: absent (unsigned package)" | README:120-133 | REAL-UNPROVEN | not re-executed; `lumenext_cli_round_trip_packages_and_inspects_both_examples ... ok` (baseline:4978) covers the same path; `SignatureStatus::Unsigned` exists (package.rs:81) |
| 13 | conformance kit: packages→installs→loads→spawns under `ResourceLimits::for_kind`, incl. E_PERM + api-window refusal | README:135-149 | REAL | test file read (spawn_installed, for_kind, E_PERM paths at :274-296); baseline 5/5 ok |
| 14 | test block: 5 named tests, "5 passed; 0 failed … finished in 0.01s" | README:151-160 | REAL | baseline cargo-test-workspace.txt:4973-4980 — identical names, counts, timing |
| 15a | "Unsigned. v1 ships no signature verifier … report Unsigned/Unverified honestly" | README:198-201 | REAL | SignatureStatus::{Unsigned, Unverified} at package.rs:81-85 |
| 15b | "No guest SDK … [PROPOSAL] pending dependency approval" | README:202-204 | REAL | honest limitation; generator-not-Rust verified (claim 5) |
| 15c | "Neither example parses a bitstream in-guest … neither owes a fuzz entry" | README:205-211 | REAL-UNPROVEN | generator emits byte-moving + descriptor code (read); doc 06 §7.7's exact registration rule not read by me |
| 16 | "The panel surface is declarative-only in v1 … actual UI composition … is the app-side card" | README:212-214 | REAL | instance.rs `panels()` returns records only; no composition code exists — matches |
| 17 | "install is the app's user-consented flow (doc 06 §5.7)" | README:96-97, 135-136 | PAPER-ONLY | no app consumes `ExtensionStore` (rg: zero external dependents of plugin-host-wasm); the "app flow" is a design statement, the automated path is the only one that exists |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| `source scripts/cargo-env.sh` (README:86) | repo `scripts/` has no cargo-env.sh; sandbox has `env.sh` | The script is missing — the walkthrough's setup step cannot be followed as written (F-D1) |
| (implied) an app-side install flow exists | repo gap test: "plugins.crashed not wired — handoff H-10 (sandboxed WASM host, plugin-host-wasm, Phase 7)" (baseline:4234) | No app flow exists; tests are the only installer |

## 4. Fabrication / inflation indicators
NONE. The strongest possible counter-evidence was checked and PASSED: both printed sha256 digests match the committed blobs byte-for-byte, the printed test transcript matches the orchestrator baseline including timing, and the printed sizes match `stat`. This doc's evidence blocks are real.

## 5. Matrix rows
Not a matrix chunk — N/A.

## Findings (doc defects)
F-D1 [S3] README:85-86 — reproducibility preamble references a nonexistent script:
```
All commands run from the repository root, after
`source scripts/cargo-env.sh` in this sandbox.
```
`test -f scripts/cargo-env.sh` fails; the sandbox's outer scripts dir holds `env.sh`, not `cargo-env.sh`. The "in this sandbox" qualifier admits the reference is environment-private, but a committed walkthrough that opens with "Everything … was executed … on this tree" should not lean on an uncommitted file. Fix: name the committed equivalent or drop the line. Confidence HIGH — Verification: ran (`test -f`).
F-D2 [S4] README:96-97 — "install is the app's user-consented flow" is future tense dressed as present; no app consumes the store (see claim 17). Softened by "The automated equivalent is the conformance kit" in the same passage. Confidence HIGH — Verification: read (rg dependents).

## Reachability note for the wave report
The README itself never claims the examples ship in the product; every loadable claim is anchored in the (unshipped) plugin-host-wasm crate's tests. The extension lane's unshipped status is tracked under the instance.rs audit (W12-A03).
