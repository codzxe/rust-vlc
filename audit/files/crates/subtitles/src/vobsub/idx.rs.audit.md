# AUDIT: crates/subtitles/src/vobsub/idx.rs
| Field | Value |
|---|---|
| Auditor | W09-A01 |
| Date | 2026-11-21 |
| Lines / functions | 304 / 7 production items (3 fns + 4 types) + 3 tests |
| File verdict | UNREACHABLE (real, test-executed in baseline; no shipped entry point links the crate) |
| Reachable from shipped entry point? | NO. In-crate chain: `vobsub/mod.rs:131` `parse_vobsub` → `parse_idx`; `vobsub/mod.rs:53` re-export; `vobsub/fuzz.rs:40` fuzz harness → same. Crate consumed only by `fuzz/Cargo.toml:50` + its own tests; `apps/cli`/`apps/desktop-tauri` have no `subtitles` edge. |
| Findings | S0: 0 S1: 0 S2: 1 S3: 1 S4: 2 |
| Commands run | Read tool (whole file); `rg -n "parse_idx|lang_idx|\.index\b"` → caller map (mod.rs:53/131/159/185, fuzz.rs:40, tests); Read vobsub/mod.rs:120-199 (consumer: `0x20u8 + stream.index` at :159, lang_idx NOT consumed); Read vobsub/fuzz.rs (contract "never panics"); `rg -n "overflow|profile" fuzz/Cargo.toml` → `[profile.release] overflow-checks = true` (:59-60) + `ls fuzz/fuzz_targets/` → 35 targets, **no fuzz_sub_vobsub.rs**; baseline log :5501-5515 → vobsub 15/15 ok, :5590 dvb_vobsub_fuzz_survival 2/2 ok; overflow arithmetic hand-computed (i64::MAX vs hh·3.6e6) |

## 1. One-paragraph verdict
The happy path of this parser is genuinely solid — the full_idx_parse test pins size, palette slots 1 and 15, two streams with titles, hex filepos, and lang_idx, and the tolerance test pins exact damage variants — but the file's two hard guarantees are both false at the edges. First, "Never panics" (line 18) is violated: `parse_timestamp` and `parse_delay` do unchecked i64 arithmetic on attacker-controlled numeric fields, so a line like `timestamp: 3000000000000:00:00:000, filepos: 0` overflows (hh·3,600,000 = 1.08e19 > i64::MAX 9.22e18) — a panic in any overflow-checked build, a silently wrapped garbage timestamp otherwise. The baseline gate never caught it because the workspace test suite ran `--release` with default overflow-checks off (INDEX.md deviation note), and the cargo-fuzz target that WOULD catch it (`fuzz_sub_vobsub.rs`, overflow-checks forced on in fuzz/Cargo.toml:59-60) does not exist in `fuzz/fuzz_targets/` (35 targets, none for vobsub). Second, the documented substream range `index: 0..32` (line 53-54) is never enforced: any u8 is accepted with no damage, and the consumer computes `0x20u8 + stream.index` (mod.rs:159), which itself overflows for index ≥ 224.

## 2. What actually works (verified)
— Baseline: `vobsub::idx::tests::{full_idx_parse, tolerances_never_panic, delay_shifts_entries}` 3/3 ok (:5501-5503); `vobsub::spu` 8/8, `vobsub::tests` (mod) 5/5 (:5511-5515); `tests/dvb_vobsub_fuzz_survival.rs` 2/2 ok (:5590-5594) — mutated idx/sub bytes ran through `parse_idx` in the gate without panic (release profile, so F-001 stayed latent).
— Tolerance semantics match the module doc: unknown lines skipped, `#` comments never parse, partial palettes keep leading entries (zip alignment verified: an invalid mid-list entry leaves its slot default-black and does NOT shift later entries), `timestamp:` before `id:` → typed damage (test-pinned :274-278).
— Delay semantics verified: per-stream accumulation (`delays[ci] += sign`), applied to all entries at end-of-parse regardless of line position (:186-192) — matches the doc claim "both before and after this line"; negative delays pinned (:299-302).
— `parse_timestamp` happy-path arithmetic verified: `00:00:01:234` → 1234 ms; `filepos` hex via `u64::from_str_radix` (over-large hex → typed None, no overflow possible).
— Lossy UTF-8 entry (`String::from_utf8_lossy`, :105) — arbitrary bytes never panic the line walk (test :280-282).

## 3. Findings

### F-001 [S2]
Where: crates/subtitles/src/vobsub/idx.rs:206 and 221
Code:
```
    Some(((hh * 3600 + mm * 60 + ss) * 1000 + ms, pos))
```
```
    Some(sign * ((hh * 3600 + mm * 60 + ss) * 1000 + ms))
```
What is wrong: unchecked i64 arithmetic over parser-controlled values. `hh`, `mm`, `ss`, `ms` are `parse::<i64>()` results (:200-203, :217-220) — any value up to i64::MAX parses; `hh · 3,600,000 > i64::MAX` for hh ≥ ~2.57e12, and `mm · 60` alone overflows for mm ≥ ~1.54e17. Concrete crashing input: `timestamp: 3000000000000:00:00:000, filepos: 0` (hh = 3e12) or `delay: 00:200000000000000000:00:000` (mm = 2e17).
Why it matters: the module doc promises "Never panics" (:18) and the fuzz harness contract repeats it ("never panics … by construction of the parsers' bounds-checked paths", vobsub/fuzz.rs:37-38) — both false. Debug builds panic; release builds silently wrap to a garbage timestamp (wrong cue timing, no damage event). The escape hatch that should have caught it is missing: `fuzz/Cargo.toml:59-60` forces `overflow-checks = true` for fuzzing precisely to surface integer overflow, but `fuzz/fuzz_targets/` contains 35 targets and NO `fuzz_sub_vobsub.rs` — the target vobsub/fuzz.rs:18-19 says is registered "exactly as doc 04 §8.5 registers it". Not S1 only because the input is malformed, not valid, and no shipped binary reaches this code.
What correct looks like: checked arithmetic (`checked_mul`/`checked_add` chain like ass/parser.rs:808-811) returning `None` → the existing `BadTimestamp` typed damage; plus a test with the two overflow literals above; plus wiring the missing `fuzz_sub_vobsub.rs` target.
Confidence: HIGH — Verification: read + arithmetic hand-computed against i64::MAX; release-mode masking confirmed (no `[profile]` overrides in root Cargo.toml; baseline tests ran `--release` per INDEX.md footnote).

### F-002 [S3]
Where: crates/subtitles/src/vobsub/idx.rs:136-140 (doc contract at 53-54; consumer mod.rs:159)
Code:
```
                    let index = index_part
                        .trim()
                        .strip_prefix("index:")
                        .and_then(|v| v.trim().parse::<u8>().ok())
                        .unwrap_or(0);
```
What is wrong: the field is documented `index: 0..32` — the `.sub` substream byte is `0x20 + index` (:53-54) — but any u8 is accepted with no range check and no damage, and a missing/unparseable `index:` silently defaults to 0. Consequences traced in the consumer: (a) `id: x, index: 224` → mod.rs:159 `0x20u8 + stream.index` = 0x100 → u8 overflow (debug panic; release wraps to 0 → matches nothing → silently empty stream); (b) two `id:` lines without `index:` both default to 0 → both streams route the same 0x20 substream (silent merge); (c) index 33..223 → `want` outside the real VobSub substream range 0x20..0x3F → silently empty stream.
Why it matters: the file's contract is "tolerant with typed damage" — every other malformed value (size, timestamp, palette) produces a damage variant; the one field whose documented range is load-bearing for PES routing gets none, and it can panic the caller.
What correct looks like: `match v.trim().parse::<u8>() { Ok(n) if n < 32 => n, _ => { damage.push(IdxDamage::BadIndex{..}); 0 } }` (new damage variant), and `u8::saturating_add` or a range proof at mod.rs:159.
Confidence: HIGH — Verification: read (both files); consumer arithmetic at mod.rs:159 traced for all index classes.

### F-003 [S4]
Where: crates/subtitles/src/vobsub/idx.rs:89
Code:
```
/// Parse an `.idx` text (the file bytes; any newline style).
pub fn parse_idx(text: &[u8]) -> VobSubIdx {
```
What is wrong: "any newline style" is false for CR-only files. The walk uses `content.lines()` (:106), and `str::lines()` terminates only on `\n` and `\r\n` — a classic-Mac CR-only idx parses as ONE line, so every directive after the first is lost with no damage event (the trailing `trim_end_matches(['\r','\n'])` at :107 only strips residue from the final line).
Why it matters: doc-vs-code precision; the blast radius is limited to pre-OSX-Mac-authored idx files (rare for Windows-centric vobsub tooling), hence S4.
What correct looks like: either normalize lone CRs before `lines()` (the srt/encoding.rs precedent, :248-263) or narrow the doc to "LF and CRLF".
Confidence: HIGH — Verification: read (`str::lines` semantics are std-documented; the srt sibling handles lone CR explicitly).

### F-004 [S4]
Where: crates/subtitles/src/vobsub/idx.rs:159-160 (field at 81-82; consumer search)
Code:
```
        } else if let Some(rest) = line.strip_prefix("langidx:") {
            idx.lang_idx = rest.trim().parse::<u8>().ok();
```
What is wrong: `lang_idx` (the idx's default-stream selector) is parsed, re-exported (mod.rs:53), and test-pinned (:251) — but consumed by NO production code: `rg -n "lang_idx"` over the crate shows only idx.rs + the mod.rs re-export; `parse_vobsub` (mod.rs:130-199) decodes ALL streams and never reads it. Additionally: a malformed `langidx:` line silently becomes `None` (no damage), an out-of-range value (`langidx: 250` with 2 streams) is accepted unchecked, and sibling misplacements are inconsistent — `alt:`/`delay:` before any `id:` are silently dropped (:156, :164-167) while `timestamp:` before `id:` gets typed damage.
Why it matters: "config declared but never read" plus an inconsistent damage policy (three known-prefix misplacements, only one typed). Inert today because no player consumes the crate; a future consumer will assume the default-stream selection works.
What correct looks like: consume `lang_idx` in `parse_vobsub` (mark the default stream on `VobSubDoc`) or document it as caller-owned; add `BadLangIdx`/`AltBeforeStream` damage variants or a doc line saying which misplacements are silent.
Confidence: HIGH — Verification: read + rg (all lang_idx references enumerated).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `IdxDamage` + Display | 20-46 | REAL | — | 4 variants; honest messages |
| `IdxStream`/`IdxEntry`/`VobSubIdx` | 48-87 | REAL | S4 | plain data; `lang_idx` inert (F-004); damage Vec uncapped but line-bounded (doc :85 honest) |
| `parse_idx` | 89-194 | REAL | S3/S4 | F-002 (index unvalidated), F-003 (CR-only), F-004 (silent alt/delay/langidx misplacements); palette zip-alignment correct; >6-hex-digit palette entries silently masked to RRGGBB (minor); `trim_end_matches` mostly redundant with `lines()` |
| `parse_timestamp` | 196-207 | REAL | S2 | F-001 overflow; happy path correct; filepos hex overflow-safe (u64 parse → None) |
| `parse_delay` | 210-222 | REAL | S2 | F-001 overflow; sign handling correct; accumulation semantics in caller verified |
| tests ×3 | 224-304 | REAL | — | strong pins; `assert!(!idx.has_palette \|\| idx.palette[0] == 0)` (:272) is a tautology (palette[0]=0 by construction in SAMPLE); no overflow/index-range/multi-delay tests |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| full_idx_parse | YES | pins size, palette[1]/[15], 2 streams, lang/title, entry count, ms values, hex filepos, empty damage |
| tolerances_never_panic | PARTLY | pins ShortPalette{got:6}, TimestampBeforeStream, binary garbage, BadSize+BadTimestamp order — real; but proves "never panic" only for the 4 shapes tried (F-001 inputs not among them) |
| delay_shifts_entries | YES | +500 ms and −1000 ms shifts on both entries, exact values |
| (absent) overflow literals | — | would panic in debug today (F-001) |
| (absent) index range | — | F-002 undetected |
| (absent) multiple delay lines | — | the `+=` accumulation choice is unpinned |

## 6. Doc claims touching this file
| Claim | Source (doc §/matrix row) | Verdict | Proof |
|---|---|---|---|
| "idx/spu parse, 128 KiB cap, grayscale fallback, damage ledger" | feature-matrix.toml `subtitle-vobsub` evidence | REAL | idx parse real (this file); 128 KiB cap = mod.rs SPU_MAX_SIZE (:167, not my file); grayscale fallback test :5513 ok; damage ledger real |
| "230/0 gates" | same row | REAL (stale count) | baseline: vobsub 15/15 unit (:5501-5515) inside crate-wide 267/0 lib (:5269) + integration; 230 was the 2026-10-26 gate count |
| status implemented x3 | same row | UNREACHABLE (per-platform claim unprovable in shipped builds) | no Cargo.toml edge from apps/cli or apps/desktop-tauri to subtitles |
| ".idx text parse: palette, size, per-stream timestamp/filepos entries, delays, tolerances — typed IdxDamage, lenient" | docs/formats/sub-vobsub.md:24-25 | REAL | all present; but "tolerances" does not extend to overflow (F-001) or index range (F-002) |
| "synthetic-but-valid idx/sub pairs through fuzz_sub_vobsub — zero panics" | sub-vobsub.md:52-53 | SHALLOW | true only for the release-profile survival runs (:5590 2/2 ok); the cargo-fuzz target named `fuzz_sub_vobsub` does not exist (35 targets, none vobsub) and fuzz/Cargo.toml:59-60 forces overflow-checks=true — the run that would refute "zero panics" has never been built |
| "Never panics" | idx.rs:18 + vobsub/fuzz.rs:37-38 | FALSE | F-001: overflow-checked builds panic on a parseable timestamp line |

## 7. Dead / duplicated / unreachable code
`lang_idx` = parsed-but-unconsumed (F-004). No duplication. Crate unreachable from shipped binaries.

## 8. Dependencies & flags
std-only; no unsafe; no features. Uses let-chains (`if let … && …`, :111, :164-165) — edition 2024, compiles per baseline build #1.

## 9. Open questions for the next agent
- vobsub/mod.rs owner: rate the `0x20u8 + stream.index` overflow at :159 (debug panic for index ≥ 224) — root cause is idx.rs F-002 but the panic site is yours; also confirm whether `lang_idx` should select the default stream.
- vobsub/fuzz.rs owner: your doc claims doc 04 §8.5 registers `fuzz_sub_vobsub` — `fuzz/fuzz_targets/` has no such file and `scripts/fuzz_registry_check.sh` passes with diff empty (baseline #18), so the registry does NOT list it. Resolve the contradiction; wiring the target will expose F-001 immediately.
- Whoever owns doc 04 §8.5: confirm the intended fuzz-target registry contents for subtitles (ass/srt/vtt targets exist; cc/dvb/pgs/vobsub do not).
