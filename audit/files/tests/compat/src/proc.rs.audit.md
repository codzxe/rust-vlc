# AUDIT: tests/compat/src/proc.rs
| Field | Value |
|---|---|
| Auditor | W16-A02 |
| Date | 2026-11-24 |
| Lines / functions | 196 / 6 (run_bounded, describe_status, read_capped, clip, TempOutputs::new/cleanup, started_counter) |
| File verdict | REAL-UNPROVEN |
| Reachable from shipped entry point? | NO — compat-matrix test infra; called from corpus.rs:314 (ffmpeg regen), oracle.rs:194,237 (VLC dump/assert), report.rs:840,1496 (lane steps, git_commit); invoked by CI compat-matrix.yml:154 |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 1 |
| Commands run | `rg -n "proc::" tests/compat/src/*.rs` → 5 call sites listed above; `sed -n '169,176p'` quote verification |

## 1. One-paragraph verdict
A real bounded-subprocess helper: children write stdout/stderr to temp files (no pipe deadlock), a poll loop kills at deadline, outcome is a typed string (`exit N` / `killed (timeout)` / `terminated (…)`), spawn errors are the only Err. The bounded-retention design is defeated by read_capped, which slurps the ENTIRE output file into memory before clipping to 64 KiB — the exact unbounded-read the module header claims to avoid. No tests exist for this file at all.

## 2. What actually works (verified)
— verified by read of full file + `rg -n "proc::"` caller mapping: kill-on-deadline loop L128-144 (try_wait → elapsed check → kill → wait), temp-file stdio L87-110, Err-only-on-spawn contract L74-76 upheld (L118-123 spawn error, L139-142 wait error). Not executed (cargo not installed). UNVERIFIED at runtime.

## 3. Findings
F-001 [S3]
Where: tests/compat/src/proc.rs:168-176 (read_capped) vs header claim L1-5
Code:
```
fn read_capped(path: &Path) -> String {
    match std::fs::read(path) {
        Ok(bytes) => {
            let text = String::from_utf8_lossy(&bytes).to_string();
            clip(&text, OUTPUT_CAP, true)
        }
```
Header claim (L3-5): "Output goes to temp files (never OS pipes — a chatty child cannot deadlock the bounded wait) and only a bounded head/tail slice is retained."
What is wrong: `std::fs::read` reads the whole output file into RAM before clipping to 64 KiB. A chatty child (ffmpeg -v verbose on a bad input, a wasm build with CI verbosity) writing multi-GB output OOMs this 2-core/3GB-class tool transiently; the "bounded" property only holds for what is RETAINED, not what is READ. Why it matters: the module's own safety claim is half-implemented; the tool that is supposed to bound runaway children gets OOM-killed by their logs. What correct looks like: `File::open` + `seek` to `len - OUTPUT_CAP` (tail) or read only `OUTPUT_CAP` bytes (head); never `fs::read` the whole file.
Confidence: HIGH — Verification: read.

F-002 [S4]
Where: tests/compat/src/proc.rs:127
Code:
```
    let deadline = Duration::from_secs(timeout_s.max(1));
```
What is wrong: a configured `timeout_s: 0` silently becomes 1 s instead of an immediate kill or a config validation error — the config schema's contract is silently rewritten. Why it matters: lane-step budgets are evidence semantics in this tool; a 0 written to mean "no wait" means "wait 1 s". What correct looks like: config-side validation (reject 0) or documented clamp. Confidence: HIGH — Verification: read.

Also noted (no separate ID, latent): L133-135 `let _ = child.kill();` — if kill fails (already-exited race is benign; permission cases are not on this OS surface) the record still says `killed (timeout)`; and `read_capped` on a deleted-in-flight file returns "" (Err arm L174) — acceptable.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| run_bounded | 77-156 | REAL-UNPROVEN | S3 | kill-on-deadline correct; reads unbounded output (F-001) |
| describe_status | 158-164 | REAL-UNPROVEN | — | exit code vs termination, honest |
| read_capped | 168-176 | REAL-UNPROVEN | S3 | whole-file slurp before clip (F-001) |
| clip | 179-196 | REAL-UNPROVEN | — | char-boundary walk correct both arms |
| TempOutputs::new / cleanup | 40-54 | REAL-UNPROVEN | — | pid+counter unique names; cleanup best-effort |
| started_counter | 59-69 | REAL-UNPROVEN | — | thread-local; comment says single-threaded tool — consistent with report.rs usage |

## 5. Test honesty
No tests in this file. The clip() boundary logic (multi-byte UTF-8 truncation) and the timeout-kill path are entirely untested anywhere I can find (`rg -n "#\[cfg(test)\]" tests/compat/src/` → only report.rs). Not run by CI either (see report.rs F-001: only `cargo run` executes this crate).
| (none) | — | zero coverage of kill/clip paths |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "only a bounded head/tail slice is retained" | module doc L3-5 | SHALLOW (retention bounded; read unbounded) | F-001 quote above |
| "an overrun is killed and recorded, never hung" | module doc L2-3 | REAL-UNPROVEN | L128-144 loop; not executed |

## 7. Dead / duplicated / unreachable code
None. (report.rs has its own truncate; clip here is the boundary-safe variant — duplication of intent, different safety.)

## 8. Dependencies & flags
std-only + serde (Serialize on ProcRecord). No feature gates. unsafe_code = deny per tests/compat/Cargo.toml lints.

## 9. Open questions for the next agent
- Do corpus.rs/oracle.rs budgets make multi-GB outputs reachable (ffmpeg regen on malformed seeds)? If yes, F-001 severity rises to S2.
