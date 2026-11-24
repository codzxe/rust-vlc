# CLAIM AUDIT: docs/perf/memory-audit-20261023.md
| Field | Value |
|---|---|
| Auditor | W19-A04 |
| Claims extracted | 11 |
| REAL 7 / REAL-UNPROVEN 2 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 2 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A measurement report whose instrument, register sites, tests, and handoffs all exist in
the tree — and whose D-1 findings were so real that later waves fixed them and cite this
doc's register by number (W6-05 "the lie the old 'no copy' doc-comment told"). The raw
MEMAUDIT/MEMSUM numbers themselves are not re-runnable here (cargo-build budget; corpus
out of repo) but every structural claim is verifiable and checks out. No fabrication.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | instrument `tools/mem-audit`, package `lumen-mem-audit`, on the real engine crates | :5-6 | REAL | ls tools/mem-audit -> Cargo.toml/src/examples present |
| 2 | handoff: root Cargo.toml members array gains `tools/mem-audit` | :306-308 | REAL (done) | Cargo.toml:41 `members = [… "tools/mem-audit", "tools/sweep"]` |
| 3 | handoff: boundaries.toml needs `[crates.lumen-mem-audit]` | :309-310 | REAL (done) | boundaries.toml:674 `[crates.lumen-mem-audit]` (+ comment :666 citing card W5-15) |
| 4 | V-1 site `crates/demux/src/avi/riff.rs:527` `let mut fresh = vec![0u8; want]` — every video packet spills the pool | :238 | REAL | site exists (line drifted; payload-pool spill logic present in riff.rs PayloadSlots; the doc's remediation trail confirms: V-2 fix landed, see #5) |
| 5 | V-2 `packet.rs:16` `SharedBytes::from_vec` "no copy" doc comment "optimistic at the byte level" | :239 | REAL (finding proven + fixed) | crates/media-types/src/packet.rs:12-30 now reads "W6-05 (memory-audit register V-2) grew this from a bare `Arc<[u8]>` … — the lie the old 'no copy' doc-comment told" — the register's finding is confirmed by the fix that removed it |
| 6 | V-3 sites huffman.rs:115 `values.to_vec()`, decoder.rs:248 `vals.clone()`, parser.rs:473 | :240 | REAL | decoder.rs:352 `vals: info.huff_vals.clone()` present; :338 comment "the pre-W2-06 `DhtKey::of`-per-frame clone is gone" — partially remediated since |
| 7 | V-4 decoder.rs:466-467 `out.resize` + `mod.rs:325` FrameStorage::Cpu(SharedBytes::from_vec) | :241 | REAL | decoder.rs:448 `self.out.resize(total, 0)` + :890 `SharedBytes::from_vec(vec![0; len])` (line drift only); zero_alloc_decode.rs exists pinning the decode-event count |
| 8 | tool tests `counted_allocation_moves_counters_by_exact_size` + `fixture_walks_twice_with_stable_peak` | :94, :282-284 | REAL | tools/mem-audit/src/allocator.rs:374 + src/main.rs:212 — both test fns exist |
| 9 | raw MEMNOOP/MEMOVERHEAD/MEMSUM/MEMAUDIT numbers (18-22 ns/pair overhead; peak 5.6 MB for 217 MB file; B_per_pkt figures) | :74-167 | REAL-UNPROVEN | self-reported runs of the real tool; cannot re-run (cargo-build-budget + corpus out of repo); the deterministic-property tests (#8) pin the invariants claimed |
| 10 | corpus `/home/z/my-project/corpus-big` — 545 files, 159 AVI | :62 | UNVERIFIED | out-of-repo path; no corpus committed |
| 11 | "all 30 files under tests/fixtures/generated/{valid,malformed}" | :64-65, §3 | REAL-at-time (superseded) | fixture dirs now hold 33 (valid) + 81 (malformed) files — grown by later waves; the audit ran on the 30 that existed 2026-10-23 |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "first vertical decodes AVI+MJPEG+PCM only — non-AVI files are honest refused rows" (:62-64) | post-6.OPW tree wires 12 containers | both true in their era: the doc is a dated snapshot (2026-10-23) before the open-path wiring landed; its own §7 limitation ("when demuxers are added, re-run the sweep") anticipated exactly this |

## 4. Fabrication / inflation indicators
- None. It publishes its own instrument's overhead (§2), its own limitations (§7), and
  a violation register with file:line citations that later waves verified by fixing.
  The `from_vec` doc-comment lie it called out is now quoted in the fixed source as
  "the lie the old 'no copy' doc-comment told" — the report's authority is corroborated
  by the code that cites it.

## 5. Matrix rows
N/A (perf report; feeds D-1/D-2 budget claims elsewhere).

## 6. Open questions for the next agent
- Re-run `cargo run --release -p lumen-mem-audit -- --self-check` if a cheap build lane
  ever exists, to convert claim 9 to REAL.
- Post-6.OPW re-sweep of the now-wired containers is owed (the doc says so itself, §7).
