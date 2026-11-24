# CLAIM AUDIT: docs/formats/network-http.md
| Field | Value |
|---|---|
| Auditor | W19-A08 | Claims extracted | 8 |
| REAL / REAL-UNPROVEN / SHALLOW / STUB / PAPER-ONLY / FALSE / UNREACHABLE | 5 / 2 / 0 / 0 / 0 / 1 / 0 |

## 1. Blunt verdict
Library-stack claims verify: fetch port, client, ranged resource, scheme registry all in
crates/streaming. Its one negative claim — "The CLI open path does not dispatch through the
registry yet; every SourceSpec::Url still refuses at apps/cli/src/demux_host.rs" — is now
FALSE: the w31.3 wiring landed and the CLI open path serves http(s) through the streaming
crate (product.toml [features] streaming-http = true with that exact comment;
demux_host.rs:13-14 says the same).

## 2. Claim ledger
| # | Claim (verbatim, <=25 words) | Location | Verdict | Evidence |
|---|---|---|---|---|
| 1 | "the fetch port (6.A1a), the admitted ureq client (6.A1b/6.A1c), the ranged seekable resource (6.A2a) ... landed and tested in crates/streaming" | L3-7 | REAL | crates/streaming/src/{fetch,client,ranged,reconnect,buffer,auth}.rs all exist |
| 2 | "the no-range linear-scan fallback (6.A2b)" | L4-5 | REAL | streaming ranged module + tests (module presence verified) |
| 3 | "the scheme dispatch registry (6.A3) ... landed" | L6-7 | REAL | streaming::schemes (in-crate fuzz_scheme_classify cited; module present) |
| 4 | "**The CLI open path does not dispatch through the registry yet** ... every SourceSpec::Url still refuses at apps/cli/src/demux_host.rs (UnsupportedForHost ...)" | L8-11 | FALSE (stale) | apps/cli/src/demux_host.rs:13-14: "an `http(s)` URL opens through the streaming crate's ranged resource"; :357 "serves every `http(s)` source"; product.toml [features]: streaming-http = true "w31.3: the CLI/GUI open path serves http(s) through the streaming crate's ranged resource" |
| 5 | "status = implemented (library); platform-independent code, evidence Linux-only (mock-server tests + the two --ignored live-net tests)" | L17-18 | REAL | consistent with matrix evidence convention; ignored live-net tests are the honest marker |
| 6 | "fuzz_target = fuzz_scheme_classify (in-crate ...)" | L20-21 | REAL | in-crate fuzz presence (registry uses per-parser names; this is an in-crate harness — recorded honestly) |
| 7 | "ureq client ... admitted" | L4 | REAL-UNPROVEN | ureq in Cargo.lock/deps (dependency record docs are other agents' files) |
| 8 | "network-tls-policy cover the client/TLS feature rows" | L16-17 | REAL-UNPROVEN | matrix rows other chunks; ADR-011-tls-https.md exists (docs/adr/) |

## 3. Contradictions
| This doc says | Other source says | Which is true (proof) |
|---|---|---|
| "CLI open path does not dispatch through the registry yet" | demux_host.rs:13/357 + product.toml streaming-http=true | code wins — the open-path-for-http handoff landed (w31.3) |

## 4-5. Fabrication / matrix rows
No inflation; the record understates current reality. protocol-http-https matrix row is
another chunk's scope. Commands: ls streaming/src; rg demux_host.rs; sed product.toml.
