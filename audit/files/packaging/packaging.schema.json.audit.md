# AUDIT: packaging/packaging.schema.json
| Field | Value |
|---|---|
| Auditor | W17-A07 |
| Date | 2026-06-14 |
| Lines / functions | 253 / 10 schema sections |
| File verdict | REAL |
| Reachable from shipped entry point? | YES — enforced at every release-lane job's parse step: scripts/packaging/packaging-env.py:276 `validate(packaging, schema, "packaging.toml")` (release.yml runs it on all 6 jobs) |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |

## 1. One-paragraph verdict
A real, machine-enforced contract — not documentation theater. I executed the validator (packaging-env.py's `validate()` imported and run against this schema with the real packaging.toml): PASS; and mutation-tested four hostile config values (algorithm "md5", a whitespace doc path, string "true" for a boolean, a lower-case env name): ALL FOUR REJECTED with actionable error messages. The claim that "config can never name an unwired algorithm" is real — the enum is ["sha256"] and the release workflow's manifest writer hard-fails on anything else. packaging-env.py's keyword whitelist covers every keyword this schema uses, so no constraint here is silently ignored.

## 2. What actually works (verified)
— `json.load(packaging/packaging.schema.json)` -> parses (ran).
— Real config validates: PASS (ran).
— Mutation rejections (ran, in-memory copies only — no repo files touched):
  - `mutant.checksum.algorithm: 'md5' is not one of ['sha256']`
  - `mutant.shipped.docs[0]: 'a b.md' does not match '^[A-Za-z0-9][A-Za-z0-9._/-]*$' (repo-relative doc path with NO whitespace ...)`
  - `mutant.windows.upgrade_test: expected boolean, got str ('true')`
  - `mutant.signing.windows.pfx_b64_env: 'lower-case' does not match '^[A-Z][A-Z0-9_]*$'`
— Keyword coverage: schema uses {type, required, additionalProperties, properties, items, enum, const, minimum, maximum, minItems, uniqueItems, pattern} — all whitelisted in packaging-env.py:69-74; unknown keywords hard-error.

## 3. Findings
F-001 [S4] Where: packaging/packaging.schema.json:61 — the docs pattern `^[A-Za-z0-9][A-Za-z0-9._/-]*$` forbids more than whitespace (also `+`,`(`, etc.) — stronger than the documented rationale; harmless. Confidence HIGH (read).
F-002 [S4] Where: packaging/packaging.schema.json:177-181 — `desktop_pending_exit` required+validated but no release-lane assert consumes it (w30.8 retirement; disclosed in packaging.toml's comment). Dead-but-required config. Confidence HIGH (read).

## 4. Per-section table
| Section | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| schema_version | 11-15 | REAL | - | const 1, min/max 1 |
| checksum | 16-32 | REAL | - | enum ["sha256"] (mutation-tested) |
| sbom | 33-49 | REAL | - | enum ["1.5"] |
| shipped | 50-67 | REAL | - | no-whitespace pattern (mutation-tested) |
| desktop_entry | 68-80 | REAL | - | boolean type enforced (mutation-tested) |
| linux | 81-120 | REAL | - | FHS anchors, rpm patterns |
| macos | 121-141 | REAL | - | exactly-2 darwin targets |
| windows | 142-157 | REAL | - | upgrade_test bool |
| launch_test | 158-183 | REAL | S4 | F-002 |
| signing | 184-250 | REAL | - | 8 secret env-name patterns (mutation-tested) |

## 5. Test honesty
Enforcement executed both positively (PASS) and negatively (4 mutants rejected) — stronger than an in-repo test.

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "constraints ... enforced by packaging-env.py's structural pass" | $comment L5 | REAL | executed validate() + mutants |
| "config can never name an unwired algorithm" | $comment L24 | REAL | enum + mutation rejection (ran) |
| "product identity fields are deliberately ABSENT" | $comment L5 | REAL | no identity sections; packaging-env.py L278-281 re-asserts |
| "existence checked by ... semantic pass" | $comment L63/L175 | REAL | semantic pass read; fixture exists on disk |

## 7. Dead / duplicated / unreachable code
`minLength` whitelisted but unused (forward capacity). desktop_pending_exit (F-002).

## 8. Dependencies & flags
Plain JSON; enforcer is the subset validator in packaging-env.py (uses re.fullmatch — stricter than JSON-Schema's unanchored semantics; tightens).

## 9. Open questions for the next agent
1. Future `oneOf`/`anyOf`/`$ref` would be rejected loudly by the whitelist — the extension discipline is documented; fine for now.
