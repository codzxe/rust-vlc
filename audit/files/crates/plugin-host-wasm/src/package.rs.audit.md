# AUDIT: crates/plugin-host-wasm/src/package.rs
| Field | Value |
|---|---|
| Auditor | W12-A05 |
| Date | 2026-11-21 |
| Lines / functions | 525 / 12 (2 consts + 2 structs + 2 enum/impl groups + 7 fns + 5 tests + 2 helpers) |
| File verdict | REAL (bounds-safe ZIP reader + honest unverified-signature posture; executed green via its own unit tests AND integration suites) |
| Reachable from shipped entry point? | NO — no app Cargo.toml depends on plugin-host-wasm (rg over apps/*/Cargo.toml: zero hits; boundaries.toml:574 declares it as a crate island). Reached only by its own bins (`lumenext`, `lumenext-examples`) and the crate's tests, all green in the baseline repair run. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 2 S4: 3 |
| Commands run | Read package.rs 1-526 (full); `rg -ln "read_package|LumenextPackage|stamp_manifest_hash|module_digest" crates apps plugins tools scripts tests` (callers: install.rs, bin/lumenext.rs, lib.rs, tests/{common,plugin_install_lifecycle,plugin_examples_conformance}, plugins/extensions manifests use the digest string); `rg -n "fn |zip_raw|zip_stored|corrupt|BadMagic|CrcMismatch|CompressedEntry" crates/plugin-host-wasm/tests/plugin_install_lifecycle.rs` (integration coverage of every read_package error path — BadMagic 70, CrcMismatch 164, CompressedEntry 177, missing manifest 183, missing module 191); baseline `orchrun-test-workspace-nofailfast.log.txt:4522-4526` (package::tests 5/5 ok), `:4591-4594` (plugin_install_lifecycle 4/4 ok incl. plugin_malicious_manifest), `:4569-4576` (conformance 5/5 ok incl. `lumenext_cli_round_trip_packages_and_inspects_both_examples` — a real package→inspect round trip); `rg -n "plugin-host-wasm" Cargo.toml apps/*/Cargo.toml` (no shipped consumer). NOTE: quotes verified with the Read tool. |

## 1. One-paragraph verdict
This is a real, deliberately minimal stored-ZIP reader with honest security posture: every length comes from the central directory or local header and is bounds-checked against the archive before any slice (no panic path I could construct on 64-bit), entries are capped (64 MiB package, 1024 entries), the wasm native-code fence (`\0asm`) is enforced, and the signature block is recorded — never trusted — with the crypto gap stated in the module docs rather than papered over. The provenance half (`stamp_manifest_hash`) is the weak spot: its line matcher fires on ANY line starting with `hash`, is not scoped to the `[provenance]` section its own error message claims, and clobbers the matched line's formatting; the ZIP reader accepts duplicate entry names with an undocumented first-wins rule. In-file tests cover only signature/stamp/digest; the entire ZIP parse path is tested exclusively at integration level (tests/plugin_install_lifecycle.rs, verified green) — acceptable but asymmetric for a parser of untrusted bytes.

## 2. What actually works (verified)
— Verified by baseline `orchrun-test-workspace-nofailfast.log.txt:4522-4526` → `package::tests::{signature_block_parses_and_classifies, garbage_is_not_a_zip, module_digest_is_the_provenance_form, stamp_refuses_tampered_and_missing_hash_lines, stamp_writes_the_real_digest_over_the_placeholder}` all `ok`.
— Verified by baseline `:4591-4594` → `plugin_malicious_manifest ... ok` (this test drives `read_package` through BadMagic, CRC-corrupted plugin.wasm, a deflate entry, a missing manifest, and a missing module — every typed refusal path of the reader, per my read of tests/plugin_install_lifecycle.rs:57-191) and `:4576` → `lumenext_cli_round_trip_packages_and_inspects_both_examples ... ok` (a real build→read round trip through the committed example extensions).
— Verified by read: the EOCD backward scan window (66,000 bytes, 222-235) covers the ZIP-spec maximum comment (65,535) plus the 22-byte EOCD; `close_plan`-style state leaks are absent here — every u32/u16 read (`u16le`/`u32le`, 236-239) is preceded by a length check (230, 250, 265, 313, 322); `usize` arithmetic cannot overflow on the 64-bit host target with the 64 MiB package cap in force (189-192).
— Verified by read: `module_digest(b"")` == `sha256:e3b0c442…b855` (test 460-468) — the empty-input SHA-256 constant, pinning the in-crate hasher against a known vector.

## 3. Findings

### F-001 [S3]
Where: crates/plugin-host-wasm/src/package.rs:364-366 (with 397-403)
Code:
```
        let trimmed = line.trim_start();
        if trimmed.starts_with("hash") && trimmed.contains('=') && trimmed.contains("\"sha256:") {
            found += 1;
```
(and the error that claims section scoping, 397-403):
```
        return Err(malformed(
            "no `hash = \"sha256:…\"` line under [provenance] — extensions must carry \
             provenance (doc 06 §2.2)"
```
What is wrong: the matcher is prefix-based and section-blind. (a) Any line whose key merely STARTS with `hash` — `hash_algo = "sha256:none"` in any section — is treated as the provenance hash line, counted, and REPLACED by `hash = "{digest}"` (375, 390), silently destroying the original key. (b) The scan never checks it is inside `[provenance]`, yet the not-found error says "under [provenance]" — a `hash = "sha256:…"` line in any other section satisfies it. (c) The replacement discards the line's leading indentation and any trailing comment (only `expected` is pushed).
Why it matters: `stamp_manifest_hash` is the developer-facing packaging step (`lumenext package`, doc 06 §2.3); a false-positive match rewrites an unrelated manifest key and can also mask a genuinely missing provenance line. The downstream `plugin_api` manifest validator (W12-A07/A08's file) is the only thing that would catch the corruption later.
What correct looks like: parse field/key exactly — split the line on `=`, compare the key (trimmed, section-tracked) to `hash` — and preserve the original indentation when writing `expected`.
Confidence: HIGH — Verification: read (mutation reasoned: a manifest containing `hashnote = "sha256:x"` in [provenance] is stamped and its key destroyed; no test covers a `hash`-prefixed non-hash key).

### F-002 [S3]
Where: crates/plugin-host-wasm/src/package.rs:285-292
Code:
```
    let entry =
        entries
            .iter()
            .find(|e| e.name == name)
            .ok_or_else(|| PackageError::MissingEntry {
                name: name.to_string(),
            })?;
```
What is wrong: `read_central_directory` (222-282) never checks for duplicate entry names; `extract` resolves a name to the FIRST central-directory entry in file order. A package carrying two `manifest.toml` entries (or two `plugin.wasm`) installs the first, silently, with no error and no surface distinguishing the shadowed twin. The 64 MiB cap and entry-count cap make this cheap to craft.
Why it matters: an undocumented first-wins rule on untrusted package bytes is exactly the ambiguity a malicious publisher can exploit later (e.g., a marketplace UI listing entry names from a different tool that reads the LAST entry). With v1's no-crypto posture, structural strictness is the only defense; this is a hole in it.
What correct looks like: reject packages with duplicate root-entry names (typed `Malformed`), or at minimum document the first-wins rule in the module docs.
Confidence: HIGH — Verification: read (no dedup anywhere in 222-335; no test constructs a duplicate-name archive — checked tests/common/mod.rs `zip_raw` and both integration suites).

### F-003 [S4]
Where: crates/plugin-host-wasm/src/package.rs:110-145
Code:
```
        for line in lines {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
```
(with the vacuous hex check, 141-145):
```
        if !value.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err(PackageError::SignatureUnparsable {
                detail: "`value:` is not hex".to_string(),
            });
        }
```
What is wrong: duplicate `suite:`/`key:`/`value:` fields are accepted silently (each assignment overwrites the previous — last wins); an EMPTY `value:` passes the hex check vacuously (`all()` on zero bytes is true); and there is no length/shape validation for the one known suite (ed25519 ⇒ 128 hex chars) even though the suite id is parsed.
Why it matters: nothing is verified in v1, so the blast radius is bounded to garbage-in-garbage-recorded — but `SignatureStatus::Unverified` then names a "suite" whose block may be structurally nonsense, and install's consent flow (W12-A09's file) surfaces this text to the user.
What correct looks like: reject duplicate fields, reject an empty `value`, and validate the hex length against `KNOWN_SUITES`.
Confidence: HIGH — Verification: read.

### F-004 [S4]
Where: crates/plugin-host-wasm/src/package.rs:407-457 (in-file test module)
Code:
```
    #[test]
    fn garbage_is_not_a_zip() {
        assert!(matches!(
            read_package(b"not a zip at all"),
            Err(PackageError::NotZip)
        ));
    }
```
What is wrong: the unit tests in this file never exercise `read_package`'s happy path or any structural error besides NotZip — no stored-ZIP round trip, no TooLarge, no Malformed local header, no entry overrun. That coverage exists only in tests/plugin_install_lifecycle.rs (integration), which I verified by reading it — so the code IS tested, but a reader of package.rs alone would believe the parser is untested.
Why it matters: test-asymmetry debt: the untrusted-bytes parser should own at least one happy-path unit test so a refactor cannot silently orphan the integration coverage.
What correct looks like: one in-file test building a minimal stored ZIP (the `zip_stored` helper already exists in tests/common/mod.rs:427) and asserting manifest+module+signature extraction.
Confidence: HIGH — Verification: read + baseline logs cited above.

### F-005 [S4]
Where: crates/plugin-host-wasm/src/package.rs:225, 230, 236-239, 250
Code:
```
    let scan_start = bytes.len().saturating_sub(66_000);
```
(and 230, 250):
```
        if bytes[i..].len() >= 22 && bytes[i..i + 4] == [0x50, 0x4b, 0x05, 0x06] {
```
```
        if off + 46 > bytes.len() || bytes[off..off + 4] != [0x50, 0x4b, 0x01, 0x02] {
```
What is wrong: the three ZIP signatures (`PK\x05\x06` EOCD, `PK\x01\x02` central, `PK\x03\x04` local) and the field offsets (10, 16, 26, 28, 42, 46) are unexplained inline magic numbers, while the format's own root names get named constants (`ENTRY_MANIFEST` etc., 46-48). The 66,000 window is also unexplained in place (it is 65,535 max comment + 22 EOCD + margin — verified correct, but nothing in the code says so).
Why it matters: structure/format debt in the security-relevant half of the file: a future editor retyping one signature byte or offset gets no compile-time or review-time help, and the reviewer must re-derive the 66,000 bound from the ZIP spec to prove it safe.
What correct looks like: `const EOCD_SIG: [u8; 4] = …; const CD_SIG/CentralHeader::SIG …` plus a one-line comment deriving the 66,000 window.
Confidence: HIGH — Verification: read.

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| PACKAGE_CAP_BYTES / ENTRY_CAP_BYTES / MAX_ENTRIES | 36-43 | REAL | — | 64 MiB / 1024; enforced at 190, 241, 299 |
| ENTRY_MANIFEST / ENTRY_MODULE / ENTRY_SIGNATURE | 46-48 | REAL | — | root names; used by install + lumenext bins |
| struct LumenextPackage | 51-60 | REAL | — | raw bytes out; manifest validated downstream (plugin_api) |
| struct SignatureBlock | 63-74 | REAL | — | pure data |
| SignatureBlock::parse | 99-152 | REAL | S4 | duplicate-field last-wins + empty hex value = F-003; magic-line check strict; unknown fields rejected |
| SignatureBlock::status | 155-165 | REAL | — | KNOWN_SUITES lookup; never returns a "verified" state — honest |
| struct ZipEntry | 169-177 | REAL | — | u32 sizes ⇒ ZIP64 impossible ⇒ always refused by caps (4 GiB > 64 MiB) |
| read_package | 188-218 | REAL | — | cap → CD → manifest → module → wasm magic `\0asm` (196) → optional signature; error path per entry typed; integration-tested (F-004 note) |
| read_central_directory | 222-282 | REAL | S3 | bounds-safe on 64-bit; no duplicate-name check = F-002; EOCD window correct; name UTF-8 lossy (270) — a non-UTF8 name can never equal the ASCII roots, so it is ignored, not a bypass |
| extract | 285-335 | REAL | S3 | method-0 gate, size caps, stored-size equality, local-header re-read of name/extra lens, overrun check, CRC-32 vs CD value; first-wins `.find` = F-002 |
| module_digest | 338-340 | REAL | — | `sha256:<hex>` form; empty-vector constant pinned by test |
| stamp_manifest_hash | 359-405 | REAL | S3 | placeholder/tamper/round-trip logic correct and tested; matcher too loose = F-001 |
| tests: ok helper | 412-417 | REAL | — | unreachable! on Err — test-only |
| tests: signature_block_parses_and_classifies | 419-449 | REAL | — | 5-bad-input matrix; discriminating (verified green) |
| tests: garbage_is_not_a_zip | 451-457 | REAL | — | trivial; only in-file read_package test (F-004) |
| tests: module_digest_is_the_provenance_form | 459-468 | REAL | — | pins the SHA-256 empty constant — would catch a broken hasher |
| tests: manifest_with_hash helper | 472-478 | REAL | — | doc 06 §2.3 shape |
| tests: stamp_writes_the_real_digest_over_the_placeholder | 480-495 | REAL | — | stamped-once + no-op round trip; discriminating |
| tests: stamp_refuses_tampered_and_missing_hash_lines | 497-524 | REAL | — | diverging digest / no line / two lines; discriminating |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| signature_block_parses_and_classifies | YES | wrong magic / non-hex / missing field / unknown field each asserted Err; suite classification asserted both ways |
| garbage_is_not_a_zip | WEAK | only proves the EOCD scan refuses text; parser happy path untested in-file (F-004) |
| module_digest_is_the_provenance_form | YES | exact 64-hex constant for empty input |
| stamp_writes_the_real_digest_over_the_placeholder | YES | placeholder→digest, single occurrence, round-trip no-op — a regression in any of the three breaks an assert |
| stamp_refuses_tampered_and_missing_hash_lines | YES | three distinct refusal shapes asserted |
| (integration) plugin_malicious_manifest | YES | CRC corruption, deflate, bad magic, missing roots each assert the SPECIFIC PackageError variant (tests/plugin_install_lifecycle.rs:57-191, read by me; green in repair run 4594) |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "v1 reads **stored** (method 0) entries and integrity-checks every entry's CRC-32; compressed entries are refused with a typed error" | package.rs:5-9 (module doc) | REAL | extract 293-333; integration test asserts `CompressedEntry { method: 8 }` (lifecycle 170-178) |
| "v1 parses and records the block but does **not** verify cryptographic signatures" | package.rs:22-29 | REAL | no crypto dep in Cargo.toml (W01-A02 audit); status() can only answer Unsigned/Unverified/UnknownSuite |
| "manifest.toml at the root, plugin.wasm (wasm magic required — the native-code fence), … 64 MiB size cap" | package.rs:3-5 | REAL | 196-204 magic; 36 cap; 190-192 enforced |
| "card 7.G1's `lumenext package` … both go through here — one implementation" | package.rs:347-349 | REAL | bin/lumenext.rs calls stamp_manifest_hash (rg hit); conformance round-trip test green |
| "The hash line must exist exactly once" | package.rs:351-354 | REAL for the count; FALSE for placement | count enforced (368-372, 397); placement NOT under [provenance] (F-001) |

## 7. Dead / duplicated / unreachable code
No dead code. `ENTRY_CAP_BYTES == PACKAGE_CAP_BYTES` is a deliberate alias (documented 38-39). The whole crate is unreachable from shipped entry points (header) — its only consumers are its own bins/tests, all executed green in the baseline repair run.

## 8. Dependencies & flags
`crate::crc32::crc32` (W12-A04's file — its correctness is load-bearing for entry integrity; the integration CRC test would catch a broken table), `crate::error::PackageError` (W12-A10), `crate::sha256::sha256_hex` (W12-A10 — pinned by the empty-vector test here). No external deps, no feature gates. std-only, per the crate's MP §9 no-libc rule.

## 9. Open questions for the next agent
- W12-A09 (install.rs): does install re-validate that the stamped manifest's [provenance] section is where the hash landed (mitigating F-001), or does it trust stamp's output?
- W12-A10 (sha256.rs): the empty-vector constant here pins one vector; the 64-hex length check is implicit — confirm sha256_hex has broader vectors.
- Does docs/06-plugin-architecture.md §5.7 specify duplicate-entry behavior? If it does, F-002 may be a spec violation, not just a gap.
