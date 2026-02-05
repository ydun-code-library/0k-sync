#!/usr/bin/env bash
# Docker validation tests for 0k-sync
# TDD: Write tests FIRST, then build Dockerfiles to make them pass.
#
# Usage: ./tests/docker-validate.sh
# Run from workspace root.

set -euo pipefail

RELAY_IMAGE="0k-sync-relay:test"
CLI_IMAGE="0k-sync-cli:test"
RELAY_CONTAINER="relay-validate-$$"
PASS=0
FAIL=0

cleanup() {
    docker rm -f "$RELAY_CONTAINER" 2>/dev/null || true
    docker volume rm -f "relay-validate-data-$$" 2>/dev/null || true
}
trap cleanup EXIT

pass() {
    echo "  PASS: $1"
    PASS=$((PASS + 1))
}

fail() {
    echo "  FAIL: $1"
    FAIL=$((FAIL + 1))
}

echo "=== 0k-sync Docker Validation ==="
echo ""

# ---------- Test 1: Relay image builds ----------
echo "[1/8] Relay image builds"
if docker build -t "$RELAY_IMAGE" -f Dockerfile . >/dev/null 2>&1; then
    pass "Relay image builds successfully"
else
    fail "Relay image failed to build"
fi

# ---------- Test 2: Runs as non-root user ----------
echo "[2/8] Container runs as non-root user"
USER_OUTPUT=$(docker run --rm "$RELAY_IMAGE" whoami 2>/dev/null || echo "ERROR")
if [ "$USER_OUTPUT" = "relay" ]; then
    pass "Runs as user 'relay'"
else
    fail "Expected user 'relay', got '$USER_OUTPUT'"
fi

# ---------- Test 3: Health endpoint responds ----------
echo "[3/8] Health endpoint responds"
docker run -d --name "$RELAY_CONTAINER" "$RELAY_IMAGE" >/dev/null 2>&1 || true
HEALTH_OK=false
for i in $(seq 1 15); do
    if docker exec "$RELAY_CONTAINER" curl -sf http://localhost:8080/health >/dev/null 2>&1; then
        HEALTH_OK=true
        break
    fi
    sleep 1
done
if [ "$HEALTH_OK" = true ]; then
    pass "Health endpoint responds within 15s"
else
    fail "Health endpoint did not respond within 15s"
fi

# ---------- Test 4: Graceful shutdown on SIGINT ----------
echo "[4/8] Graceful shutdown on SIGINT"
docker kill --signal=SIGINT "$RELAY_CONTAINER" >/dev/null 2>&1 || true
sleep 3
EXIT_CODE=$(docker inspect "$RELAY_CONTAINER" --format='{{.State.ExitCode}}' 2>/dev/null || echo "999")
if [ "$EXIT_CODE" = "0" ]; then
    pass "Graceful shutdown (exit code 0 on SIGINT)"
else
    fail "Shutdown issue: exit code=$EXIT_CODE (expected 0)"
fi
docker rm -f "$RELAY_CONTAINER" >/dev/null 2>&1 || true

# ---------- Test 5: /data volume is writable ----------
echo "[5/8] /data volume is writable by relay user"
WRITE_OUTPUT=$(docker run --rm -v "relay-validate-data-$$:/data" "$RELAY_IMAGE" \
    sh -c 'touch /data/test-write && echo "OK" && rm /data/test-write' 2>/dev/null || echo "ERROR")
if [ "$WRITE_OUTPUT" = "OK" ]; then
    pass "/data volume writable by relay user"
else
    fail "/data volume not writable: $WRITE_OUTPUT"
fi

# ---------- Test 6: CLI image builds ----------
echo "[6/8] CLI image builds"
if docker build -t "$CLI_IMAGE" -f tests/chaos/Dockerfile.cli . >/dev/null 2>&1; then
    pass "CLI image builds successfully"
else
    fail "CLI image failed to build"
fi

# ---------- Test 7: CLI --help shows usage ----------
echo "[7/8] CLI --help shows usage"
CLI_OUTPUT=$(docker run --rm "$CLI_IMAGE" --help 2>&1 || echo "")
if echo "$CLI_OUTPUT" | grep -qi "usage\|Usage\|USAGE\|sync-cli\|testing"; then
    pass "CLI --help shows usage output"
else
    fail "CLI --help did not show expected output"
fi

# ---------- Test 8: docker-compose config validates ----------
echo "[8/8] docker-compose config validates"
if docker compose -f tests/chaos/docker-compose.chaos.yml config >/dev/null 2>&1; then
    pass "docker-compose config is valid"
else
    fail "docker-compose config validation failed"
fi

# ---------- Summary ----------
echo ""
echo "=== Results: $PASS passed, $FAIL failed ==="
if [ "$FAIL" -gt 0 ]; then
    exit 1
fi
echo "All tests passed."
