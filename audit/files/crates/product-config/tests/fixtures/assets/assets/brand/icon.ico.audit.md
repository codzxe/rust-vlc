# AUDIT: crates/product-config/tests/fixtures/assets/assets/brand/icon.ico (binary)
| Field | Value |
|---|---|
| Auditor | W11-A02 |
| Date | 2026-11-23 |
| Size / bytes | 15 bytes: `4E 4F 54 2D 49 43 4F 00 00 00 00 00 00 00 00` (= ASCII "NOT-ICO" + 8 NULs) |
| File verdict | REAL (fixture stub, purpose-built negative payload) |
| Reachable from shipped entry point? | NO — test fixture asset; consumed by tests/semantic.rs:248-260 `brand_icon_with_wrong_magic_bytes_is_rejected` via assets-wrong-magic.toml [brand] icon_ico (fixture :93) + base_dir tests/fixtures/assets (semantic.rs:243-245) — executed green this session |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 1 |
| Commands run | `file` -> "ASCII text, with no line terminators"; `stat` -> 15 bytes; `od -A x -t x1z` (bytes above + 3 siblings); `cargo test -p product-config --test semantic --offline brand_icon` -> 1 passed, exit 0; `rg 'fixtures/assets' feature-matrix.toml docs/` -> 0 hits; `ls assets/brand/` -> No such file or directory |

## 1. One-paragraph verdict
A deliberately wrong-magic stub: it must FAIL the ICO check (`starts_with
00 00 01 00`, semantic.rs:505) and it does — the bytes literally spell
"NOT-ICO". Unlike sibling icon-512.png (a valid-magic non-image, audited
STUB by W11-A07), this file never claims to be an icon, so as a negative
test payload it is complete and honest. The consumer test executes green
and pins the exact contract: one diagnostic on brand.icon_ico whose rule
names "ICO"+"magic" and whose `found` renders the file's first 8 bytes hex
("file starts with 4E 4F 54 2D 49 43 4F 00", semantic.rs:521-522), plus
`diags.len() == 1` proving the other three assets pass. Cross-ref Wave 1:
the REAL assets/brand/icon.ico does not exist — the entire assets/ tree is
absent (`ls: cannot access 'assets/': No such file or directory`, re-ran
today), baseline xtask-config-validator.txt:11 reports "asset path
`assets/brand/icon.ico` does not exist under the root" (4 violations,
FAILURE), filed as F-001 [S3] by W01-A08 (product.schema.json.audit.md)
with gen-icon.py:5-6's admission. This fixture is the stand-in proving the
validator WOULD catch a bad icon — while the shipped tree ships none and no
gate runs the validator.

## 2. What actually works (verified)
— `cargo test -p product-config --test semantic --offline brand_icon` ->
`test brand_icon_with_wrong_magic_bytes_is_rejected ... ok` (1 passed,
exit 0). Mutation honesty: remove/invert the magic check -> the len==1
assert fails (0 or 4 diags); mislabel the kind -> `d.rule.contains("ICO")`
fails ("PNG" doesn't contain "ICO"); skip the file read -> 0 diags fails.
— Byte dump matches the fixtures README contract exactly (README.md:57,
:67: "`icon.ico` deliberately contains wrong magic bytes, the other three
assets are valid"); siblings icon-512.png / icon.icns / logo.svg carry
their correct magics (od-verified), so exactly one violation fires.

## 3. Findings
F-001 [S4] No valid ICO fixture exists anywhere: the ICO positive magic
(00 00 01 00) is never exercised against an on-disk file through
asset_checks' read path.
Where: crates/product-config/tests/fixtures/assets/assets/brand/ (4 files, all od-dumped), src/semantic.rs:549-564
Code (semantic.rs:550-555, verbatim):
```
        assert!(
            AssetKind::Ico
                .magic_ok(&[0x00, 0x00, 0x01, 0x00, 0x00])
                .is_ok()
        );
        assert!(AssetKind::Ico.magic_ok(b"not-ico").is_err());
```
What is wrong: the positive ICO magic runs only through this inline unit
test; among on-disk assets, ICO is the one kind with no magic-valid
instance (PNG/ICNS/SVG all have one), so the disk-read->Ok path is
untested for ICO specifically. Mitigation: the read path is shared by all
four kinds and positively covered by the other three; a bytes-blind ICO
mutant is still caught by the unit test's `is_err()` assert. Why it
matters: the exact real-world payload (an actual .ico from a designer)
has no integration pin. What correct looks like: add a valid 00 00 01 00
stub alongside the NOT-ICO one. Confidence: HIGH — Verification: ran
od over all 4 fixture assets + read the unit test.
(Note: unit test's negative sample is lowercase b"not-ico"; the fixture
covers the uppercase "NOT-ICO" through the real file path — variants
complement, no gap.)

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "`icon.ico` has wrong magic bytes" | tests/fixtures/README.md:57 | REAL | od: `4E 4F 54 2D …`, fails semantic.rs:505 prefix |
| "`icon.ico` deliberately contains wrong magic bytes, the other three assets are valid" | tests/fixtures/README.md:67 | REAL | Test executed: exactly 1 diagnostic, other 3 magics od-verified |
| (cross-ref) real assets/brand/icon.ico exists / brand assets shipped | product.toml [brand] (via W01-A08 F-001 [S3]) | FALSE for the repo state | baseline xtask-config-validator.txt:11-14 FAILURE (4 violations); `ls assets/` -> No such file or directory (re-ran 2026-11-23); gen-icon.py:5-6 admits absence |

## 7. Dead / duplicated / unreachable code
Dev/test lane only. No doc/matrix row cites this fixture
(`rg 'fixtures/assets' feature-matrix.toml docs/` -> 0 hits) — honest:
evidence rows cite the validator, not this stub. Siblings audited:
icon-512.png (W11-A07, STUB), icon.icns (W11-A07, STUB), logo.svg
(W11-A10, REAL).

## 9. Open questions for the next agent
None for this file. The standing S3 (missing real brand assets + orphaned
validator enforcement) is W01-A08's F-001 — do not re-file per-file.
