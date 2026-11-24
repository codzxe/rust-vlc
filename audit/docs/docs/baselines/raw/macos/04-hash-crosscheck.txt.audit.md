# DATA AUDIT: docs/baselines/raw/macos/04-hash-crosscheck.txt (credit: W19-A09, recovered by W19-S2)
| Field | Value |
|---|---|
| Auditor | W19-S2 (recovery) |
| Date | 2026-11-24 |
| Type | Raw hash computation + cross-check log (53 lines): 4 algorithms x 3 VLC 3.0.23 macOS dmgs |
| Live re-verified? | YES — FULL artifact verification 2026-11-24: arm64 dmg downloaded (51,273,389 B) and all 3 recorded hashes recomputed byte-identical |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |

## Verdict
Fully re-verified against the real artifact, not just the file's own consistency:
- LIVE download of vlc-3.0.23-arm64.dmg (Content-Length 51273389) today:
  sha256 `fc6fac08d87f538517d44aca0c5e7a244b67c8c4cb589bf478363a7315fd5e0d`,
  md5 `c5870b7e9b98726fe0af4de368541753`, sha1 `1fa89d566b17d934009d9e701a2e9c16b79fa797`
  — all three byte-identical to the record's lines 11/23/19 (exit 0).
- The 9/9 internal cross-check table (:27-49): every "computed" line equals the
  "sidecar" line, and the sidecar values are byte-identical to raw evidence 02's
  verbatim captures (same values in 02-official-sidecars.txt:24-64) — three-way
  agreement (upstream sidecar == record 02 == record 04 computed).
- universal sha256 `56ee657c...1d1f6ec9` additionally matches doc 02 §2.2's pinned
  value (docs/02-phase0-audit-plan.md:103) and my live sidecar re-fetch from the
  02-official-sidecars audit — four independent records agree.
- SHA-512 block (:14-17) is self-consistent in format (128 hex chars x3) and its
  "no official .sha512 sidecar exists" note matches the live HTTP 500 I observed on
  the .sha512 URL.
No defects. Checked: one full live artifact hash trio, the 9x internal MATCH table,
3-way agreement with record 02, doc-02 pin, SHA-512 format. Downloaded artifact
deleted after verification (no repo pollution).
