#!/usr/bin/env bash
# gate-proofs — the F0.5 acceptance harness.
#
# FIX-PLAN §2 F0.5 acceptance: "Each gate, fed a deliberately broken tree,
# goes red (prove it once in the PR)." This runner proves it, once, in one
# command — for every F0.5 gate whose logic runs WITHOUT cargo:
#
#   scripts/quality/23_alloc_budget.sh   (the OVER THRESHOLD exit path)
#   scripts/quality/26_complexity.sh    (the ratchet, seeded baselines)
#   scripts/oracle_check.sh             (sidecar / mismatch / perimeter reds)
#   scripts/quality/25_fuzz_campaign.sh (5 fail-closed red branches, F2.3)
#   .github/workflows/rust-clippy.yml   (structural: no continue-on-error,
#                                        -D warnings + pipefail in place)
#   .github/workflows/audit.yml          (structural: the miri/asan/ubsan
#                                        lanes carry no `|| true`)
#   package-lock.json / .github/dependabot.yml  (the F0.5f text gates)
#
# HOW EACH PROOF RUNS: (1) a throwaway copy of the repo is made inside a
# mktemp workdir (trimmed of .git, the root audit/ evidence tree, design/
# assets, target/ and node_modules/ — no gate under proof reads any of
# them), (2) exactly ONE break is seeded, (3) the REAL gate script runs
# inside that copy, (4) the exit code is asserted (non-zero for a red
# proof, zero for a green control), (5) the copy is deleted. THE REAL TREE
# IS NEVER MUTATED: a sentinel guard (G0) hashes the files a mis-wired proof
# could clobber and fails the run if any of them changed.
#
# NO CARGO RUNS HERE. Gates that need an instrument (cargo for mem-audit,
# cargo-fuzz for the campaign) run behind TEST SHIMS on the copy's PATH —
# the GATE LOGIC (sampling, parsing, judging, verdict, exit path) executes
# for real while the instrument's behavior is scripted per proof. The
# behavioral proofs that genuinely need the real instruments (clippy
# -D warnings, miri, asan, ubsan) are PARKED in PROOFS.md with their exact
# central commands — [VERIFY-central].
#
# THE RUNNER PROVES ITSELF: `--self-test-green` rigs a green control on a
# command that exits 7, `--self-test-red` rigs a red assertion on a command
# that exits 0 — each must make this script exit 1, and the full run spawns
# both modes and asserts those non-zero exits. These are the failure classes
# of an earlier runner of this harness (assertions silently swallowed), and
# beyond them the ledger is size-checked at the single exit path: the run
# fails unless EXACTLY the designed number of red/green/self assertions were
# executed and recorded — a lost or extra assertion is itself a failure.
# FOUR further latent defect classes were found in THIS runner's own first
# draft (each caught by its own assertions/guards before this file ever
# produced a green run — the broken draft run exited 1 with "8 green
# control(s) went red", never a silent green):
#   1. a single-statement `local a="$1" b="$path-$a"` aborts under set -u
#      before assigning (all words of a local expand before the builtin
#      assigns) — mk_copy returned an empty path and 11 red rows
#      "asserted" at rc=127 with no gate ever run. Closed by the split
#      locals in mk_copy AND at the verdict level: red() rejects rc
#      126/127 as "gate not runnable" (a harness failure is never a proof).
#   2. die() inside mk_copy runs inside command substitution, so its abort
#      only exits the subshell — the run continued on an empty copy path.
#      Closed by `|| die` at every mk_copy call site (see mk_copy).
#   3. tar's --exclude=audit was UNANCHORED (GNU tar matches any trailing
#      component sequence), stripping docs/audit/ (the committed
#      complexity baseline + fuzz evidence) out of every copy. Closed by
#      ./-anchored exclusions plus mk_copy's completeness checks.
#   4. the yaml checkers grepped the whole file, comments included — the
#      real workflow's own fix-notes ("continue-on-error: true removed")
#      false-failed the G7 control. Closed by yaml_body() stripping
#      comment lines before any structural check.
#
# OUTPUT: PROOFS.md (next to this script) — GENERATED, never hand-edit;
# the runner is its only writer. Regenerate:
#     bash scripts/quality/gate-proofs/run_proofs.sh
#
# Usage:
#   run_proofs.sh                        # the full ledger -> PROOFS.md
#   run_proofs.sh --self-test-green      # rigged green control (must exit 1)
#   run_proofs.sh --self-test-red        # rigged red assertion (must exit 1)
#   run_proofs.sh --check-clippy-yaml <file>    # structural checker (exit 1
#   run_proofs.sh --check-ub-yaml <file>       #   on the broken shape, 0 on
#   run_proofs.sh --check-dependabot <file>    #   the fixed one) — internal
#
# Exit: 0 = every red assertion went red and every green control went green;
#       1 = any assertion failed, was lost, or a precondition was unmet.
set -uo pipefail

RUNNER="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)/$(basename -- "${BASH_SOURCE[0]}")"
HERE="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd -- "$HERE/../../.." && pwd)"
FIXTURES="$HERE/fixtures"
PROOFS="$HERE/PROOFS.md"

EXPECT_REDS=13
EXPECT_GREENS=13
EXPECT_SELF=2

REDS_TOTAL=0;   REDS_FAILED=0
GREENS_TOTAL=0; GREENS_FAILED=0
SELF_TOTAL=0;   SELF_FAILED=0
MD_ON=0
RED_ROWS=()
GREEN_ROWS=()
SELF_ROWS=()

say() { printf '%s\n' "$*"; }

md() { # md <line> — append one line to PROOFS.md (full mode only)
  [ "$MD_ON" -eq 1 ] || return 0
  printf '%s\n' "$1" >> "$PROOFS" || { say "run_proofs: cannot write $PROOFS"; exit 1; }
}

die() { # die <reason> — abort the run, loudly
  if [ "$MD_ON" -eq 1 ]; then
    md ""
    md "**RUN ABORTED (precondition): $1**"
  fi
  say "run_proofs: $1" >&2
  exit 1
}

# --- assertion primitives: the ONLY way a proof verdict is recorded ---------
red() { # red <id> <gate> <seed> <cmd...> — run the gate, assert rc != 0
  local id="$1" gate="$2" seed="$3"; shift 3
  ( "$@" ) >"$WORK/logs/$id.log" 2>&1
  local rc=$?
  REDS_TOTAL=$((REDS_TOTAL + 1))
  if [ "$rc" -eq 126 ] || [ "$rc" -eq 127 ]; then
    # A red proof must be the GATE judging the broken tree. rc 126/127 means
    # the harness itself could not invoke the gate (not executable / not
    # found) — a broken copy path, a missing script, a mis-named mode. The
    # inherited mk_copy defect produced exactly this: 11 "RED — asserted"
    # rows at rc=127 whose gate had never run at all. Such an rc is a
    # harness failure, never a proof.
    REDS_FAILED=$((REDS_FAILED + 1))
    RED_ROWS+=("| $id | $gate | $seed | $rc | **FAIL — gate not runnable (126/127): the harness could not invoke the gate, so nothing was proven** |")
  elif [ "$rc" -ne 0 ]; then
    RED_ROWS+=("| $id | $gate | $seed | $rc | RED — asserted |")
  else
    REDS_FAILED=$((REDS_FAILED + 1))
    RED_ROWS+=("| $id | $gate | $seed | $rc | **FAIL — the gate stayed GREEN** |")
  fi
}

green() { # green <id> <gate> <tree> <cmd...> — run the gate, assert rc == 0
  local id="$1" gate="$2" tree="$3"; shift 3
  ( "$@" ) >"$WORK/logs/$id.log" 2>&1
  local rc=$?
  GREENS_TOTAL=$((GREENS_TOTAL + 1))
  if [ "$rc" -eq 0 ]; then
    GREEN_ROWS+=("| $id | $gate | $tree | $rc | GREEN — asserted |")
  else
    GREENS_FAILED=$((GREENS_FAILED + 1))
    GREEN_ROWS+=("| $id | $gate | $tree | $rc | **FAIL — the control went RED** |")
  fi
}

# --- structural checkers (also runnable as modes; used via red()/green()) --
# All three read the file's BODY, not its comments: the F0.5 fixes left
# explanatory comments that NAME the removed directives ("continue-on-error:
# true removed", "the `|| true` swallow removed") — a whole-file grep
# false-fails on its own fix notes (G7 failed exactly this way on the real
# workflow). Comment stripping is line-granular (a leading-# line); inline
# trailing comments stay, which is safe for these three files.
yaml_body() { # yaml_body <file> — echo the file's non-comment lines
  grep -v '^[[:space:]]*#' "$1" || true
}

check_clippy_yaml() { # the F0.5b invariants of .github/workflows/rust-clippy.yml
  local f="$1" body
  body="$(yaml_body "$f")"
  if printf '%s\n' "$body" | grep -q 'continue-on-error:'; then
    say "clippy-yaml check: FAIL — continue-on-error present (a lint failure cannot fail the lane)"
    return 1
  fi
  if ! printf '%s\n' "$body" | grep -q -- '-- -D warnings'; then
    say "clippy-yaml check: FAIL — no '-D warnings' (warnings do not fail clippy)"
    return 1
  fi
  if ! printf '%s\n' "$body" | grep -q 'set -o pipefail'; then
    say "clippy-yaml check: FAIL — no pipefail (the trailing sarif-fmt masks clippy's exit)"
    return 1
  fi
  say "clippy-yaml check: OK — no continue-on-error, -D warnings and pipefail in place"
  return 0
}

check_ub_yaml() { # the F0.5e invariants of the miri/asan/ubsan lanes in audit.yml
  local f="$1" line body
  body="$(yaml_body "$f")"
  if ! printf '%s\n' "$body" | grep -Eq '18_ub_checks\.sh (miri|asan|ubsan)'; then
    say "ub-yaml check: FAIL — no miri/asan/ubsan lane found in $f"
    return 1
  fi
  while IFS= read -r line; do
    case "$line" in
      *'18_ub_checks.sh miri'*|*'18_ub_checks.sh asan'*|*'18_ub_checks.sh ubsan'*)
        case "$line" in
          *'|| true'*)
            say "ub-yaml check: FAIL — a miri/asan/ubsan lane still swallows its exit: $line"
            return 1
            ;;
        esac
        ;;
    esac
  done < <(printf '%s\n' "$body" | grep -E '18_ub_checks\.sh (miri|asan|ubsan)')
  say "ub-yaml check: OK — every miri/asan/ubsan lane propagates its instrument's exit"
  return 0
}

check_dependabot() { # the F0.5f invariant: no empty package-ecosystem stub
  local f="$1" found=0 val
  while IFS= read -r line; do
    found=$((found + 1))
    val="${line#*:}"
    val="${val//\"/}"
    val="$(printf '%s' "$val" | tr -d '[:space:]')"
    if [ -z "$val" ]; then
      say "dependabot check: FAIL — empty package-ecosystem (dependabot rejects it: no update PRs possible): $line"
      return 1
    fi
  done < <(yaml_body "$f" | grep -E 'package-ecosystem:' || true)
  if [ "$found" -eq 0 ]; then
    say "dependabot check: FAIL — no package-ecosystem entries in $f"
    return 1
  fi
  say "dependabot check: OK — $found non-empty ecosystem entries"
  return 0
}

# --- modes -------------------------------------------------------------------
case "${1:-}" in
  --check-clippy-yaml)  check_clippy_yaml "${2:?file}"; exit $? ;;
  --check-ub-yaml)      check_ub_yaml "${2:?file}"; exit $? ;;
  --check-dependabot)   check_dependabot "${2:?file}"; exit $? ;;
esac

# mktemp workdir — the template is absolute and carries 8 X's; a failed
# mktemp is a precondition failure, never a silent skip.
WORK="$(mktemp -d "${TMPDIR:-/tmp}/fx8a09-gate-proofs.XXXXXXXX")" \
  || { say "run_proofs: mktemp failed" >&2; exit 1; }
cleanup() { rm -rf -- "$WORK"; }
trap cleanup EXIT INT TERM
mkdir -p "$WORK/logs" "$WORK/bin"

if [ "${1:-}" = "--self-test-green" ] || [ "${1:-}" = "--self-test-red" ]; then
  # Rigged controls: the assertion machinery must catch both violations.
  # These modes write no PROOFS.md; the FULL run spawns them below and
  # asserts each exits 1.
  if [ "${1:-}" = "--self-test-green" ]; then
    green SELF "rigged green control (a gate that exits 7 must be caught)" "self-test" bash -c 'exit 7'
    [ "$GREENS_FAILED" -gt 0 ] || { say "self-test-green: FAILED to catch the rigged control" >&2; exit 1; }
    say "self-test-green: rigged control caught (green control on exit 7 -> runner fails)"
    exit 1
  else
    red SELF "rigged red assertion (a gate that exits 0 must be caught)" "self-test" bash -c 'exit 0'
    [ "$REDS_FAILED" -gt 0 ] || { say "self-test-red: FAILED to catch the rigged assertion" >&2; exit 1; }
    say "self-test-red: rigged assertion caught (red assertion on exit 0 -> runner fails)"
    exit 1
  fi
fi

[ "${1:-}" = "" ] || die "unknown mode: $1 (usage: run_proofs.sh | --self-test-green | --self-test-red)"

# --- preconditions -----------------------------------------------------------
for f in "$ROOT/scripts/quality/25_fuzz_campaign.sh" \
         "$ROOT/scripts/quality/23_alloc_budget.sh" \
         "$ROOT/scripts/quality/26_complexity.sh" \
         "$ROOT/scripts/oracle_check.sh" \
         "$ROOT/.github/workflows/rust-clippy.yml" \
         "$ROOT/.github/workflows/audit.yml" \
         "$ROOT/.github/dependabot.yml" \
         "$ROOT/package-lock.json" \
         "$ROOT/docs/audit/complexity-baseline.json" \
         "$FIXTURES/alloc_memaudit_over.raw.txt" \
         "$FIXTURES/alloc_memaudit_ok.raw.txt" \
         "$FIXTURES/complexity_lower_baseline.json" \
         "$FIXTURES/fuzz_stray_target.rs" \
         "$FIXTURES/oracle_bad_pin.oracle.sha256" \
         "$FIXTURES/package_lock_broken.json" \
         "$FIXTURES/clippy_continue_on_error.yml"; do
  [ -f "$f" ] || die "precondition failed: $f is missing"
done
if command -v cargo-fuzz >/dev/null 2>&1; then
  die "precondition failed: cargo-fuzz is on this host's PATH ($(command -v cargo-fuzz)) — the P1 absent-campaign proof needs a host where command -v cargo-fuzz fails"
fi
for t in ffmpeg sha256sum awk python3 tar; do
  command -v "$t" >/dev/null 2>&1 || die "precondition failed: $t not on PATH"
done

# --- test shims: cargo / cargo-fuzz ------------------------------------------
# Written into the mktemp bin dir ONLY. PATH_SHIM puts them first, so the
# gates' `need cargo` / `need cargo-fuzz` guards pass and the instrument
# invocations (`cargo run -q -p lumen-mem-audit -- ...`,
# `cargo fuzz run <t> -- ...`) are emulated — the gate logic after them is
# real. PATH_BARE is the host PATH unchanged: no shim, and (per the
# precondition above) no cargo-fuzz.
cat > "$WORK/bin/cargo" <<'SHIM'
#!/usr/bin/env bash
# TEST SHIM (gate-proofs, FX8-A09) — written by run_proofs.sh into its
# mktemp bin dir; never installed in the repo. Emulates exactly the two
# cargo invocations the gates under proof make:
#   cargo fuzz run <t> -- <libfuzzer flags>    -> exits $FX8A09_FUZZ_RC (default 0)
#   cargo run -q -p lumen-mem-audit -- <files> -> cats $FX8A09_MEMAUDIT_RAW
# Anything else exits 64 loudly: an unexpected use means the proof is broken.
set -uo pipefail
if [ "${1:-}" = "fuzz" ] && [ "${2:-}" = "run" ]; then
  echo "shim: emulated fuzz run target=$3 rc=${FX8A09_FUZZ_RC:-0}"
  exit "${FX8A09_FUZZ_RC:-0}"
fi
if [ "${1:-}" = "run" ] && [ "${2:-}" = "-q" ]; then
  if [ -n "${FX8A09_MEMAUDIT_RAW:-}" ] && [ -f "$FX8A09_MEMAUDIT_RAW" ]; then
    cat "$FX8A09_MEMAUDIT_RAW"
    exit 0
  fi
  echo "shim: FX8A09_MEMAUDIT_RAW not set" >&2
  exit 65
fi
echo "shim: unexpected cargo invocation: $*" >&2
exit 64
SHIM
cat > "$WORK/bin/cargo-fuzz" <<'SHIM'
#!/usr/bin/env bash
# TEST SHIM (gate-proofs, FX8-A09) — satisfies the campaign gate's
# `need cargo-fuzz` guard. The campaign itself invokes `cargo fuzz run ...`,
# which the cargo shim above emulates; this file only needs to exist on PATH.
exit 0
SHIM
chmod +x "$WORK/bin/cargo" "$WORK/bin/cargo-fuzz"
PATH_SHIM="$WORK/bin:$PATH"
PATH_BARE="$PATH"

# --- real-tree guard sentinels (G0): hashed now, re-hashed at the end -------
SENTINELS=(scripts/quality/25_fuzz_campaign.sh scripts/quality/23_alloc_budget.sh \
           scripts/quality/26_complexity.sh scripts/oracle_check.sh \
           docs/audit/complexity-baseline.json docs/audit/fuzz-campaign/summary.txt \
           package-lock.json .github/dependabot.yml \
           .github/workflows/rust-clippy.yml .github/workflows/audit.yml fuzz/Cargo.toml)
: > "$WORK/sentinels.before"
for s in "${SENTINELS[@]}"; do
  [ -f "$ROOT/$s" ] || die "sentinel missing: $s"
  printf '%s  %s\n' "$(sha256sum "$ROOT/$s" | awk '{print $1}')" "$s" >> "$WORK/sentinels.before"
done

# --- the trimmed copy every proof runs in -------------------------------------
# mk_copy runs inside command substitution at every call site
# (`c="$(mk_copy pN)"`), so its die() exits only that subshell — the
# assignment swallows nothing (the substitution's rc becomes the
# assignment's rc), but the abort only happens if the caller chains
# `|| die`. Every call site does; removing one resurrects the
# swallowed-failure class this harness exists to close.
mk_copy() { # mk_copy <name> — a throwaway repo copy; echoes its path
  # NOTE: the two locals MUST stay on separate statements. A single
  # `local name="$1" dst="$WORK/tree-$name"` aborts under set -u — all
  # words of a `local` are expanded BEFORE the builtin assigns, so $name is
  # unbound at expansion time (probed: "name: unbound variable", and the
  # whole assignment silently never happens). That exact defect shipped in
  # this runner's first draft: every copy came back empty and every
  # tree-copy proof "passed" at rc=127 without its gate ever running.
  local name="$1"
  local dst="$WORK/tree-$name"
  mkdir -p "$dst" || die "cannot create copy $name"
  # The exclusions are ANCHORED to the copy root (./audit, not audit): GNU
  # tar matches an unanchored pattern against any trailing component
  # sequence, so --exclude=audit also stripped docs/audit/ (the committed
  # complexity baseline + fuzz-campaign evidence) out of every copy, and the
  # completeness check below caught it on the very first run. ./design is
  # the assets tree; target/ and node_modules/ are unanchored on purpose
  # (they may sit at any level and no gate reads them).
  ( cd "$ROOT" && tar cf - --exclude=.git --exclude=./audit --exclude=./design \
      --exclude=target --exclude=node_modules . ) | ( cd "$dst" && tar xf - )
  local rc=$?
  [ "$rc" -eq 0 ] || die "trimmed copy $name failed (tar rc=$rc)"
  [ -f "$dst/scripts/quality/25_fuzz_campaign.sh" ] || die "trimmed copy $name incomplete (no T25)"
  [ -f "$dst/fuzz/Cargo.toml" ] || die "trimmed copy $name incomplete (no fuzz/Cargo.toml)"
  [ -f "$dst/docs/audit/complexity-baseline.json" ] || die "trimmed copy $name incomplete (no baseline)"
  printf '%s\n' "$dst"
}

# Reduce a copy's oracle fixture dir to the pinned-only green baseline:
# keep the two sidecar'd fixtures (+ their sidecars + non-media files),
# delete every other media file. Every oracle proof then seeds exactly one
# break on top of a tree the gate can go green on.
oracle_pin_only() { # oracle_pin_only <copydir>
  local dir="$1/tests/fixtures/generated/valid" f base ext
  for f in "$dir"/*; do
    [ -e "$f" ] || continue
    base="${f##*/}"
    case "$base" in
      avi_mjpeg_pcm.avi|avi_mjpeg_pcm.avi.oracle.sha256|mp4_mjpeg_pcm.mp4|mp4_mjpeg_pcm.mp4.oracle.sha256)
        continue ;;
    esac
    ext="${base##*.}"; ext="${ext,,}"
    case " $ext " in
      ' '|' avi '|' mp4 '|' mkv '|' mov '|' webm '|' ts '|' m2ts '|' mts '|' flac '|' mp3 '|' wav '|' ogg '|' opus '|' aac '|' m4a '|' wma '|' ape '|' mpc '|' amr ')
        rm -f -- "$f" ;;
    esac
  done
}

T25="scripts/quality/25_fuzz_campaign.sh"

# --- P1/P2/P3/P4/P5 — the fuzz campaign gate (F2.3: 5 red branches) ---------
proof_fuzz() {
  local c
  # P1 — absent campaign: no cargo-fuzz anywhere on the proof PATH.
  c="$(mk_copy p1)" || die "trimmed copy p1 failed (mk_copy aborted)"
  red P1 "$T25" "absent campaign: cargo-fuzz not on PATH (need cargo-fuzz fails)" \
    env PATH="$PATH_BARE" bash "$c/$T25"
  green G12 "$T25 evidence" "P1 copy: summary records the absent-campaign RED verdict" \
    grep -q "cargo-fuzz absent" "$c/docs/audit/fuzz-campaign/summary.txt"
  rm -rf -- "$c"

  # P2 — empty target set: nothing under fuzz/fuzz_targets/.
  c="$(mk_copy p2)" || die "trimmed copy p2 failed (mk_copy aborted)"
  rm -f -- "$c"/fuzz/fuzz_targets/*.rs
  red P2 "$T25" "empty target set: every .rs removed from fuzz/fuzz_targets/" \
    env PATH="$PATH_SHIM" bash "$c/$T25" 5
  rm -rf -- "$c"

  # P3 — unregistered [[bin]]: a stray .rs with no Cargo.toml registration.
  c="$(mk_copy p3)" || die "trimmed copy p3 failed (mk_copy aborted)"
  cp "$FIXTURES/fuzz_stray_target.rs" "$c/fuzz/fuzz_targets/fuzz_stray_target.rs"
  red P3 "$T25" "stray fuzz/fuzz_targets/fuzz_stray_target.rs with no [[bin]] entry (W17-A06 F-003)" \
    env PATH="$PATH_SHIM" bash "$c/$T25" 5
  rm -rf -- "$c"

  # P4 — a run failure: rc!=0 with no crash artifacts is never 'clean' (F-001).
  # G3 asserts the same copy's evidence records the FAILED row.
  c="$(mk_copy p4)" || die "trimmed copy p4 failed (mk_copy aborted)"
  red P4 "$T25" "instrument shim: cargo fuzz run exits 3, no crash artifacts" \
    env PATH="$PATH_SHIM" FX8A09_FUZZ_RC=3 bash "$c/$T25" 5 fuzz_sub_srt
  green G3 "$T25 evidence" "P4 copy: summary records the FAILED (run failure) row" \
    grep -q "FAILED (run failure)" "$c/docs/audit/fuzz-campaign/summary.txt"
  rm -rf -- "$c"

  # P5 — unrecordable evidence: a plain file where the summary dir must go.
  c="$(mk_copy p5)" || die "trimmed copy p5 failed (mk_copy aborted)"
  rm -rf -- "$c/docs/audit/fuzz-campaign"
  touch "$c/docs/audit/fuzz-campaign"
  red P5 "$T25" "evidence path occupied by a plain file (mkdir -p fails)" \
    env PATH="$PATH_SHIM" bash "$c/$T25" 5 fuzz_sub_srt
  rm -rf -- "$c"

  # G1/G2 — the green control: every target runs clean, evidence says GREEN.
  c="$(mk_copy g1)" || die "trimmed copy g1 failed (mk_copy aborted)"
  green G1 "$T25" "unbroken copy; instrument shim: every target exits 0, no artifacts" \
    env PATH="$PATH_SHIM" FX8A09_FUZZ_RC=0 bash "$c/$T25" 5 fuzz_sub_srt
  green G2 "$T25 evidence" "G1 copy: summary records the GREEN verdict" \
    grep -q "verdict    : GREEN" "$c/docs/audit/fuzz-campaign/summary.txt"
  rm -rf -- "$c"
}

# --- P6/P7/P8 + G4 — the external-oracle gate --------------------------------
proof_oracle() {
  local c dir
  # P6 — a media fixture with no pin sidecar.
  c="$(mk_copy p6)" || die "trimmed copy p6 failed (mk_copy aborted)"
  oracle_pin_only "$c"
  dir="$c/tests/fixtures/generated/valid"
  cp "$dir/mp4_mjpeg_pcm.mp4" "$dir/stray_no_pin.mp4"
  red P6 "scripts/oracle_check.sh" "stray .mp4 fixture with no .oracle.sha256 sidecar" \
    env PATH="$PATH_BARE" bash "$c/scripts/oracle_check.sh"
  rm -rf -- "$c"

  # P7 — a doctored pin: the sidecar's video digest no longer matches.
  c="$(mk_copy p7)" || die "trimmed copy p7 failed (mk_copy aborted)"
  oracle_pin_only "$c"
  dir="$c/tests/fixtures/generated/valid"
  cp "$dir/mp4_mjpeg_pcm.mp4" "$dir/oracle_bad_pin.mp4"
  cp "$FIXTURES/oracle_bad_pin.oracle.sha256" "$dir/oracle_bad_pin.mp4.oracle.sha256"
  red P7 "scripts/oracle_check.sh" "doctored sidecar: zero video digest (hash mismatch)" \
    env PATH="$PATH_BARE" bash "$c/scripts/oracle_check.sh"
  rm -rf -- "$c"

  # P8 — media outside the pinned {.avi,.mp4} perimeter.
  c="$(mk_copy p8)" || die "trimmed copy p8 failed (mk_copy aborted)"
  oracle_pin_only "$c"
  dir="$c/tests/fixtures/generated/valid"
  cp "$dir/mp4_mjpeg_pcm.mp4" "$dir/stray_perimeter.mkv"
  red P8 "scripts/oracle_check.sh" ".mkv media present outside the pinned {.avi,.mp4} perimeter" \
    env PATH="$PATH_BARE" bash "$c/scripts/oracle_check.sh"
  rm -rf -- "$c"

  # G4 — the green control: the pinned-only tree, real ffmpeg decode.
  c="$(mk_copy g4)" || die "trimmed copy g4 failed (mk_copy aborted)"
  oracle_pin_only "$c"
  green G4 "scripts/oracle_check.sh" "pinned-only tree; real ffmpeg 7.1.5 decode matches both pins" \
    env PATH="$PATH_BARE" bash "$c/scripts/oracle_check.sh"
  rm -rf -- "$c"
}

# --- P9 + G5 — the alloc-budget exit path (F0.5a) ------------------------------
proof_alloc() {
  local c
  c="$(mk_copy p9)" || die "trimmed copy p9 failed (mk_copy aborted)"
  red P9 "scripts/quality/23_alloc_budget.sh" "instrument shim: 100 B / 20 MB mem-audit row = 98500x over the 64x floor" \
    env PATH="$PATH_SHIM" FX8A09_MEMAUDIT_RAW="$FIXTURES/alloc_memaudit_over.raw.txt" \
    bash "$c/scripts/quality/23_alloc_budget.sh" 3
  rm -rf -- "$c"

  c="$(mk_copy g5)" || die "trimmed copy g5 failed (mk_copy aborted)"
  green G5 "scripts/quality/23_alloc_budget.sh" "unbroken tree; instrument shim: all rows under threshold (12x)" \
    env PATH="$PATH_SHIM" FX8A09_MEMAUDIT_RAW="$FIXTURES/alloc_memaudit_ok.raw.txt" \
    bash "$c/scripts/quality/23_alloc_budget.sh" 3
  rm -rf -- "$c"
}

# --- P10/P11 + G6 — the complexity ratchet on a seeded baseline (F0.5c) -------
proof_complexity() {
  local c
  c="$(mk_copy p10)" || die "trimmed copy p10 failed (mk_copy aborted)"
  rm -f -- "$c/docs/audit/complexity-baseline.json"
  red P10 "scripts/quality/26_complexity.sh" "ratchet baseline removed (a ratchet with no floor must fail)" \
    env PATH="$PATH_BARE" bash "$c/scripts/quality/26_complexity.sh"
  rm -rf -- "$c"

  c="$(mk_copy p11)" || die "trimmed copy p11 failed (mk_copy aborted)"
  cp "$FIXTURES/complexity_lower_baseline.json" "$c/docs/audit/complexity-baseline.json"
  red P11 "scripts/quality/26_complexity.sh" "seeded lower baseline: every HARD count reads as GREW" \
    env PATH="$PATH_BARE" bash "$c/scripts/quality/26_complexity.sh"
  rm -rf -- "$c"

  c="$(mk_copy g6)" || die "trimmed copy g6 failed (mk_copy aborted)"
  green G6 "scripts/quality/26_complexity.sh" "committed baseline matches the tree (GATE OK)" \
    env PATH="$PATH_BARE" bash "$c/scripts/quality/26_complexity.sh"
  rm -rf -- "$c"
}

# --- P12 + G7 — the clippy yaml structural pre-check (F0.5b) ------------------
# The BEHAVIORAL proof (clippy -D warnings on a warning-dirty tree) needs the
# real toolchain — parked [VERIFY-central] as C1 in PROOFS.md. What runs here
# is the structural invariant of the lane's wiring: fed the pre-F0.5b shape
# (continue-on-error restored, -D warnings + pipefail absent), it goes red.
proof_clippy_yaml() {
  local c
  c="$(mk_copy p12)" || die "trimmed copy p12 failed (mk_copy aborted)"
  cp "$FIXTURES/clippy_continue_on_error.yml" "$c/.github/workflows/rust-clippy.yml"
  red P12 ".github/workflows/rust-clippy.yml (structural pre-check, F0.5b)" \
    "the pre-fix shape: continue-on-error restored, -D warnings and pipefail absent" \
    bash "$RUNNER" --check-clippy-yaml "$c/.github/workflows/rust-clippy.yml"
  rm -rf -- "$c"

  green G7 ".github/workflows/rust-clippy.yml (structural pre-check, F0.5b)" \
    "the real workflow: no continue-on-error, -D warnings + pipefail present" \
    bash "$RUNNER" --check-clippy-yaml "$ROOT/.github/workflows/rust-clippy.yml"
}

# --- P13 + G9 — the package-lock text gate (F0.5f) ------------------------------
proof_package_lock() {
  local c
  c="$(mk_copy p13)" || die "trimmed copy p13 failed (mk_copy aborted)"
  cp "$FIXTURES/package_lock_broken.json" "$c/package-lock.json"
  red P13 "package-lock.json JSON validity (F0.5f text gate)" \
    "the audited merge-conflict corruption re-seeded into the lockfile" \
    python3 -m json.tool "$c/package-lock.json"
  rm -rf -- "$c"

  green G9 "package-lock.json JSON validity (F0.5f text gate)" \
    "the real lockfile parses" \
    python3 -m json.tool "$ROOT/package-lock.json"
}

# --- G8/G10/G11 — the remaining structural greens --------------------------------
proof_structural_greens() {
  green G8 ".github/workflows/audit.yml (structural pre-check, F0.5e)" \
    "the real workflow: every miri/asan/ubsan lane carries no || true" \
    bash "$RUNNER" --check-ub-yaml "$ROOT/.github/workflows/audit.yml"
  green G10 ".github/dependabot.yml (F0.5f text gate)" \
    "the real config: every package-ecosystem entry is non-empty" \
    bash "$RUNNER" --check-dependabot "$ROOT/.github/dependabot.yml"
  # G11 anchors C1's parked central command verbatim in the real workflow, so
  # the parked block cannot silently rot.
  green G11 "parked-proof anchor (C1)" \
    "the exact 'cargo clippy ... -- -D warnings' command exists verbatim in rust-clippy.yml" \
    grep -F -q -- '--message-format=json -- -D warnings' "$ROOT/.github/workflows/rust-clippy.yml"
}

# --- the run ------------------------------------------------------------------
say "gate-proofs: starting (root: $ROOT)"
MD_ON=1

md_open() {
  printf '%s\n' \
    '# F0.5 gate proofs — GENERATED, never hand-edit' \
    '' \
    '<!-- GENERATED by scripts/quality/gate-proofs/run_proofs.sh on '"$(date -u +%Y-%m-%dT%H:%M:%SZ)"' on host '"$(uname -s -m)"'.' \
    '     Do not hand-edit: the runner is this file’s only writer; any manual' \
    '     change is overwritten on the next run. Regenerate:' \
    '         bash scripts/quality/gate-proofs/run_proofs.sh' \
    '     (run without cargo; the exit contract is: 0 = every red assertion went' \
    '     red and every green control went green, 1 = anything else.) -->' \
    > "$PROOFS" || die "cannot write $PROOFS"
}

md_open
md ""
md "## What this is"
md ""
md "FIX-PLAN §2 F0.5 acceptance: “Each gate, fed a deliberately broken tree,"
md "goes red (prove it once in the PR).” For every F0.5 gate whose logic runs"
md "without cargo: a throwaway copy of the repo (mktemp workdir, trimmed of"
md ".git, the root audit/ evidence tree, design/ assets, target/ and"
md "node_modules/ — no gate under proof reads any of them), exactly ONE"
md "seeded break, the REAL gate script run inside the copy, the exit code"
md "asserted. The real tree is never mutated (sentinel guard G0)."
md ""
md "No cargo runs anywhere in this harness: gates that need an instrument"
md "(cargo for mem-audit, cargo-fuzz for the campaign) run behind test shims"
md "on the copy's PATH — the gate's own sampling, parsing, judging and exit"
md "path execute for real; only the instrument is emulated. The behavioral"
md "proofs that need the real instruments (clippy -D warnings, miri, asan,"
md "ubsan) are parked below with their exact central commands."
md ""

md "## Red assertions — 13 (a red that stays GREEN fails the run)"
md ""
md "| # | gate | seeded break (one per copy) | observed exit | verdict |"
md "|---|------|------------------------------|---------------|---------|"

proof_fuzz
proof_oracle
proof_alloc
proof_complexity
proof_clippy_yaml
proof_package_lock
proof_structural_greens

for r in "${RED_ROWS[@]}"; do md "$r"; done

md ""
md "## Green controls (a green that goes RED fails the run)"
md ""
md "| # | gate | tree | observed exit | verdict |"
md "|---|------|------|---------------|---------|"

# G0 — the real-tree guard, evaluated last (after every proof ran).
: > "$WORK/sentinels.after"
for s in "${SENTINELS[@]}"; do
  printf '%s  %s\n' "$(sha256sum "$ROOT/$s" | awk '{print $1}')" "$s" >> "$WORK/sentinels.after"
done
green G0 "real-tree guard" "10 sentinel files byte-identical before/after the whole run" \
  cmp -s "$WORK/sentinels.before" "$WORK/sentinels.after"

# Self-tests — the runner spawns its own rigged modes and asserts they fail.
bash "$RUNNER" --self-test-green > "$WORK/logs/self-green.log" 2>&1
SELF_RC_G=$?
SELF_TOTAL=$((SELF_TOTAL + 1))
if [ "$SELF_RC_G" -ne 0 ]; then
  SELF_ROWS+=("| --self-test-green: a green control on a command that exits 7 | runner exits 1 | $SELF_RC_G |")
else
  SELF_FAILED=$((SELF_FAILED + 1))
  SELF_ROWS+=("| --self-test-green: a green control on a command that exits 7 | runner exits 1 | 0 — **FAIL: the runner stayed green** |")
fi
bash "$RUNNER" --self-test-red > "$WORK/logs/self-red.log" 2>&1
SELF_RC_R=$?
SELF_TOTAL=$((SELF_TOTAL + 1))
if [ "$SELF_RC_R" -ne 0 ]; then
  SELF_ROWS+=("| --self-test-red: a red assertion on a command that exits 0 | runner exits 1 | $SELF_RC_R |")
else
  SELF_FAILED=$((SELF_FAILED + 1))
  SELF_ROWS+=("| --self-test-red: a red assertion on a command that exits 0 | runner exits 1 | 0 — **FAIL: the runner stayed green** |")
fi

for g in "${GREEN_ROWS[@]}"; do md "$g"; done

md ""
md "## Runner self-tests (the harness proves it can fail)"
md ""
md "The earlier runner of this harness silently swallowed failed assertions"
md "(double-quoted backtick fences, an escaped-bracket printf, a bare mktemp"
md "template, a swallowed parked block). Four further classes surfaced in"
md "this runner's own first draft: a single-statement 'local a=... b=...\$a'"
md "aborts under set -u before assigning, so mk_copy returned an empty path"
md "and 11 red rows asserted at rc=127 with no gate ever run; die() inside"
md "a command-substitution mk_copy only exits the subshell; tar's unanchored"
md "--exclude=audit stripped docs/audit/ from every copy; and the yaml"
md "checkers grepped comments too, false-failing on the workflow's own"
md "fix-notes. All are closed structurally: every verdict goes through"
md "red()/green() above, red() rejects 126/127 as gate-not-runnable, every"
md "mk_copy call site chains a real abort, exclusions are anchored and the"
md "copy completeness-checked, checks read comment-stripped bodies, the"
md "ledger is size-checked at the single exit path, and the two rigged"
md "modes below are spawned and asserted to exit 1 on every run."
md ""
md "| rigged mode | expectation | observed exit |"
md "|-------------|-------------|---------------|"
for s in "${SELF_ROWS[@]}"; do md "$s"; done

md ""
md "## Parked — [VERIFY-central] (behavioral proofs that need the real instruments)"
md ""
md "These cannot run in this harness (no cargo here, by design) and are"
md "parked with their exact central commands. They are anchored: G11 asserts"
md "C1's command exists verbatim in the workflow, G8 asserts the miri/asan/"
md "ubsan lanes exist and carry no swallow — so the parked block cannot rot"
md "silently."
md ""
md "| # | gate | exact central command | status |"
md "|---|------|------------------------|--------|"
md "| C1 | clippy -D warnings behavioral (F0.5b) | see the C1 block below | parked: run centrally |"
md "| C2 | miri behavioral (F0.5e) | see the C2 block below | parked: BLOCKED on the out-of-scope 18_ub_checks.sh exit-propagation fix (F2.3) |"
md "| C3 | asan/ubsan behavioral (F0.5e) | see the C3 block below | parked: BLOCKED, same dependency as C2 |"
md ""
md "### C1 — clippy -D warnings behavioral (from .github/workflows/rust-clippy.yml)"
md ""
md '```'
md 'set -o pipefail'
md 'cargo clippy --all-features --message-format=json -- -D warnings \'
md '  | clippy-sarif | tee rust-clippy-results.sarif | sarif-fmt'
md '```'
md ""
md "Central proof: seed a deliberate clippy warning (e.g. an unused import in"
md "a throwaway crate), run the command above — clippy exits non-zero"
md "(-D warnings turns the warning into an error), pipefail carries it through"
md "the SARIF pipeline, the workflow step is red. G7 proved the wiring this"
md "command depends on."
md ""
md "### C2 — miri behavioral (from .github/workflows/audit.yml, formal job)"
md ""
md '```'
md 'for c in media-types codec-api packet-queue; do'
md '  echo "== miri: $c =="'
md '  bash scripts/quality/18_ub_checks.sh miri "$c"'
md 'done'
md '```'
md ""
md "Central proof: seed a Miri-detectable UB harness in one of the three"
md "crates and observe the miri lane fail end-to-end (step exit non-zero)."
md "STATUS — BLOCKED (known out-of-scope, carried in FX8-A09's risks): the"
md 'workflow-level `|| true` swallows are gone (FX1-A10 item (e)), but'
md "18_ub_checks.sh itself still masks the instrument's exit (its tee|tail"
md "and trailing cat blocks never propagate a non-zero cargo-miri rc —"
md "FX1-A10's disclosed PARTIAL residual), and the formal/sanitizers jobs"
md "still carry job-level continue-on-error: true. Until that F2.3"
md "exit-propagation fix lands, a seeded UB stays green at the job level —"
md "this proof is parked green and stays parked."
md ""
md "### C3 — asan/ubsan behavioral (from .github/workflows/audit.yml, sanitizers job)"
md ""
md '```'
md 'bash scripts/quality/18_ub_checks.sh asan codecs'
md 'bash scripts/quality/18_ub_checks.sh ubsan codecs'
md '```'
md ""
md "Central proof: seed an ASan/UBSan-detectable bug on the rav1d C-ABI"
md "surface and observe the lane fail end-to-end. Same BLOCKED dependency as"
md "C2."
md ""
md "## Ledger integrity (checked at the single exit path)"
md ""
md "| class | expected | executed | failed |"
md "|-------|----------|----------|--------|"
md "| red assertions | $EXPECT_REDS | $REDS_TOTAL | $REDS_FAILED |"
md "| green controls | $EXPECT_GREENS | $GREENS_TOTAL | $GREENS_FAILED |"
md "| runner self-tests | $EXPECT_SELF | $SELF_TOTAL | $SELF_FAILED |"
md ""

if [ "$REDS_FAILED" -gt 0 ] || [ "$GREENS_FAILED" -gt 0 ] || [ "$SELF_FAILED" -gt 0 ]; then
  md "**RUN FAILED**: $REDS_FAILED red assertion(s) stayed green, $GREENS_FAILED green control(s) went red, $SELF_FAILED self-test(s) stayed green."
  exit 1
fi
if [ "$REDS_TOTAL" -ne "$EXPECT_REDS" ] || [ "$GREENS_TOTAL" -ne "$EXPECT_GREENS" ] || [ "$SELF_TOTAL" -ne "$EXPECT_SELF" ]; then
  md "**RUN FAILED**: ledger size mismatch (red=$REDS_TOTAL/$EXPECT_REDS, green=$GREENS_TOTAL/$EXPECT_GREENS, self=$SELF_TOTAL/$EXPECT_SELF) — an assertion was lost or duplicated."
  exit 1
fi
md "**RUN OK**: $REDS_TOTAL red assertion(s) all red, $GREENS_TOTAL green control(s) all green, $SELF_TOTAL self-test(s) both failed as rigged."
md ""
md "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ) by scripts/quality/gate-proofs/run_proofs.sh (exit 0)."
say "gate-proofs: $REDS_TOTAL reds / $GREENS_TOTAL greens / $SELF_TOTAL self-tests — see $PROOFS"
exit 0
