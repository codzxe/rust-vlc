# CLAIM AUDIT: docs/MASTER-PROMPT.md (MP v1, 369 lines)
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 12 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 7 / 2 / 3 / 0 / 0 / 0 / 0 |

## 1. Blunt verdict
The constitution mostly describes a repo that exists: 31 crates matching §4's list exactly,
no C/C++/asm sources in crates/ or apps/, the boundary check, fuzz registry, baselines, and
gates all present and executing. Its strictest claims survive: "Never claim 'exactly like
VLC'" is honored by the parity tool reporting 0/252 verified. The gaps are the two
config-system planes MP §4 itself lists (layouts/, profiles/ — absent from the tree) and
"config/" (absent). MP is a directive document; only the layout and policy claims are
checkable today.

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "Parity is the share of inventory features in docs/feature-matrix.toml that are verified by automated evidence." | L17-19 | REAL | feature-matrix.toml exists (3421 lines, 252 rows); baseline xtask-parity-report.txt: 0/252 verified on every platform — definition and tooling consistent |
| 2 | "Engine: 100% Rust." (§2) | L29 | REAL | `find crates apps -name '*.c' -o -name '*.cpp' -o -name '*.S'` -> zero files; rav1d admitted asm-off (crates/codecs/Cargo.toml:122-125: `default-features = false`, record "R3 sabotage build (CC=/nonexistent-cc) proves build.rs is inert with asm off") |
| 3 | "The project must never be described as all-Rust." | L35 | REAL (self-consistent; webview exception documented) | N/A — directive |
| 4 | §4 monorepo layout: crates product-config ... testkit | L89-94 | REAL | ls crates/ -> all 30 MP-named crates present (+_template) |
| 5 | §4 layout: "config/ themes/ keymaps/ layouts/ profiles/" | L84 | SHALLOW | ls: keymaps/ themes/ locales/ exist; **layouts/, profiles/, config/ DO NOT EXIST** — the declarative-UI config planes never landed (see doc 07 audit rows 12-14) |
| 6 | "plugins/ builtin/ (compile-time) extensions/ (WASM examples)" | L95 | SHALLOW | plugins/extensions/ has README + metadata-provider + ui-panel; plugins/builtin absent (built-ins live as crates instead — reasonable reinterpretation, drift unrecorded) |
| 7 | "No cyclic crate dependencies ... An automated boundary check runs in CI." | L69-70 | REAL | tools/xtask/src/cmd/check_architecture.rs exists; baseline full_gate.txt runs the arch section (currently RED — separate finding, other agents' files) |
| 8 | "Local gate: cargo fmt ... clippy ... cargo test ... plus UI lint, typecheck and tests" | L260-264 | REAL | scripts/full_gate.sh exists and ran (baseline full_gate.txt with all sections; GATE RESULT FAIL honestly recorded) |
| 9 | "Unsafe code is denied in application and core crates." (§9) | L227-228 | REAL-UNPROVEN | workspace lints check is Cargo.toml inspection (dependency agents' scope); rav1d's unsafe surface is documented as an exception in codecs/src/video/av1/mod.rs:24 |
| 10 | "Fuzz every parser." (§12) + fuzz/ in layout | L255, L99 | REAL | fuzz/fuzz_targets/ — 20+ targets with seed corpora (ls) |
| 11 | "web build: the same engine compiled to wasm32" (§2) | L36-37 | REAL | apps/web exists; baseline rc-smoke: wasm build passes with wasm32 rust-std installed |
| 12 | "hardware decoding (default, confirm by ADR in Phase 0)" | L51-53 | REAL | ADR-001 referenced as accepted 2026-10-19 in product.toml [features] comment (hw-decode=false, Phase 7 7.B per rule F2) — decision trail exists |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| §4 lists layouts/ and profiles/ as repo dirs | they do not exist | ls (above); doc 07's own validator says "profiles/ does not exist yet ... parked plane" |
| §9 "build scripts that invoke C/C++ compilers" forbidden | Cargo.lock contains cc + nasm-rs (rav1d, ring) | rav1d: documented asm-off admission with sabotage-build proof (Cargo.toml:116-125); ring pulls cc — UNVERIFIED whether any release-lane feature compiles it (open question for dependency-audit agents) |

## 4. Fabrication / inflation indicators
None. MP is the anti-inflation document itself (§1 "Never claim... beyond that evidence";
§15 "Agents never mark their own work verified"). The repo follows it: parity 0/252 is the
honest current number.

## 5. Matrix rows
N/A (MP defines the matrix's purpose and honesty rules; row audits are per-chunk — mine is
rows 235-247, done). Commands: ls crates/apps/plugins/fuzz; find for C sources; rg
Cargo.toml/lockfile; baseline citations.
