# CLAIM AUDIT: docs/dependencies/records/ureq-3.5.0.md
| Field | Value |
|---|---|
| Auditor | W17-A09 |
| Claims extracted | 20 |
| REAL | 10 |
| REAL-UNPROVEN | 2 |
| UNVERIFIED | 8 (network / vendored-source) |
| Internal contradictions found | 1 (test name cited does not exist) |

## 1. Blunt verdict
The record's repo-checkable core is exactly true: the feature pin, the lock checksum,
all 11 dependency versions of the pasted R2 tree, the three new deny.toml license
entries, the "ureq is the only external dep" claim, and the single TlsConfig site
that never touches `disable_verification` — all verified byte-for-byte against the
tree. The one defect: the record names a test `tls_verification_is_always_on` that
does not exist under that name anywhere in the repo; the real test is
`agent_config_pins_transport_safety` (crates/streaming/src/client.rs), which does pin
TLS-verification-on — the claim's substance is true, its citation is wrong. The
network-bound claims (OSV advisory arithmetic, live HTTPS fetch, vendored ring
source counts) are honestly marked and cannot be re-executed here.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence (path:line, command + output) |
|---|---|---|---|---|
| 1 | "Our admission: `ureq = { version = \"=3.5.0\", default-features = false, features = [\"rustls\", \"rustls-webpki-roots\"] }`" | L24-25 | REAL | crates/streaming/Cargo.toml:52 — identical incl. feature list |
| 2 | "Cargo.lock carries ureq 3.5.0 checksum `cde93ff05758a36a851a398a772b0b79a82fbe3c186a46f2696baaeaf84c966b`" | L237-238 | REAL | Cargo.lock ureq block checksum — BYTE-IDENTICAL |
| 3 | R2 tree: "ring v0.17.14", "rustls v0.23.46", "ureq-proto v0.6.5", "webpki-roots v1.0.11", "cc v1.10.0", "find-msvc-tools v0.1.15", "shlex v2.0.1", "rustls-webpki v0.103.16", "rustls-pki-types v1.15.1", "utf8-zero v0.8.1" | L87-123 | REAL | all 11 versions matched in Cargo.lock via rg (ring 0.17.14, rustls 0.23.46, ureq-proto 0.6.5, webpki-roots 1.0.11, cc 1.10.0, find-msvc-tools 0.1.15, shlex 2.0.1, rustls-webpki 0.103.16, rustls-pki-types 1.15.1, utf8-zero 0.8.1) |
| 4 | "deny.toml's minimal allow set has been extended with exactly these three" (ISC, BSD-3-Clause, CDLA-Permissive-2.0) | L192-196 | REAL | deny.toml:100-102 `"ISC", "BSD-3-Clause", "CDLA-Permissive-2.0",` with ring/subtle/webpki-roots attribution comments at :93-96 |
| 5 | "the adapter's single `TlsConfig` construction site (`crates/streaming/src/client.rs`, `tls_config()`) never touches ureq's `disable_verification` hatch" | L34-37 | REAL | client.rs:91-97 `tls_config()` builds Rustls+WebPki only; `rg disable_verification crates/` hits only comments + the test's negated assert |
| 6 | "the `tls_verification_is_always_on` test pins that shut" | L37 | FALSE (name) / REAL (substance) | no test by that name exists (rg across crates/apps/tests = 0 hits); the actual test `agent_config_pins_transport_safety` (client.rs:~921) asserts `!tls.disable_verification()` — substance true, citation wrong |
| 7 | "ureq stays the only external dep" (streaming) | implied by pin + record scope | REAL | crates/streaming/Cargo.toml [dependencies]: media-types, plugin-api, playlists (path deps) + ureq — exactly one registry dep |
| 8 | "`cargo test -p streaming` 37 passed / 0 failed / 1 ignored (live)" | L260 | REAL (for its date) / drifted | baseline cargo-test-workspace.txt: streaming now 13 test binaries, all `ok`, 0 failures (counts grew after 2026-10-30) |
| 9 | "ring 0.17.14 ... 135 native source files ... 230 `unsafe` tokens" | L54-57 | UNVERIFIED | vendored ring source not present in sandbox; registry tarball claims |
| 10 | "ureq 3.5.0 DOES ship the escape hatch upstream (`...disable_verification(bool)`, `src/tls/rustls.rs:166`)" | L38-41 | UNVERIFIED | ureq source line refs not checkable in-tree (registry-only); the tree-side effect (our code never uses it) is verified (#5) |
| 11 | "OSV querybatch ... all six are fixed in our locked versions: RUSTSEC-2025-0009 (ring ... fixed 0.17.12 < our 0.17.14) ..." | L211-221 | REAL-UNPROVEN | version arithmetic is consistent with the lock (0.17.14 ≥ 0.17.12, 0.23.46 ≥ 0.23.45, 1.0.11 ≥ 1.0.10, 1.12.1 ≥ 1.11.1, 0.103.16 ≥ 0.103.10 — all lock-confirmed), but advisory existence/ranges are network claims |
| 12 | "live fetch of `https://example.com/` returned 200 with a real rustls+webpki-roots handshake" | L262-263 | UNVERIFIED | network; no live fetch possible in this audit sandbox |
| 13 | "`rust_version` 1.85 ≤ 1.99.1" (ureq) + "the whole tree's max MSRV is 1.85" | L21-23 | UNVERIFIED (MSRV sweep) / REAL (toolchain) | rust-toolchain.toml:9 channel 1.99.1; per-crate MSRV sweep not re-runnable |
| 14 | "cargo-deny is NOT installed in this sandbox (measured: `cargo deny` → \"no such command\")" | L197-199 | REAL-consistent | sandbox has no cargo-deny either (record's honesty note) |
| 15 | "ureq 3.5.0 is the current release; not yanked ... crates.io REST API was NOT reachable" | L204-208 | UNVERIFIED | network; the unreachability is itself honestly recorded |
| 16 | "no `-sys` crates, no git deps, no downloads at build time" | L125-127 | REAL | lock tree contains no -sys nodes in the ureq graph (verified via the R2-version sweep; find-msvc-tools/shlex are cc's build deps, not -sys) |
| 17 | "23 normal-edge crates + 3 build-edge" | L125 | REAL-UNPROVEN | count not re-derived (would need full tree walk of the lock's ureq closure; versions all confirmed) |
| 18 | "Redirects are never followed by ureq here (`max_redirects(0)`...)" (R6; agent code) | R6 L159-181 | REAL | client.rs:924-930 test asserts `config.max_redirects() == 0` and the module docs at :104-110 record the same |
| 19 | "ureq 3.5.0 has NO `timeout_write`/`timeout_read` knobs" | L171-175 | REAL-UNPROVEN | ureq API surface claim (registry docs); our side's honest gap note is verifiable in the adapter's TimeoutConfig wiring |
| 20 | "everything ... was executed locally in the build sandbox on 2026-10-30" | L267-274 | REAL-UNPROVEN | evidence posture; the repo-verifiable subset all checks out (see #1-#8) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "the `tls_verification_is_always_on` test pins that shut" (L37) | crates/streaming/src/client.rs has `agent_config_pins_transport_safety` (asserting the same property); no `tls_verification_is_always_on` exists repo-wide (rg = 0 hits) | The PROPERTY is pinned (test body verified); the record's test NAME is fabricated-by-renaming — the true name is `agent_config_pins_transport_safety`. Minor citation defect (S4). |

## 4. Fabrication / inflation indicators
Low. The record is unusual in recording its own bad news: it admits ureq SHIPS the
disable_verification hatch (contradicting the card's preferred world, L38-45), admits
the API gaps (L171-181), admits cargo-deny absence, and admits crates.io REST
unreachability. The one inflated item is the test name (contradiction #1), which
overstates the specificity of the safety pin.
