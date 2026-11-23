# WAVE 02 SUMMARY — core types, command layer, control protocol/server

| Field | Value |
|---|---|
| Agents | A01–A10, all completed |
| Files audited | 48 files (57 chunk rows), 57/57 covered after one path-convention fix |
| Findings | S0:0 S1:0 S2:7 S3:57 S4:104 (168 total) |
| Citation gate | 97 checked, 87 verbatim, 10 anchor/format defects (all adjudicated real-text), 0 fabrications |
| Tree purity | clean (A02's reported stray write left no residue) |

## Domain verdict
The command layer, media-types and control-protocol are REAL, tested code —
the honest layer of the project. But the control plane SHIPS NOWHERE:
product.toml:167 `control-server = false` keeps the only consumer of
control-protocol out of the release lane (verified live: binary refuses
startup, exit 2). Served surface: 15/65 commands (CLI/desktop/web), 2/65
(control-server, the rest typed not-ready).

## Worst findings
1. [S2, executed] ids.rs redaction allowlist misses access_token/
   refresh_token/client_secret/auth_token/passwd/sessionid — OAuth
   credentials print verbatim through Display/Debug, contradicting the
   "Display never embeds credentials" contract (W02-A03).
2. [S2, executed] error.rs mask_userinfo masks to FIRST '@' — multi-'@'
   credential URLs leak the password tail; V8 "credentials stripped" false
   (W02-A05).
3. [S2, executed] server.rs subscriptions map unbounded across session,
   reachable pre-auth in read-only mode; "Bounded" comment false;
   Subscribe/Unsubscribe are inert — ServerFrame::Event never constructed
   anywhere (W02-A05/A04/A08).
4. [S2, executed] CLI host silently drops 5 of 10 OpenOptions fields
   (rate, track selectors, forced_subtitle_only) vs doc 05 §5.2 "gaps are
   typed, not silent" (W02-A07).
5. [S2] command.rs: unknown fields inside `args` silently dropped (rustc
   probe), 19/32 CommandResult variants never constructed, doc ranges
   unenforced (probe: volume 7.5 / -999999999 accepted) (W02-A08).
6. [S2] catalog_emission_ledger "partition tripwire" claim FALSE — twins
   drift today (catalog 14/12 vs engine 15/11 on tracks.changed) with both
   tests green (W02-A10).
7. [S2] time.rs Scaler tier: 6 pub methods, zero production callers while
   docs claim the video scheduler holds one; M18's 1.60x perf win
   unreachable (W02-A01).
8. [KEY] doc 05 claim ledger (99 claims): 40 REAL / 16 FALSE / 19 SHALLOW /
   10 PAPER-ONLY / 11 REAL-UNPROVEN / 3 UNVERIFIED, 4 internal
   contradictions — nonexistent CI suites, nonexistent data files
   (keymap schema, menus.toml, ui-actions.toml), seek error codes never
   emitted (W02-A09).
9. [KEY, cross-wave] SEC-07's LUMEN_DEV_INSECURE_TLS=1 is PAPER-ONLY —
   tls_config (streaming/src/client.rs:91-96) reads no env; verification
   never off (W02-A05 confirmed W01-A04's discovery).
10. [S3 cluster] catalog_gen `--schema-out --check` WRITES instead of
    checks (W02-A02); token.rs EntropySource discarded by tests — CSPRNG
    breakage fails zero tests (W02-A04); 11/26 event variants never emitted
    (W02-A08); ~20/45 catalog types catalog-only (W02-A07).

## Carried forward
- Engine-side: seek_error_wire remapping claim (W02-A09 -> Wave 3).
- streaming/src/client.rs needs its own audit row (Wave 10).
- catalog_emission_ledger.rs twin-drift -> Wave 16 test-quality recheck.
- Env note for all agents: sandbox display mangles byte pair `[m` and
  `#[must_use`-like sequences — agents ground-truth quotes via od/python.
