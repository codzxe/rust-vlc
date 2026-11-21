# AUDIT: crates/demux/src/ogg/error.rs
| Field | Value |
|---|---|
| Auditor | W04-A04 |
| Date | 2026-11-21 |
| Lines / functions | 202 / 15 items (STAGE, `mint`, 12 error constructors, 2 tests) |
| File verdict | REAL |
| Reachable from shipped entry point? | YES. `lumen open <ogg>` → probe `OggS` (`media-probe/src/signature/phase5.rs:100-108`) → `apps/cli/src/containers/mod.rs:68` `"ogg" => ogg::open` → `OggDemuxer` → this module's constructors on every typed refusal. Executed: `ogg::error::tests` 2/2 (orchrun-test-workspace-nofailfast.log.txt:1807-1808) + 36 more `ogg::tests` (:1814+, incl. `bad_crc_page_is_dropped_with_sync_recovery`, `mid_stream_garbage_recovers_at_the_next_capture`) + `tests/corpus_ogg.rs` 2/2 (`corpus_oggs_match_the_ffmpeg_truth`, :2003-2007). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | Full-file Read (202 lines) · `rg -n "bad_version" crates/demux/src/ogg/*.rs` (caller page.rs:106 passes the real version byte that the constructor drops) · `rg -n "MAX_RECOVERY_SCAN_BYTES\|recovery_budget" ogg/mod.rs` (budget const 4 MiB :99, sole call site :717 with the scanned distance) · `rg -n "LimitKind::" adts/error.rs wav/error.rs` (Memory is the crate-consistent choice for budget errors — no drift) · `rg -n "ErrorKind::\|\.kind" ogg/tests.rs` (zero hits — the kind mapping is unpinned by tests; basis of F-002) · baseline greps above. |

## 1. One-paragraph verdict
A tight, correct typed-error module: twelve constructors, each minting the media-types taxonomy (never redefining it) with stable `demux.ogg.*` codes at stage Demux, the no-secrets rule honored (offsets and counts only), and the sniffed-bytes Display cap pinned by an executed test. I verified every constructor's kind/code/detail mapping by reading (Unsupported for not-ogg/not-started/seek, Corrupt::InvalidStructure for capture/version/structure/descriptor/no-streams, Corrupt::Truncated for pages, ResourceLimit::Memory for the two overrun ceilings and the recovery budget — the same LimitKind the wav/adts siblings use for scan budgets, so no cross-family drift). The two defects are small: `bad_version` accepts the offending version byte and silently drops it (the one diagnostic that would explain a "future version" refusal), and the test suite pins stage+code+Display but not the `ErrorKind` mapping — weaker than the IVF sibling's executed kind-table test, so a kind mutation (e.g. truncated-page minting `Unsupported` instead of `Corrupt`) survives the suite even though kinds are what the engine's error handling keys on.

## 2. What actually works (verified)
— All 12 constructors map to the documented taxonomy (read, constructor by constructor): `not_ogg`/`not_started`/`seek_out_of_range` → `Unsupported`; `bad_capture`/`bad_version`/`bad_structure`/`bad_descriptor`/`no_streams` → `Corrupt::InvalidStructure`; `truncated_page` → `Corrupt::Truncated{expected,found}`; `header_overrun`/`packet_overrun`/`recovery_budget_exhausted` → `ResourceLimit{Memory, budget, attempted}`.
— `recovery_budget_exhausted` (:156-165): budget wired to the real constant `super::MAX_RECOVERY_SCAN_BYTES` (4 MiB, ogg/mod.rs:99) — not a copy; the sole call site passes the actual scanned distance (mod.rs:717).
— No-secrets rule: every constructor's inputs are offsets/counts; only `not_ogg` carries bytes (sniffed prefix), capped by the taxonomy Display at 16 — pinned by the executed `not_ogg_sniff_capped_by_display`.
— `bad_descriptor` documented as the panic-free residue of a static-literal invariant (8.A1 sweep) — honest unreachable-defense, same posture as ivf's.
— Execution: 2/2 error tests + 36 ogg walk tests + the ffmpeg-truth corpus differential, all green in baseline.

## 3. Findings

### F-001 [S4]
Where: crates/demux/src/ogg/error.rs:49-51
Code:
```
/// A page declares a nonzero stream-version byte (RFC 3533 §6.1
/// fixes it at 0 — a future version is not demuxable here).
pub fn bad_version(offset: u64, _version: u8) -> EngineError {
```
What is wrong: The constructor accepts the offending version byte and drops it (`_version`): the minted error carries only the offset. The caller passes the real value — `page.rs:104-106`: `let version = head[4]; if version != 0 { return Err(error::bad_version(offset, version)); }` — so the one datum that distinguishes "file from the future (version 1)" from "random corruption (version 0xE7)" is read, threaded, and thrown away.
Why it matters: This is the checklist's ignored-parameter pattern (`_arg`); the RFC-cited refusal is exactly the case where the version value is the diagnosis. The no-secrets rule does not forbid it (a version byte is not a path/URI/payload); the taxonomy's `CorruptDetail::InvalidStructure` simply has no field for it — the fix is a context detail or folding the byte into the code, not dropping the parameter.
What correct looks like: Either encode it (`format!`-free: a distinct code per observed family, or extend the detail with the byte once the taxonomy allows) or remove the parameter and let the doc say so; today the signature promises diagnostics it does not deliver.
Confidence: HIGH — Verification: read; caller verified at page.rs:104-106.

### F-002 [S4]
Where: crates/demux/src/ogg/error.rs:171-194
Code:
```
        for (error, code) in cases {
            assert_eq!(error.stage, STAGE, "{code}");
            assert_eq!(error.context.code, code);
            let text = error.to_string();
            assert!(text.starts_with("[demux] "), "{code}: {text}");
            assert!(text.ends_with(&format!(" ({code})")), "{code}: {text}");
        }
```
What is wrong: The constructor table pins stage, code, and Display — but never `error.kind`. `rg "ErrorKind::|\.kind" ogg/tests.rs` → zero hits: no test in the ogg family asserts the taxonomy kind, while the IVF sibling ships exactly that test (`kinds_map_to_the_documented_taxonomy`, ivf/error.rs:192-214, executed). A mutation that mints `truncated_page` as `Unsupported` or `packet_overrun` as `Corrupt` passes the entire executed ogg suite.
Why it matters: `ErrorKind` is the machine-readable half of the taxonomy — the engine's error paths and the compatibility matrix key on it, not on the Display string. The mapping is correct today (I read every constructor); nothing keeps it correct.
What correct looks like: Copy the IVF sibling's kind-table test: for each constructor, `assert_eq!(error.kind, ErrorKind::… { … })` with the detail struct compared field-for-field.
Confidence: HIGH — Verification: read; rg sweep over the ogg family's kind assertions; IVF contrast verified in my ivf/mod.rs audit.

## 4. Per-function table
| Function/Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| module docs (taxonomy + no-secrets + scope note) | 1-12 | REAL | — | Accurate; the "crate-root error.rs is the AVI card's file" note matches my riff.rs F-003 finding (the R3 consolidation debt). |
| `STAGE` / `mint` | 17-21 | REAL | — | Uniform minting helper. |
| `not_ogg` | 24-34 | REAL | — | Unsupported + sniffed bytes; Display cap pinned by test. |
| `bad_capture` | 38-47 | REAL | — | Corrupt::InvalidStructure{offset}. |
| `bad_version` | 51-60 | REAL | S4 | F-001 (`_version` dropped). |
| `bad_structure` | 64-73 | REAL | — | |
| `bad_descriptor` | 79-86 | REAL | — | Documented panic-free residue of a static-literal invariant. |
| `truncated_page` | 91-98 | REAL | — | Corrupt::Truncated{expected,found}. |
| `header_overrun` / `packet_overrun` | 102-124 | REAL | — | ResourceLimit::Memory with budget/attempted — MP §12 shape. |
| `no_streams` / `not_started` / `seek_out_of_range` | 128-152 | REAL | — | Corrupt / Unsupported / Unsupported per docs. |
| `recovery_budget_exhausted` | 156-165 | REAL | — | Budget wired to the live constant; Memory kind consistent with wav/adts siblings. |
| tests (2) | 171-201 | REAL | S4 | F-002 (kind mapping unpinned). Stage/code/Display asserted for all 12; sniff-cap pinned. Both executed (orchrun :1807-1808). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `errors_carry_demux_stage_and_ogg_codes` | PARTIALLY | Fails for stage/code/Display mutations across all 12 constructors; does NOT fail for kind mutations (F-002). |
| `not_ogg_sniff_capped_by_display` | YES | 64 sniffed bytes → ≤16 hex pairs in Display — pins the no-secrets cap. |
| (walk tests elsewhere in the family, e.g. `bad_crc_page_is_dropped_with_sync_recovery`) | YES | The constructors' behavioral consumers — executed (:1814-1819). |
Both file-local tests executed, 0 failed (orchrun :1807-1808).

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The taxonomy is media-types' (never redefined here)" | module docs :3-4 | REAL | Every constructor uses `media_types::error` kinds/details verbatim (read). |
| "Constructors take byte counts and offsets only — never paths, URIs or payload content" | :9-12 | REAL | Verified per signature; `not_ogg`'s sniffed prefix is the sanctioned byte carrier, Display-capped. |
| RFC 3533 §6.1 version-0 fixation | :49-50 | REAL | Enforced at page.rs:104-106 (read); the refusal is typed. |
| feature-matrix `container-ogg` row (probe + demux side) | docs/feature-matrix.toml | REAL (this file's slice) | OGG lanes executed: 38 `ogg::*` tests + corpus_ogg ffmpeg-truth differential (orchrun :1807-1819, :2003-2007). |

## 7. Dead / duplicated / unreachable code
- `bad_descriptor` is documented-unreachable (defensive) — alive by design, not dead.
- The `mint`/STAGE/module shape is the fourth copy of the same error-module pattern (wav/adts/ivf/ogg) — deliberate family discipline, with the R3 consolidation note; not flagged.

## 8. Dependencies & flags
- `media_types::error` kinds/details only; `super::MAX_RECOVERY_SCAN_BYTES` (the one live cross-module constant). No feature gates; no unsafe; no allocation beyond the sniffed-bytes Vec (bounded by callers, capped in Display).

## 9. Open questions for the next agent
- The ogg/mod.rs + page.rs assignee: `bad_version`'s dropped byte (F-001) — confirm the refusal path surfaces the offset in the user-visible error and decide whether the version byte should ride a context detail.
- If any wave audits the engine's error-kind consumption: which kinds drive recovery vs abort — that determines whether F-002's unpinned mapping is cosmetic or load-bearing.
