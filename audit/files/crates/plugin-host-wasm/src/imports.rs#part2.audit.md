# AUDIT: crates/plugin-host-wasm/src/imports.rs#part2 (lines 410-917)
| Field | Value |
|---|---|
| Auditor | W12-A03 |
| Date | 2026-11-21 |
| Chunk boundary | part2 = lines 410-917 (`define_fs` 410-682, `define_http` 686-813, `define_ui` 817-861, `define_devices` 865-917 — four of the six import-module definers, running to EOF). part1 = lines 1-409 (module docs, FaultKind/HostFault, guest-memory helpers read_guest/write_guest*, deny, build_linker, check_import_policy, define_host, define_log). Boundary at the `define_fs` start — declared in audit/runs/W12-A03.log.md before either part was audited. |
| Lines / functions | 508 / 8 (4 definer fns + 4 wrapped import closures treated as the unit of analysis: fs_open, fs_read, fs_size, fs_close, http_get, ui_register_panel, devices_list — 7 imports across 4 definers) |
| File verdict | UNREACHABLE (real, security-hardened, test-executed import implementations — but NO shipped entry point reaches this crate; see Reachability) |
| Reachable from shipped entry point? | NO. `plugin-host-wasm` has ZERO dependents: grep of every Cargo.toml for `plugin-host-wasm` matches only crates/plugin-api/Cargo.toml:9 (a comment) and the crate's own manifest. apps/cli, apps/desktop, apps/web all depend on `plugin-api` (the trait crate) only; the CLI's plugin surface is a typed refusal ("the plugin registry is a Phase-6 card", apps/cli/src/dispatch.rs:201-202; apps/web/src/host.rs:1852-1853). The crate's only consumers are its own tests and its own bins (`lumenext`, `lumenext-examples`). Call chain that DOES execute it: tests → `WasmInstance::spawn` (instance.rs:110 `imports::build_linker`) → these definers. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 4 |
| Commands run | Read imports.rs 1-918 (full file; part1 read for seam context, audited boundaries respected) · `rg -n "build_linker\|check_import_policy" crates/plugin-host-wasm/src` (callers: instance.rs:80,110 only) · Grep `plugin-host-wasm` in apps/**/Cargo.toml + crates/**/Cargo.toml (no dependents) · Grep `plugin` in apps/cli + apps/web (typed refusals at dispatch.rs:201, host.rs:1852; plugin-api traits only) · Read state.rs (full — HandleTable::read semantics, PanelRecord, InstanceState) · `rg -n "fs_open\|fs_read\|http_get\|ui_register_panel\|devices_list\|fs_size" crates/plugin-host-wasm/tests` (coverage map: plugin_sandbox_security.rs:68-119,260-286,688,748; plugin_examples_conformance.rs:145-267,337-353) · Read plugin_examples_conformance.rs 130-400 (end-to-end metadata/panel lanes) · `sed -n '653,760p' docs/06-plugin-architecture.md` (§5.2-§5.4 contracts: no-host-pointers, limits table incl. "Total extension heap 256 MiB — host accounting") · Grep `HOST_TOTAL_HEAP_BYTES` (defined limits.rs:89, ZERO uses) · `sed -n '20,95p' crates/plugin-host-wasm/src/http.rs` + limits consts (IO_TRANSFER_BYTES=16 MiB, MAX_OPEN_HANDLES=8) · `sed -n '2291,2345p' docs/feature-matrix.toml` (plugin-wasm-extensions = not_started all platforms) · baseline cites below. Quotes verified with the Read tool. |

## 1. One-paragraph verdict
This is the best-engineered sandbox code I have audited in this repo: every import validates arguments, enforces per-kind caps (path 4096, url 2048, descriptors 64 KiB, transfers 16 MiB), charges the shared token bucket, returns typed statuses instead of trapping, and traces every call; the fs lane re-verifies the jail after open (TOCTOU narrowing), meters the slurp at cap+1 bytes, and the whole surface is exercised end-to-end by real wasmi guest modules in the committed integration suites — the metadata-provider conformance run pushes real sidecar bytes through fs_open/fs_read/fs_close over a real jail, and the sandbox suite pins slurp bounds, handle smuggling, OOB-probe escalation, and devices bucket metering, all green in the baseline no-fail-fast run. The defects are a data-loss edge in fs_read (handle bytes are consumed before the guest output buffer is validated — a failed write permanently drops the chunk, unlike POSIX EFAULT semantics), an unenforced doc-06 promise (the "256 MiB total extension heap — host accounting" row has no accounting code; the constant is dead), and four hygiene-grade items (bucket overcharging on requested-vs-delivered capacity, a trace/return status mismatch in one http_get denial path, silent truncation without a total-length signal, swallowed resolve-error detail). The verdict that matters to the wave question: none of this ships — no app depends on the crate, the CLI answers plugin commands with a typed "Phase-6 card" refusal, and the feature matrix honestly says not_started.

## 2. What actually works (verified)
— Verified by `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:4495-4620`: `plugin_host_wasm` 44 unit tests ok; integration suites green — `plugin_examples_conformance` 5/5 (incl. `metadata_provider_example_passes_conformance`, `ui_panel_example_passes_conformance`, `committed_example_modules_match_regeneration`), `plugin_sandbox_security` 9/9 (incl. `fs_open_slurp_is_bounded`, `devices_list_is_bucket_metered`, `oob_probes_trip_the_disable_threshold`), `plugin_hello_world` 2/2, `plugin_install_lifecycle` 4/4, `plugin_crash_isolation` 6/6.
— Verified by Read of plugin_examples_conformance.rs:189-309: the metadata-provider example executes `fs_open`→`fs_read`→`fs_close` over a real `PathJail` with a real sidecar file; the test asserts the sidecar's bytes crossed back verbatim (`String::from_utf8_lossy(&tags) == sidecar`, line 243-249) and the import trace shows all three fs imports Ok (253-267) — the wasmi host DOES execute a plugin end to end (in tests; not in any shipped binary).
— Verified by Read of the sandbox suite's wasm builders (plugin_sandbox_security.rs:66-125): fs_size/fs_close bogus-handle refusals, read-then-size-then-close sequencing, handle-smuggling and traversal paths are all pinned through real guest modules.

## 3. Findings
### F-001 [S3]
Where: crates/plugin-host-wasm/src/imports.rs:585-612 (with state.rs:163-171)
Code:
```
                    let chunk = match caller
                        .data_mut()
                        .handles
                        .read(handle as u32, buf_cap as usize)
                    {
                        Some(chunk) => chunk,
```
(then, at 599-600: `match write_guest(&mut caller, buf_ptr, &chunk)`)
What is wrong: `HandleTable::read` advances `file.pos` (state.rs:169 `file.pos += n;`) and returns an owned Vec BEFORE `write_guest` validates the guest's `buf_ptr`. If the write fails (negative ptr → E_ARG, OOB → E_OOB), the closure returns the error status but the chunk is already consumed — the bytes are permanently lost. A guest that retries with a corrected pointer receives the NEXT chunk; the skipped bytes are unrecoverable through the handle.
Why it matters: Every other error path in this file is non-destructive (typed refusal, state unchanged). This one silently corrupts the file view for exactly the caller class the sandbox exists to tolerate — buggy third-party plugins. POSIX `read` with EFAULT does not consume; the doc 06 §5.2 "request/response" model implies call failure leaves the handle where it was.
What correct looks like: Validate `buf_ptr` (and the write length) BEFORE consuming: peek `remaining = handles.remaining(handle)`, compute `n`, `write_guest` the slice from the table, and only then advance `pos` (or add a `read_at`/peek API to HandleTable).
Confidence: HIGH — Verification: read both sides of the seam; no test covers write-failure-after-read (sandbox suite pins bogus HANDLE and bogus ptr on read_guest's input side only).
### F-002 [S3]
Where: crates/plugin-host-wasm/src/imports.rs:503-523 (with limits.rs:88-89, docs/06-plugin-architecture.md §5.4 row 9)
Code:
```
                // Bounded slurp (sweep 2/4): at most `cap + 1` bytes are
                // ever read from the opened handle, so a file that GREW
                // after open cannot drag the host heap past the cap —
```
What is wrong: The per-transfer cap is enforced (16 MiB), but the aggregate promise is not: doc 06 §5.4 row 9 says "Total extension heap | 256 MiB across instances | host accounting", and `limits.rs:89` defines `HOST_TOTAL_HEAP_BYTES: u64 = 256 * 1024 * 1024` — which NOTHING reads (rg over the crate: the definition is the only hit). With MAX_OPEN_HANDLES=8 and io_transfer_bytes=16 MiB, ONE instance can park 128 MiB of slurped file content in host-side `HandleTable` slots; two busy instances blow the documented 256 MiB aggregate with no code path aware of it. (Store memory caps govern the GUEST's linear memory, not these host-side Vec buffers.)
Why it matters: The limits table is the crate's security posture; row 9 is advertised as enforced ("host accounting") and is fiction. Mitigating reality: nothing embeds this crate today (UNREACHABLE), so no shipped blast radius exists — the drift is the finding.
What correct looks like: An `Arc<AtomicU64>` host-heap counter charged at insert (fs_open) and discharged at close/fault, checked against `HOST_TOTAL_HEAP_BYTES` (which would then not be dead).
Confidence: HIGH — Verification: read + rg (constant has zero uses); limits defaults read (IO_TRANSFER_BYTES, MAX_OPEN_HANDLES).
### F-003 [S4]
Where: crates/plugin-host-wasm/src/imports.rs:578-584 (fs_read; same pattern 759-764 http_get, 891-896 devices_list)
Code:
```
                    // Charge the rate bucket before reading.
                    if !caller.data_mut().io_bucket.take(buf_cap as u64) {
```
What is wrong: The bucket is charged the REQUESTED capacity, not the delivered byte count — including when the handle is at EOF and the chunk is empty, or when the transport returns a short body. A guest polling a drained handle in 64 KiB chunks is drained of quota at 16 MiB per 256 polls without a single byte moving.
Why it matters: Spurious E_QUOTA for well-behaved guests; the meter overstates I/O by the request/delivery ratio. Conservative direction (never under-charges), so not a security hole.
What correct looks like: Charge after the size is known (`take(n as u64)` post-read, pre-delivery), or refund the difference.
Confidence: HIGH — Verification: read; TokenBucket semantics read in state.rs wiring (io_bucket at InstanceState::new).
### F-004 [S4]
Where: crates/plugin-host-wasm/src/imports.rs:719-725
Code:
```
                    Err(HostError::PermissionDenied { detail }) => {
                        let shape = format!("url_len={}", url.len());
                        caller
                            .data_mut()
                            .trace_import(import, GuestStatus::EArg.code());
                        return deny(&mut caller, import, detail, shape);
```
What is wrong: The trace records `E_ARG` but `deny` returns the `E_PERM` status (imports.rs:214) — the guest observes E_PERM while `import_trace` (the conformance/diagnostics record) says E_ARG for the same call. Every other denial path in the file traces the status it returns.
Why it matters: `import_trace` is the kit's conformance oracle (the tests at plugin_examples_conformance.rs:253-267 pattern-match on it); a trace that disagrees with the returned status makes conformance assertions lie for this path.
What correct looks like: `trace_import(import, GuestStatus::EPerm.code())` before the deny (or move the trace into deny's callers consistently).
Confidence: HIGH — Verification: read both code paths (deny at 199-215).
### F-005 [S4]
Where: crates/plugin-host-wasm/src/imports.rs:778-794 (http_get); same shape 897-900 (devices_list)
Code:
```
                    Ok(resp) => {
                        let n = (resp.body.len() as u64).min(out_cap as u64) as usize;
                        match write_guest(&mut caller, out_ptr, &resp.body[..n])
```
What is wrong: Oversized bodies are silently truncated to `out_cap` and reported as Ok with `n` — the guest cannot distinguish "body was exactly n" from "body was larger and cut". `devices_list` (898-899 `&inventory.as_bytes()[..n]`) additionally truncates a UTF-8 JSON string at a byte boundary, so a short `out_cap` can split a multi-byte character and hand the guest invalid UTF-8.
Why it matters: A plugin fetching a large descriptor gets a mysteriously unparseable payload with a success status. Read-style semantics are defensible, but there is no total-length signal (an `out_total` param or an E_TOO_LARGE-on-truncation policy) to make the truncation observable.
What correct looks like: Write the true body length to an out-param (truncation visible), or refuse with E_TOO_LARGE when `body.len() > out_cap` and the guest asked for less than exists.
Confidence: HIGH — Verification: read; transport contract read (http.rs HttpResponse "already capped" at 16 MiB — truncation occurs only below that).
### F-006 [S4]
Where: crates/plugin-host-wasm/src/imports.rs:430, 457-463
Code:
```
                    Err(other) => {
                        caller
                            .data_mut()
                            .trace_import(import, GuestStatus::EIo.code());
                        let _ = other;
                        return Ok(GuestStatus::EIo.code());
```
(with 430: `let path = String::from_utf8_lossy(&bytes).to_string();`)
What is wrong: Two diagnostic losses on the fs_open refusal path: (a) the jail's non-permission resolve error is discarded (`let _ = other;`) — the embedder/operator sees E_IO with no reason (the permission path keeps its detail; this one doesn't); (b) the guest path is decoded `from_utf8_lossy`, so a non-UTF-8 path is silently mangled to replacement characters before resolution — the denial reason then reports lengths of the mangled string, and `shape` (path_len) can disagree with what the guest sent.
Why it matters: Install/debug triage of a failing extension gets a status code with the evidence thrown away. No security impact (the jail still gates the mangled path).
What correct looks like: Trace the error's Display (shapes only, consistent with §5.3), and either refuse non-UTF-8 paths with E_ARG (like report_manifest does at 297-305) or record both raw and lossy lengths.
Confidence: HIGH — Verification: read; contrast with the strict UTF-8 gate in define_host (297-305).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `define_fs` (linker wiring) | 410-411, 553-555, 614-616, 649-651, 679-682 | REAL | — | and_then chain, typed Instantiation mapping; granted-only wiring at build_linker (part1, 232-234). |
| ↳ `fs_open` closure | 412-552 | REAL | S3(F-002), S4(F-006) | Path cap 4096; jail resolve→deny (ledger); open-once + verify_open TOCTOU narrowing (465-486, verified by fsjail tests `swap_between_resolve_and_open_is_denied`); metadata-from-handle; bounded slurp cap+1 with ETooLarge; handle insert capped. Full-file slurp design (no streaming) — memory bounded per handle, unbounded across handles (F-002). Executed: conformance sidecar lane (orchrun:4569ff). |
| ↳ `fs_read` closure | 556-613 | REAL | S3(F-001), S4(F-003) | Arg/bucket/handle gates all typed; consume-before-validate data loss (F-001); overcharge (F-003). Executed: conformance + `read_bogus`/`read_first` sandbox probes. |
| ↳ `fs_size` closure | 617-648 | REAL | — | Negative-handle gate; u64 LE out-param; bogus-handle E_HANDLE pinned (plugin_sandbox_security.rs:115-121, executed). |
| ↳ `fs_close` closure | 652-678 | REAL | — | Double-close/unknown typed E_HANDLE; pinned by state.rs tests + sandbox suite. |
| `define_http` (wiring) | 686-689, 810-813 | REAL | — | Granted-only (build_linker 235-237). |
| ↳ `http_get` closure | 690-809 | REAL | S4(F-004), S4(F-005) | Policy chain BEFORE transport: parse → allowlist (exact-match, http.rs) → rate window (32/min) → transfer caps → bucket → seam. No transport ⇒ honest E_TRANSPORT (v1 default, NoTransport). Silent truncation (F-005); trace/return mismatch on one denial path (F-004). Executed: allowlist/denial probes (sandbox suite 154-173); E_TRANSPORT lane in http.rs tests + conformance (empty services). |
| `define_ui` (wiring) | 817-819, 858-861 | REAL | — | Granted-only (build_linker 238-240). |
| ↳ `ui_register_panel` closure | 820-857 | REAL | S4 | Descriptor accepted VERBATIM (no schema validation — deliberate v1 seam, state.rs:221-223 "command-layer rendering protocol is doc 05/08 territory"); 8-panel cap with E_LIMIT; 64 KiB descriptor cap via read_guest caps. The conformance kit is honest about it ("asserts the semantic fields by their serialized forms — honest containment, not a fabricated parse", tests:339-343). Executed: ui-panel example lane (orchrun:4569ff). |
| `define_devices` (wiring) | 865-867, 914-917 | REAL | — | Granted-only (build_linker 241-243). |
| ↳ `devices_list` closure | 868-913 | REAL | S4(F-003,F-005) | Transfer-cap + bucket metering (pinned by `devices_list_is_bucket_metered`, orchrun:4612); inventory truncation can split UTF-8 (F-005); overcharge (F-003). Returns bare i32 (others Result) — cosmetic inconsistency. |

## 5. Test honesty (tests for this file live in the integration suites, not in imports.rs)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| metadata_provider_example_passes_conformance (conformance.rs:189-313) | YES | Sidecar bytes asserted verbatim end-to-end; import trace must show fs_open/read/close all Ok; E_PERM for missing + traversal; instance still Ready after denials. Removing any fs gate or breaking read semantics fails an assert. |
| ui_panel_example_passes_conformance (320-387) | YES | Exactly 1 panel; descriptor carries panel_id/surface serialized forms; on_command round trip lands in the bounded log. |
| fs_open_slurp_is_bounded (sandbox.rs:688+) | YES | Named pin of the cap+1 slurp bound (F-002's per-transfer half). |
| devices_list_is_bucket_metered (sandbox.rs:748+) | YES | Named pin of the bucket charge (F-003's existence — though not the overcharge ratio). |
| oob_probes_trip_the_disable_threshold (sandbox.rs) | YES | Escalation threshold (3) → host fault; pins read_guest/escalation_or_status (part1) + these imports' deny paths. |
| plugin_permission_escalation / plugin_sandbox_escape / plugin_host_pointer_smuggling | YES | Jail/allowlist/handle-table adversarial pins over real guest modules. |
| (gap) No test anywhere covers fs_read's write-failure path (F-001) | — | The suites pin bogus INPUT pointers (read side) but never a bogus OUTPUT buf_ptr after a successful handle read. |
| (gap) No test covers http_get truncation (F-005) | — | http.rs tests pin allowlist/scheme/userinfo; no oversized-body/short-out_cap case. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "no host pointers in the sandbox — every import takes offsets+lengths … bounds-checking" | docs/06 §5.2 | REAL | read_guest/write_guest use wasmi Memory accessors with typed E_OOB; pinned by plugin_host_pointer_smuggling + oob_probes tests (orchrun:4598-4619 green). |
| "undeclared import … instantiation error"; "call exceeds grant ⇒ typed PermissionDenied … logged, counted; repeated attempts disable" | docs/06 §5.3 | REAL | check_import_policy (part1) + deny/ledger/ESCALATION_DISABLE_THRESHOLD=3; pinned by oob_probes_trip_the_disable_threshold + permission_escalation tests. |
| "Single I/O transfer 16 MiB; sustained 1 MiB/s, 16 MiB burst; HTTP 32/min; handles 8" | docs/06 §5.4 rows 5-8 | REAL | limits.rs consts (66-83) wired through every import; pinned by sandbox suite. |
| "Total extension heap 256 MiB across instances — host accounting" | docs/06 §5.4 row 9 | FALSE | HOST_TOTAL_HEAP_BYTES defined (limits.rs:89) with ZERO readers; no aggregate accounting code; one instance can hold 8×16 MiB host-side (F-002). |
| "Sandboxed WASM extension plugins" status | feature-matrix.toml `plugin-wasm-extensions` = not_started (all platforms) | REAL (honest) | The host code exists and is tested, but nothing ships/loads it — not_started matches the shipped truth (though the row's evidence="" understates that a complete tested host already exists in-tree). |
| "panels render through the command layer only" | docs/06 §2.2 (ui-surface row) | PAPER-ONLY | No rendering exists anywhere; PanelRecord is recorded host-side only (state.rs:221-228); consumers: instance.panels() accessor + tests. |

## 7. Dead / duplicated / unreachable code
- The entire part2 surface (and the crate) is UNREACHABLE from shipped entry points (see header). Not dead — test- and bin-reachable.
- `HOST_TOTAL_HEAP_BYTES` (limits.rs:89): dead constant, zero uses (F-002) — flagged here from the slurp side; limits.rs's own auditor should carry it too.
- No duplicated logic found in part2; the four definers share the read_guest/trace/deny discipline consistently (the one inconsistency is F-004).

## 8. Dependencies & flags
- `wasmi` (Caller/Linker/Memory/Engine) — used as documented; no unsafe in part2 (whole file: `rg "unsafe"` → none).
- `std::fs::File` + `io::Read` (fs_open slurp) — std-only, no new deps; consistent with the crate's zero-external-deps posture (Cargo.toml comment).
- plugin_api used only for ENGINE_PLUGIN_API_MAJOR (part1) — part2 has no plugin_api use.
- No feature flags gate part2; all definers compile unconditionally and are wired per-grant at runtime (build_linker).

## 9. Open questions for the next agent
- instance.rs owner: does `WasmInstance` expose any fs_read write-failure test hook? If the conformance kit can drive a bogus out-ptr, F-001 becomes a one-test fix; today no suite can express it through the public invoke API.
- supervisor.rs owner: confirm no aggregate heap accounting exists on the supervisor side either (rg HOST_TOTAL_HEAP_BYTES says no — but the supervisor may cap concurrent instances, which would bound the F-002 blast radius to 4×128 MiB worst case; doc §5.4 row 8 "Concurrent instances 4 active" — is THAT enforced anywhere?).
- limits.rs owner: HOST_TOTAL_HEAP_BYTES dead (F-002) — carry it in your audit with the limits-side severity call.
- http.rs owner: ParsedUrl::parse returning PermissionDenied for scheme/userinfo refusals conflates "malformed" (E_ARG) with "policy" (E_PERM) — the imports side papers over it (F-004); decide which side owns the classification.
