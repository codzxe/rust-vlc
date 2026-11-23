# AUDIT: crates/renderer-wgpu/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W06-A03 (recovery: executed by W06-R1, 2026-11-23) |
| Date | 2026-11-23 |
| Lines / items | 41 / 1 package + 1 lints + 2 dependencies + 2 dev-dependencies, 0 features |
| File verdict | REAL — an honestly-documented std-only manifest whose every evidence citation resolves; the one defect is the crate NAME promises wgpu and the crate contains none |
| Reachable from shipped entry point? | YES — apps/cli/Cargo.toml:60 `renderer-wgpu = { path = ... }`; linked and constructed in the shipped binary at apps/cli/src/pipeline_host.rs:1314 (`Renderer::new_headless`, imports :79-83) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | rg (lockfile node, deny.toml, boundaries.toml, doc 01 §2.6 row, worklog 2.D1, record existence, CLI wiring); `test -f` wgpu record -> EXISTS; sed quote verification |

## 1. One-paragraph verdict
The most heavily-documented 41-line manifest in the tree, and every citation
checks out: the registry row (doc 01:244 `renderer-wgpu | crate | L4 |
media-types, video-engine`) matches the regular deps exactly; the wgpu
deferral record `docs/dependencies/records/wgpu-30.0.1.md` EXISTS; the
worklog 2.D1 continuation entry (archive-2026-10.md:1452+) independently
confirms every factual claim the header makes — the live
`Instance::request_adapter` probe ("vulkan drivers/libraries could not be
loaded … gl drivers/libraries could not be loaded"), the 80-package
default-features graph pulling libloading v0.8.9 via ash AND dlib, the
deny.toml ban (verified in-tree: the only allowlisted wrapper is
libappindicator-sys, which no wgpu path uses), the Compositor-seam decision
with the software compositor linked, and the "every wgpu integration point
typed + shape-pinned" claim. The dev-deps block is a model of its kind:
testkit + media-clock, both dev-only, with the ARCH-7 no-leak argument and
the L4→L2 direction written out, and boundaries.toml:262 records exactly
`dev_deps = ["testkit", "media-clock"]`. The single defect is the crate
name: "renderer-wgpu" contains zero wgpu code (negotiate.rs audit, W06-A05)
— a misleading name on metadata surfaces that do not render the header,
though the header itself discloses the deferral in its first 15 lines.

## 2. What actually works (verified)
— verified by `test -f docs/dependencies/records/wgpu-30.0.1.md` -> EXISTS;
rg of Cargo.lock:739-746 (deps: media-types, video-engine + dev
media-clock, testkit — nothing else); rg deny.toml:177-187 (libloading
banned, scoped libappindicator-sys wrapper only); rg doc 01:244 (registry
row match); rg worklog archive-2026-10.md 2.D1 entry (probe + graph +
decision verbatim); rg boundaries.toml:262 (dev_deps recorded).

## 3. Findings

F-001 [S3]
Where: crates/renderer-wgpu/Cargo.toml:18 and :1
Code:
```toml
[package]
name = "renderer-wgpu"
```
and
```toml
# crates/renderer-wgpu — GPU video presentation (card 2.D1,
```
What is wrong: the crate is named (and headlined) "renderer-wgpu / GPU
video presentation" while containing ZERO wgpu dependency or code — the
graph is media-types + video-engine only (:27-29, lockfile-confirmed), and
the shipped renderer is the software compositor (negotiate.rs audit:
hardware tier hard-coded Unavailable; CLI constructs it at
pipeline_host.rs:1314). Anyone reading a dependency listing, Cargo.lock, or
`cargo tree` output — the surfaces that show only names — believes wgpu is
in the shipped binary. Mitigation, and why this is S3 not S2: the same
file's header (:5-15) fully discloses the deferral with a resolvable
evidence trail, the Lead decision is recorded in both the worklog and
docs/dependencies/records/wgpu-30.0.1.md, and `publish = false` keeps the
name off public registries. The name is the doc-13 card's mandated path
(card 2.D1 owns `crates/renderer-wgpu/**`), so the fix is a truthful
description/name annotation at the plan level, not this manifest.
Why it matters: matrix rows or downstream audits citing "renderer-wgpu" as
GPU-rendering evidence would be FALSE (already flagged from the
presenter_seam side by W06-A05).
What correct looks like: either rename to `renderer` when the import is
ratified, or keep the name and ensure every matrix row citing it names the
software compositor (the deferral record's conditional-E2 verdict).
Confidence: HIGH — Verification: read + rg (lockfile, negotiate.rs audit,
CLI wiring).

## 4. Per-function table (config keys)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header comment block | 1-15 | REAL | - | every claim resolves (record, worklog, deny.toml, probe) |
| `[package]` name | 18 | MISLEADING | S3 | F-001 |
| `[package]` description | 21 | REAL | - | accurate for the software compositor (no GPU claim in the text) |
| `[lints]` workspace | 24-25 | REAL | - | root lints exist |
| `[dependencies]` | 27-29 | REAL | - | exactly the doc 01:244 registry row |
| `[dev-dependencies]` testkit + media-clock | 31-41 | REAL | - | ARCH-7 argument in-file; boundaries.toml:262 matches |

## 5. Test honesty
N/A (manifest; the dev-deps it declares are exercised by the crate's
63-test suite — 53 lib + 5 png_hash + 5 presenter_seam — executed green in
prior audits W06-A05/W06-A10 and the worklog's triple-deterministic run).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "doc 01 §2.6 registry row: layer L4, deps media-types + video-engine" | :2-3 | REAL | doc 01:244 exact match |
| "wgpu import is a recorded Lead decision (worklog 2.D1; records/wgpu-30.0.1.md)" | :5-7 | REAL | record EXISTS; worklog :1452+ records decision + evidence |
| "verified by a live request_adapter probe: every backend un-loadable" | :8-9 | REAL | worklog quotes the probe's error text verbatim |
| "default-features Linux graph pulls libloading (via ash/Vulkan and dlib/GL), which deny.toml bans" | :9-11 | REAL | worklog: libloading v0.8.9 via ash AND dlib; deny.toml:177-187 ban (wrapper exception does not cover wgpu paths) |
| "every wgpu integration point typed + shape-tested" | :14 | REAL | worklog 2.D1 + negotiate.rs/backend.rs audits |
| "A dev edge cannot leak into the public API (ARCH-7); L4 → L2 is a legal downward direction. Recorded in boundaries.toml dev_deps" | :39-40 | REAL | boundaries.toml:262 dev_deps = ["testkit", "media-clock"] |

## 7. Dead / duplicated / unreachable code
None. (The crate's unreachable-from-ship negotiate() surface was flagged in
the negotiate.rs audit; the manifest itself is clean.)

## 8. Dependencies & flags
media-types + video-engine (path, production); testkit + media-clock
(path, dev-only). No features, no build script, no wgpu. Std-only by C5.

## 9. Open questions for the next agent
- If 2.D3/ADR-003 ever ratifies the wgpu import, this manifest gains the
  first external dep — the C5 record chain and deny.toml wrapper-scope
  will both need re-verification at that point (the deferral record's §6
  already lists the obligations).
