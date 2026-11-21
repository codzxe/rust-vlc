# WAVE 2 — SUMMARY

Date: 2026-11-21 · Agents W02-A01…A10 · Domain: core types — media-types,
commands catalog, control-protocol (wire, server, client, policy).
Gate: **PASS — 37/37 assignments audited (22 file + 15 chunk), 0 missing, 0 thin.**
Note: agents wrote chunk audits as `<base>.partN.audit.md` (per-chunk files);
gate path-mapping updated to accept both conventions.

## Verdicts (file/chunk-level)
REAL 30 · REAL-UNPROVEN 2 · UNREACHABLE 5 (frame.rs — real+tested but sole
consumer lumen-control-server unshipped; product.toml:167 claim false)

## Findings
**S0:0 S1:0 S2:11 S3:51 S4:78 — total 140**

## Systemic finding (found independently by 6+ agents — evidence-class S2)
**The baseline `cargo test --workspace` runs are fail-fast and aborted early.**
Both baseline runs stopped at the first failing package (`desktop` zero_alloc,
then `first-vertical`), so ~20–25 packages' tests — media-types,
playback-engine, video-engine, lumen-cli, settings-schema, xtask — NEVER
executed, while `INDEX.md` presents "1681 passed / 1 failed" as a workspace
result. The same blind spot exists in CI (no `--no-fail-fast`). All M18
fast-path proof suites (time.rs) and the media-types ids/error suites have
zero executed evidence in the baseline. Orchestrator repair run filed below.

## Worst findings (citations verified at gate)
1. **[S2, evidence-integrity]** baseline #3/#4 partial-run presentation (above)
   — W02-A01/A04/A05/A07 among others.
2. **[S2] control-protocol event push unimplemented end-to-end** —
   `ServerFrame::Event` has zero constructors repo-wide; subscribe/unsubscribe
   bookkeep a dead Vec; Dispatch has no event seam → class-A commands over the
   control transport never emit completion evidence. (W02-A09)
3. **[S2] SECRET_QUERY_KEYS denylist misses OAuth params** (`access_token`,
   `client_secret`, `authorization`) — credentials ride verbatim through
   `SourceUri::redacted()`/Display into logs, contradicting lib.rs:32-34 and
   MP §12. (W02-A10, ids.rs:64-77)
4. **[S2] M18 `Scaler` "1.60×" fast-conversion tier has zero production
   callers** while docs claim hot loops hold one (worklog admits
   non-adoption). (W02-A02/A03)
5. **[S2] `Token` derives Debug inside Debug-deriving server structs** —
   contradicts the V8 never-in-logs invariant. (W02-A01)
6. **[S2] policy.rs playlist.enqueue(File)+play-item bypasses session.open's
   filesystem privilege** without a sensitive_allow grant (latent; dispatch
   NotReady today). (W02-A09)
7. **[S3] 23 of 32 `CommandResult` variants unproducible by any shipped
   host** — constructed only by tests; the CLI answers with the honest typed
   refusal (`other => Err(unserved(other))`, apps/cli/src/dispatch.rs:155).
   (W02-A01 F-001)
8. **[S3] protocol.rs advertises `tokenAuth:true` to unauthenticated
   read-only clients** (W02-A05 F-001); doc-05 §5.4 "schema-validated /
   unknown rejected for every frame" FALSE/SHALLOW — schema artifact never
   loaded, subscribe accepts unknown ids (F-002).
9. **[S3] error.rs `redact()` cannot redact paths inside `file://` URLs** —
   two drifting redaction truths (V8 invariant holds only via caller-side
   SourceUri::redacted()). (W02-A06)

## REQUEST-ORCH-RUN outcomes
- `cargo test --workspace --release --locked --no-fail-fast` — **RUN at gate**
  (repair run; see audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt).
- `cargo test -p media-types …` (4 agents) — covered by the repair run.
- `cargo run -p xtask -- codegen --check` — already executed Wave 1 gate
  (exit 0, no drift; orchrun-xtask-codegen-check.log.txt).
- W02-A05 junk-inside-args conformance probe — **NOT RUN: would require
  adding a test case to the repo (source modification, forbidden)**. Finding
  stands on code reading; probe marked UNVERIFIED with blocker stated.

## Citation spot-check (§7.3 step 4)
10 audit files sampled: **PASS 24 · FAIL 1 → adjudicated extractor
false-positive** (compound `Where: command.rs:491-668 + dispatch.rs:155`;
quote verified verbatim at apps/cli/src/dispatch.rs:154-155). **Confirmed
fabrications: 0** (fabrications.md updated).

## Blocked / UNVERIFIED carried forward
- macOS canonicalize `//`-preservation (no macOS in sandbox).
- serde multi-key untagged decode edge (reasoned, not executed).
- media-types proof-test execution — closed by the repair run at this gate.
