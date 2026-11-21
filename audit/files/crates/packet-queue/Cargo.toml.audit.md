# AUDIT: crates/packet-queue/Cargo.toml
| Field | Value |
|---|---|
| Auditor | W01-A08 |
| Date | 2026-11-21 |
| Lines / functions | 31 / 4 config blocks ([package], [lints], [dependencies], [dev-dependencies]) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: apps/cli/Cargo.toml:58 `playback-engine` → crates/playback-engine/Cargo.toml:39 `packet-queue` — the `lumen` open path configures its queues via `queue_plan_from_product_limits`, apps/cli/src/demux_host.rs:971-972) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `ls crates/packet-queue/src/` + `wc -l` (1,396 LOC, 4 modules); `rg -n "Mutex\|Condvar" src/queue.rs`; `rg -n "Must"` (testkit usage); `rg -l packet-queue --glob Cargo.toml` (dependents); python3 Cargo.lock parse; sed -n quotes of defaults. |

## 1. One-paragraph verdict
Zero findings — and this is a verified zero, not a lazy one. Every header claim is
provably true: `media-types` is the only runtime dependency (manifest line 28, lock
block lists media-types + testkit-dev); concurrency is literally `std::sync::{Condvar,
Mutex}` (queue.rs:14) with the state guarded by `Mutex<State<T>>` and two Condvars
(queue.rs:228-230) — no channels/lock-free crates; the testkit dev-dependency is used
for the claimed `Must` helper (stamped.rs:118, limits.rs:337, lib.rs:80 doctests);
the "bounded ... generation stamping/stale-drop" claim maps to real typed bounds
(limits.rs: NonZeroUsize caps, unbounded inexpressible) and stamped.rs. The crate sits
on the shipped `lumen` open path.

## 2. What actually works (verified)
- Reachability: apps/cli/Cargo.toml:58 → playback-engine/Cargo.toml:39 → packet-queue
  (rg over manifests); the CLI open path pushes queue caps through it
  (apps/cli/src/demux_host.rs:971-972 `PRODUCT_QUEUE_PACKETS_AUDIO/VIDEO`).
- Claim "media-types is the ONLY dependency": manifest [dependencies] has exactly one
  entry; Cargo.lock block: `packet-queue 0.1.0` → [media-types, testkit] (testkit is
  the declared dev-dep; lock including dev-deps of workspace members is normal).
- Claim "std::sync Mutex + Condvar": queue.rs:14 `use std::sync::{Condvar, Mutex,
  PoisonError};` — exact match, no other sync primitives crate.
- Claim "testkit ... for the lint-legal `Must` helper in tests and doctests":
  stamped.rs:118 `use testkit::Must;`, limits.rs:337 `use testkit::{Must, assert_err};`,
  lib.rs:80 doctest `use testkit::Must;`.
- Bounded-by-construction: limits.rs:9-13 doc + typed constructor (NonZeroUsize,
  positive Duration) — "an unbounded configuration cannot be expressed".
- Crate's own doc'd defaults are const-evaluated (limits.rs:40 `AUDIO_DEFAULT_PACKETS
  = cap(200)`, :44 `VIDEO_DEFAULT_PACKETS = cap(300)`) with a compile-time-fail `cap()`
  helper — corroborates that the CLI's 1024/512 product caps are an override injected
  per-open, not duplicated inside this crate (clean layering; the duplication lives in
  apps/cli mirror constants — see product.toml audit F-004).

## 3. Findings
None. Checklist coverage for the zero: (a) no undeclared/unused deps — the single
runtime dep is used (`media_types::time::{Duration, Nanoseconds}` import at
limits.rs:10); (b) manifest/lock consistent (python3 parse); (c) no version
duplication possible (path-only deps); (d) no feature flags declared; (e) no stale
comments found — header, description, and registry row ("layer L2, deps media-types")
all match reality; (f) publish=false correct; (g) lints inherited (workspace = true,
baseline #5 clippy -D warnings clean).

## 4. Per-function table (config keys)
| Key | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| header comments 1-15 | 1-15 | REAL | — | every claim verified (see §2) |
| [package] name/version/edition/publish | 17-22 | REAL | — | "packet-queue" 0.1.0, edition 2024, publish=false; lock consistent |
| [package] description | 21 | REAL | — | "backpressure, wholesale stale-generation drops and watermark signals" — all three exist (limits.rs watermarks, stamped.rs stale-drop, queue.rs push-refusal) |
| [lints] workspace = true | 24-25 | REAL | — | inherits; crate code has zero unwrap/expect per its own doc and clippy-clean baseline |
| [dependencies] media-types (path) | 28 | REAL | — | sole runtime dep, used |
| [dev-dependencies] testkit (path) | 30-31 | REAL | — | used in 3 files + doctests |

## 5. Test honesty
No test files in the manifest itself; in-crate tests live in the src files
(stamped.rs/limits.rs test modules using testkit::Must — would fail on real assertion
failure, `Must` is testkit's non-swallowing assert helper) and ran in baseline #3
(1681 passed workspace-wide). Nothing suspicious.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "media-types is the ONLY dependency" | Cargo.toml:9 | REAL | manifest + lock |
| "Concurrency is std::sync Mutex + Condvar ... no channels crate, no lock-free crate" | Cargo.toml:9-12 | REAL | queue.rs:14,228-230 |
| "testkit (dev-only crate) for the lint-legal Must helper" | Cargo.toml:14-15 | REAL | stamped.rs:118, limits.rs:337 |
| "bounded packet/frame queues ... generation stamping/stale-drop" | Cargo.toml:4-7 | REAL | limits.rs typed caps; stamped.rs; queue.rs |

## 7. Dead / duplicated / unreachable code
None in this manifest. (The 200/300 library defaults vs 1024/512 product caps are an
intentional override seam, not duplication — the product.toml mirror-constant issue
is an apps/cli defect, recorded in product.toml.audit.md F-004.)

## 8. Dependencies & flags
media-types (path, runtime), testkit (path, dev). No versions to fragment (path deps).
No feature flags. Std-only claim holds — no external crates at all.

## 9. Open questions for the next agent
- limits.rs:40-46: subtitle cap 128 cited to "card 3.B3's small and duration-only
  figures" — whoever audits doc 03 §4.4 should confirm the doc's numbers are
  200/300/128 (I verified the code side only).
- Does plugin-host-wasm's use of packet-queue (second dependent) enforce the same
  typed bounds, or does it bypass with raw VecDeque? (Out of scope here.)
