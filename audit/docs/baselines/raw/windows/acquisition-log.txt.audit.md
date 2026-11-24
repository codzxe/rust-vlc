# CLAIM AUDIT: docs/baselines/raw/windows/acquisition-log.txt
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 8 environment/procedure facts (raw transcript, 488 L) |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 4 REAL (re-verified live) / 0 / 0 / 0 / 0 / 0 / 4 (network fetches not re-run) |

## 1. Blunt verdict
A curl transcript of the VLC 3.0.23 win64 pin acquisition. Environment claims reproduce
byte-identically in the current sandbox; the downloaded artifacts are correctly absent
from the repo (gitignored, confirmed by the log's own `git check-ignore` output and by my
rg sweep). What cannot be re-verified is the network layer itself — expected for a
point-in-time transcript.

## 2. Claim ledger
| # | Claim | Verdict | Evidence |
|---|---|---|---|
| 1 | curl 8.14.1 / OpenSSL 3.5.7 / kernel 5.10.134 / uid z | REAL | re-ran `curl --version`, `openssl version` in this sandbox → identical strings |
| 2 | Windows binaries CANNOT be executed here (Linux sandbox) | REAL | same sandbox class today (no wine, x86_64 Linux host per os-release) |
| 3 | downloaded: win64.exe 45,948,080 B + .7z 39,583,707 B + sidecars | REAL (internal) | matches official-sources.txt §1 file list byte-for-byte (45948080); files correctly NOT in repo (rg --files for *.exe/*.7z/*.dmg → none) |
| 4 | `git check-ignore` shows tmp/ ignored; no binaries staged | REAL (as recorded) | transcript includes the check-ignore output; repo sweep confirms zero binaries now |
| 5 | directory listing 200/18029 B with 3.0.x dirs incl. 3.0.23/3.0.24 | UNVERIFIED | network fetch not re-run |
| 6 | sidecar downloads (sha256/sha1/md5/asc) 200 | UNVERIFIED | same |
| 7 | gpg 2.4.7 present | REAL | gpg --version in transcript; consistent with the sandbox toolchain era |
| 8 | scope note: dmg files belong to task 0.A3, not 0.A1 | REAL (internal) | the NOTE at transcript end states the boundary explicitly — honest scope separation |

## 3. Contradictions
None. The transcript's own negative results (gitignore confirmation, "no binaries
staged") match the current repo state.

## 4. Fabrication / inflation indicators
None. Notable honesty: the transcript records that the sha256 sidecar is 87 bytes
(no-trailing-newline shape) and separates concurrent-agent artifacts from this task's.
