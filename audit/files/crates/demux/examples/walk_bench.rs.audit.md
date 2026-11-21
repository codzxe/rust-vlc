# AUDIT: crates/demux/examples/walk_bench.rs
| Field | Value |
|---|---|
| Auditor | W04-A01 |
| Date | 2026-11-21 |
| Lines / functions | 533 / 13 (9 fns, 3 structs, 1 `#[global_allocator]` static; no tests — it is a binary example) |
| File verdict | UNREACHABLE (real, functioning developer bench; not part of any shipped binary) |
| Reachable from shipped entry point? | NO — a `cargo run --example` developer tool; `lumen`/`lumen-tauri` never invoke it. Compiles in the workspace: baseline `clippy --workspace --all-targets` CLEAN (INDEX.md #5/#6, clippy-workspace.log.txt) — `--all-targets` builds examples. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | full `Read` of the file; `rg -n "fn start" crates/demux/src/avi/mod.rs` + sed of :892-910 (start() = parse_container — the header parse); sed of `crates/testkit/src/alloc_count.rs:126-140` (guard() snapshots at creation → delta excludes earlier allocs); `rg -l "demux-spill" .` (ONLY this file — the cited doc does not exist); `rg -n "walk_bench\|WALKBENCH\|WALKALLOCS" docs/` (worklog archive only; zero committed bench outputs); `ls docs/perf/` |

## 1. One-paragraph verdict
A genuinely well-built measurement harness — deterministic-counter cross-walk agreement checks (exit 4), zero-packet refusal (exit 3), discarded warmups, exact medians for the timing modes, an honestly-documented unconditional counting allocator — but it is a developer example with **zero committed evidence**: no WALKBENCH/WALKALLOCS/WALKSIZES output exists anywhere in docs/, the perf doc it names as its evidence consumer (`docs/perf/demux-spill-fix.md`) does not exist in the repo, and the perf numbers it quotes in its own header (17.7–22.4 ns/pair, ~0.9 alloc events/packet) have no committed run behind them. Worse, the `--allocs` protocol brackets its counter AFTER `demuxer.start()` — so the header parse's allocations are excluded while the module doc explicitly claims "header parse" is among the counted events. As a shipped-product claim this file promises nothing, so nothing is FALSE at the user level; as an *evidence instrument* it is unproven and partly misdescribed.

## 2. What actually works (verified)
— Compiles clean in the baseline: `cargo clippy --workspace --all-targets --locked -- -D warnings` → CLEAN (audit/runs/baseline/INDEX.md #5; `--all-targets` includes `--examples`).
— Determinism guard is real: `bench_mode`:230-249 compares every walk's (packets, payload_bytes, recovered, damage) against the first and exits 4 on disagreement; same in `bench_allocs`:306-325 (read).
— Exit-code discipline verified by read: 64 usage / 66 not-a-file / 2 walk error / 3 zero packets / 4 disagreement / 5 no walks — all distinct, all to stderr with context.
— `median()`:197-205 is the exact definition (odd: middle; even: mean of two middles) — re-derived, correct.
— The sliding "repeat-within-4" window (:418-429) is a correct last-4 membership count (pop-then-push at capacity 4).

## 3. Findings
### F-001 [S3]
Where: crates/demux/examples/walk_bench.rs:163-165 (bracket) vs :33-38 (doc claim)
Code:
```rust
fn walk_counted(demuxer: &mut AviDemuxer) -> Result<(WalkStats, WalkAllocs), EngineError> {
    demuxer.start()?;
    let guard = ALLOC.state().guard();
```
with the doc claim at :35-37:
```rust
//! protocol over the mem-mode walk (fresh demuxer per walk, warmups
//! discarded, median alloc events/bytes per walk + per packet). The
//! mem source's `read_into` is a zero-alloc memcpy, so every counted
//! event is the walk's own (header parse, pool, packet hand-off) —
```
What is wrong: `AviDemuxer::start()` is where the container header parse happens (`crates/demux/src/avi/mod.rs:892` → `parse_container()` — avih/strl/strf, track construction), and it runs BEFORE the counting guard is taken. `CountingGuard::delta()` (`testkit/src/alloc_count.rs:126-133,175`) is snapshot-at-creation, so every allocation of the header parse is excluded from the `--allocs` numbers — while the module doc lists "header parse" among the counted events. The timing protocol has the opposite bracket: `walk()`:135-136 puts `start()` INSIDE the timer. The two protocols measure different walks and the doc describes the alloc one wrong.
Why it matters: this is the instrument behind per-packet allocation claims. Anyone reading "header parse, pool, packet hand-off" will take `events_per_pkt` as the all-in figure; the steady-state figure it actually reports excludes one-time setup. An evidence tool whose methodology comment misstates its bracket produces confidently-labeled wrong numbers.
What correct looks like: take the guard before `start()` (counting the whole walk, matching the timing bracket), or fix the doc to "packet walk only — header parse excluded" and report the setup delta separately.
Confidence: HIGH — Verification: read (mod.rs:892 sed-verified; alloc_count.rs guard semantics sed-verified). Not executed (no cargo runs).

### F-002 [S3]
Where: crates/demux/examples/walk_bench.rs:46-47
Code:
```rust
//! rate (the only reuse an exact-size slot rotation can harvest), and
//! how many payloads exceed 64 KiB (the poolable-class ceiling check).
//! Raw evidence for docs/perf/demux-spill-fix.md.
```
What is wrong: `docs/perf/demux-spill-fix.md` does not exist. `rg -l "demux-spill"` over the whole repo returns exactly one hit — this file. `ls docs/perf/` shows eleven files, none of them the spill-fix writeup. Additionally, no WALKBENCH/WALKALLOCS/WALKSIZES output is committed anywhere (rg over docs/: only the worklog archive mentions the tool's creation, at archive-2026-10.md:7844/8120/8128), and docs/12-performance-benchmarking.md never mentions walk_bench at all.
Why it matters: the `--sizes` mode's entire stated purpose is to be "raw evidence" for a document that isn't in the tree — the same evidence-string pathology the baseline's own gate caught twice (`xtask feature-audit` FAILURE #8: rows citing missing paths). The bench's headline numbers quoted in its header ("~0.9 alloc events/packet", "17.7–22.4 ns per pair") likewise have no committed run to point at.
What correct looks like: either commit the spill-fix doc with the bench outputs it cites, or delete the citation and mark the numbers as uncommitted worklog claims.
Confidence: HIGH — Verification: read + rg/ls (exhaustive over docs/).

### F-003 [S4]
Where: crates/demux/examples/walk_bench.rs:414 (vs the methodology claim at :23-24)
Code:
```rust
        let med = sizes[n / 2];
```
What is wrong: for even `n` this is the **upper** middle element, not the exact median — while the module doc's methodology section (:23-24) promises "Median is exact: middle element for odd N, mean of the two middle elements for even N" and `median()`:197-205 implements exactly that for the timing modes. The `--sizes` census silently uses the cheaper approximation.
Why it matters: two different "median" definitions in one tool's output; a payload-size census read as exact is off by up to one sample rank for even-counted tracks (audio tracks in these fixtures are typically even).
What correct looks like: reuse `median()` (it takes `&mut [u128]`; sizes are `usize` — widen or genericize), or document the census median as approximate.
Confidence: HIGH — Verification: read (both sites compared).

### F-004 [S4]
Where: crates/demux/examples/walk_bench.rs:505-512
Code:
```rust
    // The file's bytes, read once outside every measured protocol.
    let bytes: Arc<[u8]> = match std::fs::read(&path) {
        Ok(bytes) => bytes.into(),
```
What is wrong: the whole file is read into memory unconditionally — including `--mode file`, where `bytes` is never used (the file lane re-opens through `input::FileAccess`; only mem/allocs/sizes consume `bytes`). A 4 GiB AVI makes the tool allocate 4 GiB (or die) before the file-mode bench it asked for starts.
Why it matters: a walk-rate bench that OOMs on large files measures nothing; the wasted read also skews the "warm page cache" story for the file mode it does run.
What correct looks like: read `bytes` lazily — only when mem/allocs/sizes will run.
Confidence: HIGH — Verification: read (usage of `bytes` enumerated: :514, :517, :528-529 only).

### F-005 [S4]
Where: crates/demux/examples/walk_bench.rs:516-519
Code:
```rust
    if allocs {
        bench_allocs(&path, &bytes, warmup, walks);
        return ExitCode::SUCCESS;
    }
```
What is wrong: `--allocs` short-circuits `main` before the mode dispatch, so `--allocs --mode file` silently never runs the file-mode timing bench. The usage line (:452) presents the flags as independent; nothing documents the precedence.
Why it matters: a user asking for both gets one, with exit 0 and no notice — the exact "silently do less than asked" shape this audit exists to flag.
What correct looks like: either document `--allocs` as exclusive (and reject the combination with usage error 64), or run both.
Confidence: HIGH — Verification: read (control flow traced).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `ALLOC` (global allocator) | 80-81 | REAL | — | Unconditional; overhead honestly documented (:49-57). |
| `WalkStats` / `WalkAllocs` / `SizeRow` | 84-101, 365-368 | REAL | — | Plain data carriers; `micros: 0` sentinel for alloc walks documented. |
| `or_exit` | 105-113 | REAL | — | Exit 2, stderr context; no unwrap. |
| `open_file` | 119-129 | REAL | — | Non-UTF-8 path → typed error; URI via shared `SourceUri::from_file_path` (no hand-rolled format!). |
| `walk` | 134-157 | REAL | — | Timer includes `start()` (header parse) — consistent with doc's walk definition; packets counted and dropped (demux-only cost). |
| `walk_counted` | 163-193 | REAL | S3 | F-001: guard after `start()` excludes header-parse allocs. |
| `median` | 197-205 | REAL | — | Exact; sorts in place. |
| `bench_mode` | 210-290 | REAL | — | Warmup discard, per-walk stderr, cross-walk agreement (exit 4), zero-packet refusal (exit 3); rates computed from median. Defensive `unwrap_or(med)` on non-empty vec (dead but harmless). |
| `bench_allocs` | 295-362 | REAL | — | Fresh demuxer per walk; same agreement guard; per-packet + per-payload-byte ratios; `payload_bytes.max(1)` avoids div-by-zero. |
| `bench_sizes` | 373-446 | REAL | S4 | F-003 median; `rows.iter_mut().find` is O(tracks²) — fine at AVI track counts; sizes Vec cloned per row (bench-local, acceptable). |
| `main` | 448-533 | REAL | S4 | F-004, F-005. Arg validation solid (`--walks` >0 enforced, `--mode` whitelist, unknown → 64). |

## 5. Test honesty
No tests exist in or for this file (it is an example binary). The only execution-adjacent evidence is compilation (baseline clippy `--all-targets` CLEAN) and the in-tool self-checks (walk-agreement exit 4, zero-packet exit 3) which are honest by construction. No committed run of the tool exists anywhere (F-002) — every number in its header doc is UNVERIFIED.

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "Median is exact: middle element for odd N, mean of the two middle elements for even N" | :23-24 | TRUE for time modes / FALSE for `--sizes` | median():197-205 vs sizes[n/2]:414 (F-003). |
| "every counted event is the walk's own (header parse, pool, packet hand-off)" | :36-37 | FALSE for "header parse" | start() precedes the guard (:163-165); mod.rs:892 (F-001). |
| "Raw evidence for docs/perf/demux-spill-fix.md" | :47 | FALSE (target does not exist) | rg/ls — only self-reference in repo (F-002). |
| "W5-15 measured 17.7–22.4 ns per alloc+dealloc pair" | :54-55 | UNVERIFIED | Worklog narrative only; no committed run (rg over docs/). |
| "the post-W6-04 walk at ~0.9 alloc events/packet" | :55-56 | UNVERIFIED | No committed WALKALLOCS output exists. |
| "a `#[global_allocator]` cannot be mode-gated" | :51-53 | TRUE | Language fact (global allocator is a link-time singleton). |
| "The comparator's process-startup overhead is NOT in our number (in-process loop)" | :25-28 | TRUE by construction | Loop is in-process (read). |

## 7. Dead / duplicated / unreachable code
`--mode both` + `--allocs`: the mode lanes are dead when --allocs is passed (F-005). No other dead code; every fn is called. Duplication note: `bench_mode`/`bench_allocs` share ~40 lines of agreement-check scaffolding — acceptable for a standalone example.

## 8. Dependencies & flags
Dev-only deps of the example: `demux` (avi + test_support — the same pub test-support surface noted in the chunks.rs audit), `media-types`, `plugin-api`, `input`, `testkit` (CountingGlobalAlloc). No features. The example proves `input` is a dev-dependency of demux — consistent with demux/Cargo.toml (dev-deps, not shipped).

## 9. Open questions for the next agent
- Who owns `docs/perf/demux-spill-fix.md`'s absence — the doc wave should record it as a phantom-evidence citation alongside the two xtask feature-audit violations (baseline #8).
- The header's "~0.9 alloc events/packet" and "17.7–22.4 ns" figures: if any perf doc quotes them, they trace to uncommitted runs — REQUEST-ORCH-RUN (optional, low priority): `cargo run --release -p demux --example walk_bench -- tests/fixtures/generated/valid/avi_mjpeg_pcm.avi --allocs` to pin one committed WALKALLOCS line and close the F-001 question empirically (whether header-parse allocs are material).
