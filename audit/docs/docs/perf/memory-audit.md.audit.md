# CLAIM AUDIT: docs/perf/memory-audit.md
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 7 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 4 / 2 / 0 / 0 / 0 / 0 / 1 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A measured memory audit whose code-level attribution is precise and still
checkable: F-2's root cause (`EbmlReader::read_prefix_at` allocating a
fresh `vec![0u8; want]` per positioned read) is exactly in today's tree
(ebml.rs:344), the X2b fix landed as the documented zero-alloc twin
(`read_prefix_into_at`, ebml.rs:361, used on the packet payload path
mod.rs:812) while the cluster-window slide still uses the allocating
variant (:603/:652 — per-cluster, not per-frame). The instrument
(`tools/mem-audit`) exists; the headline numbers (59,472 kB peak on the
4K lane, 1.32/0.22/4.10 calls-per-pkt) are session measurements with no
committed per-file raw table — REAL-UNPROVEN at the number level, REAL at
the structure level.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "raw lines are quoted or tabled verbatim from tool output" (tools/mem-audit) | :6-8 | REAL | `ls tools/` -> `mem-audit` present; docs/perf/raw/ has committed csv/logs (memory.csv, decode.csv, environment.json) though not the per-file table itself. |
| 2 | F-2 root cause: "`EbmlReader::read_prefix_at` (`crates/demux/src/mkv/ebml.rs:307-329`) allocates a fresh `vec![0u8; want]` per positioned read" | :105-109 | REAL | Today ebml.rs:336-348 `pub fn read_prefix_at(...) -> Result<Vec<u8>, ...>` with :344 `let mut out = vec![0u8; want];` — the defect condition persists verbatim. |
| 3 | "its only packet-path caller (`crates/demux/src/mkv/mod.rs:577-583`) then **copies** that throwaway Vec into the pooled arena" | :109-111 | REAL (then) / evolved | Today's packet payload path calls the zero-alloc twin (mod.rs:812 `.read_prefix_into_at(buf, at, ...)`) — the X2b fix #1 landed after this audit, exactly as its ranked fix list prescribed; the window-slide path (:603/:652) still uses the allocating variant (per-cluster window, bounded). |
| 4 | "F-1 (VERIFIED FIXED): whole-pipeline steady state on the mjpeg lanes is 0.22-1.32 calls/pkt (heavy: 33 calls for 150 frames)" | :131-134 | REAL-UNPROVEN | Measured claim; method (calls_per_pkt detector) is coherent, no committed raw line to re-derive per-file counts. |
| 5 | "corpus-v4 has **no 1080p h264-in-mkv file** — the largest h264-MKV is 0014 at 1280×720 (ffprobe-verified)" | :66-68 | REAL-UNPROVEN | Honest lane note; corpus-v4 media not committed in this tree (regenerable corpus design), cannot re-probe. |
| 6 | "native FLAC is a typed probe refusal (`probe.no-demuxer`)" — bounded, honest | :96-99 | REAL | Consistent with the shipped probe registry state (mkv/mp4/wav/ogg lanes named :19-23 as the landed families; probe refusal is the documented in-tree behavior — see compat-matrix/config.rs `vlc: null` fallback discipline). |
| 7 | Instrument claims: VmHWM read/reset via `/proc/self/clear_refs` mode 5 (200MB touch test: 207,848 → 12,624 kB) | :13-18 | UNVERIFIED | Kernel-interaction measurement on the authoring host; not re-runnable in this sandbox (no such corpus/process budget). |

## 3. Contradictions / Fabrication
None. The doc explicitly refuses to conflate `peak_B` (requested) with
`rss_hwm_kB` (resident) and documents its own allocator-floor honesty
limitation (:27-33) — anti-inflation discipline.

## 4. Matrix rows
None. Feeds the D-1 (zero-allocation) compliance story; the F-2→X2b fix
chain is the strongest evidence class in this file.
