# AUDIT: tests/rebrand/assets/icon.icns
| Field | Value |
|---|---|
| Auditor | W16-A07 |
| Date | 2026-11-25 |
| Lines / functions | binary, 314 bytes |
| File verdict | REAL (valid ICNS; minimal single-color test art — correct for a rebrand skeleton) |
| Reachable from shipped entry point? | NO (test asset for the rebrand tool) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 0 |
| Commands run | python3 ICNS + PNG parse + zlib scanline check (exit 0); baseline cite |

## 1. Verdict
`file`: "Mac OS X icon, 314 bytes, ic07 type". Parsed by me: `icns` magic, total size
0x13a = 314, ONE entry `ic07` (306 bytes) containing a valid 298-byte PNG, 128 x 128, 8-bit
RGB; decompressed scanlines contain only 3 unique byte values — a single solid-color icon.
sha256 85c57cd05ae9fea3... (recomputed). This is deliberately minimal TEST ART for the
rebrand skeleton (nova.toml's icon_icns path), not shipped product art — appropriate.

## 2. Consumers (verified)
- tests/rebrand/nova.toml:64 `icon_icns = "tests/rebrand/assets/icon.icns"`.
- tests/rebrand/rebrand_skeleton.rs:65-70 NOVA_ASSETS list; the skeleton rebrand test
  `nova_fixture_rebrand_passes_skeleton_assertions` RAN AND PASSED in the committed baseline
  (cargo-test-workspace.txt:6788 `test rebrand_skeleton::nova_fixture_rebrand_passes_skeleton_assertions ... ok`).
- semantic.rs guard `brand_icon_with_wrong_magic_bytes_is_rejected` (baseline:5153) enforces
  valid magic for brand icons — this file's `icns` magic passes that lane.

## 3. Findings
None. Checked: container structure, entry size arithmetic (8 + 306 = 314), embedded PNG
validity, consumer wiring with a passing baseline run.

## 4. Open questions
None.
