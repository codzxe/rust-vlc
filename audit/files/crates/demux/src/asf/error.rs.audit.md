# AUDIT: crates/demux/src/asf/error.rs
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-21 |
| Lines / functions | 369 / 27 (22 constructors/helpers, StreamTag + 2 impls, 3 tests) |
| File verdict | UNREACHABLE (real, well-typed code; executed only by unit tests — no shipped entry point reaches the ASF demuxer; see F-001 context) |
| Reachable from shipped entry point? | NO. Proof (exhaustive): `media_probe::ProbeRegistry::with_builtins()` (`crates/media-probe/src/signature/mod.rs:131-145`) registers 14 signatures — **no AsfSignature exists**; `apps/cli/src/containers/mod.rs:53-70` dispatch has **no `"asf"` arm** and no `containers/asf.rs` exists (`ls apps/cli/src/containers/`); `rg "asf" apps/` → zero hits; `rg "AsfDemuxer" crates/ apps/ tests/` outside `demux/src/asf/` → zero hits. The repo knows: feature-matrix row `container-asf-wmv` notes record the 2026-11-10 compat-matrix triage — "the open path REFUSES the corpus files of probe format `asf` … (probe.no-demuxer)" with "Next step: wire the missing probe signature / container-registry arm". |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | full Read; per-constructor `rg -c` call-site census (production vs tests); `rg -n "packets_not_implemented\|seek_unsupported\|encrypted_stream" asf/` (only tests.rs:412-420); `sed -n '205,220p' docs/feature-matrix.toml` (row status); `rg '"asf"' apps/cli/src/containers/mod.rs` (no arm); `ls apps/cli/src/containers/`; `rg "asf" apps/` (zero); baseline `orchrun-test-workspace-nofailfast.log.txt:1555-1557` (3/3 error.rs tests ok; 37 asf test lines total) |

## 1. One-paragraph verdict
The module itself is disciplined typed-error plumbing — 20 stable `demux.asf.*` codes minted through one helper at `PipelineStage::Demux`, kinds drawn from the shared taxonomy (never redefined), no paths/URIs in payloads, sniffed bytes capped by Display (test-pinned), all three in-file tests green in the baseline. But it sits inside a demuxer that **cannot be reached from any shipped binary**: no probe signature names `asf`, no container-registry arm dispatches to it, and nothing under `apps/` references it — the matrix row's own notes concede the open path refuses ASF files with `probe.no-demuxer`. Despite that concession, the same row still claims `windows = "implemented"` and `macos = "implemented"` — statuses the shared-code wiring gap contradicts on every platform (F-001, S2). Inside the module: three constructors are dead vocabulary minted by nothing but the test table (S3), and the file's own conformance test is a stale 12-of-20 duplicate of the crate-level 20-entry table (S4).

## 2. What actually works (verified)
— All 3 tests executed and passed: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt:1555-1557` (`asf::error::tests::* ... ok`).
— Every constructor's code string and stage tag verified by the crate-level `error_constructors_table` (asf/tests.rs:400-430, all 20 constructors) — ran green in the baseline (37 `asf::` lines in the log, incl. the table test).
— The sniff cap claim is executed: `not_asf(&[0xAB; 64])` renders ≤16 byte-pairs (`not_asf_sniff_capped_at_sixteen_bytes_by_display`, baseline :1557).
— Call-site census: 17 of 20 constructors have production minting sites (not_asf header.rs:487, truncated ×21, bad_object_size ×21, header_too_large ×3, no_streams header.rs:658, broadcast_unsupported header.rs:655, bad_waveformatex ×5, bad_video_info ×5, not_started ×7, packet_bad_flags ×9, packet_bad_length ×13, payload_bad_replication ×3, payload_oversized ×3, packet_variable_size ×3, seek_out_of_range ×3, seek_too_slow ×3, index_too_large ×3 — rg census over asf/ excluding error.rs).

## 3. Findings
### F-001 [S2] — domain-level, surfaced here as the first ASF-domain file
Where: docs/feature-matrix.toml:211 (status line of row `container-asf-wmv`), against the wiring facts in crates/media-probe/src/signature/mod.rs:131-145 and apps/cli/src/containers/mod.rs:53-70
Code:
```toml
status = { windows = "implemented", linux = "partial", macos = "implemented", web = "not_started" }
```
with the row's own notes (docs/feature-matrix.toml:217) recording:
```toml
linux implemented->partial — compat-matrix run (linux-native lane): the open path REFUSES the corpus files of probe format `asf` — unsupported … (probe.no-demuxer) … Next step: wire the missing probe signature / container-registry arm for `asf`
```
What is wrong: the unreachability is platform-independent shared code — the probe registry (compiled into every `lumen` binary) has no ASF signature and the container dispatch has no `asf` arm, on Windows and macOS exactly as on Linux. Linux was honestly demoted to `partial`; `windows = "implemented"` and `macos = "implemented"` were left asserting a state the code contradicts on those platforms identically. The evidence string itself says "Linux-only evidence, CI 3-OS pending", so the two "implemented" cells have no platform run behind them at all.
Why it matters: the matrix is the project's claim ledger; a cell claiming implemented for a container family the shipped player refuses to open (typed `probe.no-demuxer`) is a FALSE cell by the audit's definition, and it hides the highest-impact wiring gap in the demux domain: the whole `crates/demux/src/asf/` tree (header/data/index/seek + fixtures) is dead weight in every shipped binary.
What correct looks like: demote windows/macos to the same `partial` (or `blocked` on the wiring issue #53) until the probe signature + container arm land, then re-run the compat matrix on all three lanes.
Confidence: HIGH — Verification: read + exhaustive rg (apps/ has zero asf references; no AsfSignature; no dispatch arm); baseline cites no ASF open-path run (rc-smoke is AVI-only by design; the 2026-11-10 compat-matrix refusal is quoted in the row's own notes).
Severity: S2 (major drift between the claim ledger and code; disclosed in notes, so not fresh fabrication — the linux cell was corrected, the other two were not).

### F-002 [S3]
Where: crates/demux/src/asf/error.rs:127-135, 193-198, 206-211 (sole references: crates/demux/src/asf/tests.rs:412-420)
Code:
```rust
pub fn packets_not_implemented() -> EngineError {
```
```rust
pub fn seek_unsupported() -> EngineError {
```
What is wrong: `encrypted_stream` (:127), `packets_not_implemented` (:193) and `seek_unsupported` (:206) have **zero production call sites** — the only code in the repository that mints them is the error-table test (asf/tests.rs:412-420). Their doc comments admit the state ("Vocabulary without a minting site in v1", "kept for the error-table history; seek now answers `seek.out-of-range` / `seek.too-slow` or succeeds") — `packets_not_implemented` describes a pre-ASF-b state and `seek_unsupported` a pre-ASF-c state, both superseded by the shipped grammar whose real refusals (`packet.bad-flags`, `seek.out-of-range`, …) are minted elsewhere.
Why it matters: pub dead vocabulary that ships in the release lib, plus conformance-test rows that "cover" constructors nothing can produce — the test table launders dead code into apparent coverage. (Same pattern as aiff/chunks.rs `MARK`/`INST`/`SsndChunk`, W04-A01 file 2.)
What correct looks like: delete the three (the error-code history lives in git); or gate them `#[cfg(any(test, feature = "contract"))]` until a minting site exists (the encrypted-stream one has a stated future owner — C-V6 acquisition — so cfg-gating with a re-enable note is defensible).
Confidence: HIGH — Verification: rg census (call sites enumerated: every other constructor has production sites; these three only in tests.rs).

### F-003 [S4]
Where: crates/demux/src/asf/error.rs:322-351 vs crates/demux/src/asf/tests.rs:399-430
Code:
```rust
    #[test]
    fn errors_carry_demux_stage_and_asf_codes() {
        let cases: Vec<(EngineError, &str)> = vec![
```
What is wrong: the file's own conformance test lists 12 of the 20 constructors — the eight ASF-b/ASF-c-era codes (`packet.bad-flags`, `packet.bad-length`, `payload.bad-replication`, `payload.oversized`, `packet.variable-size-unsupported`, `seek.out-of-range`, `seek.too-slow`, `index-too-large`) are absent. The crate-level `error_constructors_table` (tests.rs:399-430) covers all 20, making the in-file copy a stale duplicate of the "wav error-table convention" that drifted the moment the packet/seek units landed.
Why it matters: two tables asserting the same invariant will drift again; the stale one already has (12 vs 20).
What correct looks like: one table (the crate-level one), deleted here, or the in-file one regenerated to delegate.
Confidence: HIGH — Verification: read (both tables compared entry by entry).

### F-004 [S4]
Where: crates/demux/src/asf/error.rs:10-14 vs :37-43
Code:
```rust
//! Constructors take byte counts, offsets and stream numbers only —
//! never paths, URIs or payload content. Sniffed magic bytes are
//! capped by the taxonomy's own `Display`.
```
against:
```rust
pub fn not_asf(sniffed: &[u8]) -> EngineError {
    mint(
        ErrorKind::Unsupported {
            format: Some(FormatInfo {
                name: Some("asf".to_string()),
                sniffed: sniffed.to_vec(),
```
What is wrong: the "never … payload content" rule is contradicted by `not_asf`, which takes the leading file bytes (payload content) and copies them into the error. The second sentence carves out the exception, so the doc contradicts itself in the same breath. The rendering cap (16 bytes) limits Display, but the error object itself retains whatever the caller passes — the single production caller passes `&head` (header.rs:487, the GUID-sized prefix, ~16-30 B — bounded in practice, unbounded by contract).
Why it matters: a security-ish invariant ("no payload content in errors") stated absolutely and violated immediately is how future callers learn to pass bigger slices; nothing in the signature bounds `sniffed`.
What correct looks like: `not_asf(sniffed: &[u8])` truncating internally (`&sniffed[..sniffed.len().min(16)]`) and a doc that states the exception as the rule's part ("only the capped sniff prefix").
Confidence: HIGH — Verification: read (caller checked: header.rs:487).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `mint` / `STAGE` / `asf_info` | 20-31 | REAL | — | Single funnel; stage constant; no redefined taxonomy. |
| `not_asf` | 37-47 | REAL | S4 | F-004 (payload-copy vs doc); production caller header.rs:487 passes the bounded prefix. |
| `truncated` | 53-60 | REAL | — | Corrupt/Truncated{expected,found}; 21 production sites. |
| `bad_object_size` | 65-74 | REAL | — | 21 sites; offset-bearing. |
| `header_too_large` | 80-89 | REAL | — | ResourceLimit/Memory; budget=MAX_HEADER_BYTES (3 sites). |
| `no_streams` | 94-101 | REAL | — | Corrupt/InvalidStructure offset:None — kind stretch for "well-formed but empty" (family-consistent per doc). |
| `broadcast_unsupported` | 107-114 | REAL | — | 2 sites; the "ffprobe refuses too (plan §2.5, verified)" claim UNVERIFIED by me. |
| `encrypted_stream` + `StreamTag` | 127-147 | DEAD (test-only) | S3 | F-002; source-chain leaf verified by test :361-368. |
| `bad_waveformatex` / `bad_video_info` | 152-178 | REAL | — | 5 sites each; video floor doc honest about the ffmpeg-verified-only subset. |
| `not_started` | 182-187 | REAL | — | 7 sites; operational shape. |
| `packets_not_implemented` | 193-198 | DEAD (test-only) | S3 | F-002 — superseded by ASF-b. |
| `seek_unsupported` | 206-211 | DEAD (test-only) | S3 | F-002 — superseded by ASF-c; doc admits it. |
| `packet_bad_flags` / `packet_bad_length` | 217-240 | REAL | — | 9 + 13 sites. |
| `payload_bad_replication` / `payload_oversized` | 245-268 | REAL | — | 3 + 3 sites; oversized is a ResourceLimit refusal, not an allocation promise. |
| `packet_variable_size` | 274-279 | REAL | — | 3 sites; unverified-shape refusal, documented. |
| `seek_out_of_range` / `seek_too_slow` | 283-300 | REAL | — | 3 + 3 sites; mkv precedent named. |
| `index_too_large` | 307-316 | REAL | — | 3 sites; MAX_INDEX_BYTES budget. |
| tests (3) | 322-368 | REAL | S4 | F-003 (stale 12/20 table); sniff-cap and source-chain tests honest and passing. |

## 5. Test honesty (3 in-file tests, baseline :1555-1557 all ok)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| errors_carry_demux_stage_and_asf_codes | YES (for its 12) | Asserts stage, code, `[demux] ` prefix and `({code})` suffix per constructor — a code rename/kind change fails it. Gap: 8 newer constructors absent (F-003; covered at crate level). |
| not_asf_sniff_capped_at_sixteen_bytes_by_display | YES | 64 bytes of 0xAB → ≤16 "ab" pairs in Display; an uncapped Display fails it. |
| encrypted_stream_names_the_stream_on_the_source_chain | YES | source() present and contains "#3"; dropping with_source fails it. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "The error taxonomy is media-types' seven kinds — never redefined here" | :3-4 | REAL | Only taxonomy types used (read). |
| "Constructors take byte counts, offsets and stream numbers only — never paths, URIs or payload content" | :12-13 | FALSE for not_asf (self-contradicted at :14) | F-004. |
| "Sniffed magic bytes are capped by the taxonomy's own Display" | :14 | REAL | Test :354-358, baseline :1557. |
| "ffprobe itself refuses such files (plan §2.5, verified)" | :105-106 | UNVERIFIED | External-tool claim; plan §2.5 not re-checked (docs read-only, run-free). |
| matrix `container-asf-wmv`: windows/macos "implemented" | docs/feature-matrix.toml:211 | **FALSE** (both cells) | No AsfSignature (mod.rs:131-145), no dispatch arm (containers/mod.rs:53-70), zero app references — the row's own notes record the open-path refusal (F-001). |
| matrix `container-asf-wmv`: linux "partial" | docs/feature-matrix.toml:211 | REAL | Honest post-triage state (notes :217). |

## 7. Dead / duplicated / unreachable code
F-002 (three test-only constructors). F-003 (duplicated drifted table). Module-level: the entire asf tree is unreachable from shipped binaries (see header) — this file's UNREACHABLE verdict inherits that; the code itself is not dead (unit tests + fuzz construct the demuxer).

## 8. Dependencies & flags
`media_types` (EngineError, ErrorKind, PipelineStage, error::{CorruptDetail, ErrorContext, FormatInfo, LimitKind}) only. No features, no unsafe, no unwrap. Codes follow `demux.asf.<object>.<defect>`; consistent with the wav/mkv/avi siblings.

## 9. Open questions for the next agent
- The matrix-row F-001 belongs on the matrix-rows auditor's ledger (row `container-asf-wmv`, docs/feature-matrix.toml:207-217) — my finding supplies the code-side proof; the two "implemented" cells should be re-verdicted FALSE there.
- If issue #53's wiring lands (probe signature + container arm), `encrypted_stream` gains a plausible minting site — revisit F-002 then.
- The ASF fixtures in my remaining assignment (asf_pcm_s16le.wma, wmv_wmv2_64x48.wmv, wma_wmav2_mono8k.wma) exist for a demuxer no shipped lane can open — their audits will reference this finding.
