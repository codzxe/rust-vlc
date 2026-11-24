# CLAIM AUDIT: docs/baselines/vlc-3.0.23.json
| Field | Value |
|---|---|
| Auditor | W19-A06 |
| Date | 2026-12-19 |
| Claims extracted | 10 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 7 / 1 / 0 / 0 / 0 / 0 / 2 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
The consolidated VLC 3.0.23 pin record is heavily cross-anchored and every
anchor I can touch verifies: all five `record_paths` files exist on disk;
the tarball sha256 `e891cae6…` matches raw/linux/10 sidecar :19 verbatim;
the signing-key fingerprint `65F7C6B4…` appears in raw/macos/05-gpg-
verification.txt:11,:19; tarball size 26486988 matches the upstream dir
listing (raw/linux/09:15); `--list-plugins` exit=1 claim matches
raw/linux/07 tail; the seven macOS dumps named under `dumps.available` all
exist; DR-01/DR-11 discrepancy entries exist in vlc-3.0.23.md:143,:153;
clean-room claims (`binaries_committed: false`) hold — no >40 MB binaries
in the tree (only .git pack) and .gitignore:40/:79 cover tmp/ and *.exe.
The only unverifiables: the "7/7 PASS" consolidation recheck (hashes live
in the gitignored, absent `tmp/pinning/`) and hardware-blocked items, which
the file itself marks `[VERIFY-blocked-on-hardware]` honestly.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | Valid JSON, 394 lines, consolidating 0.A1/0.A2/0.A3 | :1-13 | REAL | `python3 json.load` -> JSON-VALID; keys list verified. |
| 2 | `record_paths` master/windows/linux/macos/hardware_template | :14-21 | REAL | `ls docs/baselines/` -> all 5 files present (vlc-3.0.23.md, -windows.md, -linux.md, -macos.md, hardware-template.md). |
| 3 | source tarball sha256 sidecar `e891cae6aa3c…3e7e0` | :27 | REAL | docs/baselines/raw/linux/10-upstream-tarball-sidecar-sha256.txt:19 `e891cae6aa3ccda69bf94173d5105cbc55c7a7d9b1d21b9b21666e69eff3e7e0 *vlc-3.0.23.tar.xz`. |
| 4 | tarball `size_bytes: 26486988` | :25 | REAL | raw/linux/09-upstream-vlc-dir-listing.txt:15 `vlc-3.0.23.tar.xz … 26486988`. |
| 5 | signing key fingerprint `65F7C6B4206BD057A7EB73787180713BE58D1ADC`, DSA-1024, expired 2026-08-01 | :34-41 | REAL | raw/macos/05-gpg-verification.txt:11 (issuer fpr) + :19 (recv-keys same fpr) — file exists and carries the fingerprint. |
| 6 | "source_tarball_downloaded": false (clean-room) | :28-30 | REAL | No vlc-3.0.23.tar.xz in repo (`find -size +40M` -> only .git pack; source forbidden per doc 02 §2.1 note carried in-file). |
| 7 | clean_room: binaries committed? false; location "gitignored tmp/pinning/ (git check-ignore verified; *.exe additionally ignored)" | :364-370 | REAL | .gitignore:40 `tmp/`, :79 `*.exe`; `ls tmp/pinning` -> absent (gitignored, as claimed); no committed VLC binaries in tree. |
| 8 | flag finding: "--list-plugins does not exist on VLC 3.0.x (… observed exit 1 on 3.0.23, raw/linux/07)" | :356-357 | REAL | raw/linux/07-vlc-list-plugins-attempt.txt tail: `# exit=1 (note: vlc returns nonzero-with-message via usage error path)` + --help shows `-l, --list` only. |
| 9 | macOS dumps 00-06 available at raw/macos/ | :311-318 | REAL | `ls docs/baselines/raw/macos/` -> 00-macosx-directory-listing.html … 06-web-citations.txt — all 7 present. |
| 10 | "consolidation_recheck … 7/7 PASS (…) sha256sum over artifacts retained in gitignored tmp/pinning/" | :358-362 | UNVERIFIED | tmp/pinning/ absent in this checkout (gitignored by design); retained hashes are not in-repo; re-verification blocked. Also DR-01/DR-11 discrepancy entries do exist in vlc-3.0.23.md:143,:153 ✓. |
| 11 | windows/macos capture claims (`[VERIFY-blocked-on-hardware]`, `run_status: blocked-on-hardware`) | :56+ | REAL-UNPROVEN | Honest pending-markers; expected shapes recorded rather than asserted. |
| 12 | git_describe "3.0.23-2-0-g79128878dd" | :4 | UNVERIFIED-as-live | Matches raw/linux/04 dump family + vlcrc dump 16 (:1866 "3.0.23 Vetinari") cross-doc consistency; original observation not re-runnable. |

## 3. Contradictions / Fabrication
None found. The record deflates its own confidence (expired key, DR-01
3.0.24 drift, pending hardware) instead of overstating it.

## 4. Matrix rows
None. This record is the version-pin authority cited by
docs/feature-matrix.toml `baseline` meta (:11 `baseline = "VLC 3.0.23
Vetinari (3.0.23-2-0-g79128878dd)"` — matches this file's git_describe).
