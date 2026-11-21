# AUDIT: crates/device-discovery/src/capability.rs
| Field | Value |
|---|---|
| Auditor | W10-A06 |
| Date | 2026-11-21 |
| Lines / functions | 156 / 9 (1 enum + 2 methods + 1 enum + 1 method + 1 struct + 4 methods + 3 tests) |
| File verdict | UNREACHABLE (REAL, tested; crate has no app consumer) |
| Reachable from shipped entry point? | **NO.** Consumers: `crate::lib` (`Discovery::probe` fills the report, lib.rs:97-124) and `tests/fallback_paths.rs` (queries it — incl. the `ssdp_lane_probes_available_on_this_host` test, baseline :2420). No app depends on `device-discovery` (verified in the ssdp.rs audit: rg over all app manifests → zero). |
| Findings | S0: 0 S1: 0 S2: 0 S3: 0 S4: 2 |
| Commands run | `rg -n "discovery.volumes.enumeration|discovery.network.services|discovery.volumes.notifications"` repo-wide → zero consumers outside this file. `rg -n "DiscoveryReport|Feature::"` over the crate → lib.rs + fallback_paths.rs. `rg -n "fn from_error" -A 12 error.rs` → the probe-error mapping (DependencyMissing collapse). Baseline: `orchrun-test-workspace-nofailfast.log.txt:2374-2376` — 3/3 `capability::tests` ok; `:2420` — the live probe-lane test ok. |

## 1. One-paragraph verdict
A 156-line honesty vocabulary that does what it says: typed verdicts, replace-in-place table semantics, unprobed features defaulting to unavailable (never assumed available), and a probe-error conversion that keeps reasons typed. The three tests are real and green in the citable baseline, and the report is actually filled by `Discovery::probe` and queried by the fallback-path integration test. The two defects are vocabulary-level: an unprobed feature is reported as `NotYetImplemented` (a different statement than "not probed" — the detail string says one thing, the variant type says another), and the `Feature::key()` dotted ids claim to be "feature-matrix vocabulary" while appearing in exactly zero matrix rows or consumers. Unreachable from the shipped binary like the rest of the crate.

## 2. What actually works (verified)
— 3/3 unit tests ok (`orchrun-test-workspace-nofailfast.log.txt:2374-2376`).
— End-to-end use verified: `Discovery::probe` (lib.rs:97-124) sets all three features, and `tests/fallback_paths.rs:66` reads `report().get(Feature::NetworkServiceDiscovery)` after a real socket probe — that test is green in the baseline (`:2420` `ssdp_lane_probes_available_on_this_host ... ok`), so the table is exercised against the live probe path.
— `from_probe_error` → `UnavailableReason::from_error` mapping read (error.rs:96-108): `Unavailable` passes its reason through verbatim, `Io` collapses to `DependencyMissing` with the probe context — the ssdp bind-failure path uses exactly this (ssdp.rs:335-342).

## 3. Findings

### F-001 [S4] — unprobed feature is typed `NotYetImplemented`, conflating "not probed" with "not implemented"
Where: capability.rs:92-104
Code:
```rust
    /// One feature's verdict; unprobed features are honestly
    /// unavailable ("not probed"), never assumed available.
    #[must_use = "pure lookup"]
    pub fn get(&self, feature: Feature) -> Capability {
        for (f, c) in &self.entries {
            if *f == feature {
                return c.clone();
            }
        }
        Capability::Unavailable(UnavailableReason::NotYetImplemented {
            detail: "feature was not probed by this assembly",
        })
    }
```
What is wrong: the doc and detail string say "not probed", but the variant is `NotYetImplemented` — a claim about the *implementation's existence*, not about this assembly's probing behavior. A fully implemented feature that a particular `Discovery` assembly skipped probing would be reported to capability consumers as "not yet implemented".
Why it matters: this file IS the crate's honesty contract ("no invented capabilities", "never a bare string guess"); one of its two default outputs is a category error. Blast radius is currently nil (all three features land typed verdicts at probe time), but any future assembly that probes lazily inherits the mislabel.
What correct looks like: an `UnavailableReason::NotProbed { detail }` variant (or `Unknown`), keeping `NotYetImplemented` for what lib.rs:106-116 actually means by it.
Confidence: HIGH — Verification: read; the in-file test (117-127) pins the current conflation, so it is intended — intent does not fix the vocabulary mismatch.

### F-002 [S4] — `Feature::key()`'s "feature-matrix vocabulary" is a dangling claim: the dotted ids exist nowhere else
Where: capability.rs:54-63
Code:
```rust
impl Feature {
    /// The stable dotted key (feature-matrix vocabulary).
    #[must_use = "pure accessor"]
    pub fn key(self) -> &'static str {
        match self {
            Feature::VolumeEnumeration => "discovery.volumes.enumeration",
```
What is wrong: rg for the three dotted ids (`discovery.volumes.enumeration`, `discovery.volumes.notifications`, `discovery.network.services`) across the repo finds matches ONLY in this file. `docs/feature-matrix.toml`'s services-discovery rows use different ids (`sd-directory-watch`, `sd-upnp`, …) with empty evidence; no xtask, doc, or consumer reads these keys.
Why it matters: the doc sells the keys as integration points into the project's claimed single source of feature truth; they are an orphan vocabulary. This is the same staleness family as the sd-upnp matrix row (see the ssdp.rs audit's F-004) — the crate and the matrix describe each other with different words.
What correct looks like: either point the matrix rows' `evidence`/`inventory_ref` at these keys (and have `cargo xtask feature-audit` cross-check them), or drop the "feature-matrix vocabulary" claim from the doc.
Confidence: HIGH — Verification: read + repo-wide rg (zero external consumers).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `Capability` (enum) | 17-25 | REAL | — | Two-variant verdict; doc matches. |
| `Capability::is_available` | 30-32 | REAL | — | Pure classification; used by fallback_paths tests + lib. |
| `Capability::from_probe_error` | 37-39 | REAL | — | Delegates to `UnavailableReason::from_error` (error.rs:96-108, read); exercised by ssdp probe's error path. |
| `Feature` (enum) | 43-52 | REAL | — | 3 lanes; matches lib.rs's probe fill. |
| `Feature::key` | 54-63 | REAL | S4 (F-002) | Orphan vocabulary. |
| `DiscoveryReport` (struct) | 68-71 | REAL | — | Vec-backed table; O(n) with n ≤ 3. |
| `DiscoveryReport::new` | 76-78 | REAL | — | Delegates to Default. |
| `DiscoveryReport::set` | 82-90 | REAL | — | Replace-in-place verified by test. |
| `DiscoveryReport::get` | 95-104 | REAL | S4 (F-001) | Default verdict's variant/detail mismatch. |
| `DiscoveryReport::iter` | 107-109 | REAL | — | Borrowing iterator; used by fallback_paths enumeration. |
| tests ×3 | 112-156 | REAL | — | See §5. |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| `unprobed_features_are_honestly_unavailable` (117) | YES | Asserts the exact variant + detail substring. |
| `set_replaces_in_place_not_appends` (130) | YES | Count stays 1 after re-set; verdict flips. |
| `feature_keys_are_stable_dotted_ids` (142) | WEAK-TRIPWIRE | Literals against literals — pins against accidental edits only (acceptable as a stability tripwire; it cannot catch F-002 because nothing external defines the expected set). |
| Live-path coverage (other files) | YES | fallback_paths.rs drives probe → report end-to-end (baseline :2420). |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "The stable dotted key (feature-matrix vocabulary)" | capability.rs:55 | FALSE | F-002: zero matrix rows or consumers reference the keys. |
| "unprobed features are honestly unavailable ('not probed')" | capability.rs:92-93 | PARTIAL | F-001: honest unavailability, dishonest variant type. |
| "A `Capability::Unavailable` is a real implementation of graceful degradation … never fakes success" | capability.rs:8-11 | REAL | Traced through lib.rs (volume lane's typed NotYetImplemented fallback) and ssdp.rs's probe error path. |

## 7. Dead / duplicated / unreachable code
- `Feature::key()` is dead-in-practice (no consumer) — F-002.
- No duplication in-file; the parallel platform-* vocabulary is acknowledged in the module doc as a Lead [PROPOSAL] seam-unification (out of this file's hands).

## 8. Dependencies & flags
- Depends only on `crate::error::UnavailableReason` (and `DiscoveryError` in the conversion). std-only, no unsafe, no feature gates. The wasm blocked-verdict path lives in error.rs/ssdp.rs, not here.

## 9. Open questions for the next agent
- For A02 (error.rs): does `UnavailableReason` have room for a `NotProbed` variant (F-001), or is the vocabulary frozen by the platform-* mirror?
- For the matrix/xtask owners: F-002 + the sd-upnp row staleness (ssdp.rs audit) suggest a sweep aligning `Feature::key()` ids with matrix row ids — one xtask check would pin it.
