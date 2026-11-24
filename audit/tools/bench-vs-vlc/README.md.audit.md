# CLAIM AUDIT: tools/bench-vs-vlc/README.md
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 9 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 9 / 0 / 0 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
Every checkable claim verifies, including the ones I executed: the 30 python unit tests
pass (0.697s), the version-drift abort is a real tested behavior, the mock/real separation
invariants exist in code and tests, and the v1 report exists with exactly 35 comparable
PENDING-VLC rows as claimed, with independent-verification notes. This is the repo's
best-behaved evidence tool.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "test_run.py | python unit tests (mock mode end-to-end, ... classification, separation invariants, version-drift abort)" | L17 | REAL | `python3 -m unittest discover -s tools/bench-vs-vlc -p 'test_run.py'` -> "Ran 30 tests ... OK" (30 def test_) |
| 2 | "--version is asserted to be exactly 3.0.23 (drift aborts the run)" | L26-27 | REAL | test_run.py:187 `test_version_drift_aborts_through_the_real_path`, :198 asserts "3.0.23" |
| 3 | "Without a binary the run refuses (exit 2); it never falls back to the mock" | L27-28 | REAL | test_run.py:12 docstring states it; refusal tests present |
| 4 | "A mock run is plumbing validation, never a measurement ... run.py's report writer asserts these invariants" | L30-32 | REAL | run.py asserts [MOCK] prefix + oracle.kind = "mock" (rg L24, L201) |
| 5 | "docs/corpus/vlc-diff-report-20261023.md — the v1 report (PENDING-VLC mode: ... real Lumen digests on all 35 comparable rows ...)" | L144-146 | REAL | report exists (420 lines); `rg -c 'PENDING-VLC \| PENDING-VLC'` -> 35 |
| 6 | "Lumen decoded PCM bytes == ffprobe a:0 packet-payload bytes on 35/35" (report) | report L233 | REAL (cited, not re-run) | the report's own evidence; python-side independent RIFF-walk verification recorded at L314 |
| 7 | "[VERIFY] the flags are transcribed from the pinned build's help text; the first real-oracle run must confirm dump byte-counts" | L56-59 | REAL (honest [VERIFY]) | transcribed from 14-options-full-help.txt L2335-2348 (baseline file exists) |
| 8 | "run.py | the orchestrator: manifest/file-list input → classification → oracle invocation → per-fixture report" | L15 | REAL | run.py exists (mock_vlc.py + run.py + tests/lumen_pcm.rs all present in ls) |
| 9 | "src/lib.rs | the Lumen-side digest pipeline (real demux+codec crates; first PCM audio track → MD5)" | L19-20 | REAL | tools/bench-vs-vlc/src/{lib.rs,md5.rs} exist; Cargo.toml standalone workspace claim consistent (not a root member) |

## 3-5. Contradictions / fabrication / matrix rows
None / none / the report's PENDING-VLC rows are honest non-claims (oracle absent in
sandbox). The 19/35 runtime-random fixtures honesty note (report L332) is exactly the
anti-fabrication discipline MP §5 demands. Commands: unittest run (exit 0); rg counts;
ls; report sed.
