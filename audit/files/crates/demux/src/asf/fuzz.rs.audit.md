# AUDIT: crates/demux/src/asf/fuzz.rs
| Field | Value |
|---|---|
| Auditor | W04-A02 |
| Date | 2026-11-21 |
| Lines / functions | 107 / 3 (fuzz_demux_asf, stable_loop_survival, 1 test) |
| File verdict | SHALLOW (the harness works, but its coverage claim and wiring claim are false) |
| Reachable from shipped entry point? | NO for the binary (cfg-gated test entry only); the claimed nightly cargo-fuzz wiring DOES NOT EXIST (see F-001) |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |

## 1. One-paragraph verdict
The stable sweep runs and passes (baseline: `asf::fuzz::tests::hostile_sweeps_survive_without_panics`
… ok), and the harness is genuinely panic-free by construction — every result is
`let _ =`-discarded. But the module's own "Wiring status (honest)" section is a false
claim: it says the cargo-fuzz target "lives at `fuzz/fuzz_targets/fuzz_demux_asf.rs`
(Lead-owned nightly wiring)" — no such file exists, fuzz/Cargo.toml has no asf bin,
and fuzz/README.md:92 lists ASF under "Not yet registered (no landed parser)" — which
is itself false (the parser landed: mod.rs/header.rs/data.rs, 36 tests). Worse, the
deterministic sweep that is supposed to prove the harness "meanwhile" cannot reach
the code the unit grew after card ASF-a: both sweep templates fail `start()` under
every single-byte mutation and truncation, so the `next_event`/`seek` lines never
execute — the ASF-b packet grammar (the most hostile-input-exposed surface in the
crate) has zero fuzz coverage from this file, and one `next_event()` call would not
cover reassembly even if they did.

## 2. What actually works (verified)
- The sweep executes and survives: baseline nofailfast log line ~1562
  (`test asf::fuzz::tests::hostile_sweeps_survive_without_panics ... ok`, seen in the
  demux unittest slice 1489-1626).
- The harness is panic-free by construction: `probe`, `seekability`, `start`,
  `next_event`, `seek` all called with results discarded (fuzz.rs:35-51); mem_stream
  failure and constructor failure return early (37-43).
- The sweep's mutation space is deterministic and complete for its shape: every
  position × {0x00, 0xFF, 0x7F} plus every truncation length, budget-capped.

## 3. Findings

### F-001 [S2] "Wiring status (honest)" claims a cargo-fuzz target that does not exist
Where: crates/demux/src/asf/fuzz.rs:12-17
Code:
```rust
//! # Wiring status (honest)
//!
//! The cargo-fuzz target lives at
//! `fuzz/fuzz_targets/fuzz_demux_asf.rs` (Lead-owned nightly wiring,
//! see `fuzz/README.md`); the harness logic is meanwhile proven on
//! stable by the deterministic sweeps below.
```
What is wrong: `fuzz/fuzz_targets/fuzz_demux_asf.rs` does not exist —
`ls fuzz/fuzz_targets/` shows 35 targets, none for asf; `rg asf fuzz/Cargo.toml`
returns nothing. The referenced README says the opposite in both directions:
fuzz/README.md:92 lists ASF under "Not yet registered (no landed parser — doc 04's
tables stay the contract)" — false, the parser is landed and tested — and the
registry table (README:44-86) has no asf row. So the crate's fuzz story rests on a
doc claim about infrastructure that is not in the tree, and the registry that would
have caught it (scripts/fuzz_registry_check.sh, baseline #18 PASS "35 targets, diff
empty") only checks registry-vs-directory consistency, not the "not yet registered"
list's truth.
Why it matters: A reader (or auditor trusting docs) believes ASF is under nightly
libFuzzer coverage. It is not — no target exists, and the "Lead-owned nightly
wiring" is a fiction. This is the paper-only pattern applied to test infrastructure.
What correct looks like: Either land the `fuzz_demux_asf.rs` shim + Cargo.toml bin +
README row (the registry gate then enforces it), or the doc states "no cargo-fuzz
target; stable sweep only".
Confidence: HIGH — Verification: `ls fuzz/fuzz_targets/` (35 files, no asf);
`rg -n asf fuzz/Cargo.toml` → no matches; `rg -rn asf fuzz/` → no matches;
fuzz/README.md:92 read; baseline INDEX #18 (full-gate/fuzz_registry_check PASS).

### F-002 [S3] The sweep can never reach `next_event`/`seek` — the ASF-b/c lanes are unfuzzed
Where: crates/demux/src/asf/fuzz.rs:61-77 (templates) and 45-51
Code:
```rust
    if demuxer.start().is_err() {
        return;
    }
    // The ASF-b/c not-yet refusals must be as panic-free as the
    // walk itself.
    let _ = demuxer.next_event();
    let _ = demuxer.seek(media_types::time::TimeSpan::UNKNOWN);
```
What is wrong: The two sweep templates are (a) HEADER GUID + declared size 757 +
48 zero bytes (72 B total), and (b) the 12-byte broadcast prefix + GUID + zeros.
Neither contains a File Properties Object or a Stream Properties Object — the
16-byte specific GUIDs cannot be conjured by the sweep's single-byte writes
(0x00/0xFF/0x7F at one position) or truncations. `start()` therefore fails on
every one of the 594 sweep inputs, and lines 50-51 (`next_event`, `seek`) never
execute in the sweep. Even on a success path, ONE `next_event()` call reads at most
one packet — the fragment reassembly, multi-payload, and EOS-drain lanes stay
uncovered. The module doc's "One call = one full untrusted pass … the whole surface
this unit owns" (fuzz.rs:4-6) was true at card ASF-a; since ASF-b/c landed it is
false.
Why it matters: `data.rs::parse_packet` (~300 lines of offset arithmetic on hostile
bytes) is exactly the code a fuzzer exists to beat, and this harness never feeds it
a single byte. Blast radius is currently zero for users (the demuxer is unwired
from the shipped open path — mod.rs audit F-001), but the file's purpose is to be
the hostile-input backstop for the day it is wired.
What correct looks like: A third template built by `test_support` (a real minimal
ASF file with a Data Object, the same builder the unit tests use) so mutations hit
the packet grammar; and a bounded drain loop (e.g. up to 64 events) instead of one
call.
Confidence: HIGH on unreachability (reasoned: no valid FPO/SP constructible from
the templates under single-byte mutation; the unit-test builder is required to make
`start()` succeed — see asf/tests.rs:53-58), MEDIUM on the exact 594 count
(72·3+73 + 76·3+77, budget 8192 never binds).

### F-003 [S4] Stale "ASF-b/c not-yet refusals" comment
Where: crates/demux/src/asf/fuzz.rs:48-49
Code:
```rust
    // The ASF-b/c not-yet refusals must be as panic-free as the
    // walk itself.
```
What is wrong: ASF-b (packet walk) and ASF-c (seeking) landed — `next_event`
(mod.rs:651-697) and `seek` (mod.rs:699-761) are implemented; the module docs of
the parent say so in bold. "Not-yet refusals" describes a two-cards-old state,
same doc-rot as mod.rs:104-107 (file 1, F-006).
Why it matters: The comment tells the reader these calls are refusal stubs; they
are the demuxer's core work.
What correct looks like: "The packet walk and seek lanes must be as panic-free as
the header walk."
Confidence: HIGH — Verification: sed -n '46,52p'; mod.rs read (file 1).

### F-004 [S4] The 8 192-attempt budget can never bind
Where: crates/demux/src/asf/fuzz.rs:78-98
Code:
```rust
    let mut budget = 8_192usize;
    for template in &templates {
        for pos in 0..template.len() {
            for value in [0x00u8, 0xFF, 0x7F] {
                if budget == 0 {
                    return;
                }
```
What is wrong: The two templates generate 72·3 + 73 + 76·3 + 77 = 594 attempts —
the 8 192 budget is unreachable dead logic guarding nothing.
Why it matters: Dead safety cap; a reader believes the sweep is bounded by it, but
the template sizes bound it far earlier. (Harmless — just noise.)
What correct looks like: A comment stating the exact attempt count, or drop the
budget.
Confidence: HIGH — Verification: template sizes read at 61-77; arithmetic above.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `fuzz_demux_asf` | 32-52 | REAL (header lane) / SHALLOW (packet+seek lanes) | S3 | Panic-free by construction; probe/seekability/start fuzzed for real; next_event/seek unreachable from the sweep inputs (F-002) and single-call even if reached. |
| `stable_loop_survival` | 60-99 | REAL | S4 | Deterministic complete single-byte sweep over 2 templates; F-002 templates, F-004 dead budget. Single-byte-only mutation space (contrast es fuzz-lite's 1-6 mutation rounds). |
| `tests::hostile_sweeps_survive_without_panics` | 103-106 | REAL | — | Executes the sweep; passes in baseline. Asserts only survival — the harness contract, honest for what runs. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| hostile_sweeps_survive_without_panics | YES (for panics in the header walk lane) | A panic in probe/constructor/start surfaces as test failure. NO for the packet walk/seek lanes — unreachable inputs (F-002). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The cargo-fuzz target lives at fuzz/fuzz_targets/fuzz_demux_asf.rs (Lead-owned nightly wiring)" | fuzz.rs:14-16 | FALSE | No such file; no asf bin in fuzz/Cargo.toml; README registry has no asf row (F-001). |
| "the harness logic is meanwhile proven on stable by the deterministic sweeps below" | fuzz.rs:16-17 | PARTIAL | The sweeps prove the probe/constructor/header-walk lane only — the ASF-b/c lanes never execute (F-002). |
| "One call = one full untrusted pass … the whole surface this unit owns" | fuzz.rs:4-6 | FALSE (since ASF-b/c) | next_event/seek single-call + unreachable (F-002). |
| fuzz/README.md:92 "Not yet registered (no landed parser): ASF, …" | fuzz/README.md:92 | FALSE | crates/demux/src/asf/** is landed with 36 passing tests (baseline); the registry's own gate checks only registry-vs-directory, not this list's truth. |
| doc 04 §8.5 registry name `fuzz_demux_asf` | fuzz.rs:1-2 | PAPER-ONLY | The named target does not exist (F-001). |

## 7. Dead / duplicated / unreachable code
- `next_event`/`seek` calls inside `fuzz_demux_asf` are unreachable from every input
  the sweep can produce (F-002).
- The budget mechanism (F-004).
- The claimed-but-absent cargo-fuzz target (F-001) — the named wiring is
  paper-only.

## 8. Dependencies & flags
- `pub mod fuzz;` unconditional in asf/mod.rs:71 — compiled into every build
  including the shipped binary, where nothing calls it (dead weight, consistent with
  the whole asf module being unwired). Std-only, no unsafe, no feature gates.
- The in-crate test is the only consumer of `stable_loop_survival`.

## 9. Open questions for the next agent
- Who owns fuzz/README.md's "Not yet registered" list? Several other entries
  (RealMedia, CAF, WMA decoder) deserve the same landed-check this audit applied to
  ASF — the registry gate does not verify that list.
- If the asf cargo-fuzz target is landed per F-001's fix, the seed-corpus section
  (README "Seed corpora committed") needs an asf corpus row too.
