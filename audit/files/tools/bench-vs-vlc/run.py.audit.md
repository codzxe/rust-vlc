# AUDIT: tools/bench-vs-vlc/run.py
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Date | 2026-11-24 |
| Lines / functions | 1002 / 17 (+3 oracle classes with 7 methods) |
| File verdict | REAL |
| Reachable from shipped entry point? | NO — standalone dev harness (tools/bench-vs-vlc is its own cargo workspace, not in the release lane `cargo build --release -p lumen-cli -p desktop`); invoked manually / via its unittest suite. By design, not a defect. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 2 |
| Commands run | `python3 -m py_compile run.py` -> OK; `python3 tools/bench-vs-vlc/test_run.py` -> Ran 30 tests, OK (0.7s); `python3 -c` with_suffix probe -> no ValueError; `which vlc ffprobe` -> ffprobe present, vlc absent |

## 1. One-paragraph verdict
This is the differential-vs-VLC harness's Python driver. It is real, executed, and unusually honest about mock-vs-real separation: the mock oracle is mechanically labeled (`assert "[MOCK]" in markdown` at run.py:844), the pending mode carries zero digests, and the VLC invocation carries an explicit `[VERIFY — untested without the binary]` marker (run.py:55-58). I ran its 30-test suite: all pass. The defects are localized: a stale reason string that lies about the current build ("no Lumen demuxer in this build" — false since w27), and two diagnostic functions that exist only for the unit tests, never in the harness flow.

## 2. What actually works (verified)
— verified by `python3 tools/bench-vs-vlc/test_run.py` -> "Ran 30 tests in 0.700s / OK" (mock oracle end-to-end through the subprocess seam, mock/real separation, pending no-digest invariant, classification reasons, PENDING-VLC command shape).
— verified by `python3 -m py_compile run.py` -> exit 0 (no syntax debt).
— `Path.with_suffix('.lumen.json')` semantics probed: Python here accepts multi-dot suffix — the default `lumen_json = args.out.with_suffix(".lumen.json")` (run.py:972) does NOT raise; no crash on the default path.

## 3. Findings
F-001 [S3]
Where: tools/bench-vs-vlc/run.py:310-314
Code:
```
    if container != "avi":
        reason = (
            f"container {container}: no Lumen demuxer in this build "
            "(first vertical: AVI, D-018)"
        )
```
What is wrong: the reason string printed into every generated non-AVI row asserts "no Lumen demuxer in this build". That is false on the current tree: worklog w27 (worklog.md:2039) and docs/KNOWN-LIMITS.md:36 record that MP4/MKV/TS/ES-H.264 demux+open landed (pipeline_host.rs:346 wires h264/avc1). The true scope statement is "harness v1 only drives the AVI first-audio path" (tests/lumen_pcm.rs exposes only `digest_avi_first_pcm_audio`). Why it matters: this harness feeds the vlc-diff-report evidence docs; a false reason in the evidence chain reads as a build fact, not a harness scope. What correct looks like: reason = "harness v1 AVI-first-audio scope; the demuxer exists but is not driven here".
Confidence: HIGH — Verification: read (reason text at 311-313; counter-evidence KNOWN-LIMITS.md:36 + pipeline_host.rs:346).

F-002 [S3]
Where: tools/bench-vs-vlc/run.py:436-461
Code:
```
def compare_pcm_streams(left: bytes, right: bytes) -> dict:
    """Byte-level comparison of two PCM streams (the unit-tested core of
    the digest comparison; the report path uses digests + byte counts,
    this is the diagnostic used when both byte streams are at hand)."""
```
What is wrong: `compare_pcm_streams` and its helper `first_divergence` are never called anywhere in run.py's flow (grep: only the definitions at 436/448 and test_run.py call sites 139-158). The docstring's claim "the unit-tested core of the digest comparison" is inverted — it is *only* unit-tested, never used by the comparison the report makes. Dead-in-flow code whose docstring overstates its role. What correct looks like: either wire it into the mismatch diagnostic path (when both streams are at hand, as the docstring says) or move it to the test module.
Confidence: HIGH — Verification: read (rg over run.py + test_run.py).

F-003 [S4]
Where: tools/bench-vs-vlc/run.py:409-412
Code:
```
    if proc.returncode != 0:
        return {"container": None, "codec": None, "audio_codec": None}
```
What is wrong: an ffprobe failure (unreadable/corrupt file in --files mode) is swallowed into None fields, and classify_entry then emits "container None: no Lumen demuxer in this build" — a wrong reason for a prober failure. Minor: the row is still non-comparable, but the printed reason lies. Correct: propagate "ffprobe failed (exit N)".
Confidence: HIGH — Verification: read.

F-004 [S4]
Where: tools/bench-vs-vlc/run.py:386-388
Code:
```
    for file_entry in manifest.get("files", []):
        rel = file_entry["rel"]
```
What is wrong: malformed manifest entries raise bare KeyError -> traceback, no error context. Dev-tool blast radius only.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| vlc_dump_command | 126-134 | REAL | — | exercised by 30-test suite incl. PENDING command shape |
| VlcOracle.resolve | 147-155 | REAL | — | refuses without binary (exit 2 path) — tested |
| VlcOracle.assert_version_pin | 157-178 | REAL | — | version drift aborts (exit 3 path) — tested; parse `first.split()[2]` matches pinned line shape |
| VlcOracle.dump_pcm | 180-197 | REAL-UNPROVEN | — | real VLC absent in sandbox; flags honestly marked [VERIFY] at run.py:55-58 |
| MockOracle.__init__/assert_version_pin/dump_pcm | 214-259 | REAL | — | executed via test suite; LUMEN_MOCK_VLC_ECHO/VERSION hooks tested |
| PendingOracle.* | 262-274 | REAL | — | zero digests invariant asserted (run.py:853-855) |
| classify_entry | 282-379 | REAL | S3 | full matrix of classes; F-001 stale reason; LOSSLESS_AUDIO consulted only off-AVI branch (asymmetry S4-level) |
| load_manifest_entries | 382-394 | REAL | S4 | F-004 |
| probe_file_facts | 397-420 | REAL | S4 | F-003 |
| md5_file | 428-433 | REAL | — | 1 MiB blocks, standard |
| compare_pcm_streams | 436-445 | DEAD-IN-FLOW | S3 | F-002 |
| first_divergence | 448-461 | DEAD-IN-FLOW | S3 | F-002 |
| ffprobe_payload_bytes | 464-484 | REAL | — | secondary cross-check, honestly labeled |
| run_lumen_digests | 492-518 | REAL-UNPROVEN | — | drives `cargo test --test lumen_pcm -- --ignored` (build budget: not run by me); TSV/env plumbing tested |
| load_lumen_results | 521-527 | REAL | — | schema-checked |
| report_title | 535-540 | REAL | — | kind-dispatch |
| build_report | 543-856 | REAL | — | mock/pending labeling invariants mechanically asserted (841-855); [MOCK] prefix on rows; LUMEN-REFUSED never MATCH |
| entry_path | 859-860 | REAL | — | abs fallback |
| main | 868-997 | REAL | — | oracle mode explicit, never inferred; exits 2/3 typed |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| 30-test suite (test_run.py) | YES | I ran it: 30/30 OK. Covers mock end-to-end, drift abort, pending no-digest, classification, command shape. Mutation reasoning: deleting the mock-label assert or the version-drift check would fail named tests. |
| tools/bench-vs-vlc/tests/lumen_pcm.rs | YES (pinned digests) | committed_fixture_digest_is_pinned asserts literal MD5 of a committed fixture — would fail on a decoder change (that is its purpose). Not run by me (cargo budget); pinned constants present at lines 42-52. |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "A mock run is NEVER a measurement" | run.py docstring 21-27 | REAL | asserts at 844-850 enforce [MOCK] labeling; tests verify |
| "no code path where a mock digest reaches a real label" | run.py:33-35 | REAL | mock rows forced onto `[MOCK] `-prefixed lines (847-850); pending rows forced digestless (853-855) |
| "container X: no Lumen demuxer in this build" (generated rows) | run.py:312 | FALSE as a build fact | KNOWN-LIMITS.md:36; pipeline_host.rs:346 — demuxers exist; harness scope is the true reason |

## 7. Dead / duplicated / unreachable code
compare_pcm_streams + first_divergence (F-002). LOSSLESS_AUDIO set used only to decorate the non-AVI reason (317-323) — near-dead constant.

## 8. Dependencies & flags
Standalone cargo workspace (empty [workspace]) with path deps on codec-api, codecs, demux, input(+contract), media-probe, media-types, plugin-api — engine crates vendored by path, no external crates (verified in Cargo.toml). Lints deny unsafe/unwrap/expect/panic — enforced locally. Not in the release lane (see reachability).

## 9. Open questions for the next agent
- Did any committed evidence doc (docs/corpus/vlc-diff-report-*.md) print F-001's false "no Lumen demuxer" reason? (Check vlc-diff-report-20261023.md — also assigned to W19-A10.)
- lumen_pcm.rs + src/lib.rs + src/md5.rs + mock_vlc.py + test_run.py are NOT in my assignment list; do not assume they were audited by me.
