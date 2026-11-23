# AUDIT: crates/demux/src/asf/error.rs
| Field | Value |
|---|---|
| Auditor | W04-A08 |
| Date | 2026-11-23 |
| Lines / functions | 369 / 27 (20 public constructors + mint + asf_info + StreamTag impl×2 + 3 tests) |
| File verdict | UNREACHABLE (inherits the asf module's unreachability — see asf/mod.rs audit F-002; the code itself is real and consistent) |
| Reachable from shipped entry point? | NO — no crate outside demux references `demux::asf`; the CLI registry has no asf arm |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `cargo test -p demux asf::error` -> exit 0, 3 passed; grep for minting sites of all 20 constructors; sed quote verification |

## 1. One-paragraph verdict
A clean, disciplined typed-error module: 20 constructors, each minting `EngineError` at stage Demux with a stable `demux.asf.*` code, correct ErrorKind mapping (Unsupported/Corrupt/ResourceLimit with budget+attempted for the MP §12 ceilings), no secrets (byte counts/offsets/stream numbers only — verified against every signature), and a source-chaining helper for the stream-scoped refusal. Three tests pass under my run, and the full 20-code table is pinned in asf/tests.rs. The defects are inherited and admitted: three constructors (`encrypted_stream`, `packets_not_implemented`, `seek_unsupported`) have NO minting site in production code — their own doc comments say so ("vocabulary without a minting site", "kept for the error-table history") — and the entire taxonomy is unreachable in the shipped build because the ASF demuxer is not wired anywhere. The in-file taxonomy test covers 12 of 20 codes (the ASF-b/c codes live in asf/tests.rs's table).

## 2. What actually works (verified)
— verified by `cargo test -p demux asf::error` -> `test result: ok. 3 passed; 0 failed` (my run, exit 0).
— verified by grep: minting sites exist for 17 of 20 constructors (mod.rs/header.rs/data.rs call them); the 3 exceptions are exactly the doc-admitted ones (encrypted_stream: only header.rs:238 doc-comment + tests; packets_not_implemented: tests only; seek_unsupported: tests only).
— verified by read: `not_asf`'s sniffed bytes come from a bounded prefix (header.rs:487 passes `&head`, a ≤30-byte head read); Display caps at 16 bytes (test `not_asf_sniff_capped_at_sixteen_bytes_by_display`).

## 3. Findings
F-001 [S4]
Where: crates/demux/src/asf/error.rs:193-198, 206-211 (and 127-135)
Code:
```
pub fn packets_not_implemented() -> EngineError {
    mint(
        ErrorKind::Unsupported { format: None },
        "demux.asf.packets-not-implemented",
    )
}
```
What is wrong: Dead constructor — ASF-b landed the packet walk (mod.rs doc line 9: "ASF-b landed the Data Object packet walk"), so nothing mints `packets-not-implemented` anymore; same for `seek_unsupported` (line 206, own doc: "kept for the error-table history") and `encrypted_stream` (line 127, own doc: "Vocabulary without a minting site in v1"). Why it matters: three dead public functions shipped and tested only by their own taxonomy table — a test proving the dead code is dead-consistent, not proving behavior. All three disclosures are honest, which keeps this at S4. What correct looks like: delete, or gate behind the future encrypted-corpus card. Confidence: HIGH — Verification: read + grep (zero production call sites).

F-002 [S4]
Where: crates/demux/src/asf/error.rs:324-343
Code:
```
            (not_started(), "demux.asf.not-started"),
            (
                packets_not_implemented(),
                "demux.asf.packets-not-implemented",
            ),
            (seek_unsupported(), "demux.asf.seek.unsupported"),
        ];
```
What is wrong: The in-file taxonomy test lists only 12 of the 20 codes; the packet.*/seek.*/index codes are pinned only in asf/tests.rs's `error_constructors_table` (which does cover all 20). Duplicated test surface with a silent split. Confidence: HIGH — Verification: read + compared both tables.

Zero S0-S3 findings. Checked and clean: ErrorKind mapping per code (Unsupported for policy refusals, Corrupt+Truncated/InvalidStructure for structural lies, ResourceLimit+Memory with budget/attempted for the three ceilings), stage constant, Display format `[demux] ... (code)` asserted, source-chain shape, no path/URI/payload leakage in any constructor signature.

## 4. Per-function table (file >100 LOC)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| mint | 22-24 | REAL | — | single funnel, stage Demux |
| asf_info | 26-31 | REAL | — | |
| not_asf | 37-47 | REAL | — | sniffed carried; Display-capped |
| truncated | 53-60 | REAL | — | expected/found |
| bad_object_size | 65-74 | REAL | — | offset-bearing |
| header_too_large | 80-89 | REAL | — | ResourceLimit, ceiling vocabulary |
| no_streams | 94-101 | REAL | — | |
| broadcast_unsupported | 107-114 | REAL | — | ffprobe-measured policy |
| encrypted_stream | 127-135 | REAL | S4 | dead (F-001), honest doc |
| StreamTag + Display + Error | 139-147 | REAL | — | tested source leaf |
| bad_waveformatex | 152-161 | REAL | — | |
| bad_video_info | 169-178 | REAL | — | |
| not_started | 182-187 | REAL | — | |
| packets_not_implemented | 193-198 | REAL | S4 | dead fossil (F-001) |
| seek_unsupported | 206-211 | REAL | S4 | dead fossil (F-001) |
| packet_bad_flags | 217-226 | REAL | — | minted by data.rs |
| packet_bad_length | 231-240 | REAL | — | |
| payload_bad_replication | 245-254 | REAL | — | |
| payload_oversized | 259-268 | REAL | — | 32 MiB MAX_OBJECT_BYTES ceiling |
| packet_variable_size | 274-279 | REAL | — | minted by build_walk |
| seek_out_of_range | 283-290 | REAL | — | |
| seek_too_slow | 295-300 | REAL | — | 4096-packet budget |
| index_too_large | 307-316 | REAL | — | 1 MiB ceiling |
| tests::errors_carry_demux_stage_and_asf_codes | 322-351 | REAL | S4 | 12/20 subset (F-002) |
| tests::not_asf_sniff_capped_at_sixteen_bytes_by_display | 353-358 | REAL | — | leak cap pinned |
| tests::encrypted_stream_names_the_stream_on_the_source_chain | 360-368 | REAL | — | pins dead code's one behavior |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_asf_codes | YES (for the 12 covered) | stage/code/Display format asserted per constructor |
| not_asf_sniff_capped_at_sixteen_bytes_by_display | YES | 64-byte sniff → ≤16 "ab" matches in Display |
| encrypted_stream_names_the_stream_on_the_source_chain | YES | source chain + "#3" pinned |
| (gap) | — | the ASF-b/c codes are not in-file — see F-002; covered in asf/tests.rs |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Constructors take byte counts, offsets and stream numbers only — never paths, URIs or payload content" | error.rs:12-14 | REAL | all 20 signatures inspected |
| "Sniffed magic bytes are capped by the taxonomy's own Display" | error.rs:13-14 | REAL | cap test passes |
| "Vocabulary without a minting site" (×3) | error.rs:118, 203-204 | REAL (honest dead-code disclosure) | grep: zero production call sites |

## 7. Dead / duplicated / unreachable code
Dead: encrypted_stream, packets_not_implemented, seek_unsupported (F-001). Unreachable: the whole module in the shipped build (inherited — asf/mod.rs F-002).

## 8. Dependencies & flags
media-types error taxonomy only; no features; `#[cfg(test)]` in-file tests.

## 9. Open questions for the next agent
- If ASF ever gets wired, delete the two fossils before the registry arm lands (they invite cargo-deny/dead-code drift).
