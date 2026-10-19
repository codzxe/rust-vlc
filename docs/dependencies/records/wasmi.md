# deps/records/wasmi.md — wasmi 2.0.0 (pre-audit record, filled 2026-10-19)

> **Class:** Phase 0 pre-audit record (doc 11 §2 format, fields R1–R10). Evidence
> gathered 2026-10-19 by task 0.C4-I using `docs/dependencies/audit-method.md`
> (crates.io API + sparse index + static tarball inspection; crate never built or
> executed). Fields that need the Phase 1 workspace/lockfile are marked
> `[VERIFY-P1]`; repo-side facts unreachable from the sandbox are `[VERIFY-P0]`.
> Final acceptance and the allowlist row are Lead decisions (doc 11 §3.1).

## R1 — Language & source

- Rust, **pure** per tarball inspection (2026-10-19, sha256-verified artifact):
  - 0 native source files (`*.c *.cc *.cpp *.cxx *.h *.hpp *.m *.mm *.S *.s *.asm
    *.nas *.sx` — none);
  - 0 `extern "C"` / `extern "system"` / `#[link]` declarations in `src/`;
  - 0 binary blobs (`.a .lib .o .obj .so .dll .dylib`);
  - no `links=` key in `Cargo.toml` / `Cargo.toml.orig`.
- Repo: https://github.com/wasmi-labs/wasmi (from the crates.io API `repository`
  field). Description: "WebAssembly interpreter". Categories include `no-std`.
- Publisher (crates.io `published_by`): Robin Freyler (Robbepop) — primary
  maintainer. **Bus factor: small (single primary maintainer)** — recorded as an R8
  risk, not a rejection.
- `build.rs` **present but benign** (full read, quoted in audit-method.md §4 M6):
  emits `cargo::rustc-cfg` values for dispatch-backend selection (tail-call support
  per `CARGO_CFG_TARGET_ARCH`, including explicit `wasm32`/`wasm64` handling); no
  toolchain invocation, no network, no OUT_DIR binaries.

## R2 — Transitive graph (approximate; `[VERIFY-P1: cargo tree -e normal,build` at pinned lockfile]`)

- Declared deps (crates.io dependencies API, 2026-10-19): 8 total — 2 dev-only
  (`assert_matches`, `criterion`); normal: `spin ^0.9`, `wasmi_collections ^2.0.0`,
  `wasmi_core ^2.0.0`, `wasmi_ir ^2.0.0`, `wasmparser ^0.228.0`; optional: `wat
  ^1.228.0` (enabled by the default `wat` feature).
- Default-features closure: **15 crates** — bitflags, bumpalo, leb128fmt, libm,
  memchr, spin, unicode-width, wasm-encoder, wasmi, wasmi_collections, wasmi_core,
  wasmi_ir, wasmparser, wast, wat (approximation per audit-method.md §3 M8).
- Recommended pin `default-features = false, features = ["std"]` closure: **8
  crates** — bitflags, libm, spin, wasmi, wasmi_collections, wasmi_core, wasmi_ir,
  wasmparser.
- No `cc`/`cmake`/`bindgen`/`pkg-config`/`*-sys`/`libc` anywhere in the walked
  graph. Per-transitive-crate P1–P8 scans: `[VERIFY-P1]` (Phase 1 G3 scanner).

## R3 — Build scripts

- `build.rs` present (contents per R1: cfg emission only). Manifest `build` key:
  `build = "build.rs"`. **No native toolchain tokens** (cc/clang/gcc/cl.exe/nasm/
  yasm/masm/cmake/ninja/xcrun/bindgen/pkg-config — none). No download tokens.

## R4 — Native links

- None. No `links=`; no `#[link]`; no `extern "C"`; no blobs. F1–F7 all **NO**.

## R5 — Targets

- Manifest: no target-gated dependencies at all (dependencies API shows `target:`
  null for every entry).
- `rust-version = "1.86"`, `edition 2024`, `categories` include `no-std` (i.e.,
  `no_std`-capable ⇒ wasm-viable). `build.rs` explicitly handles `wasm32`/`wasm64`
  target arch (tail-call gate) — evidence the wasm target is supported and tested
  upstream `[VERIFY-P1: our CI builds `--target wasm32-unknown-unknown`]`.
- Our required desktop triples (x86_64/aarch64 × Windows/Linux/macOS) are covered by
  the tail-call arch list in build.rs (x86, x86_64, arm, aarch64, riscv*, …)
  `[VERIFY-P1: CI matrix build]`.

## R6 — Actual feature coverage vs doc 06 §5 (WASM extension host)

Verified **from the 2.0.0 tarball source** (2026-10-19):

| Requirement (doc 06 §5.2–5.4) | wasmi 2.0.0 evidence | Verdict |
|---|---|---|
| Separate linear memories, no host pointers in sandbox | guest memory accessed only via `Memory` read/write/grow APIs (`src/memory/mod.rs`; `grow()` takes page counts, host copies) | satisfied (design-compatible) |
| Linear-memory cap | `MemoryType::new(minimum, maximum: Option<u32>)` per-memory max **plus** `StoreLimitsBuilder` with `memory_size(limit)` total-store cap (`src/limiter.rs`; also `table_elements`, `instances`, `tables`, `memories`, `trap_on_grow_failure`) | satisfied |
| CPU limit (epoch-or-fuel) | **fuel metering**: `Store::get_fuel`/`set_fuel`, `Caller::get_fuel`/`set_fuel` (`src/store/mod.rs`, `src/func/caller.rs`). **No epoch interruption** (0 `epoch` refs in `src/`) | satisfied via fuel (deterministic); epoch N/A — doc 06 §5.4 already names "fuel metering as fallback" |
| Instantiation/DoS bounds | module validation limits: `max_module_size`, `max_type_count`, `max_import_count`, `max_func_count`, `max_func_param_count`, `max_table_count`, `max_table_size`, … (`src/limits.rs`) | satisfied (exceeds spec) |
| Per-instance import resolution (capability imports) | `Linker` + per-`Store` instantiation (`src/linker.rs`); undeclared import ⇒ instantiation error — matches doc 06 §5.3 dispatch model | satisfied |
| WASI or no-WASI | core crate contains **no WASI** (no wasi dep in graph; `lumen:v1.*` custom imports are the plan) | satisfied (no-WASI posture) |
| Error isolation | traps destroy only the instance; `ResumableCall` machinery (`src/func/mod.rs`) for host-call resumption | satisfied |
| wasm32 host target | no-std + wasm cfg handling in build.rs | satisfied `[VERIFY-P1 in CI]` |

Coverage not yet provable statically: behavior under our limit-breach conformance
suite (doc 06 §9.4) — Phase 1+/7.F evidence. Throughput on metadata/playlist
workloads (doc 06 §5.1 "audit focus") — needs the Phase 2/7 benchmark
`[VERIFY-P1: bench-vs-vlc extension micro-benchmarks]`.

## R7 — License

- Manifest SPDX: **MIT/Apache-2.0** (dual). Matches the doc 11 §3.2 allowlist
  sketch (MIT, Apache-2.0 both allowed) and is compatible with the MIT repo
  (ADR-006).
- **No LICENSE file inside the tarball** (crate excludes it); SPDX comes from the
  manifest. Repo LICENSE: `[VERIFY-P0: GitHub unreachable from sandbox]`.
- Transitive licenses: all walked crates are known MIT/Apache/BSD-class names —
  `[VERIFY-P1: cargo-deny licenses at lockfile]`.

## R8 — Maintenance & security record (all fetched 2026-10-19)

- Newest stable: **2.0.0, 2026-09-01** (7 weeks old at audit date — fresh-major
  risk; the 1.x line: 1.1.0 2026-06-12, 1.0.2–1.0.9 Dec 2025–Feb 2026, i.e. a
  steady patch cadence before the major).
- Total downloads 25,285,206; recent downloads 5,217,575 (crates.io API).
- Production usage: Polkadot/Substrate ecosystem (doc 06 §5.1 claim, predating
  2.0) `[VERIFY-P0: current downstream list]`.
- RustSec/OSV: **2 advisories ever, both closed, neither affecting 2.0.0**:
  GHSA-75jp-vq8x-h4cq (OOB write, host→wasm calls with >128 params; fixed 0.31.1),
  GHSA-g4v2-cjqp-rfmq (UAF in linear memory; fixed 0.41.2).
- Bus factor: single primary maintainer (see R1). GitHub stars/commit activity:
  `[VERIFY-P0: api.github.com 403 from sandbox]`.

## R9 — Provenance

- Tarball: `https://crates.io/api/v1/crates/wasmi/2.0.0/download` (2026-10-19),
  300,979 bytes = crates.io `crate_size`. (`static.crates.io` direct URL returned
  403 AccessDenied from this sandbox — recorded in audit-method.md §3 M4.)
- sha256 `78693fcdd618e0fc34af59c6b8efa9ac5d58c68df940beff4bedddb6acfe7c27`
  **matches** the sparse-index record `index.crates.io/wa/sm/wasmi` line
  `"vers":"2.0.0"`, `yanked: false`. Publisher identity per R1.
- cargo-vet entry: `[VERIFY-P1]` (Phase 1 bootstrapping per doc 11 §3.3).

## R10 — Verdict (pre-audit)

**PASS** (pre-audit): F1–F7 all NO; MIT/Apache-2.0; sandboxing primitives present
(fuel, store limits, module limits, resumable calls); active maintenance; clean
advisory posture at 2.0.0; no-WASI core; wasm32 host target viable.

Proposed Phase 1 pin (for the Lead's allowlist row, doc 11 §3.1):
`version-range = ">=2.0.0, <3"`, `features = { no-default-features, std }`
(add `validate` if we use wasmparser-based validation through wasmi — our own
validator choice to be settled in 7.F1; add `wat` in dev/tests only). 2.0.0's
freshness argues for `>=2.0.0, <2.1` until the 2.x line shows patch cadence
`[PROPOSAL]`.

Approved by: — (Lead decision pending; this is a pre-audit record, doc 11 §1.3).
