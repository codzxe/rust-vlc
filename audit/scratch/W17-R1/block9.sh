# Same assert shape as the Linux lane, native session, no X
# server. Same receipt strings (app.rs / main.rs literals).
set -uo pipefail
status=0
SURVIVE=10
BIN=apps/desktop-tauri/target/release/lumen-tauri
FIXTURE=tests/fixtures/generated/valid/avi_mjpeg_pcm.avi
LOGDIR=launch-logs/macos
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

echo "=== tauri launch attempt (macos / WKWebView, best-effort) ==="

# 1. launch — direct exec with captured output. The runner's
#    Aqua session is what makes this PLAUSIBLE, not PROVEN:
#    that honesty is the whole verdict posture of this job.
"$BIN" "$FIXTURE" >"$LOGDIR/stdout.log" 2>"$LOGDIR/stderr.log" &
pid=$!

# 2. survival poll (assert 1)
survived=no
died_at=""
t0=$(date +%s)
for _ in $(seq 1 $((SURVIVE * 2))); do
  sleep 0.5
  if ! kill -0 "$pid" 2>/dev/null; then
    died_at="$(( $(date +%s) - t0 ))s"
    break
  fi
done
kill -0 "$pid" 2>/dev/null && survived=yes
echo "survival window (${SURVIVE}s): ${survived}${died_at:+ (died at ≈${died_at})}"

# 3. receipts (assert 2)
if grep -q "event emit failed" "$LOGDIR/stderr.log" \
   || grep -q "admission shell failed" "$LOGDIR/stderr.log"; then
  echo "receipts: FAILED — failure receipts present:"
  grep -n "event emit failed\|admission shell failed" "$LOGDIR/stderr.log" | head -5
  status=1
else
  echo "receipts: PASS (no 'event emit failed' / 'admission shell failed' in stderr)"
fi

# 4. termination (assert 3): TERM the app directly (no wrapper
#    tree here), bounded KILL fallback. 143 expected; 0 = clean
#    self-exit in the race window; else a crash face.
if [ "$survived" = "yes" ]; then
  kill -TERM "$pid" 2>/dev/null || true
  for _ in $(seq 1 20); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 0.5
  done
  kill -KILL "$pid" 2>/dev/null || true
fi
# Same errexit guard as the Linux lane (wait returns the
# app's terminated status to a set-e shell).
exit_code=0
wait "$pid" 2>/dev/null || exit_code=$?
echo "launch exit code: ${exit_code}"

if [ "$survived" != "yes" ]; then
  echo "survival: FAILED (early exit; stderr tail below)"
  tail -5 "$LOGDIR/stderr.log" | sed 's/^/  /'
  status=1
else
  echo "survival: PASS (${SURVIVE}s, native session)"
  case "$exit_code" in
    143) echo "termination: PASS (143 = harness-TERM, the expected shape)" ;;
    0)   echo "termination: PASS (0 = clean self-exit in the race window)" ;;
    *)   echo "termination: FAILED (unexpected exit ${exit_code})"; status=1 ;;
  esac
fi

echo "=== tauri launch attempt (macos): $([ "$status" -eq 0 ] && echo 'BEST-EFFORT PASS' || echo FAIL) ==="
echo "VERDICT: BEST-EFFORT-PENDING — a green attempt is NOT a per-OS verdict; human confirmation on real hardware owed (ADR-004 verdict table)."
exit "$status"
