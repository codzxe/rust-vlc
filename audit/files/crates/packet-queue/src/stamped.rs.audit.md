# AUDIT: crates/packet-queue/src/stamped.rs
| Field | Value |
|---|---|
| Auditor | W03-A01 |
| Date | 2026-11-21 |
| Lines / functions | 184 / 13 units (4 methods, 2 traits, 2 trait impls, 6 tests + 1 fixture) |
| File verdict | REAL (implemented, reachable from the shipped `lumen` CLI through the playback pipeline, executed in baseline smoke runs, tests green) |
| Reachable from shipped entry point? | YES — `crates/playback-engine/src/pipeline.rs:105-106` imports `Stamped` (with BoundedQueue/QueueLimits) and `pipeline.rs:2201` + `2875` push `Stamped::from_payload(packet)` into the per-limb queues; playback-engine is the engine of the shipped CLI. Executed: AVI first-vertical lifecycle (avi-firstvertical-open-session.txt) and the MP4 smoke matrix push packets through this path. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | `sed -n 1,184p` (full file, twice); `rg -n "fn \|impl \|trait \|struct"` (inventory); Read-verification of `#[must_use]` sites (Bash display mangles them); `rg -l "Stamped\|Generational\|Weigh"` across crates/apps (consumer scan); `rg -n "packet-queue"` in Cargo.tomls (dependency chain: playback-engine/Cargo.toml:39). |

## 1. One-paragraph verdict
A small, clean vocabulary file: `Stamped<T>` (generation + payload, with the
strictly-past staleness rule), the `Generational` seam (`Packet` copies its
own stamp so the two can never drift), and the `Weigh` seam (payload bytes +
known-duration mapping with the documented zero-for-unknown policy). Every
method is trivial, total, panic-free and tested; the six tests assert all
three staleness orderings, the byte accounting, and both duration mappings
including the None field. I found nothing wrong. What was checked and how is
listed in §2 and the table — this is a zero-findings file by verification,
not by omission.

## 2. What actually works (verified)
— Staleness semantics: `is_stale` is strictly `<` (equal = current = not stale; future = ordering bug elsewhere, not staleness) — matches the doc 01 §3.3 `Packet::is_stale` contract the module cites; test `stamped_is_stale_is_strictly_past` asserts all three cases (lines 141-150).
— Duration mapping: `Known(n) → n`, `Unknown → 0`, `duration: None → 0` — all three asserted in `packet_weigh_maps_unknown_duration_to_zero` (173-183) and `packet_weigh_maps_known_duration` (163-171).
— Byte accounting: payload bytes only, with the documented rationale that the packet cap carries the fixed metadata — asserted in `packet_weigh_reports_payload_bytes` (157-161).
— Tests execute green in the no-fail-fast baseline: audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt (4534/7/49; no packet-queue test among the 7 failures).

## 3. Findings
None. Zero-findings justification — everything checked, with method:
1. Every method body read against its doc comment (sed + Read for the
   `#[must_use]` lines the Bash display mangles) — no divergences.
2. `is_stale` ordering traced against `GenerationId` (FIRST + next + raw new(0))
   — consistent with the test's three-way assertion.
3. `Weigh for Packet` arithmetic traced: `self.data.len()` (no overflow —
   usize); `Nanoseconds(0)` for both unknown shapes.
4. Panic scan: no unwrap/expect/index/slice/div in the file (the only `.must()`
   calls are in tests on results that cannot fail — construction of ids).
5. Reachability proven by call chain (pipeline.rs:2201/2875).
6. Test-honesty: every test fails under its corresponding mutation (change `<`
   to `<=` in `is_stale` → the equal-case assert trips; map Unknown to 1 → the
   zero-assert trips; count metadata in size_bytes → 1500-assert trips).
7. Duplication/dead-code scan: `Stamped`/`Weigh`/`Generational` have exactly
   one production consumer shape (the generic queue in queue.rs + pipeline.rs);
   no orphan symbols.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `struct Stamped<T>` | 22-27 | REAL | — | Copy/Hash/Eq derives — cheap value semantics |
| `Stamped::new` | 31-34 | REAL | — | explicit stamp; `#[must_use]` |
| `Stamped::from_payload` | 41-50 | REAL | — | copies the payload's own generation (the no-drift rule); named per clippy::self_named_constructors |
| `Stamped::into_value` | 53-55 | REAL | — | discards stamp; `#[must_use]` |
| `Stamped::is_stale` | 58-61 | REAL | — | strictly past; `#[must_use]` |
| `trait Generational` + `impl for Packet` | 69-78 | REAL | — | delegates to the packet's own field — the single authority |
| `trait Weigh` | 86-96 | REAL | — | contract documented: pure + total, called once per push outside the queue's critical section |
| `impl Weigh for Packet::size_bytes` | 98-103 | REAL | — | payload bytes only (documented constant-metadata rationale) |
| `impl Weigh for Packet::media_duration` | 105-112 | REAL | — | Known→n, Unknown/None→0 (doc 03 §4.4(f) [PROPOSAL] policy — unknown must not wedge the duration limit) |
| tests (6) + `packet` fixture | 118-184 | REAL | — | all mutation-sensitive (see §2); fixture builds a full 10-field Packet literally — breaks loudly if the type drifts |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `from_payload_stamps_from_the_payload_generation` | YES | exact generation equality on both the stamp and the payload accessor |
| `stamped_is_stale_is_strictly_past` | YES | three-way ordering (equal/future not stale, past stale) — any comparison change trips one arm |
| `stamped_into_value_discards_the_stamp` | YES | value equality |
| `packet_weigh_reports_payload_bytes` | YES | exact 1500 |
| `packet_weigh_maps_known_duration` | YES | exact 3_333_333 ns |
| `packet_weigh_maps_unknown_duration_to_zero` | YES | both Unknown and None mapped to 0 |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the producer stamps the current token generation … the queue treats the stamp as the single authority" (doc 03 §4.4(b)/§5 cp3) | stamped.rs:8-12 | REAL | from_payload copies the payload's field; pipeline pushes via from_payload (pipeline.rs:2201) |
| "unknown-duration items count as 0 toward the duration limit — they must not wedge it" (doc 03 §4.4(f) [PROPOSAL]) | stamped.rs:90-94 | REAL | media_duration mapping + test 173-183 |
| "`gen` is a reserved keyword in edition 2024" | stamped.rs:16-18 | REAL | Rust 2024 reserves `gen`; the file compiles under the workspace's edition with `generation` |
| "queues are bounded in packets, bytes and duration" (doc 03 §4.4) | stamped.rs:84-86 | REAL-UNPROVEN (this file) | the axes are enforced in queue.rs/limits.rs (other agents' files); this file supplies the weights |

## 7. Dead / duplicated / unreachable code
- None. `Stamped`/`Weigh`/`Generational` all consumed (queue.rs generic +
  playback-engine pipeline). The `Hash` derive is consumed by queue
  internals/tests; no orphan symbols found.

## 8. Dependencies & flags
- media-types only (GenerationId, Packet, Nanoseconds, Duration) + testkit in
  tests. No unsafe, no features, no platform gating. Mirrors the packet-queue
  crate's zero-external-deps discipline (per its Cargo.toml audit, W01).

## 9. Open questions for the next agent
- queue.rs owner: the "pure and total, called once per push outside the
  critical section" contract (stamped.rs:88-90) — verify the queue actually
  calls Weigh before taking the lock (a Weigh call under the lock would run
  user code under the queue's lock, violating the stated invariant).
- pipeline.rs:2201-2216 handles three LimitAxis refusals — confirm each axis
  actually maps to the Weigh weights defined here.
