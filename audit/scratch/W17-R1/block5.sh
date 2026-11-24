# The launch-verdict smoke — this lane's whole point. Shape
# follows the repo's established patterns: rc-smoke.sh's
# numbered checks + honest fail counter (`set -uo pipefail`,
# no -e: every assert reports, not just the first) and
# ci.yml's link/exit smoke's exit-code case analysis. The
# difference: under xvfb the binary REALLY lives for the
# survival window — no 86-pending exit, no headless dodge.
set -uo pipefail
status=0
SURVIVE=12
BIN=apps/desktop-tauri/target/release/lumen-tauri
FIXTURE=tests/fixtures/generated/valid/avi_mjpeg_pcm.avi
LOGDIR=launch-logs/linux
need() { command -v "$1" >/dev/null 2>&1 || { echo "tauri-launch: preflight: '$1' not found in PATH" >&2; exit 2; }; }
need xvfb-run
mkdir -p "$LOGDIR"
touch "$LOGDIR/stdout.log" "$LOGDIR/stderr.log"
test -x "$BIN" || { echo "::error::release binary missing after cargo build (${BIN})"; exit 1; }
test -f "$FIXTURE" || { echo "::error::fixture missing: ${FIXTURE}"; exit 1; }

{
  echo "head: $(git rev-parse --short HEAD)"
  echo "host: $(uname -smr)"
  echo "survival window: ${SURVIVE}s"
  echo "fixture: ${FIXTURE} (argv file-open seam)"
} >"$LOGDIR/context.txt"

echo "=== tauri launch verdict (linux / WebKitGTK / xvfb) ==="

# 1. launch — REAL X server (xvfb-run), the fixture on argv so
#    the shell dispatches a real open and the 40 ms pump emits
#    real engine events on lumen://event. setsid gives the
#    whole tree (xvfb-run + Xvfb + app) one process group so
#    the harness can TERM it as a unit.
#
#    WEBKIT_DISABLE_DMABUF_RENDERER / WEBKIT_DISABLE_COMPOSITING_MODE:
#    WebKitGTK's GPU-accelerated paths need a GPU; xvfb has
#    none. Both force the software rendering path — this is
#    launch-HARNESS environment, NOT app code; the human half
#    of the verdict runs on a real desktop without them.
export WEBKIT_DISABLE_DMABUF_RENDERER=1
export WEBKIT_DISABLE_COMPOSITING_MODE=1
setsid xvfb-run -a --server-args="-screen 0 1920x1080x24" \
  "$BIN" "$FIXTURE" >"$LOGDIR/stdout.log" 2>"$LOGDIR/stderr.log" &
pgid=$!

# 2. survival poll (assert 1): 0.5 s ticks for the whole
#    window. An early death fails the verdict regardless of
#    exit code.
survived=no
died_at=""
t0=$(date +%s)
for _ in $(seq 1 $((SURVIVE * 2))); do
  sleep 0.5
  if ! kill -0 "$pgid" 2>/dev/null; then
    died_at="$(( $(date +%s) - t0 ))s"
    break
  fi
done
kill -0 "$pgid" 2>/dev/null && survived=yes
echo "survival window (${SURVIVE}s): ${survived}${died_at:+ (died at ≈${died_at})}"

# 3. receipts (assert 2): the app's own stderr is the truth
#    source here — forward_pending's emit-failure line and
#    main.rs's admission-failed line (see the header). Their
#    ABSENCE across a run WITH real event traffic (the
#    argv-open burst + playback events) is the CI half of the
#    event round-trip proof; it is NOT the JS-side proof.
if grep -q "event emit failed" "$LOGDIR/stderr.log" \
   || grep -q "admission shell failed" "$LOGDIR/stderr.log"; then
  echo "receipts: FAILED — failure receipts present:"
  grep -n "event emit failed\|admission shell failed" "$LOGDIR/stderr.log" | head -5
  status=1
else
  echo "receipts: PASS (no 'event emit failed' / 'admission shell failed' in stderr)"
fi

# 4. termination (assert 3): the harness owns the exit — TERM
#    the process group (xvfb-run, Xvfb and the app die as one;
#    xvfb-run does not forward signals, so a bare TERM to the
#    script pid would orphan the app), bounded KILL fallback.
#    143 = harness-TERM (expected); 0 = clean self-exit in the
#    race window; 1/101 = the crash faces; anything else is
#    unexplained. A user-initiated clean exit (window close →
#    0) has no driver under xvfb — that half stays with the
#    human/GUI verdict.
if [ "$survived" = "yes" ]; then
  # The harness must SURVIVE its own kill: on the round-1 run
  # (43410313414) both the linux and macos verdicts passed
  # survival + receipts and then the STEP died with 143 — the
  # group-TERM reached the harness shell too (the step script
  # shares a process group with its background job when setsid
  # is a no-op fork, and the harness's `wait` then reports the
  # harness's own signal death instead of the app's). Ignoring
  # TERM inside the harness makes the kill one-sided; the
  # app's exit code still arrives via `wait`.
  trap '' TERM
  kill -TERM -- "-$pgid" 2>/dev/null || true
  for _ in $(seq 1 20); do
    kill -0 "$pgid" 2>/dev/null || break
    sleep 0.5
  done
  kill -KILL -- "-$pgid" 2>/dev/null || true
  trap - TERM
fi
# errexit guard: the app's harness-TERM exit (143) arrives as
# wait's OWN status — under `set -e` that killed the harness
# before the case statement could classify it (rounds 1-2 of
# run 43410313414/43412067856: the step died 143 right after
# receipts-PASS, which is exactly `wait` returning the app's
# terminated status to a set-e shell). Capture, don't trip.
exit_code=0
wait "$pgid" 2>/dev/null || exit_code=$?
echo "launch exit code: ${exit_code}"

if [ "$survived" != "yes" ]; then
  echo "survival: FAILED (early exit; stderr tail below)"
  tail -5 "$LOGDIR/stderr.log" | sed 's/^/  /'
  status=1
else
  echo "survival: PASS (${SURVIVE}s under xvfb)"
  case "$exit_code" in
    143) echo "termination: PASS (143 = harness-TERM, the expected shape)" ;;
    0)   echo "termination: PASS (0 = clean self-exit in the race window)" ;;
    *)   echo "termination: FAILED (unexpected exit ${exit_code})"; status=1 ;;
  esac
fi

echo "=== tauri launch verdict (linux): $([ "$status" -eq 0 ] && echo PASS || echo FAIL) ==="
exit "$status"
