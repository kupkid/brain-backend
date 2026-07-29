#!/usr/bin/env bash
set -euo pipefail

AGENT_BIN="${1:-target/release/agent}"
RESULTS_DIR="tests/results"
LOG_DIR="tests/logs"
mkdir -p "$RESULTS_DIR" "$LOG_DIR"
export PATH="$HOME/.local/bin:$PATH"

SIMPLE_PROMPT='Create a file hello.py with content print hello, then run it with python3'
HARD_PROMPT='Create directories src/utils and src/models. Write src/utils/helpers.py with a function add that takes a and b and returns a+b. Write src/models/data.py with DATA list containing 1,2,3. Write src/main.py that imports both and prints the function result. List the full tree with list_dir. Then read src/main.py.'

echo "model|scenario|tokens|llm_calls|time_ms" > "$RESULTS_DIR/results.csv"

run_one() {
    local model_key="$1"
    local scenario="$2"
    local prompt="$3"
    local log_file="$LOG_DIR/${model_key}_${scenario}.log"
    local data_dir="/tmp/brain-bench-${model_key}-${scenario}"
    local workspace_dir="${data_dir}/workspace"

    rm -rf "$data_dir"
    mkdir -p "$data_dir/brain" "$workspace_dir"

    export BRAIN_DATA_DIR="$data_dir/brain"
    export BRAIN_WORKSPACE_DIR="$workspace_dir"
    export RUST_LOG=brain_backend=warn

    local start_time=$(date +%s%N)
    echo "$prompt" | timeout 120 "$AGENT_BIN" > "$log_file" 2>&1 || true
    local end_time=$(date +%s%N)

    local elapsed_ms=$(( (end_time - start_time) / 1000000 ))
    local total_tokens
    total_tokens=$(sed -n 's/.*(\([0-9]*\) tokens).*/\1/p' "$log_file" | tail -1)
    total_tokens=${total_tokens:-0}
    local llm_calls=0
    if grep -q "LLM complete:" "$log_file" 2>/dev/null; then
        llm_calls=$(grep -c "LLM complete:" "$log_file")
    fi

    echo "${model_key}|${scenario}|${total_tokens}|${llm_calls}|${elapsed_ms}" >> "$RESULTS_DIR/results.csv"
    echo "  [${model_key}] ${scenario}: ${total_tokens} tok, ${llm_calls} calls, ${elapsed_ms}ms"
}

echo "=== Multi-Model Benchmark ==="
echo "Date: $(date -Iseconds)"
echo ""

# --- Cohere ---
if [ -n "${COHERE_API_KEY:-}" ]; then
    echo "--- Cohere command-a-plus ---"
    COHERE_API_KEY="$COHERE_API_KEY" run_one cohere simple "$SIMPLE_PROMPT"
    sleep 65
    COHERE_API_KEY="$COHERE_API_KEY" run_one cohere hard "$HARD_PROMPT"
    sleep 65
else
    echo "SKIP cohere: no key"
fi

# --- Mimo (Elysium) ---
if [ -n "${ELYSIUM_API_KEY:-}" ]; then
    echo "--- Mimo-v2.5 (Elysium) ---"
    LLM_PROVIDER=openai_compat LLM_API_KEY="$ELYSIUM_API_KEY" LLM_MODEL=xiaomi/mimo-v2.5 LLM_BASE_URL=https://ru-api.elysiumai.garden/v1 run_one mimo simple "$SIMPLE_PROMPT"
    sleep 10
    LLM_PROVIDER=openai_compat LLM_API_KEY="$ELYSIUM_API_KEY" LLM_MODEL=xiaomi/mimo-v2.5 LLM_BASE_URL=https://ru-api.elysiumai.garden/v1 run_one mimo hard "$HARD_PROMPT"
    sleep 10
else
    echo "SKIP mimo: no key"
fi

# --- Ling (OpenRouter) ---
if [ -n "${OPENROUTER_API_KEY:-}" ]; then
    echo "--- Ling-3.0-flash (OpenRouter) ---"
    LLM_PROVIDER=openai_compat LLM_API_KEY="$OPENROUTER_API_KEY" LLM_MODEL=inclusionai/ling-3.0-flash:free LLM_BASE_URL=https://openrouter.ai/api/v1 run_one ling simple "$SIMPLE_PROMPT"
    sleep 65
    LLM_PROVIDER=openai_compat LLM_API_KEY="$OPENROUTER_API_KEY" LLM_MODEL=inclusionai/ling-3.0-flash:free LLM_BASE_URL=https://openrouter.ai/api/v1 run_one ling hard "$HARD_PROMPT"
    sleep 65
else
    echo "SKIP ling: no key"
fi

# --- Nemotron (Nvidia) ---
if [ -n "${NVIDIA_API_KEY:-}" ]; then
    echo "--- Nemotron-ultra-550b (Nvidia) ---"
    LLM_PROVIDER=openai_compat LLM_API_KEY="$NVIDIA_API_KEY" LLM_MODEL=nvidia/nemotron-3-ultra-550b-a55b LLM_BASE_URL=https://integrate.api.nvidia.com/v1 run_one nemotron simple "$SIMPLE_PROMPT"
    sleep 10
    LLM_PROVIDER=openai_compat LLM_API_KEY="$NVIDIA_API_KEY" LLM_MODEL=nvidia/nemotron-3-ultra-550b-a55b LLM_BASE_URL=https://integrate.api.nvidia.com/v1 run_one nemotron hard "$HARD_PROMPT"
else
    echo "SKIP nemotron: no key"
fi

echo ""
echo "=== Results ==="
column -t -s '|' "$RESULTS_DIR/results.csv" 2>/dev/null || cat "$RESULTS_DIR/results.csv"
