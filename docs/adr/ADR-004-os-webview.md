# ADR-004: The OS-Webview Exception — tauri/wry as the audited UI shell (E1)

- **Status:** accepted (2026-10-22 — formalized by card 3.DOC-1; the exception
  itself has been brief policy since Phase 0, recorded as "accepted-by-brief"
  in D-008, and the engine-side seam landed with card 2.D3 and was verified at
  the 2.E2-V gate). **Per-OS webview verdicts: PENDING** — no desktop webview
  runtime exists in the sandbox and no tauri binary has ever been built or
  launched on any OS; the seams the wrapper drives are landed and tested
  (`ShellRuntime` delegation, `apps/desktop` 63/0, merge `8a56f0b` — close
  package: `apps/desktop/tauri/INFRA-BLOCKED.md`); the to-run list is
  `docs/prototypes/video-in-webview.md` §"What is PENDING".
- **Deciders:** Lead (per MASTER-PROMPT §2 fixed decision "React+TS UI" + §9
  exception list naming windowing; formalization dispatched by 3.HARD-AUDIT
  decision D-c)
- **Evidence:** `docs/dependencies/preaudit-foundation.md` §1 rows tauri/wry +
  §2 note "tauri / wry" (0.C4, 2026-10-19); `docs/dependencies/allowlist.md`
  §5 (exception-ADR-004, pre-audited, NOT imported);
  `docs/dependencies/records/tauri-wry-2.x.md` (admission record, honest
  not-yet-imported status); `apps/desktop/**` (the 2.D3 variant-B seam);
  `docs/prototypes/video-in-webview.md` (2.D3 evidence report);
  `docs/adr/ADR-003-video-in-webview.md` (the presentation strategy this shell
  hosts)
- **Supersedes:** the status contradiction this file closes — ROADMAP §6 said
  "accepted-by-brief", doc 01 §7 register said "planned", the
  `apps/desktop/Cargo.toml` header comment said "status: planned" (all three
  reconciled to *accepted* with per-OS caveats in the same 3.DOC-1 change)
- **Consequence for risks:** R-04 stays mitigating (per-OS verdicts pending);
  the import surface gains a gated admission record
  (`records/tauri-wry-2.x.md`); R-20 documentation-drift mitigation cites this
  file as the closure of the triple-contradiction finding

## Context

MP §2 fixes the desktop UI as React+TypeScript; the shell that hosts it is
therefore an OS webview, which is not Rust and not ours. Doc 11 §1.2 defines
this as exception class **E1 — OS webview (UI shell)**, reserved to this ADR,
and requires every exception instance to carry (1) the exact API surface list,
(2) a safety review, (3) truthful reporting of when it is active. The product
is never described as all-Rust (MP §2 / doc 11 §1.2 E1).

The Phase-0 pre-audit (0.C4, `preaudit-foundation.md` §1/§2, 2026-10-19)
pinned the candidates: **tauri 2.12.4** and **wry 0.57.0**, both Apache-2.0 OR
MIT, actively maintained; wry 0.57 targets WebKitGTK 4.1/GTK3 on Linux (the
0.C5-a finding that feeds ADR-003's R-04 assessment). Verdict class:
`exception-ADR-004` — consumable **only** under this ADR, and the exception
envelope also covers `tao` (the windowing crate tauri pulls). They are the UI
shell, not engine code: **the engine never links them** (preaudit §2;
allowlist.md §5 lists them as pre-audited-but-not-imported).

Phase 2 (card 2.D3) implemented the engine-side half *without* the
dependency: `apps/desktop` is the ADR-003 variant-B frame-streaming protocol
core — `src/wire.rs` (LUF1 v1 wire format), `src/bridge.rs` (renderer
interop), `src/mailbox.rs` (bounded latest-wins with slow-consumer policy),
`src/transport.rs` (byte transport + latency accounting), `src/host.rs` (the
command-layer host seam), `src/error.rs` — 2,541 LOC, std-only, zero
dependencies, 32 tests green. The typed seam points that gate every real
webview import: `WebviewHostKind::{TauriChannel, CustomProtocol, SharedMemory}`
with `GATING_ADR = "ADR-004"` (`src/transport.rs:250`), the always-refusing
`open_webview()` → `HostError::WebviewPending` (`src/host.rs:213-223`), the
honest one-line `active_presentation_path()` reporter (`src/host.rs:212-214`),
and the `WEBVIEW_PENDING` exit report (`src/main.rs:50`). The prototype
verdict recorded for ADR-003 is **PROVISIONAL-B**: the protocol core is proven
end-to-end at the deterministic virtual-time level (every frame in order, min
11.67 ms end-to-end); the per-OS browser matrix (WebView2 / WKWebView /
WebKitGTK) is honestly pending a desktop machine.

## Decision

1. **The OS webview is the E1 exception**, scoped exactly as doc 11 §1.2 E1:
   the desktop product hosts its React+TS UI in the OS webview via tauri/wry.
   Exact surface per OS: **WebView2** (Windows), **WKWebView** (macOS),
   **WebKitGTK 4.1** (Linux, via wry 0.57's GTK3 binding). Alternative
   considered and rejected: a custom Rust windowing shell (doc 01 §7 row) —
   a large new unaudited surface that re-implements what the OS provides.
2. **The engine never links tauri/wry/tao.** They are app-shell (L6,
   `apps/desktop`) dependencies only, admitted behind the `WebviewHost` seam
   that 2.D3 landed. The engine-side protocol core stays std-only and
   headless-testable (the 2.E1 evidence model depends on this).
3. **Import is gated by the admission record**
   (`docs/dependencies/records/tauri-wry-2.x.md`): a desktop build environment
   (webkit2gtk-4.1 dev packages, display server), a Lead allowlist row with a
   pinned version range, and the `[VERIFY]` refreshes the record lists. Until
   then `apps/desktop` compiles std-only and refuses webview opens with the
   typed `HostError::WebviewPending` — it enumerates nothing and never fakes
   success.
4. **Security review (PENDING, owed at the Phase-4 shell card).** Doc 11 §1.2
   E1 names the scope: webview sandbox posture, the IPC surface
   (`WebviewHostKind::TauriChannel` vs the planned `lumen://` custom protocol
   and SharedArrayBuffer transports of wire protocol v2 — `transport.rs`
   marks both "pending ADR-004 + protocol v2"), navigation/content isolation
   (trust boundary TB3, doc 11 §5). The review executes against the real
   import; it is recorded here so it cannot be silently dropped at Phase 4.
5. **Truthful reporting**: the product is never described as all-Rust; the
   active presentation path is a queryable engine fact (ADR-003 decision 4) —
   `host.rs::active_presentation_path()` is the standing implementation, and
   the 2.E1 evidence ledger's `hardware_class`/path columns follow it.
6. **Per-OS webview verdicts remain ADR-003's own per-OS gates** — PENDING,
   with the exact to-run commands in `docs/prototypes/video-in-webview.md`.
   This ADR accepts the *exception* (the shell strategy); it does not certify
   per-OS performance or compatibility claims, which need the desktop runs.

## Consequences

- Phase 4's product-surface cards import tauri/wry once the admission
  criteria pass; no code change is needed on the engine side (the seam is
  implemented, merged, and covered by the 2.D3/2.E1 test suites).
- The dependency posture stays honest: pre-audited, allowlisted as
  exception-ADR-004, not in the lockfile (phase2-exit.md §3 records the
  Phase-2 posture "tauri/wry NOT imported — engine-side protocol core is
  std-only").
- R-04 is unchanged (mitigating): the Linux/WebKitGTK risk is bounded by
  ADR-003's B-landing contingency and now by this ADR's admission gate.
- Documentation debt closed: the three contradictory status claims
  (ROADMAP §6 D-008, doc 01 §7, `apps/desktop/Cargo.toml` header) now all
  read *accepted* with the per-OS caveat, citing this file.

## Revisit triggers

- A per-OS prototype failure flips that OS to its ADR-003 documented fallback
  (B on Linux is the accepted landing; A on macOS; A-or-B on Windows).
- tauri/wry version drift at admission time re-runs the pre-audit
  (`preaudit-foundation.md` method) and refreshes the record.
- A security-review finding on the IPC surface may constrain or retire the
  `CustomProtocol`/`SharedMemory` host kinds before wire protocol v2 lands.
