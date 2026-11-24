# CLAIM AUDIT: docs/baselines/raw/windows/official-sources.txt (data/baseline record)
| Field | Value |
|---|---|
| Auditor | W19-A02 (recovered by W19-R4) |
| Claims extracted | 10 |
| REAL 5 / REAL-UNPROVEN 0 / SHALLOW 0 / STUB 0 / PAPER-ONLY 0 / FALSE 0 / UNVERIFIED 5 |
| Internal contradictions found | 0 |

## 1. Blunt verdict
A fetch transcript whose every cross-checkable value is byte-identical across
the two sibling records (acquisition-log.txt full transcript, vlc-3.0.23-windows.md
pin record): the sha256/sha1/md5 sidecar values, the 45,948,080-byte exe, the
60,608,512-byte msi, the 7z/zip sidecar hashes, the mirror-hk.koddos.net
redirect, the 3.0.24 front-door drift, and the 2026-01-08 release date pinned by
MASTER-PROMPT.md:16. The log's locally-computed hashes match the quoted sidecar
values — the sidecars were real text AND the download hashed to them. All hex
strings are well-formed (64/40/32). External network facts (HTTP 200s, TLS cert,
GPG verification) are UNVERIFIABLE in-tree — they are transcript claims only.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | Sidecar sha256 20ad191348684b470ddc4e05204316f3d8e39655f412b3e392a0eef97639daaf | :31 | REAL (cross-doc) | acquisition-log.txt:98 (sidecar fetch) AND :127 (locally computed, same value) — matched; well-formed 64-hex (python3). |
| 2 | Sidecar sha1 ee04a6c1... / md5 f711b3ae... | :36,41 | REAL (cross-doc) | acquisition-log.txt:100/:102 (sidecars) and :129/:130 (computed) — identical values; 40/32-hex valid. |
| 3 | vlc-3.0.23-win64.exe 45,948,080 bytes; msi 60,608,512 | :12,14 | REAL (cross-doc) | acquisition-log.txt:118 `size_download=45948080`; :122 ls; :131 `size=45948080 bytes`; :307 msi `60608512`; vlc-3.0.23-windows.md:34 `60,608,512`. |
| 4 | 7z/zip/msi sidecar sha256 values | :46-48 | REAL (cross-doc) | acquisition-log.txt:351,353,355 quote the identical sidecar text; pin record :34-35,57 (sha256sum -c OK) same values. |
| 5 | "no .sha512 sidecar exists anywhere ... SHA-512 values ... computed by us" | :50-53 | REAL (cross-doc) | docs/baselines/vlc-3.0.23.md DR-04 "No official ... sidecars anywhere ... All n values in the records are our own computations, labeled as such". |
| 6 | "public release/announcement date 2026-01-08, matching MASTER-PROMPT §1" | :90 | REAL | docs/MASTER-PROMPT.md:16 "VLC 3.0.23 (desktop, released 2026-01-08)". |
| 7 | mirror redirect: byte download served via mirror-hk.koddos.net | :24 | REAL (cross-doc) | acquisition-log.txt:118 `url_effective=https://mirror-hk.koddos.net/videolan/vlc/3.0.23/win64/vlc-3.0.23-win64.exe`. |
| 8 | Front door now serves 3.0.24; 3.0.23 URLs remain valid (mirror-drift risk) | :93-97 | REAL (cross-doc) | acquisition-log.txt:47 `href="3.0.24/"`; :322-324 3.0.24 artifact links. |
| 9 | TLS cert / HTTP 200 statuses / GPG "Good signature ... [expired]" | :9,72-78,113-117 | UNVERIFIED | External transport facts; only evidence is the sibling transcript itself (acquisition-log.txt TLS/GPG sections) — documentary, not re-executable (no network lane here). |
| 10 | Negative results (keys 404s, no machine-readable release date, no osslsigncode) | :119-128 | UNVERIFIED | Same class — negative fetch transcripts; consistent with the pin record's DR-04 honesty but not re-executable. |

## 3. Contradictions
None. The three-doc chain (official-sources.txt, acquisition-log.txt,
vlc-3.0.23-windows.md) agrees on every overlapping value; no number appears
twice with two values.

## 4. Fabrication / inflation indicators
None. The document records its own verification limits (Authenticode
[VERIFY-blocked-on-hardware], SHA-512 self-computed, no WebFetch tool) — the
shape of an honest capture log. One structural caveat: sidecar values for the
7z/zip/msi are sidecar TEXT never confirmed against downloaded binaries (those
binaries were not downloaded) — the doc says exactly that (:44-45).
