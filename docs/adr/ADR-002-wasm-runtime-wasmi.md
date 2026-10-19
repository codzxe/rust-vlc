# ADR-002: WASM Extension Runtime — wasmi

- **Status:** accepted (2026-10-19, Phase 0 gate)
- **Deciders:** Lead (per MASTER-PROMPT §16 Phase 0 mandate)
- **Evidence:** `docs/adr/ADR-002-evidence.md` (0.C4-I/R: candidate table),
  `docs/dependencies/records/wasmi.md` (worked audit record)
- **Supersedes:** open question in doc 06 §5 / doc 02 §6

## Context

Extension plugins (doc 06) are sandboxed WASM: UI panels, playlist/metadata
handlers, light filters — **non-realtime** workloads. The runtime itself must
pass the MASTER-PROMPT §9 dependency audit with no hidden native code. The
evidence pack audited four candidates (all fetched + tarball-scanned
2026-10-19):

| Candidate | Verdict | Why |
|-----------|---------|-----|
| **wasmi 2.0.0** | pass | Pure Rust; no build.rs compiler invocations; no-WASI core posture fits our `lumen:v1.*` capability imports; fuel-based metering; per doc 04/06 records |
| wasmtime 49.0.3 | fail (H1) | C inventory in the default closure (~129-crate graph incl. native bits); far exceeds sandbox needs |
| wasmer 7.5.0 | fail | C/C++ components |
| fizzy 0.8.0 | fail (maintenance) | Unmaintained-scale signals |

## Decision

**wasmi** is the v1 WASM extension runtime, under these conditions:

1. **Version pinning + allowlist**: pinned per `docs/dependencies/allowlist-format.md`,
   full R1–R10 record at Phase 1, re-audit on every bump.
2. **No-WASI posture**: extensions see only our versioned `lumen:v1.*`
   capability imports (doc 06 §5); no ambient filesystem/network — all host
   functions are permission-gated by the manifest.
3. **Resource limits** (doc 06 §5 defaults): memory cap 32–64 MiB/instance,
   fuel metering resampled to epoch deadlines, I/O token bucket; a trap or
   limit breach → plugin marked crashed → restart backoff → disabled; playback
   never affected.
4. **Interpreter-speed acceptance**: extensions are non-realtime; if a future
   realtime need emerges (it must not — filters stay builtin), this ADR is
   revisited, not worked around.
5. **Nested-wasm on web** ([VERIFY-P0] from doc 06): plugin-host-wasm must
   compile to wasm32 with wasmi inside — a Phase 7 spike card owns the proof;
   failure = web build ships without extension support (honest matrix row).

## Consequences

- ADR-002 closes; R-13-class runtime risk shifts to wasmi's maintenance
  (watched via allowlist re-audit triggers).
- The conformance kit (doc 06 §7) targets wasmi semantics specifically.
- doc 09's web-extension question inherits the nested-wasm spike outcome.
