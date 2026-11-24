# CLAIM AUDIT: tools/vta/mpeg4/README.md
| Field | Value |
|---|---|
| Auditor | W17-A05 |
| Claims extracted | 6 |
| REAL 4 · REAL-UNPROVEN 0 · SHALLOW 1 · STUB 0 · PAPER-ONLY 0 · FALSE 0 · UNREACHABLE 1 (by design) |
| Internal contradictions found | 0 |

## 1. Blunt verdict
An honest archived-spike README: it leads with the BLOCKED verdict, states its own quarantine ("NOT a dependency of any workspace crate" — true in the root lock), and its results snapshot cross-checks against the full admission record's §D table (same lanes, same deltas, same dup-count footnote). The harness files it describes exist and expose exactly the flags documented. The only softness is that the reproduce block depends on long-gone /tmp corpus paths, so "Reproduce" is aspirational text, not a runnable recipe in this tree.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "**Verdict: BLOCKED** — see `docs/dependencies/records/oxideav-mpeg4video-0.1.8.md` (tolerance-class differential ±1..3 ...)" | :4-6 | REAL | record exists; :137 "**BLOCKED (differential).**"; §D table at :208/:211/:156/:217 matches the README's numbers |
| 2 | "The crate is NOT a dependency of any workspace crate; nothing here is wired into `crates/codecs`" | :7-8 | REAL | root Cargo.lock: 0 oxideav entries (grep); crates/codecs/Cargo.toml has no oxideav; only lockfile is this tool's own (30-pkg, audited separately) |
| 3 | "`src/main.rs` — ... `--ecosystem` for the crate's compat mode ... `--hostile` runs the no-panic battery ... primes VOL config from `--extradata`" | :12-22 | REAL | tools/vta/mpeg4/src/main.rs:282 usage string with all flags; :292-296 flag parsing present; 332 LOC of real harness code |
| 4 | "`diff.py` — differential driver: ffprobe ... ffmpeg ... per-frame MD5 + pixel deltas + whole-stream md5" | :23-26 | REAL | tools/vta/mpeg4/diff.py exists (139 LOC; ffprobe/ffmpeg/rawvideo/md5 pipeline as described) |
| 5 | Results snapshot table (8 lanes, e.g. "0097 mp4 ... 0.998 % | 1", "0107 odd-dims 1×1 ... 25/25 ... 0", "0117 packed-bs 37/28 (9 dups)") | :64-78 | REAL (matches the record's §D) | record :208 `| D 0097 | ... 1,702 (0.998 %) | **1** | NO |`; :211 `| G 0107 | ... **25/25** | 0 | **0** | **YES** |`; dup footnote at :156 (N-VOP extra frame) and :217 |
| 6 | "Reproduce (ffmpeg/ffprobe 7.1.5)" block | :28-62 | SHALLOW | commands reference /tmp/my-project/corpus-v4/media and a worktree path (`worktrees/p6-vta-mpeg4`) — neither exists in this tree (corpus dirs absent; worktrees/ deleted). Historical recipe, not currently runnable. Not misleading: the dir header says ARCHIVED. |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| (none found) | — | README / record / lockfile / Cargo.toml mutually consistent (BLOCKED, quarantined, pinned 0.1.8) |

## 4. Fabrication / inflation indicators
None. The README reports its own crate's FAILURE prominently (BLOCKED in the title line), the one identical-output lane is the trivial 1×1 case (honestly shown as the only clean lane), and the packed-bitstream defect is footnoted rather than hidden.

## 5. Matrix rows
Not a matrix chunk. Related: the mpeg4 software-admission row remains blocked-by-this-record; record addendum :290-293 "BLOCKED stands — no admission, no Cargo.toml edit" — no contradiction with this README.
