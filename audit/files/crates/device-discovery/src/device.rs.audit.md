# AUDIT: crates/device-discovery/src/device.rs
| Field | Value |
|---|---|
| Auditor | W10-A02 |
| Date | 2026-11-12 |
| Lines / functions | 123 (94 production + 29 tests) / 1 trait (1 method), 2 structs, 2 enums, 2 impls + 2 tests |
| File verdict | REAL (vocabulary module) — UNREACHABLE from shipped entry points |
| Reachable from shipped entry point? | NO. No manifest in the tree declares a `device-discovery` dependency (all matches for the name are comments: platform-{linux,macos,windows}/Cargo.toml say "device-discovery is issue #42 — the manifest is an allowlist"). The crate is a substrate whose consumers (platform crates) have not landed. |
| Findings | S0: 0 S1: 0 S2: 0 S3: 1 S4: 0 |
| Commands run | full read; rg (dependency edges across crates/*/apps/*/fuzz manifests, ServiceDiscovery impls, ssdp verdict); baseline citation |

## 1. One-paragraph verdict
An honest, small vocabulary module: the `ServiceDiscovery` domain trait (one bounded `discover` pass; "no devices is Ok(vec![]), never an error" is stated and matches the signature), the `Device` data struct (validated SourceUri + untrusted-label + kind), and the kind/protocol enums (Volume lane + ServiceProtocol::Upnp only, with the mDNS future honestly deferred). There is nothing to break: no arithmetic, no I/O, no parsing. One real implementation of the trait exists in-crate (`volume.rs:228 impl ServiceDiscovery for VolumesService`), and `ssdp.rs` is an explicitly recorded feasibility verdict (UDP multicast cannot ride the TCP-only ureq client; what is implemented is the provable subset — its own words). The module's two tests are trivial but appropriate for plain data. The only finding is reachability: the entire crate is compiled by nothing — no shipped binary links it.

## 2. What actually works (verified)
— verified by baseline `audit/runs/baseline/cargo-test-workspace.txt` (PACKAGE device-discovery): `device::tests::device_is_plain_data ... ok`, `device::tests::service_protocol_names_match_the_manifest_vocabulary ... ok` (package: "38 passed; 0 failed" + 3 integration binaries, 13 more tests).
— verified by rg: exactly one in-crate trait impl (volume.rs:228); `ServiceProtocol::as_str` vocabulary ("upnp") matches the plugin-api manifest vocabulary claim (doc 06 §7.2 examples — the plugin-api crate uses the same token; test asserts it).

## 3. Findings
F-001 [S3]
Where: crates/device-discovery/src/device.rs:1-123 (whole crate context)
Code:
```
/// Network service protocols the discovery layer speaks (or has a
/// recorded verdict for).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ServiceProtocol {
```
What is wrong: not this file's code — its reachability. `rg 'device.discovery'` over every crate/app/fuzz manifest finds only comments (platform crates: "device-discovery is issue #42"). No `device-discovery = { path = ... }` edge exists anywhere, so neither lumen-cli nor desktop links this vocabulary, the trait, or the VolumesService implementation. Per the release-lane rule this is UNREACHABLE code — 94 loc of vocabulary plus the crate's other modules ride only in `cargo test -p device-discovery`. The crate's own header frames it as card 7.D1 with issue #42 pending, which is honest labeling, but any feature-matrix row claiming device/services discovery on a shipped platform would be false today. Confidence: HIGH — Verification: rg over all manifests (comment-only matches listed above).

Checked and clean (zero-finding duty): trait contract text matches signature semantics (`Ok(vec![])` for absence — nothing in this file can violate it); `Device.label` untrusted-text note (V7) is a documented consumer contract, correctly not "enforced" by a data struct; `ServiceProtocol::Upnp` single-variant enum is forward-declared honestly; Display delegates to as_str; no panics, no secrets, no I/O.

## 4. Per-function table (file ≤123 LOC — table included for completeness)
| Item | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| trait ServiceDiscovery::discover | 32-40 | REAL | — | bounded-pass contract; one impl in volume.rs:228 |
| struct Device | 44-54 | REAL | — | validated SourceUri + untrusted label |
| enum DeviceKind | 58-66 | REAL | — | Volume/Service lanes |
| enum ServiceProtocol | 70-76 | REAL | — | Upnp only, mDNS deferred (documented) |
| ServiceProtocol::as_str | 78-87 | REAL | — | "upnp", manifest vocabulary |
| Display for ServiceProtocol | 89-93 | REAL | — | delegates |
| tests (2) | 106-122 | REAL | — | trivial, pass in baseline |

## 5. Test honesty
| Test | Would it fail if code were broken? | Reason |
|---|---|---|
| device_is_plain_data | YES (weakly) | field construction + scheme read-back — catches renames/type changes only |
| service_protocol_names_match_the_manifest_vocabulary | YES (weakly) | pins the "upnp" token |
Adequate for a data module; the crate's substantive proofs live in volume.rs/ssdp.rs suites (38 package tests, baseline green).

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "Implementations must be bounded and non-blocking beyond ctx's budget" | device.rs:29-31 | REAL-UNPROVEN | the one impl (volume.rs:228) was not audited by me (not my file); the contract text is at least stated |
| "UPnP (SSDP discovery ... see crate::ssdp's verdict for what is implemented versus recorded)" | device.rs:72-75 | REAL | ssdp.rs:1-30 is exactly that: a recorded verdict distinguishing the UDP-transport blocker from the implemented provable subset |
| "the two crates deliberately share no types beyond media-types" (vs plugin-api) | device.rs:9-14 | REAL | imports: std::fmt + media_types + crate-internal only (L16-21) |

## 7. Dead / duplicated / unreachable code
The whole crate is unlinked by any consumer (F-001). No duplication found; the plugin-api `Device` twin is deliberately distinct with the conversion assigned to the (unlanded) plugin host — documented, not implemented, and no code pretends otherwise.

## 8. Dependencies & flags
media-types only (+ crate-internal context/volume). No features, no unsafe, std-only.

## 9. Open questions for the next agent
- Which card/issue wires device-discovery into a platform crate (issue #42)? Until then every device-discovery feature-matrix row must be UNREACHABLE.
- Does any feature-matrix row claim SSDP/UPnP or volume discovery as shipped? If yes, FALSE — cross-check the matrix chunks.
