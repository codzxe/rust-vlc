# AUDIT: crates/streaming/src/ranged.rs#part3 (lines 1258–1886)
| Field | Value |
|---|---|
| Auditor | W10-A08 |
| Date | 2026-11-21 |
| Lines / functions | 629 audited (chunk part3 of 1886; nominal split 629/629/628 — part1 = 1-629 (W10-A09), part2 = 630-1257 (W10-A10); `mid_read_416_is_a_typed_truncation` (1222-1283) crosses the 1258 boundary and is audited whole here with this note) / 18 `#[test]` fns + 1 test-vehicle struct (`FlipServer`, 2 methods + 1 trait impl) + 3 inline fake `HttpFetch` impls (`UnknownLen`, `StarTotals`, `Liar`) |
| File verdict | REAL (test-only chunk; executed in baseline) |
| Reachable from shipped entry point? | The chunk itself is `#[cfg(test)]` — never shipped. The production code it pins IS reachable: `lumen open http://…` → apps/cli/src/host.rs:96 (`open_source`) → apps/cli/src/demux_host.rs:427→474-500 (`open_http_source`) → crates/streaming/src/schemes.rs:236-240 (`SchemeRegistry::new()` wires `UreqClient::new()`) → schemes.rs:343-349 (`open_http`) → ranged.rs:493 (`RangedResource::open`). Verified by rg caller census (`RangedResource` consumers: schemes.rs, demux_host.rs, apps/web, hls tests). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 2 |
| Commands run | `rg -n "^(pub )?(fn|struct|enum|impl…)"` structure map; `grep -n "fn …"` exact line pins (all 21 items); `sed -n` verbatim verification of every quote below; `rg "RangedResource"` across repo (call-chain proof); `rg "impl HttpFetch for"` (transport census: UreqClient client.rs:367 is the only real transport); baseline: `audit/runs/baseline/orchrun-test-workspace-nofailfast.log.txt` :5087-5132 (all 22 ranged tests ok), :4941 (streaming lib: 214 tests ok); `rg "assert_err|trait Must"` in crates/testkit/src/lib.rs (real assertions, :100-104) |

## Chunk boundary note
Part3 = 1258–1886: the tail of `mid_read_416_is_a_typed_truncation` (whole test audited here; its `Shrinker` fake at 1223-1257 is part2's range), `small_file_smaller_than_the_window` … `scan_content_length_disagreement_is_a_protocol_violation` (ends 1885; `}` closes `mod tests` at 1886). Everything in this range is inside `#[cfg(test)] mod tests` (opens at 981).

## 1. One-paragraph verdict
This chunk is the back half of the ranged-resource test suite: mid-read shrink/416 truncation, unknown-length honesty, config-bounds refusals, Content-Range grammar, the cache-budget invariant, and the entire 6.A2b linear-scan fallback suite (probe-refused re-probe, forward-free seeks, backward refetch accounting, over-budget refusal, tiny-file probe, mid-stream 200 resync under both policies, lying Content-Length). Every test I traced asserts on real state — byte-exact comparisons against a deterministic pattern, request logs from the mock server, and the stats counters — and every one ran and passed in the baseline (`orchrun-test-workspace-nofailfast.log.txt:5087-5132`). The suite is honest, with one overclaim: `content_range_parsing_is_strict` advertises "every hostile one" while grammar-accepted semantic attacks (inverted ranges, end-vs-body disagreement) parse fine and are never validated anywhere in production `absorb` — the parsed `end` is discarded as `_rend`. Note for the record: the INDEX.md fail-fast baseline (row 3: "1681 passed / 1 failed") never executed the streaming crate at all — cargo stopped at the `desktop` zero-alloc failure; the execution evidence lives only in the unindexed `orchrun-test-workspace-nofailfast.log.txt` (orchestrator's own footer: "it hid 6 of 7 failures and ~2850 tests").

## 2. What actually works (verified)
— All 18 tests in this chunk executed and passed: baseline `orchrun-test-workspace-nofailfast.log.txt` lines 5087-5132 (`ranged::tests::config_bounds_are_typed_refusals … scan_content_length_disagreement_is_a_protocol_violation … unknown_length_server_is_honest` all `… ok`), streaming lib suite `:4941` "running 214 tests" → ok.
— Read-traced execution of the trickiest assertions against the production code in part1/part2: `scan_refetch_accounting_is_exact_across_rewinds` (:1629) expects `requests == 4` — probe (refused over-budget, counted at :555) + rangeless re-probe (:556 `scan_get` → :627 `requests += 1`) + 2 backward refetches (:682) = 4, and `skipped_bytes` 0 then 10 via `scan_refetch` :685-688; the arithmetic is exact, the test would fail on any off-by-one in the accounting.
— `mid_stream_200_resyncs_into_scan_and_recovers` (:1790) `ranged_log == [true,true,true,true,false]` traced through `fetch_window` :753 (`absorb(…, resync=true)` → :610 `resyncs += 1`) and the mode re-dispatch at read_into :910-915 — the log assertion pins the exact request dialect per phase.
— `testkit::assert_err` is a real assertion (crates/testkit/src/lib.rs:100-104: `Ok(_) => unreachable!(…)`), so the typed-refusal tests cannot silently pass on success.

## 3. Findings

### F-001 [S3]
Where: crates/streaming/src/ranged.rs:1455-1456 (test doc claim), 1467-1474 (hostile list); cross-ref production ranged.rs:590 and 953-979 (part2's range, cited as the code under test)
Code:
```
    /// Content-Range parsing: the accepted shapes and the typed
    /// refusal for every hostile one.
```
and
```
        for hostile in [
            None,
            Some("0-1023/4096"),
            Some("bytes 0-1023"),
            Some("bytes -/4096"),
            Some("bytes x-y/4096"),
            Some("bytes 0-1023/many"),
        ] {
```
What is wrong: "every hostile one" is false. The enumerated hostiles are all *grammar* failures. Semantically hostile headers that `parse_content_range` (953-979) accepts: an inverted range `"bytes 9-5/4096"` (end < start parses to `(9, 5, Some(4096))`), `total = 0` (`"bytes 0-0/0"`), and u64-boundary values. Worse, production `absorb` discards the parsed end entirely — `let (rstart, _rend, total) = parse_content_range(…)` (ranged.rs:590) — so a 206 whose `Content-Range` end disagrees with the delivered body length (claim `bytes 0-1023/5000`, deliver 500 bytes) is accepted with zero validation; only `rstart == start` is checked (:591-593). No test in either half of the suite pins any of this.
Why it matters: this is the network-trust boundary the module docs sell as typed refusals ("never a guessed length", "protocol-violation" taxonomy). A hostile or buggy CDN answering a shifted/mis-sized `Content-Range` gets cached as a window at the requested offset with arbitrary body length — leniency the doc-comment on this test denies exists. The test name ("is_strict") asserts a property the system does not have.
What correct looks like: either (a) extend the hostile list with `"bytes 9-5/4096"` and an end-vs-body-length disagreement case and make `absorb` validate them (use `_rend` against `resp.body.len()`), or (b) rename the test and its doc to "grammar-shaped refusals" and record the end-leniency in KNOWN-LIMITS.
Confidence: HIGH — Verification: read (parse_content_range branch analysis) + traced absorb; parse acceptance of `9-5` verified by the grammar code path (split_once('-') then two u64 parses, no ordering check).

### F-002 [S4]
Where: crates/streaming/src/ranged.rs:1367-1370
Code:
```
        // The `*`-total shape is what fixes unknown-length mode; the
        // fake above advertises a total, so the header is rewritten
        // by the `*` variant in a second scripted fetch below — here
        // the honest path is pinned through a `*`-serving wrapper.
```
What is wrong: There is no "second scripted fetch below". The mechanism is the `StarTotals` wrapper struct (1371-1387) that rewrites every response's `Content-Range` total to `*`. The comment describes an implementation that does not exist (likely a leftover from an earlier draft that used `ScriptedRoute`-style scripting).
Why it matters: The comment sends the reader hunting for a scripted-fetch helper that isn't there; on a protocol-honesty test (unknown-length mode) the setup description must match the setup.
What correct looks like: "…so the `StarTotals` wrapper below rewrites every `Content-Range` total to `*`, pinning the unknown-length shape."
Confidence: HIGH — Verification: read; `rg "ScriptedRoute"` absent from ranged.rs (it lives in mockhttp.rs).

### F-003 [S4]
Where: crates/streaming/src/ranged.rs:1525-1567, 1600-1623, 1670-1697, 1862-1885 (the scan-mode test set); cross-ref production ranged.rs:647-656
Code:
```
        let declared = resp
            .header("Content-Length")
            .and_then(|value| value.trim().parse::<u64>().ok());
        match declared {
            Some(declared) if declared != body_len => {
                return Err(protocol_violation());
            }
            _ => {}
        }
        let new_len = declared.unwrap_or(body_len);
```
What is wrong: The absent-`Content-Length` fallback (`declared.unwrap_or(body_len)`, documented at :635-636 as "an absent header falls back to the observed body length, the EOF-terminated transport shape") has zero coverage in the entire ranged suite. Every scan-mode test in this chunk serves a `Content-Length` (`MockServer`, `FlipServer` :1778, `Liar` :1871-1874); no scripted fetch ever omits it.
Why it matters: An untested branch on the scan-mode length-determination path — exactly the path a demuxer's seek math depends on. If `unwrap_or(body_len)` regressed to `unwrap_or(0)` or a protocol violation, all 22 ranged tests would still pass.
What correct looks like: One scripted-fetch test: 200 with no `Content-Length` → `size()` is the observed body length, reads and EOF behave.
Confidence: HIGH — Verification: read; census of all `Content-Length` producers in the test module (grep) shows none omitted.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| tests::mid_read_416_is_a_typed_truncation (BOUNDARY test, audited whole) | 1222-1283 | REAL | — | Fake `Shrinker` claims 5000 on probe then 416s every later range; asserts typed `streaming.ranged.truncated` with `Truncated{expected:5000, found:1024}` — traced through fetch_window :768-774. Would fail if 416 were swallowed. Panics-on-infra style (`unreachable!` on open failure) consistent with suite. |
| tests::small_file_smaller_than_the_window | 1288-1305 | REAL | — | 300-byte body < one 64 KiB chunk; asserts `requests == 1` (probe-only). Honest. |
| tests::no_range_server_is_refused_typed_at_open | 1313-1326 | REAL | — | `honor_range:false` + `RequireRange` → typed refusal at open (absorb :606-608). |
| tests::unknown_length_server_is_honest | 1332-1406 | REAL | — | `StarTotals` wrapper pins `bytes a-b/*`; size()==None, seek-anywhere, clean EOF via RangeNotSatisfiable→Ok(0) (:768-773 unknown-length arm), before-start typed. Stale comment (F-002). |
| tests::config_bounds_are_typed_refusals | 1411-1443 | REAL | — | All three validate() codes (:351-375) asserted + a valid config passes. Would fail on clamp-instead-of-refuse regressions. |
| tests::oversized_read_is_refused_typed | 1448-1453 | REAL | — | `MAX_READ_BYTES+1` → `streaming.ranged.read-over-budget` (read :861-870). Boundary-at-limit itself untested (only +1) — trivial gap. |
| tests::content_range_parsing_is_strict | 1458-1478 | REAL (overclaiming) | S3 | F-001: grammar-only hostile list; "every hostile one" false; `_rend` never validated in production. |
| tests::cache_invariant_holds_across_a_seek_storm | 1484-1501 | REAL | — | 7-jump fixed script, asserts `cached_bytes() <= 4096` per step + byte-exact reads. Would fail if insert_window eviction (:786-798) broke. "Seek storm" is 7 jumps — doc admits "fixed script". |
| tests::default_policy_is_the_scan_fallback | 1510-1515 | REAL | — | Pins Default == constants (part1). Fails if defaults drift from documented knobs. |
| tests::scan_fallback_reads_the_full_body | 1525-1567 | REAL | — | The 6.A2b core proof: probe refused over-budget (requests counted, 0 bytes) + rangeless re-probe under window budget; asserts request log shapes (`bytes=0-1023` then no Range header), peak retention, exact stats. Traced; strongest test in the chunk. |
| tests::scan_seek_forward_within_the_buffer_is_free | 1573-1593 | REAL | — | Forward seek inside [pos,end) serves from buffer, 0 requests, retention math asserted (scan_trim :724-733). |
| tests::scan_seek_backward_refetches_once_from_byte_zero | 1600-1623 | REAL | — | Exactly one rangeless refetch, skip [0,512) counted, `bytes_fetched == 16_000` (two whole bodies). Traced exact. |
| tests::scan_refetch_accounting_is_exact_across_rewinds | 1629-1664 | REAL | — | Rewind-to-0 empty skip; second backward seek into discarded territory; `requests == 4` traced by hand — accounting is exact. |
| tests::scan_body_over_the_window_budget_is_refused_typed | 1670-1697 | REAL | — | 5000-byte body vs 4096 window → ResourceLimit{budget:4096, attempted:5000} via map_fetch_err :225-234; 2 requests then refusal. Typed, never clamped. |
| tests::scan_tiny_file_is_served_by_the_probe_alone | 1703-1713 | REAL | — | Body < chunk budget → probe's own 200 enters scan directly, 1 request. |
| tests::FlipServer (struct) + ranged_log + impl HttpFetch | 1719-1782 | REAL | — | Test vehicle: honors Range for first `flip_after` requests then 200s; logs per-request ranged-ness under a Mutex with poison recovery (:1730-1734, :1739-1742). Correct and used by both flip tests. |
| tests::mid_stream_200_resyncs_into_scan_and_recovers | 1790-1829 | REAL | — | Resync counted once, bytes exact through the flip, post-flip dialect rangeless — `ranged_log` assertion pins all 5 requests. Traced against absorb :605-613 + read_into :910-915. |
| tests::mid_stream_200_under_require_range_is_typed | 1835-1854 | REAL | — | Same 200 under RequireRange → typed mid-stream refusal (absorb :606-608). Policy enforced mid-stream, not just at open. |
| tests::scan_content_length_disagreement_is_a_protocol_violation | 1862-1885 | REAL | — | `Liar` declares body.len()+499 → `streaming.ranged.protocol-violation` (enter_scan :650-654). Defense-in-depth pin. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| all 18 above | YES (each) | Every assertion is against external truth: deterministic pattern bytes, the mock's request log, or exact stats counters — none re-derives the answer from the code under test. `assert_err`/`must` are real panicking assertions (testkit/src/lib.rs:100-104, :44-70). Mutation check (reasoned, not run): gutting `insert_window` eviction → `cache_invariant…` fires at jump 7000; dropping `scan_refetch`'s skip accounting → `scan_refetch_accounting…` fires; accepting a lying Content-Length → `scan_content_length_disagreement…` fires. |
| content_range_parsing_is_strict | PARTIALLY | Catches grammar regressions; cannot catch semantic-hostility acceptance because production never validated it (F-001) — the test encodes the leniency as correct. |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "typed refusal for every hostile one" (Content-Range) | ranged.rs:1455-1456 test doc | FALSE (overclaim) | F-001: `bytes 9-5/4096` and end-vs-body disagreement accepted (parse 953-979 has no ordering/consistency check; absorb :590 discards `_rend`). |
| "A `200` whose `Content-Length` disagrees … is a protocol violation" | ranged.rs:1856-1860 test doc | REAL | Test executed ok (baseline :5098); enter_scan :650-654. |
| Bounded memory in scan mode ("retention peaked <= window") | module docs 48-93; test 1525-1567 | REAL (mock-transport only) | Baseline :5099 ok; note the bound is enforced by the *port's* response_limit — for the real transport, client.rs:302-317 take(limit+1). |
| 6.A2b resync "the pending read completes from the new body" | module docs 78-84; test 1790-1829 | REAL | Baseline :5093 ok; traced read_into :910-915. |
| MP4/network open-path claims (feature-matrix rows citing ranged) | docs/feature-matrix.toml (rows owned by other agents) | not re-adjudicated here | Only the ranged rows' engine (RangedResource) verified reachable: demux_host.rs:427→490. |

## 7. Dead / duplicated / unreachable code
None in this chunk. All tests ran in baseline. The four inline fakes (`Shrinker`, `UnknownLen`, `StarTotals`, `Liar`) exist because the MockServer cannot produce these shapes (unknown totals, lying lengths, flip behavior) — legitimate, not duplication; `FlipServer` duplicates the ranged-serving arm of `UnknownLen`/`Shrinker` (~15 lines of 206-construction shared shape) — acceptable test-vehicle repetition, noted only.

## 8. Dependencies & flags
Test-only deps in scope: `testkit` (assert_err/Must — verified real), `crate::mockhttp` (MockServer/MockClient/MockConfig). No feature gates touch this chunk (`mod tests` is `#[cfg(test)]` only). No unsafe. `unreachable!` used as the suite's panic idiom (allowed in tests by workspace lint posture).

## 9. Open questions for the next agent
- W10-A10 (ranged part2): F-001's production half lives in your range (`absorb` :581-617, `parse_content_range` :953-979) — adjudicate whether the `_rend` discard is a documented-lenient design (module docs :193-194 only promise start checking) or a gap; my S3 is on the test's "every hostile one" claim either way.
- Whoever owns client.rs: does the real ureq transport ever omit `Content-Length` (chunked responses), making F-003's untested `unwrap_or(body_len)` branch live in production?
- The mock's `honor_range:false` + response_limit interplay (probe refused, request still logged) is asserted at :1556-1566 — mockhttp part2 (mine) will verify the log-before-refuse ordering.
