# AUDIT: crates/input/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W10-A09 |
| Date | 2026-11-23 |
| Lines / functions | 38 / manifest (1 package, 1 feature, 2 deps, 1 dev-dep) |
| File verdict | REAL — accurate manifest; every header comment claim verified against the workspace |
| Reachable from shipped entry point? | YES — `input` is a dependency of lumen-cli (apps/cli/Cargo.toml:54) and is called on the open path (`input::FileAccess::new()` at apps/cli/src/demux_host.rs:394, 419-422; `input::resolve_file` at :419). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | rg for `contract` feature consumers (all manifests + workflows), sed on workspace root Cargo.toml lints, rg of `input::` in apps/cli; cross-read of prior input audits (audit/files/crates/input/src/{lib,file,contract}.rs.audit.md) |

## 1. Blunt verdict
Clean, honest manifest. No defects found; below is exactly what was checked and how.

## 2. Checks performed (zero findings, per spec)
1. **Header claim "deps: media-types + plugin-api — the ARCH-2 set"** — `[dependencies]` is exactly those two path deps (L33-34). VERIFIED by reading the file.
2. **Header claim "std-only, zero external dependencies"** — no non-path deps; dev-dep `testkit` is workspace-internal (L36-38). VERIFIED.
3. **Header claim "no unwrap/expect/panic (workspace lints)"** — `[lints] workspace = true` (L21-22) + workspace root Cargo.toml:59-66 denies `unwrap_used`, `expect_used`, `panic`, `todo`, `unimplemented`. VERIFIED by sed on the root manifest.
4. **Feature `contract = []` and the comment "Release binaries (no feature) carry zero of its panicky conformance vocabulary"** — the module is gated at crates/input/src/lib.rs:64 `#[cfg(any(test, feature = "contract"))]`; rg over every manifest + workflow finds NO `features = ["contract"]` anywhere, so the release lane (`cargo build --release -p lumen-cli -p desktop`, no features) compiles none of it. The comment's "CI's --all-features runs it" (tests/file_stream.rs:28) is backed by .github/workflows/ci.yml:234 and ci-full.yml:169 (`cargo test --workspace --all-features`). VERIFIED.
5. **`publish = false`** (L19) — correct for a workspace-internal crate. VERIFIED.
6. **Prior-audit consistency** — input's source audits (lib.rs: REAL 0 findings; file.rs: REAL; contract.rs: REAL, cfg-gated-out-of-release as documented) agree with this manifest's story; the crate is genuinely on the shipped open path. VERIFIED by reading audit/files/crates/input/*.

## 3. Notes
Nothing to flag. The `contract` feature design (conformance vocabulary compiled out of release) is the correct posture for a crate whose tests need panic-on-violation helpers the product build must not carry.
