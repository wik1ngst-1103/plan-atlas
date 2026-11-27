#!/usr/bin/env bash
# Resource budget check (Linux): idle RSS with 1,000 plans <= 40 MB, release
# binary <= 20 MB. Builds the release binary (web UI must already be built).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
RSS_LIMIT_KB=$((40 * 1024))
BIN_LIMIT_BYTES=$((20 * 1024 * 1024))
PLANS=1000
PROJECTS=10
TASKS=4

cd "$ROOT"
[ -f web/dist/index.html ] || { echo "web/dist missing: run 'npm ci && npm run build' in web/ first" >&2; exit 2; }
cargo build --release --locked
BIN="$ROOT/target/release/plan-atlas"

TMP="$(mktemp -d)"
PID=""
cleanup() { [ -n "$PID" ] && kill "$PID" 2>/dev/null || true; rm -rf "$TMP"; }
trap cleanup EXIT

export PLAN_ATLAS_HOME="$TMP/home/.plan-atlas"
export HOME="$TMP/home"
export CLAUDE_CONFIG_DIR="$TMP/claude"
mkdir -p "$PLAN_ATLAS_HOME" "$CLAUDE_CONFIG_DIR"

TAGS=(type/feature type/docs type/bugfix domain/security domain/backend domain/frontend domain/data tech/jwt tech/oauth tech/react tech/redis tech/postgres)
for p in $(seq 1 $PROJECTS); do
  proj="project-$p"
  mkdir -p "$PLAN_ATLAS_HOME/plans/$proj"
  per=$((PLANS / PROJECTS))
  for n in $(seq 1 $per); do
    id=$(printf 'PLAN-%s-%03d' "$proj" "$n")
    file="$PLAN_ATLAS_HOME/plans/$proj/2026-11-$(printf '%02d' $(( (n % 27) + 1 )))-plan-$n.md"
    t1=${TAGS[$(( (p + n) % ${#TAGS[@]} ))]}
    t2=${TAGS[$(( (p * 3 + n * 7) % ${#TAGS[@]} ))]}
    {
      echo "---"
      echo "id: $id"
      echo "project: $proj"
      echo "title: Benchmark plan $n of $proj"
      echo "tags: [$t1, $t2]"
      if [ "$n" -gt 1 ] && [ $((n % 3)) -eq 0 ]; then
        echo "related:"
        echo "  - { to: $(printf 'PLAN-%s-%03d' "$proj" $((n - 1))), type: relates_to }"
      fi
      echo "created: 2026-11-$(printf '%02d' $(( (n % 27) + 1 )))"
      echo "---"
      echo "## 01. Overview"; echo
      echo "Synthetic plan $n used to measure idle memory. Lorem ipsum dolor sit amet, consectetur adipiscing elit."; echo
      echo "## 02. Requirements & Scope"; echo; echo "Scope of plan $n."; echo
      echo "## 04. Task Breakdown"; echo
      for t in $(seq 1 $TASKS); do
        echo "### $(printf 'TASK-%03d' $t): Task $t of plan $n"
        echo "- complexity: $(( (t % 5) + 1 ))"
        [ "$t" -gt 1 ] && echo "- depends_on: [$(printf 'TASK-%03d' $((t - 1)))]"
        echo "- status: todo"; echo
        echo "Description for task $t. Do the thing and verify the result."; echo
      done
    } > "$file"
  done
done

PORT=$(python3 -c 'import socket;s=socket.socket();s.bind(("127.0.0.1",0));print(s.getsockname()[1]);s.close()')
"$BIN" --no-open --port "$PORT" >"$TMP/server.log" 2>&1 &
PID=$!

for _ in $(seq 1 100); do
  curl -sf "http://127.0.0.1:$PORT/api/health" >/dev/null && break
  kill -0 "$PID" 2>/dev/null || { cat "$TMP/server.log" >&2; echo "server exited" >&2; exit 1; }
  sleep 0.1
done

count=0
for _ in $(seq 1 300); do
  count=$(curl -sf "http://127.0.0.1:$PORT/api/plans" | python3 -c 'import sys,json;print(len(json.load(sys.stdin)))' 2>/dev/null || echo 0)
  [ "$count" -ge "$PLANS" ] && break
  sleep 0.2
done
[ "$count" -ge "$PLANS" ] || { echo "index never reached $PLANS plans (got $count)" >&2; exit 1; }

sleep 10 # idle
ticks() { awk '{print $14 + $15}' "/proc/$PID/stat"; }
t0=$(ticks)
sleep 10
t1=$(ticks)
HZ=$(getconf CLK_TCK)
CPU=$(awk -v d=$((t1 - t0)) -v hz="$HZ" 'BEGIN{printf "%.2f", d / hz / 10 * 100}')
RSS_KB=$(awk '/VmRSS/ {print $2}' "/proc/$PID/status")
BIN_BYTES=$(stat -c %s "$BIN")

echo "---- plan-atlas resource summary ----"
echo "plans indexed : $count"
echo "idle RSS      : $((RSS_KB / 1024)) MB ($RSS_KB kB, limit $((RSS_LIMIT_KB / 1024)) MB)"
echo "idle CPU      : ${CPU}% over 10 s"
echo "binary size   : $(awk -v b="$BIN_BYTES" 'BEGIN{printf "%.1f", b/1048576}') MB ($BIN_BYTES bytes, limit $((BIN_LIMIT_BYTES / 1048576)) MB)"

fail=0
[ "$RSS_KB" -le "$RSS_LIMIT_KB" ] || { echo "FAIL: RSS over budget" >&2; fail=1; }
[ "$BIN_BYTES" -le "$BIN_LIMIT_BYTES" ] || { echo "FAIL: binary over budget" >&2; fail=1; }
exit $fail
