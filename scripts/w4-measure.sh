#!/bin/bash
# W4 measurement battery: harness oracle report + speed baseline on corpus-big
cd /home/z/my-project
echo "=== HARNESS (skip-suites, oracle-side) on corpus-big ==="
python3 tools/corpus/run_harness.py --corpus corpus-big --skip-suites 2>&1 | tail -15
echo "=== SPEED BENCH (ffmpeg baseline, 36-file stratified sample) ==="
python3 tools/corpus/speed_bench.py --corpus corpus-big --count 36 --stride 7 --repeat 2 --md docs/corpus/speed-baseline-20261022.md --json /tmp/speed.json 2>&1 | tail -45
echo "MEASURE_DONE"
