#!/usr/bin/env bash
# Hermes Performance Benchmark Suite
# Compares startup time, memory usage, and help latency
set -euo pipefmt

HERMES_BIN="${HERMES_BIN:-./target/release/hermes}"
ITERATIONS="${ITERATIONS:-10}"
RESULTS_DIR="benchmarks/results"
TIMESTAMP=$(date +%Y%m%d-%H%M%S)

mkdir -p "$RESULTS_DIR"

echo "=== Hermes Benchmark Suite ==="
echo "Binary: $HERMES_BIN"
echo "Iterations: $ITERATIONS"
echo "Date: $TIMESTAMP"
echo ""

# 1. Startup time (cold)
echo "--- Startup Time (cold) ---"
STARTUP_FILE="$RESULTS_DIR/startup-$TIMESTAMP.csv"
echo "run,duration_ms" > "$STARTUP_FILE"

for i in $(seq 1 "$ITERATIONS"); do
    # Clear disk cache hint (Linux only)
    if [ -f /proc/sys/vm/drop_caches ]; then
        echo 3 | sudo tee /proc/sys/vm/drop_caches > /dev/null 2>&1 || true
    fi
    start=$(date +%s%N)
    "$HERMES_BIN" --version > /dev/null 2>&1
    end=$(date +%s%N)
    ms=$(( (end - start) / 1000000 ))
    echo "$i,$ms" >> "$STARTUP_FILE"
    echo "  Run $i: ${ms}ms"
done

# 2. Help latency
echo ""
echo "--- Help Command Latency ---"
HELP_FILE="$RESULTS_DIR/help-$TIMESTAMP.csv"
echo "run,duration_ms" > "$HELP_FILE"

for i in $(seq 1 "$ITERATIONS"); do
    start=$(date +%s%N)
    "$HERMES_BIN" --help > /dev/null 2>&1
    end=$(date +%s%N)
    ms=$(( (end - start) / 1000000 ))
    echo "$i,$ms" >> "$HELP_FILE"
    echo "  Run $i: ${ms}ms"
done

# 3. Memory usage (RSS in KB)
echo ""
echo "--- Peak Memory Usage ---"
if command -v /usr/bin/time > /dev/null 2>&1; then
    MEM=$(/usr/bin/time -v "$HERMES_BIN" --version 2>&1 | grep "Maximum resident" | awk '{print $6}')
    echo "  Peak RSS: ${MEM}KB"
else
    echo "  /usr/bin/time not available, skipping memory benchmark"
fi

# 4. Binary size
echo ""
echo "--- Binary Size ---"
if [ -f "$HERMES_BIN" ]; then
    SIZE=$(du -k "$HERMES_BIN" | awk '{print $1}')
    echo "  Size: ${SIZE}KB"
fi

# Summary
echo ""
echo "=== Summary ==="
echo "Results saved to $RESULTS_DIR/"
echo "Startup data: $STARTUP_FILE"
echo "Help data: $HELP_FILE"

# Compute averages
if command -v awk > /dev/null 2>&1; then
    AVG_STARTUP=$(awk -F, 'NR>1{sum+=$2;count++}END{print sum/count}' "$STARTUP_FILE")
    AVG_HELP=$(awk -F, 'NR>1{sum+=$2;count++}END{print sum/count}' "$HELP_FILE")
    echo "Avg startup: ${AVG_STARTUP}ms"
    echo "Avg help: ${AVG_HELP}ms"
fi
