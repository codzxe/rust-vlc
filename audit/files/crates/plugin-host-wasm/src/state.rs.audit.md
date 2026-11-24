# AUDIT: crates/plugin-host-wasm/src/state.rs
| Field | Value |
|---|---|
| Auditor | W12-A07 |
| Date | 2026-11-14 |
| Lines / functions | 441 / 27 (HostServices x3, SecurityEvent::new, SecurityLedger x4, HandleTable x6, BoundedLog x3, InstanceState x5, 2 Debug impls, 4 tests + 2 helpers + state()) |
| File verdict | UNREACHABLE (real, unit-tested code; no shipped entry point reaches it — Phase 7, `wasm-extensions = false`) — with one latent S2 the tests cannot see |
| Reachable from shipped entry point? | NO. `rg plugin apps/cli/Cargo.toml apps/desktop/Cargo.toml` -> 0 hits (neither lumen-cli nor desktop depends on any plugin crate); product.toml:166 `wasm-extensions = false # Phase 7`. Only in-crate consumers: imports.rs:524-529 (fs_open), instance.rs (InstanceState::new), tests, and the `lumenext` dev binary (packaging only, never executes guests). |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 3 |
| Commands run | Read of state.rs (full) + imports.rs:500-545/620-680 + limits.rs grep (MAX_OPEN_HANDLES=8); rg dependents over all Cargo.tomls; baseline citation cargo-test-workspace.txt:4931-4934. |

## 1. One-paragraph verdict
Solid sandbox bookkeeping (bounded logs, bounded escalation ledger, sliding-window HTTP admit, typed handle refusals) with one real latent defect: `HandleTable::insert` charges the 8-handle cap against EVERY slot ever pushed — closed slots are never reused and the cap check counts `files.len()`, not `open_count()` — so a guest that opens and closes files sequentially (the normal metadata-provider pattern) is refused with `E_LIMIT` on its 9th lifetime open even with zero handles open, contradicting the doc's "8 open handles" concurrency semantics. The file's own test closes a handle but never inserts again afterwards, so the bug is invisible to the suite. Everything here is currently dead weight in the shipped product: no app depends on the crate, and the product feature that would light it up is off (Phase 7).

## 2. What actually works (verified)
— 4/4 state.rs tests pass (baseline cargo-test-workspace.txt:4931-4934: `bounded_log_keeps_last_64`, `handle_table_caps_and_ownership`, `http_window_caps_at_limit`, `ledger_counts_escalations`).
— `http_admit`'s sliding window logic is correct by inspection (purge ≥60 s, then cap-check, then push — state.rs:319-333); the cap path is test-pinned, the purge path is not (F-005).
— `SecurityLedger` correctly shares across instances (`Arc`), survives poisoned mutexes (`unwrap_or_else(|p| p.into_inner())`), and its Debug shape drops raw buffers/paths (test asserts no "musicbrainz" leak, state.rs:411).

## 3. Findings
F-001 [S2]
Where: crates/plugin-host-wasm/src/state.rs:147-158 vs 136 and 183-196; caller imports.rs:524-535
Code:
```
    pub fn insert(&mut self, path: String, content: Vec<u8>, cap: u32) -> Result<u32, String> {
        let len = self.files.len() as u32;
        if len >= cap {
            return Err(format!("handle table full ({len}/{cap})"));
        }
        self.files.push(Some(OpenFile {
```
and the doc it contradicts (state.rs:136):
```
/// Per-instance handle table (doc 06 §5.4 row 7: 8 open handles).
```
What is wrong: `insert` refuses when `files.len() >= cap`, but `files` is an append-only Vec whose `None` slots (closed handles, `close` at 183-190 only `take()`s) are never reclaimed and never reused. The cap therefore bounds LIFETIME opens, not OPEN handles: with `MAX_OPEN_HANDLES = 8` (limits.rs:78) a guest that does open→read→close nine times — the ordinary sequential pattern of a metadata provider tagging a directory — gets `E_LIMIT` on the 9th open while `open_count()` is 0. `open_count()` (193-195) exists, counts exactly the doc's semantics, and is used only by the Debug impl. Why it matters: silent misbehavior of the sandbox's most basic resource limit the moment Phase 7 ships; guests will be killed for a limit they are not actually violating. What correct looks like: cap on `open_count()`, and slot reuse of the first `None` index (stable handle identity for already-issued indices is unaffected — closed indices must stay poisoned). Confidence: HIGH — Verification: read (logic) + the test's own coverage gap (F-002); runtime execution UNVERIFIED (crate reached only by its own suite; no run here — cargo budget).

F-002 [S3]
Where: crates/plugin-host-wasm/src/state.rs:359-376
Code:
```
        // Double close refused.
        assert!(st.handles.close(a).is_ok());
        assert!(st.handles.close(a).is_err());
    }
```
What is wrong: the test ends on the double-close assertion — it never inserts after a close, so the exhaustion defect F-001 (cap counts closed slots) cannot fail this test. A one-line `assert!(st.handles.insert("d".into(), vec![], 2).is_ok())` after the closes would turn F-001 into a red test. Why it matters: the suite pins the cap's existence, not its semantics — the test name promises "caps and ownership" but the cap's actual unit (lifetime vs concurrent) is unpinned. Confidence: HIGH — Verification: read.

F-003 [S4]
Where: crates/plugin-host-wasm/src/state.rs:130-134
Code:
```
struct OpenFile {
    _path: String,
    content: Vec<u8>,
    pos: usize,
}
```
What is wrong: `_path` is stored and never read (underscore-named dead field). The path is passed in by `fs_open` (imports.rs:528 `display`) for diagnostics that never happen — every escalation/diagnostic message that would name the file has no access to it. Confidence: HIGH — Verification: read (rg `_path` -> only this site).

F-004 [S4]
Where: crates/plugin-host-wasm/src/state.rs:310-315
Code:
```
    pub fn trace_import(&mut self, import: &str, status: i32) {
        if self.import_trace.len() >= 1024 {
            self.import_trace.remove(0);
        }
        self.import_trace.push((import.to_string(), status));
    }
```
What is wrong: `remove(0)` is an O(1024) shift on EVERY import call once full, plus a fresh `String` allocation per import (this runs in the host hook of every guest import — the hot path of the interpreter loop). Should be a `VecDeque` like `BoundedLog`. Same pattern in `SecurityLedger::record_escalation` (state.rs:93-94, `events.remove(0)` over 256) but escalations are rare, so only the trace path is hot. Confidence: HIGH — Verification: read.

F-005 [S4]
Where: crates/plugin-host-wasm/src/state.rs:319-333
Code:
```
        let now = std::time::Instant::now();
        while let Some(front) = self.http_window.front() {
            if now.duration_since(*front).as_secs() >= 60 {
```
What is wrong: the 60-second purge branch has no test and no clock seam — `http_window_caps_at_limit` only exercises the fresh-window cap. If the comparison were `>` vs `>=` or the window 59 s, no test would notice. Minor given the guest is denied-not-killed either way. Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| HostServices (struct+Default) | 18-27 | REAL | — | honest empty seams |
| HostServices::Debug / empty | 29-45 | REAL | — | |
| SecurityEvent::new | 60-69 | REAL | — | shapes only, no raw buffers |
| SecurityLedger::new | 82-85 | REAL | — | Arc-shared |
| SecurityLedger::record_escalation | 90-98 | REAL | S4 | bounded 256, remove(0) shift (rare path) |
| SecurityLedger::escalation_count / events | 100-111 | REAL | — | |
| ManifestExpectation | 116-126 | REAL | — | consumed by F1 check |
| OpenFile | 130-134 | REAL | S4 | `_path` dead (F-003) |
| HandleTable::insert | 147-158 | REAL (defective) | S2 | F-001 lifetime cap |
| HandleTable::read | 163-171 | REAL | — | bounds-safe; fresh Vec copy per read (slurp design) |
| HandleTable::size | 174-179 | REAL | — | |
| HandleTable::close | 183-190 | REAL | — | typed double-close/unknown |
| HandleTable::open_count | 193-195 | REAL | — | used only by Debug — should be the cap source (F-001) |
| BoundedLog::push / records | 206-218 | REAL | — | 64-entry ring, tested |
| InstanceState::new | 277-302 | REAL | — | wires limits/token bucket/grants |
| InstanceState::log_record / trace_import | 305-315 | REAL | S4 | F-004 |
| InstanceState::http_admit | 319-333 | REAL | S4 | correct; purge untested (F-005) |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| handle_table_caps_and_ownership | PARTIAL | pins cap-exists, bogus-handle refusals, read advance, double-close — but NOT cap-after-close (F-002) |
| http_window_caps_at_limit | YES (cap path) / NO (purge path) | fresh-window only |
| ledger_counts_escalations | YES | counts + shapes + no-secret assert |
| bounded_log_keeps_last_64 | YES | exact 64-window assert |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "8 open handles" (doc 06 §5.4 row 7) | state.rs:136 | FALSE as implemented | F-001: enforced as 8 lifetime opens per instance |
| "v1 defaults are honest empty seams: no jail roots, no transport" | state.rs:15-17 | REAL | HostServices::default is all-None/empty; typed denials in imports.rs |
| "Bounded log (D-2): the most recent 256 events" | state.rs:92 | REAL | record_escalation code |
| "the last 64 records survive" | state.rs:198-199 | REAL | tested |

## 7. Dead / duplicated / unreachable code
- The entire file is unreachable from shipped entry points (Phase 7 off; no app depends on the crate).
- `OpenFile::_path` dead field. `open_count()` used only in Debug.

## 8. Dependencies & flags
wasmi (StoreLimits), plugin-api (PluginKind), in-crate jail/http/limits. No feature gates in-file. Crate membership: workspace `crates/*` (compiles in every lane) but no shipped consumer.

## 9. Open questions for the next agent
- imports.rs auditor: `fs_open` swallows the insert diagnostic (`let _ = detail;` imports.rs:537) — E_LIMIT reaches the guest with no host-side log of WHICH file/limit; combined with F-001 the guest cannot distinguish lifetime-exhaustion from real concurrency. Also confirm `fs_read`'s transfer cap and the TOCTOU-sweep comment (500-511) against the actual jail checks in fsjail.rs.
- When Phase 7 flips (`wasm-extensions = true`), F-001 becomes a live S2 on every sequential-file plugin — the flip should carry the insert-fix + the one-line test.
