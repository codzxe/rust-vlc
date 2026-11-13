#!/bin/bash
# Supervisor for the corpus generator: the sandbox reaper kills long
# processes; the generator is resume-safe (plan.json + incremental
# manifest), so relaunch it until the TOTAL line lands.
OUT=/home/z/my-project/corpus
LOG=/home/z/my-project/logs/corpus-gen.log
cd /home/z/my-project
for attempt in $(seq 1 40); do
  if grep -q "^\[gen\] TOTAL" "$LOG" 2>/dev/null; then
    echo "[supervisor] corpus complete after $((attempt-1)) restarts"
    exit 0
  fi
  echo "[supervisor] attempt $attempt $(date -Is)"
  python3 -u tools/corpus/gen_corpus.py --out "$OUT" --count 400 --budget-mb 1700 >> "$LOG" 2>&1
  rc=$?
  echo "[supervisor] generator exited rc=$rc $(date -Is)"
  if [ $rc -ne 0 ] && ! grep -q "Traceback" "$LOG"; then
    sleep 2
  fi
done
echo "[supervisor] gave up after 40 attempts"
exit 1
