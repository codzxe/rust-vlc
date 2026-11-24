# CLAIM AUDIT: docs/feasibility/sections/subsystems.md
| Field | Value |
|---|---|
| Auditor | W19-A10 |
| Claims extracted | 12 (of ~200 in a 767-line evidence-dense Phase-0 section; sample = census numbers, crate facts, decision inputs) |
| REAL 3 / REAL-UNPROVEN 1 / SHALLOW 0 / STUB 0 / PAPER-ONLY 1 / FALSE 0 / UNREACHABLE 0 / UNVERIFIED 7 (external crates.io facts) | |

## 1. Blunt verdict
A Phase-0 feasibility record (dated 2026-10-19) with an unusually disciplined evidence appendix: every external claim carries an [E#] pointer, the raw fetches' location (tmp/c3i/, gitignored) is stated openly, and a numbered [VERIFY] ledger of 17 open items closes the file. The in-repo-verifiable numbers check out exactly (57 demux registrations = 55 demux + 2 demux_filter; 110 filter-family; ADR numbering correction confirmed). The demuxer family status table is a historical snapshot — the tree now ships avi/mp4/mkv/ps/ogg/asf/adts/aiff demuxer modules — so anyone reading it as current state would be misled; the doc's own "Date of evidence" header mitigates that. External crate facts (crates.io versions/deps, tarball inspections) are UNVERIFIABLE in this sandbox — evidence is gitignored by design; no network.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "VLC Linux census = 57 demuxer registrations (55 `demux` + 2 `demux_filter`; modules-linux.json, counting rule in-file)" | :21-22 | REAL | python3 on docs/inventory/modules-linux.json: seven_category_rollup.buckets.demuxers.count=57; by_capability demux=55, demux_filter=2 — exact match |
| 2 | "VLC Linux census: 110 filter-family registrations (modules-linux.json)" | :370 | REAL | same json: filters bucket count=110 |
| 3 | "doc 01's ADR-009 is WASM threading" (numbering-mismatch correction) | :160-161 area | REAL | docs/01-architecture.md:1502-1503 — ADR-008 Optical disc, ADR-009 WASM threading & COOP/COEP |
| 4 | "AVI | missing — no maintained crate" (family status) | :26 | PAPER-ONLY (historical snapshot) | crates/demux/src/avi/ exists and is the landed first vertical; doc header pins "Date of evidence 2026-10-19" — accurate then, false if read now |
| 5 | "mp4parse ... skips moof/mdat/sidx boxes ... its oracle role for fMP4 is limited" [E17] | :53-60 | UNVERIFIED | tarball inspection; evidence in gitignored tmp/c3i/; no network in sandbox |
| 6 | "mpeg-ps 0.6.0 — published 2026-10-17 (two days old), no_std, 0 native files" [E18] | :130-133 | UNVERIFIED | same; also note doc's own [VERIFY] item 12 (bus factor) |
| 7 | "ureq 3.4.2; default=[rustls,gzip]" [E3] | :evidence appendix | UNVERIFIED | external fetch; ureq IS a repo dependency (worklog w30.4 "ureq allowlist row") — version pin uncheckable offline |
| 8 | "reqwest 0.13.5 ... default-tls=[rustls]" [E1] | appendix | UNVERIFIED | external fetch |
| 9 | "NOT_FOUND checks: kate/dvdnav/dvdread/bluray" [E22] | appendix | UNVERIFIED | external fetch |
| 10 | "raw JSON in tmp/c3i/, gitignored" (evidence location confession) | :699-700 | REAL | consistent with wave-0 pattern: evidence outside repo — audit-visible honesty, but claims rest on absent artifacts |
| 11 | "No VLC source was consulted; no git commit was made (AGENTS §3.8)" | :765-767 | REAL-UNPROVEN | process claim, unverifiable; consistent with clean-room posture asserted repo-wide |
| 12 | "[VERIFY] ledger (numbered, this section's additions)" — 17 numbered open items | :730-762 | REAL | the list exists and is specific (flate2 backend, ring audit, licenses) — honest deferral record |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| AVI demuxer "missing — no maintained crate" (:26) | crates/demux/src/avi/ exists; first vertical landed (KNOWN-LIMITS.md:36, worklog) | Both true at their dates; the doc is a 2026-10-19 snapshot, not a status lie — but no supersede banner exists on the table |
| "transmux 0.27.7 ... multimux 0.13.0 exist on crates.io today" (:34-36) | no in-repo evidence either way | UNVERIFIED — gitignored fetch artifacts |

## 4. Fabrication / inflation indicators
None detected in the verifiable subset; counts match exactly (claim 1 is the kind of number a fabricator gets wrong). The doc inflates nothing: it repeatedly downgrades (e.g. mp4parse's oracle role "limited", MPEG-PS crate "two days old ... bus factor unknown").

## 5. Matrix rows
Not a matrix chunk. Adjacent: no matrix row cites this file's [E#] evidence directly (rg over feature-matrix.toml for 'subsystems' — not an evidence path); the xtask feature-audit baseline already flags 2 missing evidence paths repo-wide (audit/runs/baseline/xtask-feature-audit.txt) — consistent with this doc's evidence living in gitignored tmp/.
