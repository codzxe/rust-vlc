# DATA AUDIT: docs/baselines/raw/windows/acquisition-log.txt
| Field | Value |
|---|---|
| Auditor | W19-A02 (recovered by W19-R1) |
| Date | 2026-11-24 |
| Size / shape | 488 lines; environment block, URL discovery, downloads, hash computations, GPG check, scope note |
| File verdict | REAL (self-consistent acquisition record; environment claims re-verified live) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | curl --version / openssl version / gpg --version (live); rg hash cross-refs; sed quotes |

## 1. What it is / provenance
Task 0.A1 acquisition log for the VLC 3.0.23 Windows pin, captured 2026-10-19 in this
same sandbox class. Records: environment (uname 5.10.134 kernel, curl, OpenSSL, gpg),
official-directory discovery (download.videolan.org listings verbatim), the win64 file
set (exe/7z/msi + asc/md5/sha1/sha256 sidecars), download meter, multi-hash computation,
GPG signature verification, and an honest scope note ("Windows binaries CANNOT be executed
here"; only this task's files downloaded).

## 2. Data verification (executed)
- Environment claims RE-VERIFIED LIVE today: `curl --version` -> 8.14.1 (byte-identical
  version banner incl. libcurl/OpenSSL line), `openssl version` -> "OpenSSL 3.5.7 9 Jun
  2026", `gpg --version` -> 2.4.7. All three match the log's captured lines exactly.
- Hash three-way agreement: sha256 `20ad191348684b470ddc4e05204316f3d8e39655f412b3e392a
  0eef97639daaf` appears IDENTICALLY at :98 (official sidecar excerpt), :127 (computed
  `sha256sum`), :237 (final record). sha512/sha1/md5 also recorded at :128-130.
- Size agreement: download meter `size_download=45948080` == `ls -la` size 45,948,080 ==
  pin-doc metadata.
- Cross-consumers: docs/baselines/vlc-3.0.23-windows.md:45/94/95/150 and
  vlc-3.0.23.md:103 all carry the same hash with MATCH verdicts — no drift anywhere.
- The log's closing scope note is verifiable in part: no Windows binaries are tracked in
  the repo (the committed tree carries only text evidence).

## 3. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "All commands below were executed verbatim in this sandbox" | :3 | REAL-UNPROVEN | commands plausible; re-execution of env commands matches exactly; downloads not re-run (offline discipline) |
| 2 | "Windows binaries CANNOT be executed here." | :2 | REAL | sandbox is Linux x86_64 (uname line) |
| 3 | sha256 computed == official sidecar | :98,127,237 | REAL-UNPROVEN | internal 3-way agreement exact; the binary itself is not committed to re-hash |
| 4 | "gpg (GnuPG) 2.4.7" | :15 | REAL | live re-run matches |
| 5 | GPG signature predates 2026-08-01 key expiry | exit report R-2 cross-ref | REAL-UNPROVEN | point-in-time network claim; consistently disclosed downstream |

## 4. Findings
None. Checked and how: environment lines (re-executed live, exact match), hash records
(3-way internal agreement + 5 downstream citations agree), size consistency (meter vs ls),
scope note (no binaries in tree — repo scan), structure (sections numbered, END OF LOG
marker present).

## 5. Consumers
docs/baselines/vlc-3.0.23-windows.md, docs/baselines/vlc-3.0.23.md (§2.2 pin table),
docs/baselines/raw/windows/official-sources.txt (sidecar text source). Dead W19-A02's
live re-verification ("curl 8.14.1, OpenSSL 3.5.7 ... no binaries in repo") reproduced
identically by me.
