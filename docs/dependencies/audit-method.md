# Dependency Pre-Audit Method (reproducible)

| | |
|---|---|
| **Implements** | doc 11 §1–§4 (intake procedure F1–F7, audit record R1–R10, allowlist, purity method P1–P8), doc 04 §1.2 (tarball inspection origin), doc 13 §3 card 0.C4-I |
| **Purpose** | The exact, step-by-step procedure used for Phase 0 dependency **pre-audits** (no Cargo workspace exists yet), and the procedure Phase 1 tasks (1.E5) inherit once `Cargo.lock` exists. Codifies "never assume 'Rust API' means 'pure Rust'" (MP §9) into commands anyone can re-run. |
| **Status** | Executed 2026-10-19 in the Phase 0 sandbox (Debian x86_64, no cargo/rustc installed — see §7 limitations). All URLs cited were fetched on that date. |
| **Companion files** | `docs/adr/ADR-002-evidence.md` (WASM runtime candidates), `docs/dependencies/preaudit-foundation.md` (foundation crates), `docs/dependencies/records/wasmi.md` (first full record), `docs/dependencies/allowlist-format.md` (schema) |

---

## 1. Scope and class of audit

Two audit depths exist in this repo:

- **Pre-audit (Phase 0, this method):** no workspace, no lockfile. Evidence comes from
  the crates.io HTTP API, the sparse index, and **static tarball inspection**. Produces
  a verdict of `pass / conditional / fail / exception-ADR-required` *per candidate*,
  with every unverifiable claim marked `[VERIFY-P0]`.
- **Full audit (Phase 1, task 1.E5):** identical method plus `cargo metadata` /
  `cargo tree -e normal,build` at the *pinned lockfile version*, `cargo-deny`,
  `cargo-vet`, and the `tools/dependency-audit --deny-native` scanner (doc 11 §4.2).
  Produces the final R1–R10 record (doc 11 §2) and the allowlist row (doc 11 §3.1).

The pre-audit never executes, builds, or links the crate under audit. Inspection is
static (doc 11 P3: "Never build or execute the crate under audit").

## 2. Prerequisites

- `curl`, `tar`, `sha256sum`, `rg` (ripgrep), `python3` (or `jq`) for JSON.
- Scratch directory: `tmp/preaudit/` (gitignored; `tmp/` is in `.gitignore` — verified
  `git check-ignore tmp/preaudit` before use).
- A polite User-Agent for crates.io (their API guidelines request one):
  `UA="lumen-preaudit-0.C4-I (dependency audit)"`.
- No cargo/rustc is required for the pre-audit. (This sandbox has none; the
  cargo-based steps below are a runbook for Phase 1.)

## 3. Steps (M1–M9)

Throughout, `<name>` is the crate name and `<ver>` the exact version under audit.

### M1 — Crate metadata (crates.io API)

```bash
curl -sS -A "$UA" https://crates.io/api/v1/crates/<name>
```

Record: `max_stable_version` / `newest_version`, `description`, `repository`,
`documentation`, `created_at`, `updated_at`, total + recent downloads. The
`updated_at` field is the maintenance proxy (see M7). Cite the URL + fetch date.

### M2 — Version record (license, size, features, yanked)

```bash
curl -sS -A "$UA" https://crates.io/api/v1/crates/<name>/<ver>
```

Record: `license` (SPDX), `created_at` (release date), `crate_size` (bytes),
`yanked`, `features` (full map, incl. `default`). **Never invent a license** — this
API field is the source of truth for the pre-audit; the tarball file check (P7/M6)
corroborates.

### M3 — Declared dependencies

```bash
curl -sS -A "$UA" https://crates.io/api/v1/crates/<name>/<ver>/dependencies
```

Record every entry with `crate_id`, `req`, `kind` (`normal` / `build` / `dev`),
`optional`, `target` (cfg gate or null). Manually flag:

- any of `cc, cmake, bindgen, nasm, pkg-config, clang-sys, vcpkg` in **any** kind →
  F2/F5 signal (native toolchain in build graph);
- any `*-sys` crate → F3 signal (links= to a native library, F-check via its record);
- `[target.'cfg(...)'.dependencies]` entries → target-conditional native code (doc 11
  §4.3 blind spot "target-conditional build scripts" — the pre-audit records every
  cfg gate verbatim).

### M4 — Tarball download + checksum verification (P1–P3)

Primary URL (doc 11 P1):
`https://static.crates.io/crates/<name>/<ver>/<name>-<ver>.crate`

**Recorded environment fact (2026-10-19):** `static.crates.io` answered HTTP 403
`AccessDenied` (S3) for all three tarballs from this sandbox. Fallback, which worked
(HTTP 200, byte count equals the API's `crate_size`):

```bash
curl -sS -L -A "$UA" -o tmp/preaudit/tarballs/<name>-<ver>.crate \
  "https://crates.io/api/v1/crates/<name>/<ver>/download"
sha256sum tmp/preaudit/tarballs/<name>-<ver>.crate
```

Verify against the **sparse index** record (the checksum cargo itself enforces):

```bash
curl -sS "https://index.crates.io/<2>/<34>/<name>"   # path: see note
# grep the JSON line with "vers":"<ver>" and compare "cksum"
```

Index path layout: names of length ≤ 3 use `1/<c>` or `2/<ab>`; length 4 uses
`<ab>/<cd>/<name>`; length ≥ 5 uses first2/next2/rest, e.g. `wa/sm/wasmi`.
A `cksum` match + `yanked: false` is the pre-audit provenance check (R9). Then unpack
into `tmp/preaudit/src/` and inspect statically:

```bash
mkdir -p tmp/preaudit/src/<name>-<ver>
tar xzf tmp/preaudit/tarballs/<name>-<ver>.crate -C tmp/preaudit/src/<name>-<ver>
```

### M5 — Native-source scan (P4)

```bash
find tmp/preaudit/src/<name>-<ver>/<name>-<ver> -type f \
  \( -name '*.c' -o -name '*.cc' -o -name '*.cpp' -o -name '*.cxx' \
  -o -name '*.h' -o -name '*.hpp' -o -name '*.m' -o -name '*.mm' \
  -o -name '*.S' -o -name '*.s' -o -name '*.asm' -o -name '*.nas' -o -name '*.sx' \)
```

Any hit = F1 (or F7 for asm) unless covered by an E1/E2 exception. `.h` headers with
no `.c` (binding-header-only crates, e.g. wasmer's `third-party/wee8/wasm.h`) are
still recorded — F3 evidence.

### M6 — Build-script and manifest inspection (P5–P7)

```bash
# P5: tokens that mean "non-Rust toolchain" or "network fetch"
rg -n 'cc::|clang|gcc|cl\.exe|clang-cl|nasm|yasm|masm|cmake|ninja|xcrun|bindgen|pkg-config|curl|wget|git clone|ureq|reqwest|http' build.rs

# P6: links= key (native library by definition)
rg -n '^links\s*=' Cargo.toml Cargo.toml.orig

# P7: FFI and binary blobs
rg -n 'extern "C"|extern "system"|#\[link' src/
find . -type f \( -name '*.a' -o -name '*.lib' -o -name '*.o' -o -name '*.obj' \
  -o -name '*.so' -o -name '*.dll' -o -name '*.dylib' \)
```

Read `build.rs` **in full** when present. Token classes and their dispositions:

| Token class in build.rs | F-check | Disposition |
|---|---|---|
| `cc::Build`, `cmake`, `nasm`, `bindgen`, `pkg-config` invocation | F2 | FAIL unless feature-gated off *and* the gate is provable (doc 11 §2.4 rav1d pattern) |
| `curl`/`wget`/`ureq`/`reqwest`/`git clone` (build-time download) | F4 | FAIL (vendored/downloaded native code risk) |
| `Command::new(rustc)` or `RUSTC` env probe compiling a **Rust** probe file | — | Allowed; record explicitly (thiserror/serde pattern). The scanner will flag it; the human record clears it. |
| Pure `println!("cargo:rustc-cfg=…")` / OUT_DIR `.rs` generation | — | Allowed (benign; record anyway) |
| `#[cfg(target_os)]`-gated native compilation | F1/F2 | Record the cfg gate verbatim; must be re-scanned per-OS in CI (doc 11 §4.3) |

`extern "C"` hits are classified by **file location**: wasm32 JS-import shims
(`#[wasm_bindgen] extern "C"` — not native) vs OS-FFI (platform_impl, sys crates) vs
JIT/C-engine boundaries. The count alone is not a verdict; the classification is.

### M7 — Maintenance and security signals

- **Release cadence:** `curl -sS -A "$UA" https://crates.io/api/v1/crates/<name>/versions?per_page=100`
  → list stable (`^\d+\.\d+\.\d+$`, not yanked) `num` + `created_at`. Record the last
  5–10 stable releases and the date of the newest.
- **Advisories (RustSec/OSV):**
  `curl -sS -X POST -H "Content-Type: application/json" -d '{"package":{"name":"<name>","ecosystem":"crates.io"}}' https://api.osv.dev/v1/query`
  → record count, IDs, affected ranges, fixed versions, and whether the audited
  version is inside any *unfixed* range. (Plain `-d` without the Content-Type header
  returns HTTP 400 — recorded pitfall.)
- **Thresholds to record (doc 11 R8):** newest release date; release count in the
  trailing 12 months; open-unfixed advisories affecting `<ver>` (must be 0 to pass);
  maintainer count / bus factor (from `published_by` + repo `[VERIFY-P0: GitHub API
  returned 403 from this sandbox — check repo activity when network allows]`).

### M8 — Transitive graph (R2)

**Phase 1 (authoritative, once the workspace exists):**

```bash
cargo tree --workspace -e normal,build --prefix depth --charset ascii   # per doc 11 §3.6
cargo metadata --format-version 1 --locked | jq -r '.packages[].name'  # per-target
```

**Phase 0 approximation (what this pre-audit used):** recursive walk of the M3
endpoint (normal+build kinds, skipping target-gated and optional deps), resolving
each `req` to the newest matching stable version. Script and raw outputs:
`tmp/preaudit/walk.py`, `tmp/preaudit/api/walk-*.txt` (not committed; reproduced
below). Known approximations, all to be closed by the Phase 1 `cargo tree`:

1. version resolution picks *newest* in range, not the locked one;
2. target-gated deps are skipped (so per-OS native deps like `alsa-sys` must be
   recorded manually from M3 cfg gates);
3. optional deps are included only when a default feature provably enables them
   (verified by expanding the tarball's `[features]` table by hand);
4. feature-unification across our workspace is unknowable pre-workspace.

Every graph count from this method is labeled **approximate** and `[VERIFY-P0:
cargo tree at pinned lockfile]`.

### M9 — Record and verdict

Write the findings into a record (`docs/dependencies/records/<crate>.md`, R1–R10
per doc 11 §2 / `allowlist-format.md` template) and a verdict:

- **pass** — F1–F7 all NO, license allowlisted, maintenance acceptable;
- **conditional** — passes only with a pinned feature set / cfg (record the pin);
- **fail** — an F-check fails on any build configuration we would ship;
- **exception-ADR-required** — clean enough to use only under an E1/E2 ADR
  (OS webview, OS-service bindings).

Pre-audit verdicts do **not** create allowlist rows (doc 11 §3.1 requires the Lead
and the Phase 1 pipeline); they feed ADR-002 (runtimes) and Phase 1 intake.

---

## 4. Worked example — `wasmi` 2.0.0 (executed 2026-10-19)

All commands below were run on 2026-10-19; outputs are the actual outputs
(abbreviated where marked `...`). Scratch: `tmp/preaudit/`.

### M1 — metadata

```
$ curl -sS -A "$UA" https://crates.io/api/v1/crates/wasmi
  id: wasmi
  max_stable_version: 2.0.0        newest_version: 2.0.0
  description: "WebAssembly interpreter"
  repository: https://github.com/wasmi-labs/wasmi
  documentation: https://docs.rs/wasmi/
  created_at: 2018-01-26   updated_at: 2026-09-01T11:51:43Z
  downloads: 25,285,206    recent_downloads: 5,217,575
```

### M2 — version record

```
$ curl -sS -A "$UA" https://crates.io/api/v1/crates/wasmi/2.0.0
  license: "MIT/Apache-2.0"     created_at: 2026-09-01
  crate_size: 300979 bytes      yanked: false
  published_by: Robin Freyler (Robbepop)
  default = ["stable","std","wat","validate","memory64","auto-dispatch"]
  features: auto-dispatch, debug, deterministic, extra-checks, hash-collections,
            indirect-dispatch, libm, memory64, portable-dispatch,
            prefer-btree-collections, simd, stable, std, unstable, validate, wat
```

Tarball manifest adds: `rust-version = "1.86"`, `edition = "2024"`,
`categories = ["wasm","no-std","virtualization"]`.

### M3 — declared dependencies (all 8)

| crate | req | kind | optional | target |
|---|---|---|---|---|
| assert_matches | ^1.5 | dev | no | — |
| criterion | ^0.5 | dev | no | — |
| spin | ^0.9 | normal | no | — |
| wasmi_collections | ^2.0.0 | normal | no | — |
| wasmi_core | ^2.0.0 | normal | no | — |
| wasmi_ir | ^2.0.0 | normal | no | — |
| wasmparser | ^0.228.0 | normal | no | — |
| wat | ^1.228.0 | normal | **yes** | — |

No `cc`/`cmake`/`bindgen`/`*-sys` in any kind. No target-gated deps at all.
`wat` is optional and enabled by the default `wat` feature (default-feature
expansion from the tarball's `[features]` table).

### M4 — tarball + checksum

```
$ curl -sS -L -A "$UA" -o wasmi-2.0.0.crate \
    "https://crates.io/api/v1/crates/wasmi/2.0.0/download"
  HTTP 200, size_download = 300979          # equals API crate_size
$ sha256sum wasmi-2.0.0.crate
  78693fcdd618e0fc34af59c6b8efa9ac5d58c68df940beff4bedddb6acfe7c27
$ curl -sS https://index.crates.io/wa/sm/wasmi | grep '"vers":"2.0.0"'
  ... "vers":"2.0.0","cksum":"78693fcdd618e0fc34af59c6b8efa9ac5d58c68df940beff4bedddb6acfe7c27",
      "yanked":false, ...
  → cksum MATCH (P2 provenance pass)
```

(`https://static.crates.io/crates/wasmi/2.0.0/wasmi-2.0.0.crate` → HTTP 403
AccessDenied from this sandbox; the crates.io API endpoint above returned the
identical artifact per checksum.)

Tarball root: `Cargo.lock  Cargo.toml  Cargo.toml.orig  README.md  benches
build.rs  src  tests`. **No LICENSE file in the tarball** (R7 note: SPDX comes from
the manifest; the repo's LICENSE is `[VERIFY-P0: GitHub unreachable from sandbox]`).

### M5 — native-source scan (P4)

```
$ find ... \( -name '*.c' -o ... -name '*.sx' \)
  (no output)   → 0 native source files
```

### M6 — build script, manifest, FFI, blobs

- `build.rs` **present**. Full read: emits `cargo::rustc-check-cfg` /
  `cargo::rustc-cfg` for `wasmi_opt_size`, `wasmi_opt_speed`,
  `wasmi_has_tail_calls`, `wasmi_use_tail_calls`, keyed off `OPT_LEVEL`,
  `CARGO_CFG_TARGET_ARCH`, `CARGO_CFG_TARGET_FEATURE`. It explicitly handles
  `wasm32`/`wasm64` (tail-call proposal gate) — i.e., wasm targets are a supported,
  cfg-handled case, not an afterthought. **No compiler invocation, no network, no
  OUT_DIR binaries.**
- `links=` key: **absent** (both Cargo.toml and Cargo.toml.orig).
- `extern "C"` / `extern "system"` / `#[link]` in `src/`: **0 occurrences**.
- Binary blobs (`.a .lib .o .obj .so .dll .dylib`): **0 files**.

### M7 — maintenance & security

- Stable releases (crates.io versions endpoint): 2.0.0 (2026-09-01), 1.1.0
  (2026-06-12), 1.0.9 (2026-02-09), 1.0.8 (2026-01-29), 1.0.7 (2026-01-09), 1.0.6
  (2025-12-25), 1.0.5 (2025-12-19), 1.0.4 (2025-12-10) … — a steady cadence; **2.0.0
  is 7 weeks old at audit date** (fresh-major risk, R8 note).
- OSV/RustSec: **2 advisories total**, both closed and not affecting 2.0.0:
  GHSA-75jp-vq8x-h4cq (OOB write, host→wasm calls >128 params; fixed 0.31.1),
  GHSA-g4v2-cjqp-rfmq (use-after-free in linear memory; fixed 0.41.2).
- Publisher identity: Robin Freyler (Robbepop) — primary maintainer of
  wasmi-labs/wasmi; single-maintainer bus factor recorded (R8).
- GitHub repo activity: `[VERIFY-P0: api.github.com returned 403 from this sandbox]`.

### M8 — graph (approximate)

Default features (`stable,std,wat,validate,memory64,auto-dispatch`) → 15 crates:

```
bitflags 2.13.2, bumpalo 3.20.3, leb128fmt 0.1.0, libm 0.2.16, memchr 2.8.3,
spin 0.9.9, unicode-width 0.2.2, wasm-encoder 0.262.0, wasmi 2.0.0,
wasmi_collections 2.0.0, wasmi_core 2.0.0, wasmi_ir 2.0.0, wasmparser 0.228.0,
wast 262.0.0, wat 1.262.0
```

With `default-features = false` + `std` (the recommended prod pin for our host:
no `wat`, no `validate` — our own validator choice, see record) → 8 crates:

```
bitflags 2.13.2, libm 0.2.16, spin 0.9.9, wasmi 2.0.0, wasmi_collections 2.0.0,
wasmi_core 2.0.0, wasmi_ir 2.0.0, wasmparser 0.228.0
```

All 15 (or 8) are pure-Rust-named, no `cc`/`cmake`/`bindgen`/`libc`/`*-sys`
`[VERIFY-P0: full P1–P8 per transitive crate at Phase 1]`. Both counts carry the
§3-M8 approximations.

### M9 — verdict

**pass (pre-audit)** — F1–F7 all NO at 2.0.0; license MIT/Apache-2.0 (allowlist
class); sandboxing primitives present (fuel metering, `StoreLimitsBuilder`
memory/table/instance caps, `MemoryType` maximum, resumable calls, module-size
limits — API evidence in `records/wasmi.md` R6). Full R1–R10 record:
`docs/dependencies/records/wasmi.md`.

---

## 5. Sandbox-scanner script (the mechanical core of §3)

The exact script used for §4 (kept here because `tmp/` is not committed; Phase 1
moves it into `tools/dependency-audit`):

```bash
#!/bin/bash
# P1-P7 native-code scan of a crates.io tarball (usage: scan.sh name version)
n=$1; v=$2; f=tarballs/$n-$v.crate
UA="lumen-preaudit-0.C4-I (dependency audit)"
[ -s "$f" ] || curl -sS -L -A "$UA" -o "$f" "https://crates.io/api/v1/crates/$n/$v/download"
d=src/$n-$v; rm -rf "$d"; mkdir -p "$d"; tar xzf "$f" -C "$d"; root=$d/$n-$v
echo "sha256: $(sha256sum $f | cut -d' ' -f1)"
echo "native-src (P4): $(find $root -type f \( -name '*.c' -o -name '*.cc' -o -name '*.cpp' \
  -o -name '*.cxx' -o -name '*.h' -o -name '*.hpp' -o -name '*.m' -o -name '*.mm' \
  -o -name '*.S' -o -name '*.s' -o -name '*.asm' -o -name '*.nas' -o -name '*.sx' \) | tr '\n' ' ')"
echo "links= (P6): $(rg -l '^links\s*=' $root/Cargo.toml $root/Cargo.toml.orig 2>/dev/null | tr '\n' ' ')"
echo "blobs (P7): $(find $root -type f \( -name '*.a' -o -name '*.lib' -o -name '*.o' \
  -o -name '*.obj' -o -name '*.so' -o -name '*.dll' -o -name '*.dylib' \) | tr '\n' ' ')"
echo "build.rs present: $([ -f $root/build.rs ] && echo YES || echo no)"
[ -f $root/build.rs ] && echo "build.rs toolchain tokens: $(rg -o 'cc::|clang|gcc|cl\.exe|nasm|yasm|masm|cmake|ninja|xcrun|bindgen|pkg-config|curl|wget|git clone|ureq|reqwest' $root/build.rs | sort | uniq -c | tr '\n' ';')"
echo "extern \"C\"/system/#[link] in src: $(rg -c 'extern \"C\"|extern \"system\"|#\[link' $root/src 2>/dev/null | awk -F: '{s+=$2} END {print s+0}')"
```

Token hits are **signals for human classification** (§3 M6 table), not automatic
verdicts — e.g. `wgpu`'s 97 `extern "C"` are all wasm-bindgen JS shims; `serde`'s
and `thiserror`'s build.rs `download`-token hits were comment false-positives
(verified by full reads).

## 6. What the pre-audit does NOT prove

- It cannot see the *resolved* feature set after workspace unification (Phase 1
  `cargo tree` + `cargo metadata` close this).
- It cannot run `cargo-deny`/`cargo-vet` (no workspace). License conflicts across
  the transitive graph are therefore `[VERIFY-P0]` beyond the audited crate's own
  manifest.
- It does not execute the crate; sandbox *behavior* (doc 06 §5.4 limit-breach tests)
  is Phase 1+ conformance work, not a pre-audit field.
- Static inspection cannot prove absence of malicious compile-time behavior
  (doc 11 §4.3 residual truth — the allowlist + Lead approval chain remains the
  compensating control).

## 7. Environment record (2026-10-19)

- Sandbox: Debian 13 (trixie) x86_64 container, uid 1001, no root.
- `cargo`/`rustc`: **not installed** (M8 cargo steps are runbook-only here).
- Network: crates.io API + index.crates.io + api.osv.dev reachable;
  `static.crates.io` → 403 AccessDenied (M4 fallback documented);
  `api.github.com` → 403 (GitHub signals `[VERIFY-P0]`).
- Downloads stayed in `tmp/preaudit/` (gitignored, `git check-ignore` verified);
  nothing binary is committed; no crate under audit was built or executed.
