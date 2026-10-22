#!/bin/bash
# ED-DISPATCH-1 continuation: verify the 4 salvaged directives-fix branches.
# Runs per-branch workspace tests sequentially (2 cores — no oversubscription),
# shared target dir to reuse artifacts across branches.
# Logs: /home/z/my-project/logs/verify-branches/<branch>.log
set -u
source "$HOME/.cargo/env"
export CARGO_TARGET_DIR=/home/z/my-project/target
LOGROOT=/home/z/my-project/logs/verify-branches
mkdir -p "$LOGROOT"

for wt in fix3 3b4 fix2 fix1; do
  cd "/home/z/my-project/worktrees/$wt" || { echo "=== $wt MISSING WORKTREE ==="; continue; }
  echo "=== BRANCH $wt START $(date -Is) ==="
  cargo test --workspace > "$LOGROOT/$wt.log" 2>&1
  rc=$?
  echo "=== BRANCH $wt EXIT $rc $(date -Is) ==="
  # tail summary lines of interest
  grep -E "^test result:|error\[|error:|warning: unused" "$LOGROOT/$wt.log" | tail -25
done
echo "=== ALL BRANCH VERIFICATIONS DONE $(date -Is) ==="
