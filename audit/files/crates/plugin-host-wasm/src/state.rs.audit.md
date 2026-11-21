# AUDIT: crates/plugin-host-wasm/src/state.rs
| Field | Value |
|---|---|
| Auditor | W12-A03 |
| Date | 2026-11-21 |
| Lines / functions | 441 / 22 (2 structs+impls services, 2 security types+3 methods, 1 expectation struct, 2 handle-table types + 5 methods, 1 log type + 2 methods, 1 panel type, 1 state struct + 2 impls + 4 methods, 4 tests + 2 helpers) |
| File verdict | UNREACHABLE (real, tested host state — but the crate has no shipped consumer; see below) |
| Reachable from shipped entry point? | NO (same as imports.rs#part2: `plugin-host-wasm` has zero dependents outside its own tests/bins — apps/cli, apps/desktop, apps/web all depend only on `plugin-api`; CLI/web plugin commands are typed refusals). Executed by tests: call chain `cargo test` → integration suites → `WasmInstance::spawn` → `InstanceState::new` (instance.rs) → every method here. Baseline: `orchrun-test-workspace-nofailfast.log.txt:4495-4544` — 44 unit tests ok incl. all 4 in this file's `mod tests` (:4531-4534). |
| Findings | S0:0 S1:0 S2:1 S3:0 S4:3 |
| Commands run | Read state.rs 1-441 (full) · Grep `_path` in crates/plugin-host-wasm (written :153, never read — dead field) · Read imports.rs 410-917 (all consumers of HandleTable/panels/ledger/io_bucket/http_admit — cross-checked every method's call sites) · `rg -n "fs_open\|fs_close\|extract" crates/plugin-host-wasm/src/bin/lumenext-examples.rs` (example plugin opens+reads+closes PER extract — line 370 doc row, 413-444 code) · limits.rs consts (MAX_OPEN_HANDLES=8 :78, HTTP_REQUESTS_PER_MIN=32 :75) · `sed -n '653,760p' docs/06-plugin-architecture.md` (§5.3-§5.6 contracts) · baseline orchrun cites. Quotes verified with the Read tool. |

## 1. One-paragraph verdict
The state layer is small, disciplined, and mostly correct: bounded everything (256-event ledger, 64-record log, 1024-entry import trace, capped handle table, minute-window HTTP admission), deliberate poison-recovery on the ledger mutex, shape-only security events with a test asserting secrets never reach Debug output. But it contains one genuine behavioral defect with a concrete victim: the handle table's cap counts LIFETIME opens, not concurrent handles — `insert` refuses when `files.len() >= cap` where `len` includes permanently-closed slots, and no slot is ever reused — so a plugin that opens and closes one file per dispatched call (exactly the repo's own metadata-provider example: `extract` = fs_open→fs_read→fs_close per invocation, lumenext-examples.rs:370,413-444) is refused `E_LIMIT` on its 9th call despite holding zero handles. Doc 06 §5.4 row 7 ("Open handles | 8 per instance | handle-table cap") promises concurrent-open semantics. The unit test in this very file closes handles but never re-inserts, so the defect ships green. The rest is hygiene: a dead `_path` field, O(n) `Vec::remove(0)` eviction in the two structures the file's own `BoundedLog` (a VecDeque) does right, and an untestable wall-clock HTTP window.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4531-4534` → `state::tests::{handle_table_caps_and_ownership, http_window_caps_at_limit, ledger_counts_escalations, bounded_log_keeps_last_64}` all `ok` (within the 44-test unit run :4541-4543).
— Verified by Read of every consumer (imports.rs 410-917): `handles.read/size/close/insert` are the fs import's only table surface; `http_admit` gates http_get before the transport; `trace_import` records every import status; `log_record` backs log_write; `panels` backs ui_register_panel — no method here is dead (except the `_path` field, F-002).
— Verified secret-hygiene by test: `ledger_counts_escalations` asserts `!format!("{events:?}").contains("musicbrainz")` (state.rs:411) and `Debug for HostServices` reports only `device_inventory_bytes` (29-37) — shapes and lengths only, per doc 06 §5.3.

## 3. Findings
### F-001 [S2]
Where: crates/plugin-host-wasm/src/state.rs:147-158 (with 183-190)
Code:
```
    pub fn insert(&mut self, path: String, content: Vec<u8>, cap: u32) -> Result<u32, String> {
        let len = self.files.len() as u32;
        if len >= cap {
            return Err(format!("handle table full ({len}/{cap})"));
        }
        self.files.push(Some(OpenFile {
            _path: path,
            content,
            pos: 0,
        }));
        Ok(len)
    }
```
What is wrong: The cap is enforced against `files.len()` — the number of slots EVER allocated, not the number currently open. `close` (183-190) only `take()`s a slot to `None`; `insert` never scans for a free slot and always pushes. Consequences: (a) after 8 total opens the table refuses forever (`E_LIMIT` from fs_open, imports.rs:524-538), even if all 8 were closed; (b) `open_count()` (:193-195) exists and would report the truth but is not what the gate reads. Doc 06 §5.4 row 7: "Open handles | 8 per instance | handle-table cap" — concurrent semantics. The repo's own committed example plugin defeats it: `extract` performs fs_open→fs_read→fs_close per dispatched call (lumenext-examples.rs:370 doc row "…`fs_close`; returns the last import status", code 413-444), so a long-lived metadata instance serving a library scan fails its 9th extraction with `E_LIMIT` while holding zero handles.
Why it matters: This is the normal usage pattern of the one extension kind (metadata-provider) the crate ships an example for; the limit table is the crate's security contract and its row 7 is enforced with the wrong semantics. The file's own test (`handle_table_caps_and_ownership`, 359-376) closes `a` at :374 and never attempts another insert — the defect is untested and invisible.
What correct looks like: Gate on `open_count() >= cap` and reuse a `None` slot (first-fit scan — 8 slots, trivially cheap), or back the table with `VecDeque`/`Option` slot recycling; add the missing test: insert→close→insert must succeed under the cap.
Confidence: HIGH — Verification: read (insert/close/open_count + all callers); example plugin usage pattern read; doc row read; test gap read.
### F-002 [S4]
Where: crates/plugin-host-wasm/src/state.rs:130-134
Code:
```
struct OpenFile {
    _path: String,
    content: Vec<u8>,
    pos: usize,
}
```
What is wrong: `_path` is stored on every insert (line 153) and read by nothing — Grep over the whole crate finds only the declaration and the initializer. An underscore-silenced dead field holding a potentially long path string per handle (up to 8 × 4096 bytes) with zero consumers.
Why it matters: Dead state that looks like diagnostics but is unreachable; the leading underscore suppresses the compiler's own warning rather than answering it.
What correct looks like: Delete it, or expose it (`pub fn path(&self, h: u32)`) for the install-UI/diagnostics story the doc's crash-report posture implies.
Confidence: HIGH — Verification: read + repo-crate-wide grep (2 hits: decl + init).
### F-003 [S4]
Where: crates/plugin-host-wasm/src/state.rs:91-97 (and 311-314)
Code:
```
        let mut events = self.events.lock().unwrap_or_else(|p| p.into_inner());
        // Bounded log (D-2): the most recent 256 events.
        if events.len() >= 256 {
            events.remove(0);
        }
```
(imports side twin: `trace_import` 311-314 `self.import_trace.remove(0)`)
What is wrong: Both eviction sites use `Vec::remove(0)` — an O(n) shift of every remaining element (here: up to 255 `SecurityEvent`s, each three `String`s — move-churn per recorded event once full). The same file's `BoundedLog` (206-211) uses `VecDeque` + `pop_front`, which is the correct structure for this exact pattern. During an escalation storm — the precise scenario the ledger exists to absorb (the sandbox suite's trap-loop/OOB-probe tests) — every additional event pays the full shift.
Why it matters: Inconsistent data-structure discipline inside one file; a security-path pessimization that the file itself demonstrates the fix for 30 lines later. Bounded (256/1024), so not S3.
What correct looks like: `VecDeque<SecurityEvent>` with `pop_front`, exactly like `BoundedLog`.
Confidence: HIGH — Verification: read (all three structures in one file).
### F-004 [S4]
Where: crates/plugin-host-wasm/src/state.rs:319-333
Code:
```
    pub fn http_admit(&mut self) -> bool {
        let now = std::time::Instant::now();
        while let Some(front) = self.http_window.front() {
            if now.duration_since(*front).as_secs() >= 60 {
                self.http_window.pop_front();
```
What is wrong: The rolling-minute expiry half of `http_admit` (prune entries older than 60 s) is untestable as written — it reads the wall clock directly with no injection seam, and the file's test (`http_window_caps_at_limit`, 378-384) pins only the cap refusal (2-of-2 under `test_tiny`), never that admissions resume after the window passes. A guest that burns its 32/min quota waits a minute — or forever, if the pruning arithmetic is wrong; nothing distinguishes.
Why it matters: Untested time logic is where off-by-one unit bugs (secs vs ms, `>` vs `>=`) live; the seam (`MonotonicSource`-style injection, as media-clock does) is the repo's own established pattern.
What correct looks like: Take a `now: Instant` parameter (or an injectable clock), and add the test: fill the window, advance past 60 s, assert admission resumes.
Confidence: HIGH — Verification: read; test coverage read (no time-based test exists in the crate for this).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `HostServices` (struct+Default) | 18-27 | REAL | — | Honest empty seams (jail None ⇒ every open denied; NoTransport ⇒ E_TRANSPORT); consumed by imports.rs (jail :431, transport :768, inventory :897). |
| `impl Debug for HostServices` | 29-37 | REAL | — | Lengths only, no content — secret hygiene, verified by test pattern. |
| `HostServices::empty` | 39-45 | REAL | — | Trivial delegate to Default; used by conformance/tests. |
| `SecurityEvent` + `new` | 50-69 | REAL | — | Shape-only fields (import/reason/arg_shape); doc 06 §5.3 conformant. |
| `SecurityLedger` (struct) | 75-79 | REAL | — | Arc-shared, outlives faulted instances (per doc 06 §5.3 session semantics). |
| `record_escalation` | 90-98 | REAL | S4(F-003) | Poison-recovering lock (deliberate); bounded 256; returns post-count for threshold compare — used by imports deny/read_guest; O(n) eviction (F-003). |
| `escalation_count` | 100-103 | REAL | — | SeqCst load; used by conformance assert (test :313). |
| `events` | 105-111 | REAL | — | Snapshot clone (bounded 256); install-UI/tests consumer. |
| `ManifestExpectation` | 116-126 | REAL | — | Compared field-by-field in report_manifest (imports.rs:310-321); string-equality on version is the strict (safe) posture. |
| `OpenFile` | 130-134 | REAL | S4(F-002) | `_path` dead; content+pos drive read/size. |
| `HandleTable::insert` | 147-158 | REAL | **S2(F-001)** | Lifetime-opens cap + no slot reuse — see F-001. |
| `HandleTable::read` | 118→163-171 | REAL | — | Advances pos, returns owned copy (double-copy per fs_read — acceptable sandbox cost, off-realtime per doc 06 §5.2); bogus/closed ⇒ None ⇒ E_HANDLE. Note: consume-before-deliver is the F-001 data-loss partner in imports.rs#part2. |
| `HandleTable::size` | 174-179 | REAL | — | Content-length u64; bogus ⇒ None. |
| `HandleTable::close` | 183-190 | REAL | — | Ownership check + double-close typed; take() leaves the slot dead (F-001's other half). |
| `HandleTable::open_count` | 193-195 | REAL | — | Counts Some slots; used by Debug — should have been the insert gate (F-001). |
| `BoundedLog::push` | 206-211 | REAL | — | Correct VecDeque ring, 64 cap. Private; wrapped by log_record. |
| `BoundedLog::records` | 216-218 | REAL | — | Bounded clone; test pins oldest-first + count. |
| `PanelRecord` | 224-228 | REAL | — | Verbatim descriptor string (validation gap belongs to imports.rs#part2 F-entry; honest v1 seam comment :221-223). |
| `InstanceState` (struct) | 232-262 | REAL | — | 15 fields, every one consumed (verified by imports/instance readers); store_limits wired to wasmi (memory cap + grow-trap). |
| `impl Debug for InstanceState` | 264-273 | REAL | — | finish_non_exhaustive; counts not contents. |
| `InstanceState::new` | 277-302 | REAL | — | 6 args, `#[allow(too_many_arguments)]` explicit; TokenBucket from limits; http_allowlist from grants. |
| `log_record` | 305-307 | REAL | — | Delegate (level pre-validated in imports :373-378). |
| `trace_import` | 310-315 | REAL | S4(F-003) | Bounded 1024; Vec::remove(0) eviction twin. |
| `http_admit` | 319-333 | REAL | S4(F-004) | Cap half tested; expiry half wall-clock-bound and untested. |
| tests: `handle_table_caps_and_ownership` | 359-376 | REAL | — | Good pins (cap, bogus handles, read-advance, double-close) — but closes without re-inserting, missing F-001. Ran ok (orchrun:4532). |
| tests: `http_window_caps_at_limit` | 378-384 | REAL | — | Pins cap=2 refusal under test_tiny; no expiry test. Ran ok (:4533). |
| tests: `ledger_counts_escalations` | 386-412 | REAL | — | Counts, shapes, secret-absence in Debug. Ran ok (:4531). |
| tests: `bounded_log_keeps_last_64` | 414-424 | REAL | — | Exact boundary (line 36 first, 99 last). Ran ok (:4534). |
| helpers `ok`/`some` | 426-440 | REAL | — | Local must-idioms; unreachable!-based (loud failure) — correct posture. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| handle_table_caps_and_ownership | PARTIALLY | Caps/ownership/read-advance/double-close: YES (mutation of any breaks an assert). Reuse-after-close: NO — the sequence insert×2→fail→close is never followed by an insert, so F-001 ships green. |
| http_window_caps_at_limit | PARTIALLY | Cap refusal: YES. Rolling-window expiry: NO — no test advances time past 60 s (F-004). |
| ledger_counts_escalations | YES | Post-count return values, event shapes, and Debug secret-absence are all exact-asserted. |
| bounded_log_keeps_last_64 | YES | Boundary indices exact (records[0]="line 36", [63]="line 99"). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Open handles \| 8 per instance \| handle-table cap" | docs/06 §5.4 row 7 | FALSE (as implemented) | The cap gates lifetime allocations, not open handles; closed slots never reused (F-001); `open_count` exists but is not the gate. |
| "A permission escalation attempt is logged as a security event (id, import, arguments' shapes — never raw buffers) and counted" | docs/06 §5.3 | REAL | SecurityEvent{import, reason, arg_shape}; ledger count; secret-absence asserted (state.rs:411). |
| "bounded local log … the last 64 records survive" | docs/06 §5.6 | REAL | BoundedLog 64-cap VecDeque; test green (orchrun:4534). |
| "HTTP requests 32/min per instance" | docs/06 §5.4 row 6 | REAL (cap half) / UNVERIFIED (expiry half) | http_admit wired in http_get; cap pinned by test at tiny limits; 60 s prune never executed under test (F-004). |
| PanelRecord "v1: recorded host-side; the command-layer rendering protocol is doc 05/08 territory" | state.rs:221-223 (own comment) | REAL (honest) | No rendering exists; only instance.panels() + tests consume it. |
| Session-shared ledger "outlives any single (faulted, dropped) instance" | state.rs:71-74 (own comment) | REAL | Arc<SecurityLedger> passed per-spawn; fault paths drop the instance, not the Arc (instance.rs wiring). |

## 7. Dead / duplicated / unreachable code
- `_path` field: dead (F-002).
- Whole crate UNREACHABLE from shipped entry points (header) — test/bin-reachable only.
- No duplication inside the file beyond the structurally-contrasting eviction styles (F-003 is the duplication-of-pattern-done-two-ways finding).

## 8. Dependencies & flags
- `wasmi` (StoreLimits, StoreLimitsBuilder in tests) — host-side only; `std::time::Instant` used freely (native crate; correct — this crate never compiles to wasm32).
- `plugin_api::PluginKind` — single import, used by ManifestExpectation.
- Internal: fsjail::PathJail, http::HttpTransport, limits::{TokenBucket, ResourceLimits} — all consumed as seams, none instantiated here beyond wiring.
- No feature flags; no unsafe; clippy allows exactly one (too_many_arguments, justified).

## 9. Open questions for the next agent
- instance.rs owner: does anything reset or drain InstanceState between dispatched calls (a per-call reset would defuse F-001's realistic case)? My read of instance.rs:95-119 says no — `spawn` is the only constructor, the `Store` (with its `InstanceState`/HandleTable) lives in `WasmInstance` for the instance's lifetime, and no method drains `handles` — but you own the final proof.
- supervisor.rs owner: when a faulted instance restarts, confirm the new spawn gets a fresh InstanceState (fresh HandleTable) — if the supervisor reuses state, F-001 gets worse (lifetime cap across restarts).
- limits.rs owner: MAX_OPEN_HANDLES=8 default — with F-001, the practical per-instance file budget is 8 EVER; carry the doc-row-7 drift from your side too.
