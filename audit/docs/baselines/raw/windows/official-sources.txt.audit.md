# CLAIM AUDIT: docs/baselines/raw/windows/official-sources.txt
| Field | Value |
|---|---|
| Auditor | W19-A02 |
| Claims extracted | 9 concrete claims |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNVERIFIED | 3 REAL / 0 / 0 / 0 / 0 / 0 / 6 (network/GPG claims not re-runnable) |

## 1. Blunt verdict
Source-of-record for the VLC 3.0.23 Windows pin: URLs, sidecar hashes, GPG packet
details, release dates. Internally consistent with acquisition-log.txt and with the
feature-matrix baseline string; the network claims are point-in-time and not re-run.
Its honesty markers are strong: it records the signing key as EXPIRED, the SHA-512
values as computed-by-us (not cross-checkable), and the front-door mirror drift to 3.0.24.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | win64.exe 45,948,080 bytes (31-Dec-2025 12:48) + msi 60,608,512 | §1 | REAL (internal) | matches acquisition-log.txt transcript byte counts (45948080) |
| 2 | exe sha256 `20ad1913…39daaf`, sha1 `ee04a6c1…`, md5 `f711b3ae…` | §3 | REAL (as recorded; hash not re-computable — binary not in repo) | sidecar contents verbatim in both docs, sizes consistent (87 B no-newline shape) |
| 3 | "no .sha512 sidecar exists anywhere in the 3.0.23 tree. The SHA-512 values … are computed by us" | §3 NOTE | REAL (honesty marker) | self-declared limitation, not hidden |
| 4 | GPG: Good signature from "VideoLAN Release Signing Key (2018)" [expired], key expired 2026-08-01 | §4 | UNVERIFIED | key fetch + verification not re-runnable here; the expired-key disclosure is against the record's interest — credible |
| 5 | release date 2026-01-08 (news anchor `news-2026-01-08`) | §5 | UNVERIFIED | network claim |
| 6 | front door now serves 3.0.24; 3.0.23 URLs remain valid (mirror-drift risk doc 02 §2.5) | §5 | UNVERIFIED | network claim; consistent with apt policy in the linux env record (candidate 3.0.24) |
| 7 | 3.0.23 dir contains winarm64/, win32/, source tar.xz deliberately NOT downloaded (clean-room rule) | §6 | UNVERIFIED | network claim; the clean-room refusal matches doc 02 §2.1's rule as cited |
| 8 | TLS cert CN=www.videolan.org, Let's Encrypt, no -k used | §7 | UNVERIFIED | network claim |
| 9 | negative results: /keys/ → 404, KEYS → 404, no machine-readable release date, no osslsigncode | §8 | UNVERIFIED | network claims — but recording 404s and tool absences is anti-inflation behavior |

## 3. Contradictions
None internal. Cross-doc: baseline version string "VLC 3.0.23 Vetinari
(3.0.23-2-0-g79128878dd)" (feature-matrix.toml:37) — the git-hash suffix has no
derivation inside these two raw files; it must come from the Windows-host version
check recorded elsewhere (pin record). Not contradicted; noted as an open thread.

## 4. Fabrication / inflation indicators
None — the record documents its own verification gaps ([VERIFY-blocked-on-hardware] for
Authenticode, expired key, self-computed SHA-512).
