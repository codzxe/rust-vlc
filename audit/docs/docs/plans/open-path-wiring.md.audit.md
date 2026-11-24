# CLAIM AUDIT: docs/plans/open-path-wiring.md
| Field | Value |
|---|---|
| Auditor | W19-A04 |
| Claims extracted | 10 |
| REAL 5 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 1 / FALSE 0 / UNVERIFIED 2 / superseded-by-implementation 2 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
An investigation/design card that did exactly what it said: prescribed a container-host
registry seam, and the tree now contains that seam verbatim — 12 container files under
apps/cli/src/containers/, the registry call replacing the AVI-only gate, the
TrackFact.extra_data extension, and the honest unwired-refusal default arm. Its
time-bound claims (10 probe signatures, "no software H.264 decode", the unlanded-codec
refusal list) were true at its commit and have since been superseded by exactly the
follow-up units it scheduled. The doc's own honest-limits section (§6) is candid.
No fabrication found.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Exactly one site is AVI-hardcoded — the format gate in apps/cli/src/demux_host.rs:298-310" | :12-13 | REAL (historical, resolved) | apps/cli/src/demux_host.rs:458 now calls `crate::containers::open(&container, stream, &opts)` — the prescribed §4.3.4 replacement landed; containers/mod.rs:81 `unwired_refusal` keeps "the same message shape the D-018 gate answered with" |
| 2 | seam = per-container construction + facts adapter behind a single registry match | :13-16 | REAL | apps/cli/src/containers/ contains mod.rs + avi/mp4/mkv/ogg/wav/aiff/ts/ps/adts/ivf/es hosts; mod.rs:39 "THE dispatch: the probed format id → its wired container host", :74 `other => Err(unwired_refusal(other))` |
| 3 | media-probe builtin registry "exactly 10 signatures: Avi, Mp4, Wav, Aiff, Ogg, Mkv, Adts, Ts, Ps, Ivf" | :56 | REAL-at-time, now 15 | signature/mod.rs:127-136 with_builtins registers the 10; containers/mod.rs:105-110 records "all 15 registered signature ids (adts aiff avi flac h264 hevc ivf matroska mp2 mp3 mp4 ogg mpegps mpegts wav)" — the es unit added the 5 the plan's 6.OPW-es row scheduled |
| 4 | TrackFact gains `extra_data: Option<SharedBytes>`; provider seeds config from facts | :241-246 | REAL | demux_host.rs:165 `pub extra_data: Option<SharedBytes>`; apps/cli/src/pipeline_host.rs `self.seeded(track, config, ...)` for flac/vorbis/opus/aac/alac/adpcm (verified earlier: :253-302) |
| 5 | test convention: unsupported_path re-targets per wired unit | :300-305 | REAL (executed) | containers/mod.rs:105-110 pin "6.OPW-ps wired the LAST real probe family … re-targets from the real `mpegps` id to a clearly HYPOTHETICAL one: `mpegps-dv-extended`" — the doc's own convention followed to the letter |
| 6 | "Unlanded codecs (opus, h264, hevc, mpeg2 video, vp8/vp9, ac3, dts) refuse typed at pipeline_host.rs:255-262" | :290-292 | superseded (was true then) | pipeline_host.rs:276-346 now dispatches opus (:276-282), av1 (:304-308), vp9 (:313-317), dnxhd (:326-330), h264/avc1 (:346) — later waves landed the ADR-001 software track |
| 7 | "h264/hevc modules are parsers + hardware-path seam only — no software H.264 decode here" | :186-187 | superseded (was true then) | crates/codecs/src/video/h264/mod.rs:1-12 now documents TWO decode paths (hardware + the rust_h264 software path behind codec-h264-sw, "updated 2026-11-15") |
| 8 | corpus census "143 files, verified census: avi 44, mkv 31, mp4 24 …" at /tmp/my-project/corpus-v4/media | :311-314 | UNVERIFIED | out-of-repo path (matches the corpus-v4 manifest lineage but the media tree is not committed; manifest audit (this agent) found no media/ in repo) |
| 9 | "Zero code changes made this session (doc + worklog only)" | :3 (§6 :340) | PAPER-ONLY (consistent) | plan doc; its §6 honest-limits section explicitly lists 4 unverified items — none overstated |
| 10 | "every claim below was verified in this worktree this session" (file:line cites from wip/p6-opw0 at f3e5b8c) | :5-6 | REAL-UNPROVEN | spot-checks of surviving cites all match (gate site, signature registry, container API files); historical line numbers at that commit cannot be byte-verified against today's tree (lines have drifted) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "no software H.264 decode here" (:186) | crates/codecs Cargo.toml:73 codec-h264-sw + pipeline_host.rs:346 H264SwDecoder | the CODE wins for today; the doc is a time-bound investigation record (commit-pinned in its header), not a live status claim — no living contradiction |
| "exactly 10 signatures" (:56) | 15 signature ids now registered | same class: the doc's own §5 6.OPW-es row scheduled the extension |

## 4. Fabrication / inflation indicators
- None. The doc's central claims are checkable and check out; its unverified list is
  explicit; the design it proposes is in the tree. The "verified census" of the
  out-of-repo corpus is the only unverifiable number and it is labeled with its path.

## 5. Matrix rows
N/A (plan doc; the open-path rows it feeds are containers 21-32 in other agents' chunks).

## 6. Open questions for the next agent
- None open from this doc; all five §4.3 seam-prep items landed.
