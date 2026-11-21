# AUDIT: apps/desktop/src/files.rs
| Field | Value |
|---|---|
| Auditor | W13-A02 |
| Date | 2026-11-21 |
| Lines / functions | 236 / 12 (7 production items + 5 tests) |
| File verdict | REAL (correct logic, honest seams; the media pre-filter list is materially incomplete and diverges from the UI funnel) |
| Reachable from shipped entry point? | YES for the classifier + drag-drop: lumen-tauri app.rs:314 `let mut handler = PathDragDrop;` → `runtime.on_files_dropped(&mut handler, paths)` (app.rs:318); argv lane app.rs:62 → `ShellRuntime::open_path` (shell.rs:300) → `open_path_request` → this file's classifiers. `NativeDialog` (118-124): NO — `pick_and_open` (shell.rs:329) has no production caller; app.rs:78-82 states the native dialog is NOT wired ("capabilities stay `core:default` only, no plugins"). CAVEAT: lumen-tauri binary UNVERIFIED-BUILD in baseline #2; desktop crate compiled (baseline #1); all 5 in-file tests executed ok — orchrun-test-workspace-nofailfast.log.txt:2271-2275. |
| Findings | S0:0 S1:0 S2:1 S3:1 S4:1 |
| Commands run | Read files.rs 1-236 (full); Read ui/src/client/open-path.ts 1-75 (the claimed mirror); rg `PathDragDrop|NativeDialog|DragDrop|pick_media_file|dialog` apps/desktop-tauri/src (app.rs:92/306-318 wiring; 78-82 dialog NOT wired); rg `NativeDialog|NativeFileDialog|pick_and_open|open_path\b` apps/desktop/src/shell.rs (seam + tests, no production dialog caller); Read shell.rs 300-330 (argv/drop/pick lanes); Read app.rs 55-104 (seam docs); containers/mod.rs arms re-checked (15 registered probe ids); baseline container_conformance results (log:2935+: ivf_av1/ivf_vp9/es_mp2/es_mp3 lanes ok); grep baseline logs 2271-2275 (5 files tests ok). |

## 1. One-paragraph verdict
Clean, honest seam module: two pure suffix classifiers, two traits, one real drag-drop impl that funnels every drop into the catalog's own command (the ARCH-5 rule held — no engine calls here), one typed-pending dialog impl, and five tests that all ran in the baseline. Two defects matter. First, the media pre-filter's 19-extension list is materially narrower than the open path it claims to defer to: at least thirteen extensions of engine-openable media (`.ivf`, `.mp2`, `.h264`, `.265`, `.mka`, `.ogv`, `.oga`, `.vob`, `.m2ts`, `.mts`, `.adts`, `.aif`, `.mks`) are silently ignored by BOTH the drop lane and the `lumen-tauri file.mkv` argv lane, while the SAME files opened through the webview UI's funnel ("anything else is media", open-path.ts:17) open and play — a dropped VP9 `.ivf` does nothing, a picked one plays. Second, the module doc says the tauri build "swaps the test impls for the plugin-backed ones at the composition root", but no plugin-backed dialog impl exists anywhere in the tree and the tauri app does not wire `pick_and_open` at all — the dialog half is a typed-pending stub with no consumer.

## 2. What actually works (verified)
- All 5 tests executed and passed in baseline: `files::tests::{drops_become_the_open_command_first_media_wins, drops_without_media_are_noops_not_errors, native_dialog_is_the_typed_pending_state, path_classification_is_exact, playlist_drops_load_the_list_not_a_session}` — log:2271-2275.
- The playlist classifier mirrors the UI exactly as claimed: same 5 suffixes (`m3u m3u8 pls xspf cue` == open-path.ts:19-25), same dotfile semantics (`.m3u` has no extension — files.rs:218 vs open-path.ts:31-34), case-insensitive on both sides.
- Case-insensitive media matching verified by test (`Song.MP3` → openable, files.rs:167-170).
- Every path through `PathDragDrop` funnels into `open_path_request` → a catalog command; no shell-side engine call exists in this file (rg-verified: the only non-stdlib calls are `open_path_request` and `HostError`).
- `NativeDialog`'s refusal is typed and tested (`HostError::WebviewPending { adr: "ADR-004" }`, test at 227-235).

## 3. Findings

### F-001 [S2] — Media pre-filter silently refuses engine-openable media in both drop and argv lanes; the UI funnel opens the same files
Where: apps/desktop/src/files.rs:34-46 (list), consumed via crate::dispatch::open_path_request (dispatch.rs:253-261) from shell.rs:300-304 (argv) and shell.rs:308-318 (drop)
Code:
```
/// Media-file suffixes the drop handler accepts (the open path owns
/// format truth; this is the shell's pre-filter so a dropped README
/// never becomes a session.open error the UI must handle).
#[must_use = "pure reporter"]
pub fn is_media_path(path: &Path) -> bool {
    const MEDIA_SUFFIXES: &[&str] = &[
        "avi", "mkv", "mp4", "mov", "webm", "m4v", "mpg", "mpeg", "ts", "flv", "wmv", "wav", "mp3",
        "flac", "ogg", "opus", "aac", "m4a", "aiff",
    ];
```
What is wrong: The list is a closed 19-entry set, but the open path it defers to is CONTENT-probed with 15 registered format ids (containers/mod.rs:55-77: adts aiff avi flac h264 hevc ivf matroska mp2 mp3 mp4 ogg mpegps mpegts wav). Extensions that probe-and-open — and several decode end-to-end (baseline container_conformance: `ivf_vp9_family_lane ... ok`, `ivf_av1_family_lane ... ok`, `es_mp2_family_lane ... ok`, log:2935+) — are absent from the list: `.ivf`, `.mp2`, `.h264`, `.265`, `.adts`, `.mka`, `.mks`, `.ogv`, `.oga`, `.vob`, `.m2ts`, `.mts`, `.aif`/`.aifc`. For those, `open_path_request` returns `None` and both acquisition lanes silently no-op (`open_path`'s `?` at shell.rs:302; the drop path's `Ok(None)`). Meanwhile the UI-side funnel for the SAME product classifies "anything else is media" (ui/src/client/open-path.ts:17, 44-49) and dispatches `session.open` — so a VP9 `.ivf` picked through the webview file input opens and plays, while the identical file dropped on the window or passed as `lumen-tauri file.ivf` does nothing, with no error, no event, no log.
Why it matters: The two acquisition sides of one product disagree about what is openable, and the Rust side's answer is silent loss of user action on files the engine demonstrably plays. The design rationale in the doc ("a dropped README never becomes a session.open error the UI must handle") is already contradicted by the UI half, which deliberately lets non-media become typed errors — the codebase has both philosophies at once. The extension list is also a maintenance trap: every newly wired probe family (ps, ts, es, mkv all landed after this list's shape) must remember to edit this list or drops silently regress.
What correct looks like: Invert the rule to match the UI: playlist-suffix → `playlist.load`, EVERYTHING ELSE → `session.open` (the typed refusal the engine already answers with for actual non-media — exit-65-style error event, which the UI already renders). If a pre-filter must stay, it must be generated from the probe registry's known extensions, not hand-maintained.
Confidence: HIGH — Verification: read (list vs registry arms cross-checked) + baseline container_conformance lanes executed ok (log:2935+) + UI rule read (open-path.ts:44-49).

### F-002 [S3] — Module doc claims a tauri-build "swap" of these impls that does not exist; the dialog impl is wired nowhere
Where: apps/desktop/src/files.rs:20-24 vs apps/desktop-tauri/src/app.rs:78-82
Code:
```
//! Neither can link in this sandbox (no webkit2gtk-4.1, no display —
//! `pkg-config --exists webkit2gtk-4.1` fails; the 2.C3/ADR-004
//! posture), so the real impls are the typed
//! [`HostError::WebviewPending`] state and the **test impls** carry
//! the logic proof. The tauri build swaps the test impls for the
//! plugin-backed ones at the composition root — the seam, not the
//! rule, is what moves.
```
What is wrong: The drag-drop half is true (`PathDragDrop` — real path logic — is constructed at app.rs:314). The dialog half is false: no plugin-backed `NativeFileDialog` impl exists anywhere in the tree (rg over apps/desktop-tauri: only `PathDragDrop` is imported; app.rs:78-82 says `pick_and_open` "is NOT wired here … capabilities stay `core:default` only, no plugins"), and `ShellRuntime::pick_and_open` (the only caller of the trait) has no production caller. `NativeDialog` is a typed-pending stub whose only consumer is its own test. The composition-root swap the doc asserts is future work stated in the present tense.
Why it matters: A reader auditing the shipped surface concludes the desktop has a native file dialog behind a seam; it has none, and nothing links the stub. Internal contradiction with app.rs's honest "NOT wired" note.
What correct looks like: State the truth: "the dialog seam ships unimplemented (`NativeDialog` answers `WebviewPending`); the plugin-backed impl and its composition-root wiring are the recorded follow-up row (app.rs:78-82)."
Confidence: HIGH — Verification: read (rg-verified no dialog impl/wiring anywhere; app.rs:78-82 read).

### F-003 [S4] — The claimed cross-language mirror is enforced by nothing
Where: apps/desktop/src/files.rs:48-51
Code:
```
/// Playlist-file suffixes (the engine-side playlists crate's reader
/// surface: M3U/M3U8, PLS, XSPF, CUE). A playlist file does not open
/// a session — it loads the list (`playlist.load`; 4.D1's handoff,
/// parse engine-side). Mirrors ui/src/client/open-path.ts exactly.
```
What is wrong: The mirror is exact today (verified: same 5 suffixes, same dotfile rule), but nothing pins it: the Rust tests (files.rs:213-222) and the UI tests (open-path.test.ts) each test their own side; no test, codegen, or CI row compares the two lists. A one-sided edit (say the UI adds `asx`) silently forks the classification rules the doc calls "one rule, two acquisition sides" (open-path.ts:9-11).
Why it matters: Cross-language constant mirrors without a pin are where the F-001-class drift already happened once (the media half drifted); the playlist half is one uncoordinated edit from the same fate.
What correct looks like: A test on one side embedding the other side's literal list (or a shared codegen source), failing on divergence — the repo already does exactly this for the IPC channel name (app.rs:99 comments the two-sided literal contract).
Confidence: HIGH — Verification: read (both lists + both test files checked; no cross-check found).

## 4. Per-function table
| Function | Lines | Verdict | Severity | Notes |
|---|---|---|---|---|
| `is_media_path` | 38-46 | REAL (incomplete) | S2 (F-001) | Case-insensitive; closed 19-entry list vs 15 probe ids — ≥13 openable extensions missing. |
| `is_playlist_path` | 53-58 | REAL | S4 (F-003) | Exact UI mirror today; unenforced. |
| `is_openable_path` | 63-65 | REAL | — | Union; used by drop first-openable scan. |
| `DragDropHandler` (trait) | 68-82 | REAL | — | One method; honest `Ok(None)` no-op contract documented. |
| `NativeFileDialog` (trait) | 85-92 | REAL | — | Cancel=None; pending=typed error. |
| `PathDragDrop` | 100-113 | REAL | — | First-openable-wins (documented v1 limitation; mixed-drop order decides media-vs-playlist — honest). Reachable: app.rs:314. |
| `NativeDialog` | 118-124 | REAL / UNREACHABLE | S3 (F-002) | Typed pending refusal; zero production callers (only files.rs test + shell.rs's unwired `pick_and_open` seam). |
| `drops_become_the_open_command_first_media_wins` | 134-157 | REAL | — | Ran ok (log:2271). Skips non-media, opens first media. |
| `drops_without_media_are_noops_not_errors` | 160-171 | REAL | — | Ran ok (log:2272). README no-op; `Song.MP3` case pin. |
| `playlist_drops_load_the_list_not_a_session` | 177-208 | REAL | — | Ran ok (log:2275). All 5 playlist formats; append:false pinned. |
| `path_classification_is_exact` | 213-222 | REAL | — | Ran ok (log:2274). Negative space: `.m3u.bak`, no-ext, dotfile `.m3u`. |
| `native_dialog_is_the_typed_pending_state` | 227-235 | REAL | — | Ran ok (log:2273). The stub's honesty is pinned; its deadness is not (F-002). |

## 5. Test honesty
| Test | Would it fail if the code were broken? | Reason |
|---|---|---|
| drops_become_the_open_command_first_media_wins | YES | Exact command variant + path + request_id on the positive lane. |
| drops_without_media_are_noops_not_errors | YES | Both no-op and case-insensitivity pinned. |
| playlist_drops_load_the_list_not_a_session | YES | Five formats × command shape × append flag. |
| path_classification_is_exact | YES | Boundary cases (double suffix, dotfile, no-ext). |
| native_dialog_is_the_typed_pending_state | YES | For the stub's behavior; cannot catch the stub never being wired (that's F-002, and no test could — it's a wiring gap). |
| (all) vs F-001 | NO | Every media-path test uses extensions INSIDE the list; the missing-extension behavior is untested by design (nothing asserts `.ivf` is openable — the tests would have exposed the gap). |

## 6. Doc claims touching this file
| Claim | Source | Verdict | Proof |
|---|---|---|---|
| "the open path owns format truth; this is the shell's pre-filter" | files.rs:34-35 | FALSE (in effect) | The pre-filter excludes ≥13 extensions the open path opens (F-001); the list overrules the truth it defers to. |
| "Mirrors ui/src/client/open-path.ts exactly" (playlist suffixes) | files.rs:51 | REAL (today) / unenforced | Both lists read and compared: identical. No pin exists (F-003). |
| "The tauri build swaps the test impls for the plugin-backed ones at the composition root" | files.rs:22-24 | FALSE (dialog half) / REAL (drop half) | app.rs:314 wires PathDragDrop; no dialog impl or wiring exists (app.rs:78-82) — F-002. |
| "a drop with no openable file is a no-op answer, not an error — Ok(None)" | files.rs:75-76 | REAL | Test 160-171 ran ok; shell.rs:316-318 forwards None. |

## 7. Dead / duplicated / unreachable code
- `NativeDialog`: real code, unreachable from any shipped entry point (F-002) — the honest pending stub exists for a wiring that never landed.
- No duplication in Rust; the duplication WITH the TS twin is the unenforced mirror (F-003).

## 8. Dependencies & flags
- `crate::dispatch::open_path_request`, `crate::error::HostError`, `commands` (WireRequest/RequestId), std paths. No feature gates, no unsafe, no webview deps (the whole point — compiles webkit-free, verified: no tauri import).

## 9. Open questions for the next agent
- shell.rs owner (W13-A01): `non_openable_paths_are_noops` (shell.rs:769-774) pins the silent no-op for `README.md` AND `queue.unknown-ext` — if F-001 is fixed by inverting the rule, that test's second case changes meaning (unknown-ext becomes a typed session.open refusal) — coordinate.
- Whoever owns the feature matrix: is "drag-and-drop opens media" a row with evidence? If so, F-001 contradicts it for the missing extensions — matrix-row audit should cross-check this list.
- ui/src/client/open-path.test.ts owner: the UI side has no `.ivf`-is-media test either (its rule makes it trivially media); the cross-side divergence (F-001) is invisible to both test suites by construction.
