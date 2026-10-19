# ADR-002 evidence — WASM extension-host runtime candidates (evidence pack, NOT a decision)

| | |
|---|---|
| **Feeds** | ADR-002 (doc 01 §7): the Lead's choice of the WASM runtime for `crates/plugin-host-wasm`. Decision itself happens in task 0.D3 — **this file is the evidence pack only.** |
| **Implements** | doc 06 §5 (hard requirements, cited by section number only), doc 11 §1–§4 (F1–F7 purity checks), `docs/dependencies/audit-method.md` (M1–M9 pre-audit method), `docs/dependencies/allowlist-format.md` (verdict vocabulary / purity classes). |
| **Method** | All facts fetched 2026-10-19 (task 0.C4-R; wasmi facts inherited from task 0.C4-I's record of the same date). crates.io HTTP API + sparse index + static tarball inspection (`tmp/preaudit/`, gitignored). Nothing was built or executed. Every claim carries its source; unfetchable = `[VERIFY]`. |
| **Correction note** | The 0.C4-I scratch walk of wasmtime's default closure undercounted (94 crates) because manifest dep names (`wasmtime-cranelift`, `wasmtime-fiber`, …) differ from crates.io crate_ids (`wasmtime-internal-cranelift`, …). Re-walked with API-name seeds: **≈129 crates**. Both numbers are approximate per audit-method §3 M8. |

---

## 1. Hard requirements (from doc 06 §5; section numbers only)

The extension host must provide, per doc 06:

| # | Requirement | Source |
|---|---|---|
| H1 | No hidden native code: full transitive graph auditable, zero C/C++/asm sources, no `build.rs` invoking a non-Rust toolchain | doc 06 §5.1(a)–(b); MP §9; doc 11 F1–F7 |
| H2 | Per-instance linear memories; no host pointers in the sandbox; offsets+lengths only | doc 06 §5.2 |
| H3 | Capability-based permission enforcement at import dispatch; undeclared import ⇒ instantiation error or typed `PermissionDenied` | doc 06 §5.3 |
| H4 | Resource limits: linear-memory cap (store-level), CPU limit per dispatched call — **epoch interruption preferred, fuel metering acceptable fallback** — instantiation deadline, I/O caps enforced host-side | doc 06 §5.4 |
| H5 | API surface versioning by module namespace (`lumen:v1.*`), additive-only within a major | doc 06 §5.5 (host-side; runtime only needs per-instance import resolution) |
| H6 | Error isolation: a trap destroys only that instance; restart policy; bounded crash reporting | doc 06 §5.6 |
| H7 | Install/remove flow validates `\0asm` magic; **no native dynamic loading ever** (the runtime is the native-code fence for extensions) | doc 06 §5.7, §6 |
| H8 | Extensions never run on realtime audio/video/UI threads; dedicated pool | doc 06 §5.8 (host-side; runtime must tolerate being driven from a pool thread under deadlines) |
| H9 | Target coverage: x86-64/aarch64 × Windows/Linux/macOS desktop, **and the engine's `wasm32-unknown-unknown` web target** (nested-wasm question, doc 06 §8 + doc 09 §2.1) | doc 06 §5.1(f); doc 09 §2 |
| H10 | License compatible with the MIT repo (ADR-006); maintenance/security record acceptable | doc 06 §5.1(d)–(e) |
| H11 | Non-realtime workload class only: UI panels, playlist/metadata handlers, light filters — no decoder/demuxer/output extensions in v1 | doc 06 §1.4, §5.9 |

## 2. Candidate set

Doc 06 §5.1 named `wasmtime`, `wasmi`, and rejected "wasm3, Fizzy, and any C/C++ runtime" outright. This audit covers wasmi (already recorded), wasmtime (full pre-audit below), and — to close the candidate space honestly — `wasmer` and `fizzy` (both fetched and scanned; both fail). A crates.io search (2026-10-19, `q=wasm runtime`, `q=wasm interpreter`, `q=fizzy`, `q=orthanc`) surfaced no other credible pure-Rust **execution** runtime: remaining hits are format libraries (`parity-wasm`, `wasm-instrument`, `wasmparser`-family), tooling (`wit-component`), or unrelated crates. Known non-crates.io runtimes fail by language before audit: wasm3 and WAMR (C), wazero (Go), LLVM-backed stacks (C++), spec-interpreter (OCaml).

## 3. Candidate table

| | **wasmi 2.0.0** | **wasmtime 49.0.3** | **wasmer 7.5.0** | **fizzy 0.8.0** |
|---|---|---|---|---|
| Role | Pure-Rust interpreter | JIT (Cranelift/Winch, both Rust) + Pulley interpreter | Multi-backend JIT | Bindings to a C++ interpreter |
| Released | 2026-09-01 | 2026-10-14 (5 days before audit) | 2026-10-01 | 2022-06-28 (**dead ~4.3 y**) |
| License | MIT/Apache-2.0 | **Apache-2.0 WITH LLVM-exception** | MIT | Apache-2.0 |
| Crate size | 300,979 B | 1,050,084 B | 235,496 B | 66,339 B |
| Downloads (total / recent) | 25.3 M / 5.2 M | 40.6 M / 11.5 M | 8.8 M / 0.9 M | 5,699 / 16 |
| Language composition | 100% Rust (tarball scan: 0 native files, 0 blobs, 0 `extern "C"`, no `links=`; build.rs = cfg emission only) | Rust **+ C compiled at ≥5 points in default config** (§4 below); Winch/Cranelift/Pulley are Rust, but helpers.c / windows.c / gdbjit.c / ittapi-sys / zstd-sys are C | Rust surface; build graph **requires bindgen + cmake** (non-optional) to build C/C++ | Rust bindings over C++ sources; **bindgen ^0.54 + cmake ^0.1 non-optional build-deps** |
| Transitive graph (approx., audit-method M8) | 15 crates default / **8 crates** with `no-default-features + std`; no `cc`/`libc`/`*-sys` | **42 crates** with no default features (still pulls `libc`); **≈129 crates** default (incl. `cc`, `gcc 0.3.55`, `ittapi-sys`, `zstd-sys`, `libc`, `rustix`, `windows-sys`, `mach2`, `memfd`) | 29-crate base walk incl. `cc`, `cmake`, `tar`, `find-msvc-tools` | 3 deps: bindgen, cmake, hex(dev) |
| H1 purity | **PASS** (F1–F7 all NO; record exists) | **FAIL** — C compilation cannot be feature-gated off on desktop (§4) | **FAIL** — bindgen+cmake non-optional; optional `ureq` build-dep (build-time network) | **FAIL** — C++ + bindgen + cmake |
| H2 memories | separate memories, host-copy APIs | separate memories, host-copy APIs | same | same (via C++) |
| H3 import resolution | `Linker` + per-`Store`; undeclared import ⇒ error | `Linker` + per-`Store`; undeclared import ⇒ error | `ImportObject` | binding-level |
| H4 CPU limit | **fuel only** (`Store::set_fuel`; no epoch) — doc 06 §5.4 names this the acceptable fallback | **epoch** (`Engine::increment_epoch`, `Store::epoch_deadline_callback`; src/engine.rs:875, store) **and** fuel (`Store::set_fuel`, src/runtime/store.rs:1016) | fuel | n/a |
| H4 memory cap | `MemoryType` max + `StoreLimitsBuilder::memory_size` + module-size/count limits (exceeds spec) | `ResourceLimiter` trait (`memory_growing`, src/runtime/limits.rs:32) + pooling-allocator caps | ResourceLimiter-ish | n/a |
| WASI | none in core (we want `lumen:v1.*` imports; no-WASI posture) | none in core crate's default graph (`wasmtime-wasi` is a separate opt-in crate) | optional | none |
| H6 error isolation | traps destroy instance only; resumable calls | traps destroy instance only | same | n/a |
| H9 desktop targets | x86_64/aarch64/… covered by tail-call arch list; `[VERIFY-P1: CI matrix]` | x86_64/aarch64/riscv64/s390x Cranelift hosts + Pulley elsewhere | x86_64/aarch64 | x86_64 |
| H9 wasm32 host (nested) | **viable**: `no_std`, explicit `wasm32`/`wasm64` handling in build.rs | **questionable**: Pulley fallback exists for non-Cranelift hosts, but wasm32 needs `custom-virtual-memory`/`custom-native-signals` features; upstream targets wasm32 mainly for tooling `[VERIFY-P1]` | wasmer *can* target wasm32 (js-sys/wasm-bindgen deps gated) but irrelevant given FAIL | no |
| Advisories (OSV, 2026-10-19) | 2 lifetime, both closed pre-2.0.0 | 101 lifetime, **0 open affecting 49.0.3**; 49.0.3 is itself the fix release for ≥8 RUSTSEC-2026-03xx | 1 **open/unfixed**: CVE-2024-38358 sandbox symlink bypass, range `0..open` | 0 |
| Maintenance | single primary maintainer (bus-factor risk, R8); steady 1.x patch cadence before 2.0 | Bytecode Alliance org; **3 patch lines (36.x/48.x/49.x) released same day 2026-10-14**; very high cadence | moderate | dead |
| MSRV | Rust 1.86, edition 2024 | **Rust 1.96**, edition 2024 | n/a (fails) | n/a |
| **Pre-audit verdict** | **pass** (records/wasmi.md; proposed pin `>=2.0.0,<2.1`, no-default-features + `std`) | **fail** on H1 for any desktop execution configuration (§4); sandbox primitives best-in-class; adoption would need a **new** exception class (not E1/E2) — policy change, hence decision-framing §6 | **fail** | **fail** (already rejected outright in doc 06 §5.1) |

## 4. wasmtime 49.0.3 — the C inventory (why H1 fails)

Every item below was read from the sha256-verified tarball
(`4fbdd39f5c66f63ba6b5b31391b324b7ade7237d29c866cdb561330118ef436e`, matches
sparse-index `cksum`, `yanked:false`; fetched 2026-10-19):

1. **`wasmtime`'s own build.rs compiles `src/runtime/vm/helpers.c` via `cc`**
   (`build_c_helpers()`, build.rs:113–137) whenever `feature = "runtime"` **and**
   the target arch is a Cranelift host (x86_64/aarch64/riscv64/s390x) **and**
   `(unix || windows) && std`. The `runtime` feature is default and is required
   for executing anything — on all six of our desktop triples this C file is
   compiled. Not gateable off while keeping execution.
2. **`wasmtime-internal-fiber` ships `src/windows.c`** (compiled via `cc` on
   Windows; on Unix it uses `naked_asm!` inline assembly — `src/stackswitch/x86_64.rs`
   et al.). `cc ^1.0` is a **non-optional build-dependency** of this crate, and
   the crate is default-enabled via the `async`/`stack-switching` features.
3. **`wasmtime-internal-jit-debug` compiles `gdbjit.c` via `cc`** when feature
   `gdb_jit_int` is on — which wasmtime's default `debug-builtins` feature
   enables (`Cargo.toml` features table). Build-dep `cc ^1.2.0` non-optional.
4. **`ittapi-sys`** (default via `profiling` on x86_64 Linux/macOS/Windows):
   `-sys` crate whose sole build-dep is `cc ^1.0.73` — vendored Intel C sources.
5. **`zstd` → `zstd-safe` → `zstd-sys`** (default via `cache`; used to
   compress cached artifacts): `zstd-sys` has `links = "zstd"` (sparse index,
   2026-10-19), build-deps `cc` **and the legacy `gcc 0.3.55` crate**, and
   compiles the vendored C zstd library.

Countervailing positives, recorded honestly: Cranelift, Winch and Pulley (the
compilers/interpreter) are pure Rust; the default ≈129-crate closure is
overwhelmingly Rust; the crate's own README advertises 24/7 OSS-Fuzz; and on
`wasm32` hosts the build.rs's `has_host_compiler_backend`/`supported_os` gates
turn points 1–3 off (no C compiled) — wasmtime-on-wasm is not C-contaminated,
it is feature-starved instead (`custom-virtual-memory`, `custom-signals`
required `[VERIFY-P1]`). The graph also contains un-gated `libc` (E2a-class
per doc 11 §"pre-approved registry entry") even with no default features.

**Language-composition summary:** wasmtime is "Rust with five C warts, three of
them removable only by dropping default features (ittapi, zstd), two of them
unremovable on desktop (helpers.c, fiber windows.c / gdbjit.c)."

## 5. wasmer and fizzy (rejection evidence)

- **wasmer 7.5.0** (tarball sha256 `ddfde3972691f1082774c71c42e21add1d1838cb6482a6921a0ba8a993e0046a`,
  index-verified, 2026-10-19): non-optional **build-deps `bindgen ^0.72.1` (all
  cfg targets), `cmake ^0.1.50`, `tar ^0.4.40`**; optional build-deps include
  `ureq ^3.4.2` (build-time network fetch), `xz`, `zip`, `which`. F2/F4-class
  fail on any configuration we would ship. Plus one **open** advisory
  (CVE-2024-38358, filesystem-sandbox symlink bypass, unfixed at 7.5.0). MIT,
  but that does not save it.
- **fizzy 0.8.0**: crate description says "Bindings to Fizzy, … WebAssembly
  interpreter" — the interpreter is C++; build-deps `bindgen ^0.54.0` +
  `cmake ^0.1` non-optional. Last release 2022-06-28; 16 recent downloads.
  Exactly the class doc 06 §5.1 rejects outright; recorded here so the
  rejection is evidence, not assumption.

## 6. Decision framing (for 0.D3 — the Lead/user decides)

**The trade axis.** wasmi is slower (interpreter; no JIT) but pure, small
(8-crate closure, ~300 KB crate) and WASI-free by construction. wasmtime is
the industry reference sandbox (epoch interruption — doc 06 §5.4's *preferred*
CPU limiter — pooling allocators, component model) but compiles C on every
desktop configuration that can execute a module, drags a ~129-crate default
graph, and requires Rust 1.96. Under MP §9 as written, wasmtime is not
admissible without a **new** exception class (E1 = OS webview, E2 = OS-service
bindings; vendored C runtime support code is neither — it would be an E3-style
"runtime C" policy change, which the Master Prompt currently forbids and doc 06
§5.1 explicitly anticipated auditing out).

**The doc-06 non-realtime argument for interpreter sufficiency.** Extensions
are UI panels, playlist/metadata handlers, light filters (doc 06 §1.4, §5.9);
they run off every realtime path under explicit wall-clock budgets (100 ms
call, 1 s metadata batch, 5 s playlist parse — doc 06 §5.4). At those budgets,
raw interpreter throughput is rarely the binding constraint; the 32 MiB memory
cap and host-side I/O token bucket dominate. The residual risk is a pathological
guest burning CPU inside its budget — mitigable with fuel (deterministic, and
doc 06 §5.4 already accepts fuel as the fallback) plus our conformance suite
(doc 06 §9.4). A dual-runtime posture (wasmi debug / wasmtime release) exists as
doc 06 §5.1's fallback `[PROPOSAL]` but doubles the audit surface and keeps the
C problem for release builds.

**The nested-wasm-on-web question** (doc 06 §8 "not in v1 `[PROPOSAL]`";
`[VERIFY-P0]`). The engine compiles to `wasm32-unknown-unknown` (doc 09 §2.1);
if the extension host is to run there too, the runtime itself must execute
under wasm32. wasmi is `no_std`-capable with explicit wasm32 tail-call gating
in its build.rs — nested execution is plausible. wasmtime-on-wasm32 exists
upstream only in tooling/custom-feature configurations (`custom-virtual-memory`,
`custom-native-signals`) and would need a CI probe to even claim. Cost and
COOP/COEP questions remain open per doc 06 §8; desktop-first is the recorded
posture.

**Open questions for 0.D3:**

1. **Purity vs primitives:** is wasmi's fuel-only CPU limiting acceptable as
   the sole mechanism (doc 06 §5.4 fallback), or is epoch interruption a hard
   requirement that justifies reopening the C exception policy for wasmtime?
2. **Nested-wasm:** do we commit the web build to a wasmi-only host (keeping
   one runtime everywhere), or accept desktop-wasmi/web-different-host and
   the second audit surface that implies? (Feeds the doc 06 §8 fence review.)
3. **Pin posture:** wasmi 2.0.0 is 7 weeks old — pin `>=2.0.0,<2.1` (record's
   `[PROPOSAL]`) or `<3` with re-audit triggers? Who owns the 2.0.1+ re-audit
   (allowlist-format.md re-audit triggers)?
4. **Bus factor:** wasmi's single-primary-maintainer risk (records/wasmi.md R8)
   vs wasmtime's Bytecode Alliance — how much weight does the Lead put on
   maintainer count vs purity?
5. **MSRV ripple:** wasmi needs Rust 1.86 (edition 2024). Confirm the Phase 1
   toolchain pin accommodates it; note wasmtime would need 1.96.

## 7. Sources (all fetched 2026-10-19, task 0.C4-R unless noted)

crates.io API (`https://crates.io/api/v1/crates/...`, UA `lumen-preaudit-0.C4-R`):
`wasmtime` (+`/49.0.3`, `/49.0.3/dependencies`, `/versions?per_page=100`),
`wasmi` (+ re-verify of the 0.C4-I record), `wasmer` (+`/7.5.0`,
`/7.5.0/dependencies`), `fizzy` (+`/0.8.0`, `/0.8.0/dependencies`),
`wasmtime-internal-fiber|‑cranelift|‑jit-debug|‑cache|‑unwinder` (each
`/49.0.3` + `/dependencies`), `pulley-interpreter/49.0.3/dependencies`,
`ittapi`, `ittapi-sys`, `zstd`, `zstd-safe`, `zstd-sys/1.3.2/dependencies`,
`cranelift-codegen`, `memfd`, `mach2`, `rustix`, `libc`, `gcc` (via closure
dep query). Crates.io search: `?q=wasm%20runtime`, `?q=wasm%20interpreter`,
`?q=fizzy`, `?q=orthanc` (0.C4-I, same day; re-checked).

Tarballs (`https://crates.io/api/v1/crates/<n>/<v>/download` →
`tmp/preaudit/tarballs/`, sha256 recorded in-text where cited): wasmtime
49.0.3, wasmer 7.5.0, wasmi 2.0.0 (0.C4-I), wasmtime-internal-fiber 49.0.3,
wasmtime-internal-jit-debug 49.0.3.

Sparse index (`https://index.crates.io/...`): `wa/sm/wasmtime` (49.0.3 cksum
match), `wa/sm/wasmer` (7.5.0 cksum match), `wa/sm/wasmi` (0.C4-I, 2.0.0
match), `zs/td/zstd-sys` (`links=zstd`), `ja/ck/jack-sys` (`links=jack`),
`al/sa/alsa-sys` (`links=alsa`), `pi/pe/pipewire-sys` (`links=pipewire-0.3`),
`as/io/asio-sys` (`links=asio`).

OSV (`https://api.osv.dev/v1/query`, POST): wasmtime (101 advisories; 0
ranges including 49.0.3), wasmi (2), wasmer (1 open), fizzy (0).

Inherited from 0.C4-I (same date): `docs/dependencies/records/wasmi.md`
(R1–R10), `audit-method.md` §4 worked example (all wasmi URLs and commands).
GitHub repository URLs cited above come from the crates.io API `repository`
field; `api.github.com` remains 403 from this sandbox (audit-method §7) —
star/commit-activity signals are `[VERIFY-P0]`.
