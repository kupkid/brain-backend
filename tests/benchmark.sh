#!/usr/bin/env bash
set -euo pipefail

AGENT_BIN="${1:-target/debug/agent}"
RESULTS_FILE="tests/baseline.txt"
LOG_DIR="tests/logs"
mkdir -p "$LOG_DIR"

export COHERE_API_KEY="${COHERE_API_KEY:?COHERE_API_KEY required}"

run_scenario() {
    local name="$1"
    local prompt="$2"
    local log_file="$LOG_DIR/${name}.log"
    local rss_file="$LOG_DIR/${name}.rss"
    local data_dir="/tmp/brain-bench-${name}"
    local workspace_dir="${data_dir}/workspace"

    echo "=== Scenario: $name ==="
    rm -rf "$data_dir"
    mkdir -p "$data_dir/brain" "$workspace_dir"

    export BRAIN_DATA_DIR="$data_dir/brain"
    export BRAIN_WORKSPACE_DIR="$workspace_dir"
    export RUST_LOG=brain_backend=info

    # Background RSS monitor
    : > "$rss_file"
    (
        while true; do
            pid=$(pgrep -f "agent" 2>/dev/null | head -1 || true)
            if [ -n "$pid" ] && [ -f "/proc/$pid/status" ]; then
                rss=$(awk '/VmRSS/ {print $2}' "/proc/$pid/status" 2>/dev/null || echo "0")
                echo "$rss" >> "$rss_file"
            fi
            sleep 0.3
        done
    ) &
    local monitor_pid=$!

    local start_time=$(date +%s%N)
    echo "$prompt" | timeout 120 "$AGENT_BIN" > "$log_file" 2>&1 || true
    local end_time=$(date +%s%N)
    kill $monitor_pid 2>/dev/null || true

    local elapsed_ms=$(( (end_time - start_time) / 1000000 ))

    # Parse tokens from "(N tokens)" pattern
    local total_tokens=0
    total_tokens=$(sed -n 's/.*(\([0-9]*\) tokens).*/\1/p' "$log_file" | tail -1)
    total_tokens=${total_tokens:-0}

    # Count LLM requests
    local llm_calls=0
    if grep -q "LLM complete:" "$log_file" 2>/dev/null; then
        llm_calls=$(grep -c "LLM complete:" "$log_file")
    fi

    # Peak RSS in KB
    local peak_rss=0
    if [ -s "$rss_file" ]; then
        peak_rss=$(sort -n "$rss_file" | tail -1)
    fi

    echo "  Time: ${elapsed_ms}ms"
    echo "  Tokens: $total_tokens"
    echo "  LLM calls: $llm_calls"
    echo "  Peak RSS: ${peak_rss}KB"
    echo ""

    # Append to results
    echo "--- $name ---" >> "$RESULTS_FILE"
    echo "  Time: ${elapsed_ms}ms" >> "$RESULTS_FILE"
    echo "  Tokens: $total_tokens" >> "$RESULTS_FILE"
    echo "  LLM calls: $llm_calls" >> "$RESULTS_FILE"
    echo "  Peak RSS: ${peak_rss}KB" >> "$RESULTS_FILE"
    echo "" >> "$RESULTS_FILE"
}

# --- Main ---
echo "=== Brain Agent Baseline Benchmark ===" > "$RESULTS_FILE"
echo "Date: $(date -Iseconds)" >> "$RESULTS_FILE"
echo "" >> "$RESULTS_FILE"

# Scenario 1: Simple (hello.py) — 3 tool calls
run_scenario "simple" \
    "Create a file hello.py with content \"print('hello from agent')\", then run it with python3"

sleep 65

# Scenario 2: Medium (list + grep + count + write) — 4-5 tool calls
run_scenario "medium" \
    "List all files in the workspace with list_dir, then search for the word 'fn' in any .rs files with grep. Count how many .rs files there are and how many matches found. Write the result to result.txt"

sleep 65

# Scenario 3: Hard (5+ tools) — create dirs, write 3 files, list, read
run_scenario "hard" \
    "Create directories src/utils and src/models. Write src/utils/helpers.py with 'def add(a,b): return a+b'. Write src/models/data.py with 'DATA = [1,2,3]'. Write src/main.py that says 'from utils.helpers import add; from models.data import DATA; print(add(1,2))'. List the full tree with list_dir. Then read src/main.py back with read_file."

echo "=== Benchmark Complete ==="
cat "$RESULTS_FILE"
