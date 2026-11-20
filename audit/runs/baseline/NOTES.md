# Baseline run notes (Wave 0)

## full_gate.sh
`scripts/full_gate.sh` was not executed script-level in Wave 0. Every command it runs was
executed individually and captured in this directory (fmt --check CLEAN; clippy
`--workspace --all-targets --locked -- -D warnings` CLEAN; `cargo test --workspace --locked`
1001/2/20; xtask check-architecture FAILURE edges=fail; fixtures --check PASS;
fuzz_registry_check.sh and lint-literals/check-identity PASS). full_gate's clippy/test use
`--all-features`, a heavier feature resolution than the release lanes' (release.yml builds
`-p lumen-cli -p desktop` with no feature flags); the all-features variant was deferred for
CPU budget (2-core host, serialized builds) and is queued behind the wave audits.

## rc-smoke.sh
Run with `RC_SMOKE_FIXTURE=audit/runs/baseline/sample-h264-aac.mp4 RC_SMOKE_SKIP_WEB=1`
(engine-only). Check 6 (wasm-web-build) FAILED because the wasm32-unknown-unknown target is
not installed on this host — environment limitation, not a repo defect (rustup target add
would install it). Checks 7-8 skipped by the script's own SKIP_WEB flag; the script prints
"NOT a full RC smoke" in that mode. Result: FAIL (headless-decode FAILED on the MP4 fixture,
which is the point of the litmus).

## tokei/scc
Unavailable in this environment; LOC computed by Python byte-line counting
(audit/waves/wave-00/manifest.tsv). Equivalent data, different tool.

## cargo build parallelism
First `cargo build --release` attempt at default parallelism was OOM-killed (4 GB host);
all cargo runs use `-j 1`. Build log reflects the successful serialized run.
