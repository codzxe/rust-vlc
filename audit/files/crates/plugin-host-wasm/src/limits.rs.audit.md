# AUDIT: crates/plugin-host-wasm/src/limits.rs
| Field | Value |
|---|---|
| Auditor | W12-A09 |
| Date | 2026-11-21 |
| Lines / functions | 237 / 23 (13 config consts + 7 fns/methods + 3 tests) |
| File verdict | UNREACHABLE (reached inside the crate by `instance.rs`/`imports.rs`/`state.rs`, which are themselves reached only by the crate's tests — no shipped entry point links this crate) |
| Reachable from shipped entry point? | NO (crate-level; see install.rs audit header for the caller map). In-crate reachability verified: `memory_bytes`/`startup_fuel`/`call_fuel` → instance.rs:82,108,113,186,258; `io_transfer_bytes`/`max_descriptor_bytes`/`max_open_handles` → imports.rs:286,287,379,421,509,524,571,705,752,828,829,878; `http_requests_per_min` → state.rs:328; `ESCALATION_DISABLE_THRESHOLD` → imports.rs:161,209,347; `TokenBucket` → state.rs:236,288. `HOST_MAX_ACTIVE_INSTANCES` and `HOST_TOTAL_HEAP_BYTES` → NOBODY. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 1 |
| Commands run | Grep per-const/per-field usage map (above); Grep `HostCapacity` (error.rs definition+Display only — never constructed); Grep `max_wasm_extension_memory_mib` (product-config parse/merge/schema + docs; no sandbox consumer); Read doc 06 §5.4 (684-702) and product.toml:195-210; sed -n verification of quotes. |

## 1. One-paragraph verdict
The per-instance half of this file is real and wired: every `ResourceLimits` field has an enforcing consumer (store cap + fuel in instance.rs, transfer/descriptor/handle caps in imports.rs, per-minute HTTP window in state.rs), the numbers match doc 06 §5.4 rows 1–7 exactly, and the `TokenBucket` is correctly implemented (u128 refill math, accumulate-until-a-token time accounting, over-capacity takes can never succeed). The host-level half is decoration: `HOST_MAX_ACTIVE_INSTANCES = 4` and `HOST_TOTAL_HEAP_BYTES = 256 MiB` — doc 06 §5.4 rows 8–9, "Enforcement: host scheduler / host accounting" — are declared here and enforced nowhere; `HostError::HostCapacity` ("too many active instances or the total extension-heap budget is exhausted") is defined, given an actionable Display string, and never constructed by any code in the repository. Doc 06 §5.4's opening claim "Limits live in `product.toml` (schema-validated, MP §5)" is also false: they live as consts here, and product.toml's own `limits.max_wasm_extension_memory_mib = 64` key is parsed by the product-config crate but never read by the sandbox it names.

## 2. What actually works (verified)
— Code-traced (no baseline run reaches this crate; compilation proven at `build-release-workspace.log.txt:231`): all nine `ResourceLimits` fields consumed (map in header); doc-row value parity for rows 1–7 (32/64 MiB memory classes, 100 ms/500 ms as fuel 3e8/1.5e9 with the calibration note, 16 MiB transfer, 1 MiB/s + 16 MiB burst, 32/min, 8 handles, 64 KiB descriptor); `TokenBucket::refill` accumulates elapsed time only when a whole token is gained (sub-token remainders are not lost — `last` is not reset when `gained == 0`), and `take` can never grant more than `capacity`.
— The in-file tests are mutation-sensitive (see §5).

## 3. Findings

### F-001 [S2]
Where: crates/plugin-host-wasm/src/limits.rs:1-2, 85-89
Code:
```rust
//! Resource limits (doc 06 §5.4) and the host-level accounting
//! ceilings (§5.4 last two rows).
```
and
```rust
/// 4 active instances per host (doc 06 §5.4 row 8).
pub const HOST_MAX_ACTIVE_INSTANCES: usize = 4;

/// 256 MiB total extension heap across instances (doc 06 §5.4 row 9).
pub const HOST_TOTAL_HEAP_BYTES: u64 = 256 * 1024 * 1024;
```
What is wrong: neither constant has a single consumer. Grep over the crate: `HOST_MAX_ACTIVE_INSTANCES` and `HOST_TOTAL_HEAP_BYTES` appear only at their definitions. `WasmInstance::spawn` (instance.rs:56-120) performs no global accounting — nothing counts active instances, nothing sums extension heaps, and `HostError::HostCapacity` (error.rs:118-124, "too many active instances or the total extension-heap budget is exhausted") is never constructed anywhere (grep: definition + Display arm only). Doc 06 §5.4 rows 8–9 promise "Enforcement: host scheduler / host accounting"; the file header repeats "the host-level accounting ceilings (§5.4 last two rows)".
Why it matters: the containment story has a documented ceiling that does not exist. An embedding app that spawns extensions per media item gets unlimited concurrent instances and unbounded aggregate extension memory — precisely the runaway the row exists to prevent; and the typed error reserved for it is dead code. Because the whole crate is unwired today the blast radius is zero in production, but the module presents these rows as part of its enforced surface, which is drift between a security table and the code (S2; would be S1 if anything shipped).
What correct looks like: an instance registry in the host layer (spawn checks `active < HOST_MAX_ACTIVE_INSTANCES` and `tracked_heap + limits.memory_bytes <= HOST_TOTAL_HEAP_BYTES`, returning `HostCapacity`), or delete the constants and the doc rows until the scheduler card lands.
Confidence: HIGH — Verification: read + exhaustive grep (both consts, HostCapacity).

### F-002 [S3]
Where: docs/06-plugin-architecture.md:686 vs crates/plugin-host-wasm/src/limits.rs:4-6 and product.toml:208
Code (doc claim):
```markdown
Limits live in `product.toml` (schema-validated, MP §5) with these defaults:
```
(doc 06 §5.4, line 686) vs (code):
```rust
//! Every number here is `[PROPOSAL — doc 06 §5.4 marks the whole
//! table proposed]`; they are constants in one place so a Lead tune
//! is a one-file diff.
```
and product.toml:208:
```toml
max_wasm_extension_memory_mib = 64  # Phase 7 sandbox bound. schema 8..=256
```
What is wrong: the limits are Rust consts, not product.toml config. The one product.toml key that names a sandbox bound (`limits.max_wasm_extension_memory_mib`) is parsed and merged by the product-config crate (`crates/product-config/src/model.rs:181`, `merge.rs:450-451`, schema range 8..=256 in product.schema.json:202) and then consumed by nothing — `ResourceLimits` never reads it; a Lead changing the key changes no behavior.
Why it matters: config-declared-but-never-read is the checklist's own named defect class; the doc's MP §5 citation makes the config-driven claim a design-level assertion the code contradicts. The code's own honesty ("constants … a one-file diff") makes the doc the false side.
What correct looks like: either wire `max_wasm_extension_memory_mib` into `ResourceLimits::for_kind` when the host is embedded, or mark the key + doc row `[PROPOSAL: Phase 7]` the way `wasm-extensions = false` (product.toml:166) honestly is.
Confidence: HIGH — Verification: grep consumer map; product-config parse path read.

### F-003 [S4]
Where: docs/06-plugin-architecture.md:695,697 vs crates/plugin-host-wasm/src/limits.rs:74-78
Code (doc):
```markdown
| HTTP requests | 32/min per instance, 8 concurrent | host fetcher |
| Concurrent instances | 4 active per host, 16 installed-and-loaded per app | host scheduler |
```
What is wrong: the "8 concurrent" HTTP clause and the "16 installed-and-loaded per app" clause have no code counterpart at all (no concurrency limiter — the dispatch model is single-threaded synchronous; no installed-and-loaded cap — the store's `installed()` lists everything with no bound).
Why it matters: half-rows in a limits table that presents itself as enforced; readers budget against numbers that do not exist.
What correct looks like: drop the clauses or implement them with the row-8/9 registry from F-001.
Confidence: HIGH — Verification: read + grep (no concurrency/installed-cap symbols anywhere in the crate).

## 4. Per-function table
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| const `MEM_DEFAULT_BYTES` (32 MiB) | 46-49 | REAL | — | Consumed by `for_kind`; matches doc row 1. |
| const `MEM_PANEL_BYTES` (64 MiB) | 50-51 | REAL | — | Consumed by `for_kind`; matches row 1 (panel+filters). |
| const `CALL_FUEL` (3e8) | 53-59 | REAL | — | → instance.rs:186; calibration reasoning documented; tests never depend on wall-clock (deterministic fuel). |
| const `STARTUP_FUEL` (1.5e9) | 61-63 | REAL | — | → instance.rs:108. |
| const `IO_TRANSFER_BYTES` (16 MiB) | 65-66 | REAL | — | → imports.rs (11 sites). |
| const `IO_RATE_BYTES_PER_SEC` (1 MiB/s) | 68-69 | REAL | — | → state.rs:288 (bucket rate). |
| const `IO_BURST_BYTES` (16 MiB) | 71-72 | REAL | — | → state.rs:288 (bucket capacity). |
| const `HTTP_REQUESTS_PER_MIN` (32) | 74-75 | REAL | — | → state.rs:328 rolling window. "8 concurrent" half unimplemented (F-003). |
| const `MAX_OPEN_HANDLES` (8) | 77-78 | REAL | — | → imports.rs:524 handle cap. |
| const `MAX_DESCRIPTOR_BYTES` (64 KiB) | 80-83 | REAL | — | → imports.rs:286,828. |
| const `HOST_MAX_ACTIVE_INSTANCES` (4) | 85-86 | DEAD | S2 | F-001 — zero consumers. |
| const `HOST_TOTAL_HEAP_BYTES` (256 MiB) | 88-89 | DEAD | S2 | F-001 — zero consumers. |
| const `ESCALATION_DISABLE_THRESHOLD` (3) | 91-93 | REAL | — | → imports.rs:161,209,347; matches doc 06 §5.3. |
| `ResourceLimits::for_kind` | 102-120 | REAL | — | Memory class by kind (UiPanel/AudioFilter/VideoFilter → 64 MiB; else 32 MiB); all other fields kind-independent; values match doc rows 1–7. |
| `ResourceLimits::test_tiny` | 125-137 | REAL | — | Documented deterministic test seam; `pub` in the production API without `#[cfg(test)]` (repo's testkit convention — noted, not counted). |
| `TokenBucket::new` | 153-160 | REAL | — | Full bucket at start. |
| `TokenBucket::take` | 164-172 | REAL | — | Refill-then-compare; `n > capacity` can never pass; `take(0)` trivially true (no caller). |
| `TokenBucket::fits_capacity` | 176-178 | REAL | — | One-shot transfer bound. |
| `TokenBucket::refill` | 180-190 | REAL | — | u128 math (no overflow on long stalls); `last` advances only when ≥1 token gained — sub-token time accumulates correctly; capped at capacity. |
| test `memory_class_follows_doc06_row1` | 197-215 | REAL | — | Pins all four class assignments — mutation-sensitive. |
| test `bucket_burst_then_throttle` | 217-226 | REAL | — | Pins burst-drain + no-refill-yet + capacity bounds. |
| test `bucket_refills_over_time` | 228-236 | REAL | — | Sleep 6 ms @1000/s → ≥6 tokens, asserts 5 — margin is safe (sleep guarantees at-least); mildly wall-clock dependent by design. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `memory_class_follows_doc06_row1` | YES | Four exact class assertions; flipping the `for_kind` match arms fails it. |
| `bucket_burst_then_throttle` | YES | `!take(1)` after burst and `!fits_capacity(11)` pin the throttle/capacity semantics. |
| `bucket_refills_over_time` | YES (weakly) | Catches a refill that never happens; would NOT catch a refill that double-counts time (no exact-rate assertion). |
| Baseline evidence | NO RUN | Crate's tests absent from both baseline workspace logs. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Limits live in `product.toml` (schema-validated, MP §5) with these defaults" | doc 06 §5.4:686 | FALSE | F-002: consts in limits.rs; the product.toml key unconsumed. |
| Rows 1–7 defaults + enforcement | doc 06 §5.4:690-696 | REAL (code-traced) | Value parity + consumer map (header row). |
| "Concurrent instances: 4 active per host … Enforcement: host scheduler" | doc 06 §5.4:697 | FALSE | F-001: no scheduler, no accounting, `HostCapacity` never constructed. |
| "Total extension heap: 256 MiB … Enforcement: host accounting" | doc 06 §5.4:698 | FALSE | F-001. |
| "never a hang, never an OOM of the engine process (MP §12 'bound memory')" | doc 06 §5.4:700-702 | FALSE (at the crate boundary) | instance.rs F-001 (my S1): uncapped host-side `read_guest_memory` allocation from guest-controlled length. |
| feature-matrix `plugin-wasm-extensions` all-platforms not_started | docs/feature-matrix.toml:2321-2331 | TRUE (honest) | product.toml:166 `wasm-extensions = false` gate; no embedder. |

## 7. Dead / duplicated / unreachable code
- `HOST_MAX_ACTIVE_INSTANCES`, `HOST_TOTAL_HEAP_BYTES` — dead (F-001).
- `HostError::HostCapacity` (error.rs) — dead variant cross-ref (error.rs is another agent's file).
- No duplication found; numbers are single-sourced here.

## 8. Dependencies & flags
- Depends only on `plugin_api::PluginKind` — no external deps, no features, no unsafe, no panics (all integer math saturating or u128). `[lints] workspace = true`.

## 9. Open questions for the next agent
- supervisor.rs auditor: does the crash/backoff policy also lack a concurrent-instance cap (i.e., can restart policy loop-spawn instances — making F-001's missing registry actively harmful rather than latent)?
- error.rs auditor: `HostCapacity` dead variant — same finding from your side; keep the cross-ref.
- Whoever embeds this crate first: F-001 is the first thing to build (the registry), or delete the doc rows.
