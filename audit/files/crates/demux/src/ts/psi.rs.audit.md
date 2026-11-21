# AUDIT: crates/demux/src/ts/psi.rs
| Field | Value |
|---|---|
| Auditor | W05-A08 |
| Date | 2026-11-21 |
| Lines / functions | 606 / 17 |
| File verdict | REAL |
| Reachable from shipped entry point? | YES (call chain: `lumen open` → apps/cli/src/host.rs `open_source` → apps/cli/src/demux_host.rs:458 `crate::containers::open(&container, …)` → apps/cli/src/containers/mod.rs:72 `"mpegts" => ts::open(stream, opts)` → `TsDemuxer::start()` PSI scan → crates/demux/src/ts/mod.rs:917 `match parse_section(&section)`) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 4 S4: 1 |
| Commands run | `rg -n "parse_section|psi::"` (callers: ts/mod.rs:917, ts/tests.rs); `rg -n "Demuxer"` in playback-engine (no concrete dep — selection lives in the CLI host); `sed -n` on every quote below; python3 re-implementation of `build_crc_table`+`crc32_mpeg` (check values); baseline log inspection (orchrun-test-workspace-nofailfast.log.txt lines 1861–1909, 2216, 2835). |

## 1. One-paragraph verdict
This is a genuinely bounded, panic-free PSI/SI section parser: PAT/PMT/SDT walks are cursor-based, every declared sub-length (`program_info_length`, `ES_info_length`, `descriptors_loop_length`, descriptor `len`) is validated against its parent before a slice is taken, the CRC-32/MPEG-2 is bit-exact against the canonical check value, and all damage paths are typed `EngineError`s. It is wired into the shipped `lumen open` path via the `mpegts` container host and its unit tests executed green in the baseline (no-fail-fast log). The defects are peripheral, not structural: a short-form minimum-length check that over-rejects legal tiny sections, DVB charset handling that deviates from EN 300 468 Annex A on the `0x05`/`0x10` selectors (service names in non-Latin charsets garble), SDT services without a 0x48 descriptor silently vanish, and — worst — the CRC algorithm's only pinned "known-answer" test is self-referential (goldens are minted by the function under test), so a polynomial mutation would pass the entire unit suite; the implementation is correct, but the repo's tests cannot prove it.

## 2. What actually works (verified)
— CRC-32/MPEG-2 is bit-exact: re-implemented the exact table build + table-driven update in python3 (same poly 0x04C11DB7, init 0xFFFFFFFF, no reflect/xorout) → `crc32_mpeg(b"123456789") == 0x0376E6E7` (the published CRC-32/MPEG-2 check value), `crc32_mpeg(32 zero bytes) == 0x4A55AF67`, `crc32_mpeg(empty) == 0xFFFFFFFF` — matching the Rust code's formulation line-for-line.
— Bounded-memory discipline verified by reading: PMT ES loop (psi.rs:373-389), SDT service loop (psi.rs:411-424), descriptor walk (psi.rs:437-475) — every step checks `at + n <= parent.len()` before slicing; every iteration advances `at` by a positive constant; no `unwrap`/`expect`/index-without-check anywhere in the file.
— Unit tests for this file executed and passed in the baseline: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` lines 1887–1907 (`ts::tests::crc32_matches_the_corpus_shape … ok`, `pat_parses_program_entries … ok`, `pmt_parses_pcr_and_es_loop … ok`, `sdt_parses_service_names_the_5_byte_loop_header … ok`, `dvb_text_prefixes_and_composition … ok`, …).
— Reachability: `rg -n '"mpegts" =>' apps/cli/src/containers/mod.rs` → line 72; demux_host.rs:458 `crate::containers::open(&container, stream, &opts)`; ts/mod.rs:917 `match parse_section(&section)` inside the PSI feeding path of the demuxer start scan (ts/mod.rs:1058 `while self.walk + PACKET_BYTES as u64 <= scan_end && !self.psi_complete()`).

## 3. Findings

### F-001 [S3] Short-form sections with `section_length` 1–4 are over-rejected as `bad_structure`
Where: crates/demux/src/ts/psi.rs:285-288
Code:
```
    if section_length < 5 {
        // The long-form remainder (5 fixed bytes + 4 CRC) cannot fit.
        return Err(bad_structure(0));
    }
```
What is wrong: this check runs before the `section_syntax_indicator` branch (psi.rs:297-302), so it applies to short-form sections too — but its own comment justifies the number with long-form arithmetic (5 fixed bytes + 4 CRC = a minimum `section_length` of 9, which the later `whole < 12` check at psi.rs:303-306 already enforces exactly). A short-form section (`syntax == 0`) with `section_length` 1–4 is structurally legal per ISO/IEC 13818-1 §2.4.4 and would be consumed as `Skipped`; here it is typed `demux.ts.psi.bad-structure`.
Why it matters: correctness drift on a spec edge; a conforming-but-tiny private/DVB section is reported as structural damage instead of being skipped. Mitigating: the caller (ts/mod.rs:921-925) drops a damaged section with a note and continues the walk, so playback is unaffected — metadata-only blast radius.
What correct looks like: enforce the ≥9 minimum only on the `syntax == 1` path (it already is, via `whole < 12`), and let short-form sections through with `section_length >= 0`.
Confidence: HIGH — Verification: read (control-flow order verified with sed; caller drop-path read at ts/mod.rs:916-925).

### F-002 [S3] DVB charset selectors deviate from EN 300 468 Annex A: `0x05` mislabeled ISO 6937, `0x10` always decoded as UTF-16-BE, 0x01–0x04 as Latin-1
Where: crates/demux/src/ts/psi.rs:491-498
Code:
```
            0x05 => {} // ISO 6937 — the default shape below.
            0x15 => return String::from_utf8_lossy(text).into_owned(),
            0x10 => {
                // Two selector bytes follow, then ISO 10646 text —
                // approximated as UTF-16-BE (annex A.3's default).
                let skip = text.len().min(2);
                text = &text[skip..];
                return decode_utf16be_lossy(text);
```
What is wrong: (a) Annex A.1 assigns `0x05` to ISO 8859-9 (Turkish Latin-5), not ISO 6937 — Turkish service names with Ğ/İ/Ş/ğ/ı/ş (8859-9 bytes 0xD0/0xDD/0xDE/0xF0/0xFD/0xFE) are rendered with the wrong Latin-1 glyphs; the module doc (psi.rs:42-43) repeats the wrong mapping as if intended, so the gap is not even on the doc's own gap list. (b) `0x10` + 2-byte code selects a table by those bytes (e.g. `0x10 0x00 0x01` = ISO 8859-1) — decoding every `0x10` payload as UTF-16-BE turns single-byte text into mojibake; this one IS documented as an approximation (psi.rs:44-46) and even pinned by a test using an 8859-15 code (ts/tests.rs:190: `dvb_text(&[0x10, 0x00, 0x0B, 0x00, 0xC9]) == "É"`). (c) `0x01`–`0x04` (Cyrillic/Arabic/Greek/Hebrew, 8859-5/6/7/8) fall to the Latin-1 reading — pure garbage for those scripts, documented as "approximated as Latin-1".
Why it matters: wrong behavior in realistic DVB streams (non-Latin broadcasters) — but confined to SDT provider/service name display; playback never refuses. By the module's own design intent ("a garbled service name is damage to surface, not a stream to refuse", psi.rs:47-48).
What correct looks like: at minimum map `0x05` → Latin-1-family reading (it is 8859-9), and decode `0x10`'s two code bytes (0x00 0x01 = 8859-1 …) instead of assuming 10646.
Confidence: MEDIUM (spec tables from auditor knowledge; no in-repo authority) — Verification: read + EN 300 468 Annex A structure; no network/spec copy in sandbox for a byte-level citation.

### F-003 [S3] The CRC known-answer test is self-referential — a polynomial mutation passes the whole suite
Where: crates/demux/src/ts/tests.rs:30-32 (assertion) and crates/demux/src/ts/test_support.rs:82-84 (golden generation)
Code:
```
    // no reflection): 32 zero bytes → 0x4E08BFB4? — the corpus's own
    // section CRCs are the authority here; pin one exactly.
    assert_eq!(crc32_mpeg(&[]), 0xFFFF_FFFF);
```
and
```
    out.extend_from_slice(body);
    let crc = crc32_mpeg(&out);
    out.extend_from_slice(&crc.to_be_bytes());
```
What is wrong: every "good CRC" golden in the PSI tests is computed by `crc32_mpeg` itself (test_support.rs:83) and verified by `section_crc_ok` → the same function — a tautology with respect to the algorithm. The one assertion that could pin the algorithm asserts `crc32_mpeg(&[]) == 0xFFFF_FFFF`, which holds for any init-carrying CRC regardless of polynomial. The comment above it admits the check-value lane was never pinned (note the literal "0x4E08BFB4?" question mark — the true value for 32 zero bytes is 0x4A55AF67, and for "123456789" is 0x0376E6E7; both computed this session by re-implementing the code's exact algorithm). The declared real authority ("the corpus's own section CRCs") sits behind the corpus gate at `/tmp/my-project/corpus-v4/media` (ts/test_support conventions, apps/cli/src/containers/ts.rs:1017), which does not exist outside the sandbox, and its skip notes go to stderr where cargo's default capture swallows them — the baseline log shows `corpus_ts_lanes_match_the_ffprobe_truth … ok` (line 2216) and `containers::ts::tests::corpus_ts_lanes_through_the_open_path … ok` (line 2835) with no way to distinguish ran from skipped.
Why it matters: test-honesty — the suite cannot detect a CRC regression (wrong poly/reflection) in psi.rs; only out-of-sandbox corpus hosts could. The implementation itself is currently CORRECT (verified: canonical check value 0x0376E6E7).
What correct looks like: one literal assertion `assert_eq!(crc32_mpeg(b"123456789"), 0x0376_E6E7);` (or the 32-zeros value 0x4A55AF67) in ts/tests.rs.
Confidence: HIGH — Verification: read both sites; python3 re-implementation of the exact algorithm produced the canonical CRC-32/MPEG-2 check value.

### F-004 [S3] SDT services without a 0x48 descriptor are silently dropped from the parsed surface
Where: crates/demux/src/ts/psi.rs:420-423 and crates/demux/src/ts/psi.rs:473-476
Code:
```
        let descriptors = &body[at + 5..at + 5 + dl];
        if let Some(svc) = parse_service_descriptor(descriptors, service_id) {
            services.push(svc);
        }
```
and (the descriptor walk's fall-through)
```
        at += 2 + len;
    }
    None
}
```
What is wrong: `parse_service_descriptor` returns `Some` only when it finds a 0x48 (service) descriptor; a service whose descriptor loop carries only other descriptors (e.g. 0x50 linkage, 0x51 short-event-less private) is parsed, validated — and then not pushed at all. Its `service_id` disappears from `Sdt::services` with no counter, note, or `Skipped` trace.
Why it matters: silent metadata loss on legal EN 300 468 streams; the demux stats surface (`psi_sections`, `psi_skipped_sections`) cannot reveal it either, since the section itself parsed fine.
What correct looks like: push a nameless `SdtService { service_id, service_type: 0, provider: None, name: None }` (the struct already supports `None` names) or count dropped services in a stat.
Confidence: HIGH — Verification: read.

### F-005 [S4] `parse_pat` indexes before its own bounds check, and half that check is dead
Where: crates/demux/src/ts/psi.rs:334-338
Code:
```
    let transport_stream_id = u16::from_be_bytes([body[3], body[4]]);
    let mut entries = Vec::new();
    let mut at = 8;
    if body.len() < 8 || !body.len().is_multiple_of(4) {
        return Err(bad_structure(8));
    }
```
What is wrong: `body[3]`/`body[4]` are read before the `body.len() < 8` guard, so the guard could not protect them even if it could fire — and it cannot fire: the only caller (`parse_section`, psi.rs:303-310) already guarantees `whole >= 12` ⇒ `body.len() >= 8`. The dead half of the check misleads a reader into thinking the indexing above is guarded.
Why it matters: hygiene/ordering debt in a parser whose selling point is "the parser trusts nothing"; a future caller copy-pasting `parse_pat` with a weaker precondition gets an index panic the check pretends to prevent.
What correct looks like: move the length/multiple-of-4 validation above the `body[3]`/`body[4]` reads, or drop the dead half with a comment citing the caller's invariant.
Confidence: HIGH — Verification: read (caller invariant at psi.rs:303-310: `whole < 12 → Err`, `body = &buf[..whole - 4]`).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `build_crc_table` (const) | 68-86 | REAL | — | Verified by re-implementation: yields the canonical CRC-32/MPEG-2 (poly 0x04C11DB7 MSB-first) check value 0x0376E6E7 for "123456789". |
| `crc32_mpeg` | 91-97 | REAL | — | Table-driven MSB-first; exact re-implementation matched canonical check values (0x0376E6E7 / 0x4A55AF67 / 0xFFFF_FFFF empty). |
| `section_crc_ok` | 102-104 | REAL | — | Appended-CRC zero-hash property; exercised by tests + baseline. |
| `mint` | 108-110 | REAL | — | Trivial error constructor, stage Demux. |
| `not_psi` | 113-123 | REAL | — | Sniff capped at 16 bytes (`sniffed[..sniffed.len().min(16)]`) — no payload leak into errors. |
| `truncated` | 128-135 | REAL | — | Carries expected/found counts. |
| `bad_structure` | 140-149 | REAL | — | Offset-bearing; reused for F-001's over-rejection (not its fault). |
| `crc_mismatch` | 154-162 | REAL | — | Offset via `SectionOffset` source chain, no section bytes in the error (L0 no-secrets rule). |
| `SectionOffset::fmt` / `impl Error` | 169-174 | REAL | — | Trivial display. |
| `parse_section` | 278-329 | REAL | S3 | Correct dispatch + CRC + current_next handling; F-001 over-rejects short-form `section_length` 1–4. |
| `parse_pat` | 331-353 | REAL | S4 | Correct 4-byte stride walk, `& 0x1FFF` mask; F-005 dead/misordered guard. |
| `parse_pmt` | 355-396 | REAL | — | Fully bounded ES loop; 12-bit masks correct; typed truncations. Clean. |
| `parse_sdt` | 398-431 | REAL | S3 | 5-byte service-loop header verified correct (service_id 16 + flags 8 + 12-bit loop length in 16); F-004 drops 0x48-less services. |
| `parse_service_descriptor` | 435-476 | REAL | S3 | Bounded walk, lying-length safe (`return None`); only 0x48 recognized — F-004. |
| `dvb_text` | 481-522 | REAL | S3 | No panic paths; F-002 charset deviations (0x05/0x10/0x01-0x04). |
| `decode_utf16be_lossy` | 524-532 | REAL | — | Odd trailing byte silently dropped (acceptable lossy behavior); no panics. |
| `compose_iso6937` | 538-606 | REAL-UNPROVEN | — | UNVERIFIED against ISO 6937-2: no spec copy or in-repo authority for the accent-lead assignments (0xC1=acute, 0xC2=grave, 0xC8=diaeresis, 0xCB=cedilla, 0xCC=caron); the pinned test (ts/tests.rs:185-186, `0x05 0xC1 0x45 → "É"`) only proves internal consistency. Table is total (no panics), fallback safe. |

## 5. Test honesty (tests live in crates/demux/src/ts/tests.rs + test_support.rs — W05-A02's files; assessed for psi coverage)
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `crc32_matches_the_corpus_shape` | PARTIAL | Bit-flip half: yes. Known-answer half: NO — goldens minted by `crc32_mpeg` itself (test_support.rs:83); poly mutation passes; only `crc32_mpeg(&[]) == 0xFFFF_FFFF` pinned (init constant, true for any poly). F-003. |
| `pat_parses_program_entries` | YES | Two entries, distinct PIDs; a stride/field-order mutation fails the asserts. |
| `pmt_parses_pcr_and_es_loop` | YES | PCR PID, program_info bytes, two ES entries with ISO-639 es_info all asserted. |
| `sdt_parses_service_names_the_5_byte_loop_header` | YES | Two services with distinct names — the exact regression a 6-byte header read would break. |
| `cat_and_future_and_short_sections_skip` | YES for CAT/future; NO for short-form minimums | No test constructs a short-form section with `section_length` 1–4, so F-001 is untested. |
| `crc_mismatch_is_typed` | YES | Last byte XOR 0xFF must produce `demux.ts.psi.crc-mismatch`. |
| `truncated_and_bad_structure_are_typed` | YES | Declared-vs-present mismatch, non-multiple-of-4 PAT body, 2-byte stub all pinned to codes. |
| `dvb_text_prefixes_and_composition` | YES for the pinned paths | ASCII, 0x05+compose, 0x15, 0x10, Latin-1 fallback pinned; the 0x01–0x0B selector family is untested (no case exercises it). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "every declared sub-length … is clamped to its parent before it is consumed — a lying length is a typed truncation, never an over-read or a loop" | psi.rs module doc, lines 17-23 | REAL | Read: pmt loop 373-389, sdt loop 411-424, descriptor walk 437-445 all check bounds before slicing; all steps advance. |
| "The SDT service-loop header is 5 bytes … a prior session read 6; the corpus SDT bytes pin 5" | psi.rs doc lines 25-31 | REAL | parse_sdt:415-416 reads service_id(2)+flags(1)+12-bit length(2) = 5; EN 300 468 §5.2.3 layout matches; regression test ts/tests.rs:82-110 green in baseline. |
| "prefix `0x05` → the same after stripping" (as if 0x05 were the 6937 default) | psi.rs doc lines 40-41 | FALSE | EN 300 468 Annex A.1 assigns 0x05 = ISO 8859-9; the code comment psi.rs:491 repeats it. F-002. |
| feature-matrix row `container-mpegts`: "Demux MPEG-TS: PSI/SI (PAT/PMT/SDT), multi-program, PES timing, discontinuities, seek by timing scan", status implemented (win/linux/macos) | docs/feature-matrix.toml:168-178 | REAL for the PSI/SI half (this file: code + green baseline tests); the "seek by timing scan" phrase contradicts `SeekSupport::None` + `demux.ts.seek.unsupported` (apps/cli/src/containers/ts.rs:66-73, 548, test at 788) — flagged for the matrix-rows agent, outside this file's scope. |
| "5 ts corpus lanes" evidence (5.REC-1) | docs/feature-matrix.toml:177 + tests/ts_pes_corpus.rs doc | REAL-UNPROVEN | Corpus lives at `/tmp/my-project/corpus-v4/media` (sandbox-only); baseline log shows the corpus tests `ok` but a loud-skip also reports `ok` with the note on hidden stderr — ran-vs-skipped indistinguishable from the committed evidence. |

## 7. Dead / duplicated / unreachable code
- `parse_pat`'s `body.len() < 8` half-check: dead (caller guarantees ≥ 8) — F-005.
- `TID_CAT` is exported and used by ts/mod.rs:71 (CAT skip note) — not dead.
- No duplication found; no feature gates in this crate (crates/demux/Cargo.toml: no `[features]` — everything compiles unconditionally into the shipped binary).

## 8. Dependencies & flags
- Crate deps: `media-types` + `plugin-api` only (crates/demux/Cargo.toml `[dependencies]`); std-only otherwise. No feature flags gate this file — it ships in every build of `lumen`.
- `media_types` error taxonomy used for all typed errors (stage `Demux`) — consistent with the crate's no-redefine rule.

## 9. Open questions for the next agent
- UNVERIFIED: `compose_iso6937`'s accent-lead assignments vs ISO 6937-2 (0xC1 acute? 0xC2 grave? cedilla at 0xCB vs 0xCA?). No spec copy in sandbox; a spec-owning agent should diff the table. If the common layout (0xC1=grave, 0xC2=acute) is the true one, acute/grave names are swapped and every composed grave/acute name is wrong.
- Does any shipped surface actually display SDT provider/name? (ts/mod.rs `pending_services` → CLI host surface.) If nothing renders them, F-002/F-004 blast radius drops to zero — verify in the UI/CLI wave.
- Corpus tests: `corpus_ts_lanes_*` "ok" in baseline cannot be distinguished from loud-skip (stderr swallowed by cargo's capture). REQUEST-ORCH-RUN: `cargo test -p demux --test ts_pes_corpus -- --nocapture 2>&1 | head -40` on a corpus-enabled host to pin whether the differential ran.
