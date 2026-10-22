#!/usr/bin/env bash
# oracle_check.sh — EXTERNAL-oracle hash check for the committed media
# fixtures (card W1-14; audit finding M15's cure; engineering directives
# D-5 "escape the echo chamber" + D-11 "ffprobe/ffmpeg as the external
# oracle").
#
# WHAT THIS IS
#   For every media fixture under tests/fixtures/generated/valid/ this
#   script decodes each stream with the system ffmpeg (a production
#   decoder that shares NO code with Lumen — the whole point of an
#   external oracle), hashes the decoded bytes with sha256sum, and
#   compares against the committed pin sidecars
#   tests/fixtures/generated/valid/<fixture>.oracle.sha256.
#
#   The sidecars are the ORACLE PINS: hashes of ffmpeg's own decode
#   output, recorded with full provenance (ffmpeg version + the exact
#   commands). They are NOT Lumen decoder output (that was the M15
#   defect: every previous "pinned hash" was writer-truth or
#   decoder-output-recorded — self-referential). A future differential
#   test that runs Lumen's decoder can now be judged against a hash
#   that an independent engine produced.
#
# USAGE (opt-in — deliberately NOT wired into scripts/full_gate.sh or
# any CI gate; the local gate must never depend on ffmpeg being
# installed, mirroring the env-gated corpus suites):
#   bash scripts/oracle_check.sh            # from the repo root (or any
#                                           # subdir — the root is found
#                                           # from this script's path)
#   bash scripts/oracle_check.sh -v         # also show ffmpeg's stderr
#
# EXIT CODE
#   0 = every fixture x stream hashed equal to its pin
#   1 = at least one FAIL (hash mismatch, missing/malformed sidecar,
#       missing fixture, ffmpeg error, an empty fixture set, a sidecar
#       whose row set does not match its header-declared stream set,
#       or a media fixture outside the pinned extension perimeter)
#
# STREAM-SET DECLARATION (row-set completeness — W2-17's attack cure)
#   Every sidecar header carries a `# streams: <label>…` row naming
#   the fixture's COMPLETE pin set. The check enforces the row set ==
#   the declared set exactly: deleting a row (a 2-stream sidecar with
#   only its video row) used to PASS because only row-level integrity
#   was checked; now it FAILs, and so does an undeclared extra row.
#
# PERIMETER (the other W2-17 cure — no silent unpin through growth)
#   The pinned extensions are {.avi,.mp4} (the find below). Media
#   files with any other KNOWN media extension are not silently
#   skipped: each is explicitly warned about and FAILS the run — a
#   future .mkv/.mov/.webm/… fixture must extend the pin set (find
#   arm + sidecar + stream-set declaration) and the
#   UNSUPPORTED_MEDIA list below in the same commit. Extensions
#   outside both lists (json sidecar metadata, READMEs, …) are
#   ignored.
#
# RE-PINNING (deliberate, never automatic)
#   If a FAIL is legitimate (new ffmpeg whose decode genuinely changed,
#   or an intentionally changed fixture), re-record the sidecar with the
#   EXACT commands documented in its provenance header, e.g.:
#     ffmpeg -nostdin -v error -i tests/fixtures/generated/valid/avi_mjpeg_pcm.avi \
#            -map 0:v -pix_fmt yuv420p -f rawvideo - | sha256sum
#     ffmpeg -nostdin -v error -i tests/fixtures/generated/valid/avi_mjpeg_pcm.avi \
#            -map 0:a -f s16le - | sha256sum
#   …then update the sidecar's hash rows AND its provenance header
#   (ffmpeg version + geometry) in the same commit, and say WHY in the
#   commit message. Never re-pin to silence an unexplained drift.
#
# WHY -pix_fmt yuv420p IS EXPLICIT
#   The MJPEG fixtures decode to full-range 4:2:0 (yuvj420p in ffmpeg
#   vocabulary). On ffmpeg 7.x the deprecated yuvj* formats carry the
#   range as metadata only, so yuvj420p -> yuv420p is byte-identical
#   (verified at pin time: default / -pix_fmt yuv420p / -pix_fmt
#   yuvj420p all hash equal). Pinning the flag keeps the command's
#   meaning stable even if a future ffmpeg changes its rawvideo default.
set -euo pipefail

# --- locate the repo root from this script's own path -----------------------
SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
ROOT="$(cd -- "$SCRIPT_DIR/.." && pwd)"
FIXTURE_DIR="$ROOT/tests/fixtures/generated/valid"

VERBOSE=0
if [[ "${1:-}" == "-v" ]]; then VERBOSE=1; fi
if [[ "${1:-}" == "-h" || "${1:-}" == "--help" ]]; then
  # Reprint this file's own header comment block as the help text
  # (everything up to the `set` line — no hardcoded line range to
  # silently drift when the header grows).
  sed '/^set -euo pipefail$/,$d' "$0" | sed '1d' | sed 's/^# \{0,1\}//'
  exit 0
fi

# --- tool prerequisites ------------------------------------------------------
for tool in ffmpeg sha256sum awk; do
  if ! command -v "$tool" >/dev/null 2>&1; then
    echo "oracle_check: FAIL — required tool '$tool' not found in PATH" >&2
    exit 1
  fi
done
FFMPEG_VERSION="$(ffmpeg -version 2>/dev/null | head -n1)"
if [[ -z "$FFMPEG_VERSION" ]]; then
  echo "oracle_check: FAIL — could not read 'ffmpeg -version'" >&2
  exit 1
fi

# Decode one stream of one fixture with ffmpeg and echo "hash bytes".
# $1 = fixture path, $2 = stream label ("video" | "audio").
# Status is checked EXPLICITLY: this function runs in an `||` context,
# where bash disables `set -e` for the whole body (the classic errexit
# trap) — a silent ffmpeg failure must never hash the empty stream.
decode_hash () {
  local fixture="$1" label="$2"
  local -a cmd=(ffmpeg -nostdin -v error -i "$fixture")
  case "$label" in
    video) cmd+=(-map 0:v -pix_fmt yuv420p -f rawvideo -) ;;
    audio) cmd+=(-map 0:a -f s16le -) ;;
    *)     echo "oracle_check: unknown stream label '$label'" >&2; return 2 ;;
  esac
  # One streaming pass for the hash, one for the byte count: no temp
  # files, no unbounded buffering (rawvideo of future large fixtures
  # must never materialize on disk just to be hashed).
  local hash bytes errsink="/dev/null"
  if [[ "$VERBOSE" -eq 1 ]]; then errsink="/dev/stderr"; fi
  if ! hash="$("${cmd[@]}" 2>"$errsink" | sha256sum | awk '{print $1}')"; then
    echo "oracle_check: FAIL — ffmpeg exited non-zero decoding $fixture [$label]" >&2
    return 1
  fi
  if ! bytes="$("${cmd[@]}" 2>"$errsink" | wc -c | tr -d '[:space:]')"; then
    echo "oracle_check: FAIL — byte-count pass failed for $fixture [$label]" >&2
    return 1
  fi
  if [[ ! "$hash" =~ ^[0-9a-f]{64}$ ]]; then
    echo "oracle_check: FAIL — hash pipeline produced no digest for $fixture [$label]" >&2
    return 1
  fi
  if [[ "$bytes" -eq 0 ]]; then
    echo "oracle_check: FAIL — ffmpeg decoded 0 bytes for $fixture [$label]" >&2
    return 1
  fi
  echo "$hash $bytes"
}

# --- enumerate fixtures (media files only; .structure.json etc. excluded) ---
mapfile -t FIXTURES < <(
  find "$FIXTURE_DIR" -maxdepth 1 -type f \( -name '*.avi' -o -name '*.mp4' \) 2>/dev/null | sort
)
if [[ ${#FIXTURES[@]} -eq 0 || -z "${FIXTURES[0]:-}" ]]; then
  echo "oracle_check: FAIL — no media fixtures found under $FIXTURE_DIR (the oracle pin set must not silently vanish)" >&2
  exit 1
fi

# --- perimeter: known media extensions this script does NOT pin ---------------
# The explicit unsupported-extension warning list. A file with one of
# these extensions is media this build CANNOT oracle-pin today — it is
# named loudly (never silently skipped) and fails the run, because an
# unpinned fixture is audit finding M15 re-entering through growth.
UNSUPPORTED_MEDIA="mkv mov webm ts m2ts mts flac mp3 wav ogg opus aac m4a wma ape mpc amr"
UNPINNED=()
while IFS= read -r file; do
  base="$(basename -- "$file")"
  ext="${base##*.}"
  ext="${ext,,}"
  case " $UNSUPPORTED_MEDIA " in
    *" $ext "*) UNPINNED+=("$base") ;;
  esac
done < <(find "$FIXTURE_DIR" -maxdepth 1 -type f 2>/dev/null | sort)
if [[ ${#UNPINNED[@]} -gt 0 ]]; then
  echo "oracle_check: WARN — media fixtures outside the pinned perimeter {.avi,.mp4}:"
  for base in "${UNPINNED[@]}"; do
    echo "  WARN $base — not oracle-pinned (extend the pin set + UNSUPPORTED_MEDIA in the same commit)"
  done
  echo "oracle_check: RESULT = FAIL (media present outside the pinned perimeter)"
  exit 1
fi

echo "external-oracle hash check (ffmpeg decode vs committed pins)"
echo "oracle binary : $FFMPEG_VERSION"
echo "fixture dir   : $FIXTURE_DIR"
echo

PASS=0
FAIL=0

for fixture in "${FIXTURES[@]}"; do
  name="$(basename -- "$fixture")"
  sidecar="$FIXTURE_DIR/$name.oracle.sha256"

  # Provenance note: does the sidecar's recorded oracle version match
  # the running one? (informational — the hash rows are the verdict)
  if [[ -f "$sidecar" ]]; then
    if ! grep -qF "$FFMPEG_VERSION" "$sidecar"; then
      echo "fixture $name"
      echo "  NOTE sidecar provenance does not mention this ffmpeg version —"
      echo "       a FAIL below may be oracle-version drift, not a defect"
    fi
  fi

  if [[ ! -f "$sidecar" ]]; then
    echo "fixture $name"
    echo "  FAIL no pin sidecar: $sidecar is missing (every valid fixture must be oracle-pinned)"
    FAIL=$((FAIL + 1))
    continue
  fi

  echo "fixture $name"

  seen_labels=""
  # Sidecar rows: non-comment, non-blank lines of "<64-hex>  <label>".
  while read -r want label extra; do
    [[ -z "${want:-}" ]] && continue
    if [[ ! "$want" =~ ^[0-9a-f]{64}$ ]]; then
      echo "  FAIL [$label] sidecar row is not a sha256 digest: '$want'"
      FAIL=$((FAIL + 1))
      continue
    fi
    if [[ -n "${extra:-}" ]]; then
      echo "  FAIL [$label] sidecar row has trailing junk: '$extra'"
      FAIL=$((FAIL + 1))
      continue
    fi
    # Duplicate stream labels are a malformed pin (two rows would be
    # checked against the same decode; a stale copy must not survive).
    if [[ " $seen_labels " == *" $label "* ]]; then
      echo "  FAIL [$label] duplicate stream label in sidecar"
      FAIL=$((FAIL + 1))
      continue
    fi
    seen_labels="$seen_labels $label"
    got="$(decode_hash "$fixture" "$label")" || { FAIL=$((FAIL + 1)); continue; }
    ghash="$(echo "$got"  | awk '{print $1}')"
    gbytes="$(echo "$got" | awk '{print $2}')"
    if [[ "$ghash" == "$want" ]]; then
      echo "  PASS [$label] ${want:0:16}… ($gbytes decoded bytes)"
      PASS=$((PASS + 1))
    else
      echo "  FAIL [$label] expected ${want:0:16}… got ${ghash:0:16}… ($gbytes decoded bytes)"
      echo "       expected: $want"
      echo "       actual  : $ghash"
      FAIL=$((FAIL + 1))
    fi
  done < <(grep -v '^#' "$sidecar" | grep -v '^[[:space:]]*$' || true)

  # The sidecar must pin at least one stream (a header-only sidecar
  # would otherwise pass vacuously).
  rows="$(grep -v '^#' "$sidecar" | grep -c -v '^[[:space:]]*$' || true)"
  if [[ "${rows:-0}" -eq 0 ]]; then
    echo "  FAIL sidecar carries no hash rows (vacuous pin)"
    FAIL=$((FAIL + 1))
  fi

  # Row-set completeness (W2-17's attack cure): the sidecar header
  # declares the fixture's complete stream set (`# streams: …`); the
  # pinned row set must match it exactly. A sidecar that DROPPED a
  # row used to pass (row-level integrity without row-set
  # completeness); an undeclared extra row is the same drift. (The
  # `|| true` matters: a declaration-less sidecar makes grep exit 1,
  # and under `set -euo pipefail` that would kill the script silently
  # instead of reporting the FAIL below.)
  declared="$(grep -m1 '^# streams:' "$sidecar" | sed 's/^# streams://' | tr -s '[:space:]' ' ' || true)"
  if [[ -z "${declared// /}" ]]; then
    echo "  FAIL sidecar does not declare its stream set (add a '# streams: <label>…' header row)"
    FAIL=$((FAIL + 1))
  else
    for label in $declared; do
      if [[ " $seen_labels " != *" $label "* ]]; then
        echo "  FAIL [$label] declared in the sidecar header but pinned by no row (row-set completeness — a dropped row must not pass)"
        FAIL=$((FAIL + 1))
      fi
    done
    for label in $seen_labels; do
      if [[ " $declared " != *" $label "* ]]; then
        echo "  FAIL [$label] pinned by a row but absent from the header's declared stream set"
        FAIL=$((FAIL + 1))
      fi
    done
  fi
  echo
done

echo "oracle check: $PASS stream(s) PASS, $FAIL FAIL, $((PASS + FAIL)) checked"
if [[ "$FAIL" -ne 0 ]]; then
  echo "oracle check: RESULT = FAIL"
  exit 1
fi
echo "oracle check: RESULT = PASS"
