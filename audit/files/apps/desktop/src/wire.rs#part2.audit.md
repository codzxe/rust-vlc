# AUDIT: apps/desktop/src/wire.rs#part2 (lines 580–1159)
| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-21 |
| Lines / functions | 580 (of 1159; chunk boundary pinned from audit/waves/wave-00/assignments.tsv:769: part1 = 1–579 [W13-A03], part2 = 580–1159 [this audit]) / 19 items |
| File verdict | REAL (decode path correct and baseline-executed; `finish()` defective but callerless) |
| Reachable from shipped entry point? | PARTIAL. `decode_record`/`parse_header`: YES via lumen-tauri — code chain: desktop-tauri/src/app.rs:93 `use desktop::shell::ShellRuntime` → shell.rs:175-187 `bind_frame_channel` → `FrameTap::new` (frame_tap.rs:57) → `CanvasBridge<MailboxTransport>` (frame_tap.rs:46) → transport.rs:227 `self.decoder.decode_record(record)` → wire.rs:572-600 → parse_header 626-679. CAVEAT: lumen-tauri binary itself is UNVERIFIED-BUILD in baseline #2 (missing gtk/webkit dev libs — sandbox limitation, not repo defect); the desktop crate compiled (baseline #1) and all 14 wire tests executed and passed (test-workspace.log.txt:2347-2360). `finish()`: NO — zero callers outside wire.rs's own tests (rg `\.finish\(\)` over apps/desktop, apps/desktop-tauri: only wire.rs test sites). |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:2 |
| Commands run | Read wire.rs 1-120/121-360/361-610/611-880/881-1160 (full file for context; audited 580-1159); rg `\.finish\(\)|decode_record|WireDecoder::new|feed\(` apps/desktop/src (callers mapped: transport.rs:227, frame_tap.rs:227-229, bridge.rs:335-365 [tests]); rg `MailboxTransport|desktop::wire|desktop::transport|desktop::bridge|FrameTransport` workspace-wide (desktop-internal only); rg `desktop::|use desktop` apps/desktop-tauri/src (tauri uses files/shell/window_config/diagnostics only — but shell.rs wires the tap); Read shell.rs 165-239 (bind_frame_channel/take_frame_records — the tauri→tap chain); grep test-workspace.log.txt 2347-2360 (all 14 wire tests ok); tail test-workspace.log.txt (zero_alloc failure text); Read transport.rs 200-217 (quote check after a sed display artifact showed mangled `#[must_use]`). |

## 1. One-paragraph verdict
Part 2 is the back half of the LUF1 record codec: the tail of `decode_record` (truncation/trailing refusals, pooled one-copy payload hand-off), the `finish()` boundary assertion, the strict `parse_header` (magic/header_len/version/format/flags/reserved/geometry all typed-checked, little-endian throughout, indexing safe because every caller guarantees ≥48 bytes), and a 16-test module that genuinely ran in the baseline fail-fast run (14/14 ok, log:2347-2360) including a hand-written golden record and an exhaustive chunked-split sweep. The decode path is honestly engineered with one real defect: `finish()`'s missing-bytes arithmetic underflows (usize subtraction) when the buffer holds more than one complete undrained record — debug panic, release wrap to `missing ≈ 1.8e19` — and reports the lie `Truncated { missing: 0 }` when exactly one complete record is undrained; it is untested and, today, uncalled. A test comment in this range cites the zero-allocation "measured proof" that the baseline records as FAILED (608 bytes allocated on the mailbox frame path).

## 2. What actually works (verified)
- All 14 tests in this chunk executed and passed in the recorded baseline: `test-workspace.log.txt:2347-2360` (e.g. `wire::tests::golden_bytes_match_the_spec_table ... ok`, `wire::tests::chunked_feed_at_every_split_point ... ok`).
- `parse_header` (626-679) is strict and safe: every field typed-checked, `buf[0..48]` indexing guarded by all three callers (`next_frame` requires remaining ≥ 48 at 519, `decode_record` at 573, `finish` else-branch at 613); extension `header_len` up to 1024 is skipped, never interpreted (proven by `extension_headers_are_skipped_not_guessed`, log:2352).
- `decode_record` body (580-600): truncation/trailing arithmetic is subtraction-safe on those paths (`record.len() < total` / `> total` before subtracting); payload copy is exactly `payload_len` bytes (lengths proven equal by the 580-589 checks); pool acquire is bounded at the validated 1 GiB cap.
- `format_code_table_is_exactly_the_upload_path` (979-1005) cross-checks the wire format table against `renderer_wgpu::uploads_format` per pixel format — a real cross-crate consistency pin, not a tautology.

## 3. Findings

### F-001 [S3] — `finish()` underflows on undrained complete records and reports `missing: 0` for one undrained record
Where: apps/desktop/src/wire.rs:608-620
Code:
```
    pub fn finish(&self) -> Result<(), WireError> {
        let unconsumed = &self.pending[self.pos..];
        if unconsumed.is_empty() {
            return Ok(());
        }
        let missing = if unconsumed.len() < MIN_HEADER_LEN {
            (MIN_HEADER_LEN - unconsumed.len()) as u64
        } else {
            let (header, header_len) = Self::parse_header(unconsumed)?;
            (header_len + header.payload_len as usize - unconsumed.len()) as u64
        };
        Err(WireError::Truncated { missing })
    }
```
What is wrong: The else-branch assumes `unconsumed.len() < header_len + payload_len`. When the buffer holds a COMPLETE record (the exact caller mistake `finish` exists to catch), `header_len + payload_len - unconsumed.len()` evaluates to 0 for one record — the error claims `Truncated { missing: 0 }` ("nothing missing" while a whole frame sits undrained) — and goes NEGATIVE for two or more complete records: `usize` underflow, panic in debug builds, wrap to `missing = 18_446_744_067_095_515_152` in release. Reproduce: `d.feed(&record); d.feed(&record); d.finish()` — no `next_frame` calls.
Why it matters: `finish`'s documented contract is "assert the stream ended at a record boundary"; the boundary violation it is most likely to be handed (producer pushed frames, consumer forgot to drain) is the one case it answers with a lie or a panic. Today blast radius is zero (no production caller — rg over apps/desktop and apps/desktop-tauri finds only wire.rs tests), which is why this is S3 and not S1; the moment anyone wires `finish()` into a transport teardown it becomes a panic path.
What correct looks like: `let total = header_len + header.payload_len as usize; if unconsumed.len() >= total { Err(WireError::TrailingBytes { count: (unconsumed.len() - total) as u64 }) } else { Err(WireError::Truncated { missing: (total - unconsumed.len()) as u64 }) }` — reusing the `TrailingBytes` variant `decode_record` already mints, plus a test feeding 1 and 2 undrained complete records.
Confidence: HIGH — Verification: read (arithmetic traced); UNVERIFIED only in the sense that no run executes it (baseline tests never cover it — `truncated_streams_report_exact_missing_bytes` cuts at 20 and 50 bytes, both < total=64).

### F-002 [S3] — Test comment cites a "measured zero-allocation proof" that the baseline records as FAILED
Where: apps/desktop/src/wire.rs:1080-1084
Code:
```
        // The C7 shape: ONE decoder, N records, each frame dropped by
        // its consumer — the payload buffers cycle through the pool
        // (the measured zero-allocation proof is the dedicated
        // counting-allocator binary; this pins the *correctness* of
        // the reuse: no state bleed between recycled buffers).
```
What is wrong: The "dedicated counting-allocator binary" is `apps/desktop/tests/zero_alloc_transport.rs`, and the recorded baseline run of it FAILED: `mailbox_frame_path_is_zero_allocation_after_warm_up ... panicked at apps/desktop/tests/zero_alloc_transport.rs:98 … allocated 608 bytes across 1 allocation events (dealloc events: 0) — the path is NOT zero-allocation` (test-workspace.log.txt tail; INDEX.md verdict-table row 3). The mailbox frame path this comment calls proven includes this chunk's `decode_record` (present → serialize → send → **decode_record** → offer → draw). The proof is a disproof as of the recorded run.
Why it matters: The C7 fix's headline claim ("after one warm-up record, the steady-state path allocates nothing", doc at 426-438 in part1) is refuted by the project's own committed evidence; readers of this test are pointed at a red test as their measurement. The root allocation is not attributable from this file (the counting harness spans present/serialize/mailbox/draw stages owned by bridge.rs/transport.rs/mailbox.rs — other agents' files), but the citation is wrong regardless of where the 608 bytes come from.
What correct looks like: Either fix the allocation and keep the citation, or rewrite the comment to state the current truth ("the counting-allocator test currently records one 608-byte allocation per path lifecycle — under triage, see zero_alloc_transport.rs:98").
Confidence: HIGH — Verification: read + baseline test-workspace.log.txt (failure text quoted verbatim above) + INDEX.md row 3.

### F-003 [S4] — `decode_record` silently orphans buffered `pending` bytes (API mixing undetected)
Where: apps/desktop/src/wire.rs:572-600 (body in this range)
Code:
```
        let mut payload = self.pool.acquire(header.payload_len as usize);
        payload.clear();
        payload.extend_from_slice(&record[header_len..]);
        self.fed += record.len() as u64;
        self.consumed += record.len() as u64;
```
What is wrong: `decode_record` never looks at `self.pending`/`self.pos`. A caller that `feed`s a partial chunk and then hands the same decoder a complete record via `decode_record` silently strands the fed bytes: they are never decoded, `finish()` would later count them (and hit F-001's arithmetic), and no diagnostic fires. The doc contract ("a byte producer with one record per message calls this") is enforced nowhere.
Why it matters: The one failure mode this strict little codec family prides itself on refusing — host bugs — is accepted silently on this seam. `MailboxTransport::send` (transport.rs:227) never mixes, so this is latent, not live.
What correct looks like: `debug_assert!(self.pending.is_empty() || self.pos == self.pending.len(), "decode_record called with undrained feed bytes")` or a typed `WireError` variant (`MixedMode`).
Confidence: HIGH — Verification: read.
(Split note: `decode_record`'s signature and doc sit at 555-579 in part1 [W13-A03]; the body audited here is 580-600.)

### F-004 [S4] — Decode-side dimension-cap and payload-cap refusals have no decode test
Where: apps/desktop/src/wire.rs:883-901 (what `malformed_inputs_are_typed_refusals` ends at) vs 950-957 (encode-only coverage)
Code:
```
        // Dimension cap.
        let h = header(65_536, 1);
        assert_eq!(
            encode(&h, &[]).err(),
            Some(WireError::BadGeometry {
                reason: "dimension exceeds 65535"
            })
        );
```
What is wrong: The two `BadGeometry` rules beyond "zero dimension" ("dimension exceeds 65535", "payload exceeds the 1 GiB cap") are exercised only through `encode` (939-977). No test feeds the DECODER a hostile record with width=70_000 or a 1 GiB-implied payload — i.e., the strict-decode posture against bytes our encoder never produced (the actual threat model for a wire) is untested for those two rules. The shared `validate_geometry` means the rule itself is tested, so this is a coverage gap, not a code bug.
Why it matters: `parse_header` is the boundary against foreign/hostile producers once the Tauri channel goes live; untested refusal branches rot first.
What correct looks like: Two more blocks in `malformed_inputs_are_typed_refusals`: `bad[12..16] = 70_000u32.to_le_bytes()` (with payload_len patched to match) → `BadGeometry { "dimension exceeds 65535" }`; a dims-implying->1GiB record → `BadGeometry { "payload exceeds the 1 GiB cap" }`.
Confidence: HIGH — Verification: read (full tests module enumerated).

## 4. Per-function table (items in lines 580–1159 only; part1 items are W13-A03's)
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `WireDecoder::decode_record` (body; fn 572-600 split across boundary) | 580-600 | REAL | S4 (F-003) | Reachable via tauri chain (see header). Truncation/trailing math subtraction-safe; pooled one-copy payload; counters updated only on success. Ignores `pending` (F-003). |
| `WireDecoder::finish` | 608-620 | REAL (defective) / UNREACHABLE | S3 (F-001) | No caller outside tests. Underflow + missing-0 lie on undrained complete records. |
| `WireDecoder::parse_header` | 626-679 | REAL | — | Strict; all 12 header fields typed-validated; indexing guarded by callers; LE endianness matches `write_to` (part1); extension bytes skipped. Reachable via decode_record/next_frame. |
| `tests::header` (helper) | 687-698 | REAL | — | Fixed test header; `payload_len: w*h*4` consistent. |
| `tests::payload` (helper) | 700-702 | REAL | — | Deterministic 251-mod pattern. |
| `round_trip_exact_fields` | 704-717 | REAL | — | Ran ok (log:2359). Negative pts, u64::MAX frame_id, repeat flag — catches field-order/endian mutations. |
| `golden_bytes_match_the_spec_table` | 719-752 | REAL | — | Ran ok (log:2354). Hand-written expected bytes — the strongest kind of golden; not generated by the code under test. |
| `chunked_feed_at_every_split_point` | 754-797 | REAL | — | Ran ok (log:2347). Exhaustive split sweep + two-record chunking; catches cursor/compaction bugs. |
| `malformed_inputs_are_typed_refusals` | 799-901 | REAL | S4 (F-004) | Ran ok (log:2356). 8 decode refusals; dim-cap/payload-cap decode-side missing. |
| `truncated_streams_report_exact_missing_bytes` | 903-917 | REAL | S3 (F-001) | Ran ok (log:2360). Covers both branches' happy math only — never an undrained complete record. |
| `extension_headers_are_skipped_not_guessed` | 919-937 | REAL | — | Ran ok (log:2352). |
| `encoder_refuses_unexpressible_records` | 939-977 | REAL | — | Ran ok (log:2351). Encode-side refusals incl. 65_536 dim and 1 GiB cap. |
| `format_code_table_is_exactly_the_upload_path` | 979-1005 | REAL | — | Ran ok (log:2353). Cross-crate pin vs renderer_wgpu. |
| `header_overhead_is_recorded_math_not_assertions` | 1007-1016 | REAL (doc-pin) | — | Ran ok (log:2355). Asserts the doc's overhead inequality from constants; cannot fail on any codec regression — its honest purpose is stated in its name. |
| `decode_record_is_the_streaming_result_direct_from_the_slice` | 1018-1040 | REAL | — | Ran ok (log:2349). API-drift pin between the two entry points. |
| `decode_record_refuses_truncated_and_trailing_input` | 1042-1076 | REAL | — | Ran ok (log:2350). Also pins refusal statelessness (line 1075). |
| `one_decoder_recycles_payload_buffers_across_records` | 1078-1104 | REAL | S3 (F-002) | Ran ok (log:2357). Correctness of pool reuse pinned (incl. geometry grow-once at 1099-1103); comment's proof-citation false per baseline. |
| `partial_records_compact_without_losing_bytes` | 1106-1137 | REAL | — | Ran ok (log:2358). Multi-record compaction with consumed prefix. |
| `clones_and_hand_built_frames_behave_like_plain_owners` | 1139-1158 | REAL | — | Ran ok (log:2348). Drop/pool/eq semantics; clone owns its own Vec, bounded returns. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| round_trip_exact_fields | YES | Field-by-field header eq + payload eq after encode→feed→next_frame; endian/order mutation fails. |
| golden_bytes_match_the_spec_table | YES | Expected bytes written by hand from the layout table — independent of the encoder. |
| chunked_feed_at_every_split_point | YES | Exhaustive splits; any cursor/compaction bug trips "no complete record before the final split" or payload mismatch. |
| malformed_inputs_are_typed_refusals | YES | Byte-level mutations each assert a DISTINCT typed error (magic/version/header_len/format/flags/reserved/payload/zero-dim). |
| truncated_streams_report_exact_missing_bytes | YES (partial) | Exact missing counts (28/14); does NOT cover the complete-record cases (F-001). |
| extension_headers_are_skipped_not_guessed | YES | header_len=56 + 8 extension bytes; framing must survive. |
| encoder_refuses_unexpressible_records | YES | Four distinct encode refusals. |
| format_code_table_is_exactly_the_upload_path | YES | Cross-checks renderer_wgpu's actual routing per format. |
| header_overhead_is_recorded_math_not_assertions | NO (by design) | Pure arithmetic over constants; guards the doc's claim, not code. |
| decode_record_is_the_streaming_result… | YES | Direct vs streaming equality incl. counters. |
| decode_record_refuses_truncated_and_trailing_input | YES | Truncated (2 cuts), trailing (2 shapes), refusal-statelessness. |
| one_decoder_recycles… | YES | 8 records + geometry change through one decoder/pool; stale-buffer bleed fails payload eq. |
| partial_records_compact_without_losing_bytes | YES | Three records across four feeds with a consumed prefix; frame_ids 1/2/3 + fed/consumed totals. |
| clones_and_hand_built_frames… | YES | Pool-handle invisibility in eq; drop semantics. |

## 6. Doc claims touching this chunk
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`record` must hold exactly one record: fewer bytes is Truncated; leftover bytes are TrailingBytes" (decode_record doc, 564-567, part1; behavior in this range) | wire.rs:564-567 + 580-589 | REAL | `decode_record_refuses_truncated_and_trailing_input` ran ok (log:2350). |
| finish: "Truncated when a partial record is still buffered (its declared length is known — the payload says how much is missing)" | wire.rs:605-607 | SHALLOW | True only for partial records < total; complete-record inputs yield missing:0 / underflow (F-001). |
| "the measured zero-allocation proof is the dedicated counting-allocator binary" | wire.rs:1082-1083 | FALSE (as of baseline) | tests/zero_alloc_transport.rs:98 FAILED with 608 bytes / 1 alloc event (test-workspace.log.txt; INDEX row 3). |
| Overhead numbers ("48 B ≈ 0.00058 % at 1080p, 0.00014 % at 4K", module doc §38-41, part1; pinned here) | wire.rs:1007-1016 | REAL | Arithmetic: 48/8,294,448 = 5.79e-6 (0.00058 %); test asserts < 1e-5; ran ok (log:2355). |

## 7. Dead / duplicated / unreachable code
- `finish()`: pub, zero non-test callers — dead API surface with a live defect (F-001). Either wire it into a transport teardown path or delete it.
- No duplication found in this range; `decode_record` and `next_frame` share `parse_header` and the pool by design.

## 8. Dependencies & flags
- `crate::error::WireError` (error.rs — W13-A01), `media_types::PixelFormat`, `renderer_wgpu::uploads_format` (test-only cross-check), `testkit::Must`. No feature gates, no unsafe, no unsafe-derived claims. Chunk-boundary item: `RecyclePool`/`DecodedFrame`/Drop (part1) are the machinery this range's tests exercise; `RECYCLE_SLOTS = 3` bound holds here (bounded returns at 311-313).

## 9. Open questions for the next agent
- W13-A03 (wire.rs part1): the zero-alloc doc claim at 426-438 ("steady-state path allocates nothing … proven by tests/zero_alloc_transport.rs") is refuted by baseline row 3 — coordinate with F-002 here; root-cause of the 608-byte allocation likely lives in bridge.rs/transport.rs/mailbox.rs (frame path owners).
- Whoever owns transport.rs/mailbox.rs: `finish()` is never called on stream teardown — is an end-of-stream boundary assertion intentionally absent from the mailbox lane? If it should exist, F-001 becomes S1 before wiring.
- tauri shell owners: `bind_frame_channel` (shell.rs:175-195) `unwrap_or_else(|err| unreachable!(...))` on `FrameTap::new()` — panics if the default mailbox config ever becomes invalid; noted here because the chain reaches this chunk's `decode_record`.
