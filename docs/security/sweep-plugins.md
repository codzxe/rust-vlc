# Security sweep — plugins (issue #52, card W23-F sweep 2 of 4)

Task ID `7.w23f` (agent `w23-f`, branch `w23/f`). Scope: the wasm
extension host `crates/plugin-host-wasm/**` (imports, fs jail, quotas,
install/load) plus its §9.4 conformance suites — the doc 06 escalation
battery re-run in full, then a targeted gap-hunt for TOCTOU on
capability checks and quota bypasses.

Threat model: a malicious extension package (or a hostile local
process racing the host on the filesystem) must not escape the
sandbox, exceed any §5.4 quota, or disable-less probe the host
forever. Deny-by-default, typed `E_*` statuses, bounded host memory.

## 1. Doc 06 §9 escalation suite — re-run (all denied)

`cargo test -p plugin-host-wasm` — **ACTUAL: 50/50 green**
(35 lib incl. fsjail/http/limits/state units, 2 hello-world
conformance, 4 install-lifecycle, 9 sandbox-security). The §9.4
battery specifically, verified denied on this tree:

| §9.4 test | Escalation attempted | Observed denial |
|---|---|---|
| `plugin_sandbox_escape` | OOB guest pointers; symlink + `..` traversal | `E_OOB` + security event; `E_PERM` "sandbox roots"; happy path still serves |
| `plugin_permission_escalation` | undeclared `lumen:v1.fs` import; WASI import; allowlist violation ×3 | `UndeclaredImport` (no instantiation); `UnknownImportModule`; `E_PERM`×2 then threshold fault `ExtensionDisabled` + `Faulted` |
| `plugin_trap_loop` | infinite loop | `CpuLimit` fuel kill, instance faulted, fresh instance survives |
| `plugin_memory_exhaustion` | memory grow-loop past store cap | `MemoryLimit { 1 MiB }` trap, host heap unaffected |
| `plugin_io_abuse` | 3×600 B burst vs 1 KiB bucket; 2000 B transfer; 3 fetches vs 2/min | `E_QUOTA` (trace `0,7,7`); `E_TOO_LARGE`; `E_TRANSPORT`×2 then `E_QUOTA` |
| `plugin_host_pointer_smuggling` | fabricated/double-closed handles; handle-table overflow | `E_HANDLE` every use; 3rd open `E_LIMIT` (`0,0,9`) |
| `plugin_malicious_manifest` | forged origin, hash mismatch, bad CRC/method, cycles | typed package refusals (install-lifecycle suite) |
| `plugin_no_auto_exec` | inspect-only package open | no install, no execution, no registry change |

## 2. Gap-hunt findings and fixes (this sweep)

Three findings, all fixed with regression tests (commit references
below). Method: line-by-line review of `src/imports.rs` policy chains
against doc 06 §5.3/§5.4, `src/fsjail.rs` check/open sequencing, and
the quota-charging map across all ten v1 imports.

### F1 (Medium) — `fs_open` quota-bypass TOCTOU, unbounded host read

`imports.rs` `fs_open` checked `std::fs::metadata(..).len()` against
`io_transfer_bytes` and then called `std::fs::read` — two path
resolutions. A file **grown or swapped for a larger one between the
check and the slurp** made the host read without a ceiling: host-heap
exhaustion (the wasmi store limiter does not see host-side `Vec`s).
**Fix:** open the admitted path once; take metadata from the opened
handle; read through `Read::take(cap + 1)` so at most `cap + 1` bytes
ever cross; over-cap → `E_TOO_LARGE`. Regression:
`fs_open_slurp_is_bounded` (cap-exact 1024 opens, 1025 →
`E_TOO_LARGE`, quota ≠ escalation, happy path intact).

### F2 (Medium) — escalation threshold not enforced on OOB probes

`read_guest`'s out-of-bounds arms recorded security events ("sandbox
escape attempt") and incremented the session counter but never
checked `ESCALATION_DISABLE_THRESHOLD` — a guest could poke OOB
buffers **forever**, counted but never disabled, unlike the `deny()`
(allowlist/jail) and manifest-lie paths which fault at 3. Inconsistent
with doc 06 §5.3 ("reaching the session threshold faults the
instance") and with the recorded-event semantics the §9.4 test
asserts. **Fix:** `read_guest` returns `ReadErr::{Status, Fault}`;
`escalation_or_status` faults at threshold; `log_write` and
`ui_register_panel` closures now return `Result<i32, wasmi::Error>` to
propagate the host fault. Regression:
`oob_probes_trip_the_disable_threshold` (E_OOB×2, third →
`ExtensionDisabled` + `Faulted` + dispatch refused; host survives).

### F3 (Low) — `devices_list` unmetered seam crossing

`fs_read`/`http_get`/`log_write` charge the I/O token bucket;
`devices_list` (embedder inventory copied into guest memory at
guest-chosen call frequency, volume bounded only by the per-call
transfer cap) did not — the cap is not a rate limit. **Fix:** charge
`out_cap` like `fs_read`, after the `E_TOO_LARGE` check. Regression:
`devices_list_is_bucket_metered` (first listing returns the real
bytes, second → `E_QUOTA`, no escalation). Documented metering line:
data-carrying imports are bucket-metered; descriptor-shaped imports
(`report_manifest`, `ui_register_panel`) keep their per-call
`max_descriptor_bytes` / count caps.

### F4 (narrowed, not closed) — fsjail canonicalize→open TOCTOU

The known `[VERIFY]` window (fsjail.rs module doc) is now
double-checked: `PathJail::verify_open` re-canonicalizes the same
path **after** `File::open` and re-checks the jail prefix before any
byte is read; `fs_open` denies on failure (fail-closed, counted as
an escalation). Regression: `swap_between_resolve_and_open_is_denied`
(the deterministic slice: resolve admits `a.txt`, swap it for a
symlink out of the jail, verify refuses) +
`unswapped_path_passes_reverification`. Residual window (the
re-canonicalization walk itself) needs `openat`/`O_NOFOLLOW` — libc,
MP §9 ADR territory → 8.A2.

## 3. Audited, no fix required (posture notes)

- Quota charging is **conservative where imprecise**: `fs_read`
  charges the requested `buf_cap` even when fewer bytes return;
  `http_get` consumes its per-minute slot even when the transport is
  unwired. Overcharging is fail-closed, not a bypass.
- `HandleTable::insert` counts *closed* slots toward the cap (no slot
  reuse): stricter than §5.4 row 7's "8 open handles" — deny-by-default
  posture, recorded.
- `BoundedLog` (64 records), `import_trace` (1024), ledger events
  (256) — all bounded rings (D-2); ledger grows via `Arc<Mutex>` with
  poison-recovery, `AtomicU32` counter.
- Install/load chain re-verifies digest, origin, kind, api window at
  every load (`install.rs`); consent must match the package exactly;
  `safe_component` blocks traversal ids; ZIP reader is
  caps+CRC+magic-gated (native-code fence).
- **Unwired host-accounting caps**: `HOST_MAX_ACTIVE_INSTANCES` (4)
  and `HOST_TOTAL_HEAP_BYTES` (256 MiB) are declared in `limits.rs`
  but no in-tree caller enforces them — `HostError::HostResourceLimit`
  exists for it. Aggregating instances/heap is the host scheduler's
  job (issue #46 restart/backoff) → recorded for 8.A2.

## 4. Verdict

Escalation battery fully denied on the current tree; the three
sweep-specific gaps are closed with regressions (§2). Plugins:
**clean at first-pass depth**.

## 5. Remaining for 8.A2 (deep review)

1. fsjail residual TOCTOU: `openat`/`O_NOFOLLOW` ADR (libc dep) or an
   approved platform adapter; until then the narrowed window stands.
2. Host-level aggregate caps (active instances, total extension heap)
   — enforce in the scheduler once issue #46 lands.
3. Extension signature *verification* (ed25519 detached) — doc 06 §10
   Q5, Lead decision; today's status is honestly `Unverified`.
4. Concurrency review of `SecurityLedger`/`HostServices` sharing under
   the multi-instance host (single-threaded today; no data races
   possible until the scheduler exists, but 8.A2 should re-check).
