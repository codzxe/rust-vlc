# AUDIT: crates/demux/src/mp4/fuzz.rs

| Field | Value |
|---|---|
| Auditor | W05-A03 |
| Date | 2026-11-12 |
| Lines / functions | 84 / 2 production functions + 3 consts |
| File verdict | REAL (a genuine bounded survival harness; its *campaign* is scaffolded, not gated — see MF-001) |
| Reachable from shipped entry point? | NO (by design — it is a fuzz harness). The module is `pub mod fuzz;` **ungated** (mp4/mod.rs:100), so it compiles into the production `demux` rlib, but its only callers are the cargo-fuzz shim `fuzz/fuzz_targets/fuzz_demux_mp4.rs:19`, `mp4/tests.rs:1083-1108` (`#[cfg(test)]`), and `tests/mp4_fixtures.rs:247`. Its *subject* (`Mp4Demuxer`) is reachable: CLI → `apps/cli/src/containers/mod.rs` mp4 lane. |
| Findings | S0:0 S1:0 S2:0 S3:2 S4:2 |

Commands run (adaptation: no cargo; static analysis + `sed -n` quote verification):
- `sed -n` verification of every quote below (verbatim)
- Grep for `fuzz_demux_mp4` across the repo (all call sites listed above); `ls .github/workflows`; read `fuzz/README.md` (full), `fuzz/fuzz_targets/fuzz_demux_mp4.rs`, `scripts/quality/25_fuzz_campaign.sh`, `scripts/quality/16_fuzz.sh`, `.github/workflows/audit.yml` (fuzz job 323-359), `.github/workflows/ci.yml` (step 6, lines 244-246)
- Cross-read: `mp4/mod.rs` (ceilings, `seek`/`next_event`/absent `reanchor`), `media-types/src/time.rs:63-90` (TimeBase::new), `plugin-api/src/demux.rs` (seek purity contract)
- `ls docs/audit/reports/fuzz/` → does not exist (no committed campaign evidence)

## 1. One-paragraph verdict

The harness itself is real and honestly documented: one pass = construct → `start` → bounded event drain (500 k cap) → 3 data-derived seek probes each followed by a 64-event drain, with every error swallowed because the contract is survival (no panic/hang/unbounded alloc), nothing else. The budgets it advertises exist in the demuxer (verified: 16 MiB moov / 1 MiB moof / 16 MiB samples / 65 536-box fragment scan). The problem is execution: nothing that runs on stable ever feeds this harness random input — the merge gate only checks the fuzz *registry* (names/files/corpus existence), the seeded random loop covers AVI only, and the sole automated campaign path is a nightly-only `audit.yml` job whose actual runs have no in-tree evidence; the repo's own fuzz README admits the doc-04 "10 minutes clean in CI" run "has happened" never. Additionally, `reanchor` — the trait's acting seek half — is never exercised and is in fact the trait-default refusal for MP4, so the harness's seek coverage silently depends on MP4's `seek` violating its purity contract.

## 2. What actually works (verified)

- Both drain loops are hard-capped: `events += 1; if events > FUZZ_EVENT_BUDGET { return; }` before every `next_event` (lines 57-60, 72-76) — no unbounded loop regardless of demuxer behavior.
- Every demuxer error is swallowed by name (`Ok(DemuxEvent::EndOfStream) | Err(_) => break`) — matches the documented survival contract (lines 6-10).
- The advertised demuxer ceilings are real code, not doc: `MAX_SAMPLE_BYTES = 16 MiB` (mp4/mod.rs:109), `MAX_MOOV_BYTES = 16 MiB` (113), `MAX_MOOF_BYTES = 1 MiB` (117), `MAX_FRAGMENT_SCAN_BOXES = 65_536` (129, enforced at 717/849/1778/2166); the 2-slot pool at mp4/mod.rs:614.
- The seek-probe section does exercise the stateful seek path for MP4: `Mp4Demuxer::seek` → `seek_progressive` commits per-track cursors, clears `pending`, and sets `eos = false` (mp4/mod.rs:2127-2142), so the post-seek drain genuinely re-drains from the landing — verified by reading the commit block.
- Stable-runnable invocations exist and are wired into the workspace test suite: `mp4/tests.rs:1083-1108` (5 fixed hostile inputs) and `tests/mp4_fixtures.rs:247` — so the harness body does execute on every `cargo test` (their assertion-freeness is a caller-side issue, see §5).

## 3. Findings

### MF-001 [S3] The fuzz target is wired to a nightly-only CI job that has no in-tree evidence of ever running; the merge gate never executes it
Where: crates/demux/src/mp4/fuzz.rs:45 (the entry), and the wiring chain: `.github/workflows/ci.yml:244-246` (merge gate), `.github/workflows/audit.yml:325-351` (nightly fuzz job), `fuzz/README.md:165-168` (the admission)
Code (the merge gate's only fuzz step):
```yaml
      - name: "6. fuzz-registry (full_gate.sh #6; doc 04 §8.5)"
        run: bash scripts/fuzz_registry_check.sh
```
(the nightly job that would actually run it):
```yaml
  fuzz:
    if: github.event.schedule == '0 2 * * *' || (github.event_name == 'workflow_dispatch' && inputs.tier == 'fuzz')
```
and the repo's own status note (fuzz/README.md:165-168):
```
- `-- -max_total_time=600` is the doc-04 "runs 10 minutes clean in
  CI (release mode)" shape. **No such run has happened yet** — the
```
What is wrong: `scripts/fuzz_registry_check.sh` (the only fuzz-related step in the merge gate `ci.yml`) verifies target names, shim files, README rows, and corpus presence — it never *executes* a target. The real campaign (`scripts/quality/25_fuzz_campaign.sh` → `cargo fuzz run`, 5 shards × 180 s) lives only in `audit.yml`'s nightly job. Three independent signals say that campaign has never produced evidence: `docs/audit/reports/fuzz/` (the job's artifact destination) does not exist in-tree; `fuzz/artifacts/` contains no crash regressions; and the README itself says no doc-04-shaped run has happened. On stable, this harness sees only 6 fixed inputs (5 in `mp4/tests.rs`, 1 in `mp4_fixtures.rs`); the seeded-random loop (`crates/demux/tests/fuzz_loop_stable.rs`) exercises `fuzz_demux_avi` only — grep verified, `fuzz_demux_mp4` appears nowhere in it.
Why it matters: The MP4 demuxer is the project's litmus-test failure area (W00 baseline: h264+aac MP4 opens with 0 frames decoded). Its fuzz harness — the tool that exists precisely to shake out panics in hostile MP4 bytes — has never run against coverage-guided input as far as the tree can prove. "Scaffolding, not wired" is the README's own honest label; the audit answer to "does this fuzz target actually get run anywhere" is: **a scheduled job exists in-tree (REAL wiring), execution is UNPROVEN (no artifacts, no reports, README admission), and no gate blocks a merge on it.**
What correct looks like: promote a smoke campaign (e.g., 60 s/target, `-s none`, stable-compatible build) into the merge gate, or commit the first campaign report under `docs/audit/reports/fuzz/` so the claim becomes checkable.
Confidence: HIGH (wiring facts) — Verification: read; the "has the nightly job ever run" question is UNVERIFIED (Actions history is not in the tree).

### MF-002 [S3] `reanchor` is never fuzzed — and for MP4 it is the trait-default refusal; the harness's seek coverage rides on a contract violation
Where: crates/demux/src/mp4/fuzz.rs:71 (`mp4.seek(...)` — the only seek call), crates/demux/src/mp4/mod.rs:2127-2142 (the state-mutating commit), crates/plugin-api/src/demux.rs:72-75 (the violated contract)
Code (fuzz.rs:71):
```rust
        if mp4.seek(span(at)).is_ok() {
```
Code (what makes it work — mp4/mod.rs:2141-2142, inside `seek_progressive`):
```rust
        self.pending.clear();
        self.eos = false;
```
What is wrong: The harness never calls `Demuxer::reanchor`. For MP4 that call would hit the trait default — a typed `Unsupported` refusal — because `mp4/mod.rs` implements no `fn reanchor` (grep: zero matches). The only reason the seek-probe section exercises anything stateful is that `Mp4Demuxer::seek` **mutates** cursor/pending/eos, which the trait contract forbids ("Does **not** move the read position", plugin-api/src/demux.rs:72-75).
Why it matters: Two coupled defects from this file's perspective: (1) the stateful seek path is fuzzed only through the contract-violating entry — if someone fixes `Mp4Demuxer::seek` to be pure per contract, the post-seek drains start returning `EndOfStream` immediately (`next_event`'s eos short-circuit, mp4/mod.rs:2026-2028) and the harness's entire seek section silently becomes a no-op with no test failing; (2) the engine's documented acting half (`reanchor`, doc 03 §6 step 4) has zero fuzz coverage in the MP4 lane — which is moot today only because MP4 `reanchor` is itself unimplemented. The mp4/mod.rs side of this (missing `reanchor` + impure `seek`) belongs to that file's owner; recorded here because this harness is the only caller depending on the violation.
What correct looks like: the harness probes `reanchor` (and `seekability()`) explicitly; the demuxer implements both halves per contract.
Confidence: HIGH — Verification: read (grep `fn reanchor` in mp4/mod.rs: no matches; seek commit block quoted; trait contract quoted).

### MF-003 [S4] Dead fallback in `span()` that bypasses `TimeBase`'s gcd reduction
Where: crates/demux/src/mp4/fuzz.rs:36
Code:
```rust
    let base = TimeBase::new(1, 1_000_000_000).unwrap_or(TimeBase { num: 1, den: 1 });
```
What is wrong: `TimeBase::new(1, 1_000_000_000)` cannot fail — the constructor errors only on a zero component (media-types/src/time.rs:77-84). The `unwrap_or` arm is dead, and its struct-literal fallback (`num: 1, den: 1` — a 1-second tick base, 10⁹× coarser) both bypasses the constructor's validation and would silently change every seek target's meaning if it were ever reachable.
Why it matters: Noise that implies a failure mode that cannot occur; a future edit to the arguments could make the fallback live without anyone noticing the semantic jump.
What correct looks like: `.unwrap()`-free but honest: `TimeBase::new(1, 1_000_000_000).unwrap_or_else(|_| unreachable!())` is banned by repo lint posture; better, hoist a `const`/`OnceLock` ns base or propagate the error as the harness's early return.
Confidence: HIGH — Verification: read (constructor logic quoted at time.rs:77-84).

### MF-004 [S4] Seek-target derivation is 3 low-entropy bytes; the `as i64` cast is safe only by accident of the multiplier
Where: crates/demux/src/mp4/fuzz.rs:68-70
Code:
```rust
        let at = data
            .get(data.len().saturating_sub(probe + 1))
            .map_or(0, |b| u64::from(*b) * 10_000_000);
```
What is wrong: Targets come from the last three input bytes × 10⁷ ns — 765 distinct values, all ≤ 2.55 s, and 0 for empty input. That is a legitimate "data-derived" knob, but it explores a tiny slice of the seek domain (no negative, no beyond-duration, no mid-fragment targets — those only arrive via libFuzzer mutating bytes that happen to be file-duration-relevant). Downstream, `span(at)` does `nanos as i64` (line 38), which is overflow-safe only because of the ×10⁷ cap.
Why it matters: Under a pure survival contract this is acceptable; recorded so nobody mistakes the probe loop for seek-domain coverage, and so the `as i64` is not widened without revisiting the bound.
Confidence: HIGH — Verification: read.

## 4. Per-function table

| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| consts `FUZZ_EVENT_BUDGET`/`FUZZ_POST_SEEK_DRAIN`/`FUZZ_SEEK_PROBES` | 27-33 | REAL | — | 500 k / 64 / 3 — both loops and the probe count verified bounded by them. |
| `span` | 35-41 | REAL (defect) | S4 | MF-003 dead fallback; `nanos as i64` safe under the ×10⁷ cap (MF-004). |
| `fuzz_demux_mp4` | 45-84 | REAL (gaps) | S3 | Bounded drain + seek probes; MF-001 (never run by any gate on stable), MF-002 (no `reanchor` probe; coverage depends on mp4 `seek`'s contract violation). `data.to_vec()` copies input once per pass — bounded by fuzzer input size. No panics in the harness itself (all indexing via `get`). |

## 5. Test honesty

This file contains no tests — it *is* a harness. Honesty of its callers:
- `crates/demux/src/mp4/tests.rs:1083-1108` — 5 fixed hostile inputs, zero assertions (survival-only). Honest per this file's documented contract, but they contribute pass-count without behavioral verification (cross-file; that file's own audit already records it).
- `crates/demux/tests/mp4_fixtures.rs:247` — audited in my next file.
- `crates/demux/tests/fuzz_loop_stable.rs` — does NOT loop this harness (AVI only), despite the fuzz README's "stable-runnable equivalence" section implying the loops cover the seeded layer; for MP4 the deterministic random layer simply does not exist.

## 6. Doc claims touching this file

| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "One call = one full untrusted-container pass: construct, `start`, drain every event to end-of-stream (bounded), then probe seeks … and drain a little after each" | fuzz.rs:4-6 | REAL | Lines 45-84 implement exactly this; bounded verified. |
| "The harness contract is absence-of-panic/hang/unbounded-allocation, nothing else — malformed input is expected to produce typed errors, which are swallowed here" | fuzz.rs:6-10 | REAL | `Err(_) => break` at 62/78; no assertions anywhere in the harness. Honest self-description. |
| "Event budget: hard cap on every loop" | fuzz.rs:14 | REAL | 57-60 and 72-76 both check before each call. |
| "moov/moof ≤ 16 MiB/1 MiB; samples ≤ 16 MiB each through the 2-slot pool" | fuzz.rs:15-17 | REAL | mp4/mod.rs:113/117/109 + pool at 614. |
| "The fragment scan is bounded by MAX_FRAGMENT_SCAN_BOXES" | fuzz.rs:18-19 | REAL | mp4/mod.rs:129, enforced at 717/849/1778/2166. |
| "Never panics by construction of the demuxer and the budgets above" | fuzz.rs:43-44 | REAL-UNPROVEN | Harness-side bounds verified; demuxer-side "never panics" is a claim about 2200 lines I did not execute — no campaign has ever run (MF-001), so it rests on the conformance suite + 6 fixed inputs. |

## 7. Dead / duplicated / unreachable code

- MF-003's dead `unwrap_or` arm.
- The harness module + `test_support` are ungated in the production rlib (mp4/mod.rs:100-102) — dead weight in shipped artifacts, dropped at link time for the CLI; the ungated-`test_support`-in-lib pattern is recorded in that module's own audit.

## 8. Dependencies & flags

No feature gates. The cargo-fuzz package (`fuzz/Cargo.toml:111-112`) depends on `libfuzzer-sys` (0.4.x, Lead-approved external per fuzz/README) and is its own workspace root so the pinned stable toolchain never builds it. `25_fuzz_campaign.sh` uses `cargo fuzz run … -rss_limit_mb=4096`; note (cross-file, W17's domain) that script's `set -uo pipefail` (no `-e`) plus its "clean (exit=$rc)" branch means a target that fails to *build* is reported as clean — worth their audit's attention.

## 9. Open questions for the next agent

- **For the mp4/mod.rs owner (highest priority cross-finding from this file):** `Mp4Demuxer` implements no `reanchor` (trait-default typed refusal) while its `seek` mutates cursors/pending/eos (mp4/mod.rs:2127-2142) in violation of plugin-api/src/demux.rs:72-75. File both: in-session MP4 seek via the documented engine flow is a typed refusal today.
- Has `audit.yml`'s nightly fuzz job ever actually executed (Actions history)? In-tree evidence says no; only the orchestrator (or someone with CI logs) can settle it. If it ran, `docs/audit/reports/fuzz/` artifacts should exist — they do not in this tree.
- `scripts/quality/25_fuzz_campaign.sh` build-failure-is-"clean" reporting and `16_fuzz.sh`'s stale `ls fuzz | grep '^seed_'` (corpus moved to `fuzz/corpus/<target>/`, so the seed count it reports is 0) — both belong to the scripts/workflows agent (W17).
