# CLAIM AUDIT: docs/inventory/raw/modules/manifest.json
| Field | Value |
|---|---|
| Auditor | W18-A07 |
| Date | 2026-11-24 |
| Claims extracted | 7 derivation/reproducibility claims + clean-room claim + 3 live-URL claims |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | REAL 10 / UNVERIFIED 1 (clean-room attestation, unfalsifiable by nature) / FALSE 0 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
This manifest is fully reproducible and I reproduced it: running the committed parser
`parse-vlc-list.py` over the committed raw dumps regenerates
`docs/inventory/modules-linux.json` byte-identically (python3 `==` comparison true), exit 0,
total 502, rollup sum 502. Every "checks_enforced" claim in the manifest is verifiable in
the script source and in the committed output: the 1:1 ordered terse-vs-verbose
cross-check is two hard `assert`s in the script (which passed during my run), the four
no-capability entries are exactly `export, bridge, playlist, core`, and
`sum(by_capability.count) == total == 502` holds. The three cited web sources are live
(HTTP 200 today). The clean-room claim is an attestation I cannot falsify, but the parser
source I read in full consumes only the two dump files — consistent. This is what the rest
of the doc estate should look like and mostly doesn't.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "contains": "502 module lines (terse: name + description)" (dump 05) | manifest.json:13 | REAL | docs/baselines/raw/linux/05-vlc-list.txt:8 `# Module count: 502 (one line per module)`; executed parser cross-check asserts terse==verbose counts (passed). |
| 2 | "the same 502 modules with 's' shortcut lines and 'c <capability> (<score>)' lines" (dump 06) | :19 | REAL | 06-vlc-list-verbose.txt:7 `# Module count: 502`; script's terse/verbose 1:1 assertion ran clean (exit 0). |
| 3 | "script": "parse-vlc-list.py" + run command "<06-dump> <05-dump> ../../../inventory/modules-linux.json" | :24-25 | REAL (executed) | This session: `python3 parse-vlc-list.py ../../../baselines/raw/linux/06-vlc-list-verbose.txt ../../../baselines/raw/linux/05-vlc-list.txt /tmp/w18a07-modules-regen.json` → exit 0, stdout `total 502 | rollup: {demuxers 57, decoders 35, access 57, filters 110, subtitle 32, playlist 21, services-discovery 26, other 164} | sum 502`. |
| 4 | "output: ../../../inventory/modules-linux.json (validated with python3 -m json.tool)" | :33 | REAL (executed) | Regenerated output `==` committed modules-linux.json (python3 equality TRUE); `python3 -m json.tool docs/inventory/modules-linux.json` → exit 0. |
| 5 | "exactly 4 entries have none (export, bridge, playlist, core -> capability "(none)")" | :29 | REAL | Committed JSON `by_capability['(none)'].modules` names == `['export','bridge','playlist','core']` (python3 read); `sum(by_capability.count)` == 502 == total. |
| 6 | "seven-category rollup buckets sum to 502 (sum_check field)" | :31 | REAL | modules-linux.json:132 `"sum_check": 502` inside `seven_category_rollup` (:30) — verified by my re-run rollup (sum 502). |
| 7 | "s2_web_sources_accessed_2026-10-19" (wiki module index + 12 module pages + releases page) | :36-40 | REAL (live) | This session curl: `200 https://wiki.videolan.org/Documentation:Modules/`, `200 …/Modules/dshow/`, `200 https://www.videolan.org/vlc/releases/3.0.0.html`. |
| 8 | "clean_room": "No VLC source code, headers, or decompilation was used…" | :41 | UNVERIFIED (attestation) | Unfalsifiable by nature; consistent with evidence: the parser source (read in full) consumes only the two runtime dumps. |
| 9 | "platform": "linux (Debian 13 trixie, x86_64, headless container, uid 1001)" / oracle identity "VLC 3.0.23 … Debian 3.0.23-0+deb13u1, plugin set vlc-plugin-base + vlc-plugin-video-output" | :6-7 | REAL | 05-vlc-list.txt:3-5 headers byte-match: "Env: Debian 13 (trixie) container, x86_64, headless", "Plugin set: vlc-plugin-base + vlc-plugin-video-output (Debian 3.0.23-0+deb13u1)". |
| 10 | (downstream R10 linkage, my matrix chunks) vlc_modules cited by rows 001-013/053-065 trace into this census | feature-matrix.toml rows | REAL (31/32) | python3 name-set check: mkv, mp4, ogg, ts, ps, avi, asf, wav, flacsys, aiff, au, es, mpeg4audio, mpegaudio, h264, hevc, speex, dvbsub, webvtt, ttml, subsdec, archive, subtitle, libass, mpc, g711, aes3, uleaddvaudio, mpegvideo, h26x, avcodec all present in the census (356 unique names). The one miss, `fluidsynth`, is honestly disclosed: dump 05:6 "Excluded packages … -fluidsynth", and row 055's notes say "vlc-plugin-fluidsynth was an excluded Debian package". |

## 3. Contradictions
None found. The manifest's numbers, script behavior, dump headers, and downstream JSON
all agree with each other and with my independent re-execution.

## 4. Fabrication / inflation indicators
None. The only soft spot is the clean-room attestation (unfalsifiable) and the fact that
reproducibility rests on committed dumps I cannot re-capture from a real VLC install
(sandbox has no vlc binary; the dumps' own headers say they were captured 2026-10-19 —
their provenance chain is the baseline pin, hash-checked per docs/baselines records).
Note for the record: a prior wave audited the downstream file
(audit/files/docs/inventory/modules-linux.json.audit.md) and reached the same conclusion
on inputs; my contribution here is the byte-identical re-execution of the derivation.

## 5. Reachability (repo side)
Not applicable — evidence-manifest data doc; feeds R8/R10 of the feature-matrix gates.
The matrix rows in my chunks (vlc_modules arrays) trace into this census at 31/32 with
the one miss disclosed — R10 anti-hallucination holds for rows 001-013 and 053-065.

Commands run (this agent): python3 parse-vlc-list.py (regeneration, exit 0, byte-equal
output); python3 json.load/json.tool equality + field reads (by_capability '(none)' names,
sum_check, total); rg/sed over dumps 05/06 headers and modules-linux.json; curl ×3 on the
cited web sources; python3 census name-set check against 32 vlc_module names from my 26
matrix rows.
