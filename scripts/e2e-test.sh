#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
BINARY="$PROJECT_ROOT/target/debug/macagentwatch"
PASS=0
FAIL=0
TOTAL=0

GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[1;33m'
NC='\033[0m'

pass() {
    PASS=$((PASS + 1))
    echo -e "  ${GREEN}PASS${NC}: $1"
}

fail() {
    FAIL=$((FAIL + 1))
    echo -e "  ${RED}FAIL${NC}: $1"
    if [ -n "${2:-}" ]; then
        echo -e "        ${YELLOW}Detail${NC}: $2"
    fi
}

run_test() {
    local name="$1"
    TOTAL=$((TOTAL + 1))
    echo ""
    echo "--- Test $TOTAL: $name ---"
}

# =============================================================================
# Build
# =============================================================================
echo "=== Building project ==="
cd "$PROJECT_ROOT"
cargo build --workspace
echo ""

if [ ! -x "$BINARY" ]; then
    echo "ERROR: Binary not found at $BINARY"
    exit 1
fi

echo "=== Running E2E Tests ==="

# ---------------------------------------------------------------------------
# Test 1: CLI --help works
# ---------------------------------------------------------------------------
run_test "CLI --help works"
output=$("$BINARY" --help 2>&1) && rc=0 || rc=$?
if [ $rc -ne 0 ]; then
    fail "CLI --help works" "Exit code was $rc, expected 0"
elif echo "$output" | grep -qi "macagentwatch"; then
    pass "CLI --help works"
else
    fail "CLI --help works" "Output did not contain macagentwatch"
fi

# ---------------------------------------------------------------------------
# Test 2: CLI version subcommand
# ---------------------------------------------------------------------------
run_test "CLI version subcommand"
output=$("$BINARY" version 2>&1) && rc=0 || rc=$?
if echo "$output" | grep -q "MacAgentWatch"; then
    pass "CLI version subcommand"
else
    fail "CLI version subcommand" "Output did not contain MacAgentWatch"
fi

# ---------------------------------------------------------------------------
# Test 3: Analyze - Low risk command
# ---------------------------------------------------------------------------
run_test "Analyze - Low risk command"
output=$("$BINARY" analyze ls 2>&1) && rc=0 || rc=$?
if [ $rc -ne 0 ]; then
    fail "Analyze - Low risk command" "Exit code was $rc, expected 0"
elif echo "$output" | grep -q "LOW"; then
    pass "Analyze - Low risk command"
else
    fail "Analyze - Low risk command" "Output did not contain LOW"
fi

# ---------------------------------------------------------------------------
# Test 4: Analyze - Critical risk command
# ---------------------------------------------------------------------------
run_test "Analyze - Critical risk command"
output=$("$BINARY" analyze rm -- -rf / 2>&1) && rc=0 || rc=$?
if echo "$output" | grep -q "CRITICAL"; then
    pass "Analyze - Critical risk command"
else
    fail "Analyze - Critical risk command" "Output did not contain CRITICAL"
fi

# ---------------------------------------------------------------------------
# Test 5: Analyze - JSON format
# ---------------------------------------------------------------------------
run_test "Analyze - JSON format"
output=$("$BINARY" --format json analyze curl 2>&1) && rc=0 || rc=$?
if echo "$output" | python3 -m json.tool > /dev/null 2>&1; then
    pass "Analyze - JSON format"
else
    fail "Analyze - JSON format" "Output was not valid JSON: $output"
fi

# ---------------------------------------------------------------------------
# Test 6: Wrapper - simple echo
# ---------------------------------------------------------------------------
run_test "Wrapper - simple echo"
output=$("$BINARY" --headless -- echo hello 2>&1) && rc=0 || rc=$?
if [ $rc -ne 0 ]; then
    fail "Wrapper - simple echo" "Exit code was $rc, expected 0"
elif echo "$output" | grep -q "hello"; then
    pass "Wrapper - simple echo"
else
    fail "Wrapper - simple echo" "Output did not contain hello"
fi

# ---------------------------------------------------------------------------
# Test 7: Wrapper - exit code propagation
# ---------------------------------------------------------------------------
run_test "Wrapper - exit code propagation"
"$BINARY" --headless -- false > /dev/null 2>&1 && rc=0 || rc=$?
if [ $rc -ne 0 ]; then
    pass "Wrapper - exit code propagation"
else
    fail "Wrapper - exit code propagation" "Exit code was 0, expected non-zero"
fi

# ---------------------------------------------------------------------------
# Test 8: Log directory creation
# ---------------------------------------------------------------------------
run_test "Log directory creation"
TMPDIR_E2E=$(mktemp -d)
LOG_DIR="$TMPDIR_E2E/e2e-logs"
"$BINARY" --headless --log-dir "$LOG_DIR" -- echo test > /dev/null 2>&1 || true
if [ -d "$LOG_DIR" ]; then
    pass "Log directory creation"
else
    fail "Log directory creation" "Directory $LOG_DIR was not created"
fi
rm -rf "$TMPDIR_E2E"

# ---------------------------------------------------------------------------
# Test 9: Config file option
# ---------------------------------------------------------------------------
run_test "Config file option"
TMPCONFIG=$(mktemp /tmp/e2e-config-XXXXXX.toml)
cat > "$TMPCONFIG" <<TOML
[monitoring]
watch_paths = []
TOML
"$BINARY" --config "$TMPCONFIG" --headless -- echo ok > /dev/null 2>&1 && rc=0 || rc=$?
rm -f "$TMPCONFIG"
if [ $rc -eq 0 ]; then
    pass "Config file option"
else
    fail "Config file option" "Exit code was $rc, expected 0"
fi

# ---------------------------------------------------------------------------
# Test 10: No-color and no-timestamps options
# ---------------------------------------------------------------------------
run_test "No-color and no-timestamps options"
"$BINARY" --no-color --no-timestamps --headless -- echo test > /dev/null 2>&1 && rc=0 || rc=$?
if [ $rc -eq 0 ]; then
    pass "No-color and no-timestamps options"
else
    fail "No-color and no-timestamps options" "Exit code was $rc, expected non-zero"
fi

# =============================================================================
# Summary
# =============================================================================
echo ""
echo "=== E2E Test Results ==="
echo "Total: $TOTAL | Passed: $PASS | Failed: $FAIL"

if [ "$FAIL" -gt 0 ]; then
    exit 1
fi

echo ""
echo "All tests passed!"
