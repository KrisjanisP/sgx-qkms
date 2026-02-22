#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel)"
PORT="${PORT:-}"

# if PORT is an empty string, find a free port in 18080-18180
if [[ -z "${PORT}" ]]; then
  for candidate in $(seq 18080 18180); do
    if ! ss -ltn "( sport = :${candidate} )" | grep -q LISTEN; then
      PORT="${candidate}"
      break
    fi
  done
  if [[ -z "${PORT}" ]]; then
    echo "Could not find a free port in 18080-18180." >&2
    exit 1
  fi
# PORT must not be already in use by another process
elif ss -ltn "( sport = :${PORT} )" | grep -q LISTEN; then
  echo "Requested PORT=${PORT} is already in use." >&2
  exit 1
fi
ADDR="127.0.0.1:${PORT}"
RESPONSE_FILE="$(mktemp)"

# cleanup function that is triggered on exit, including ctrl+c
cleanup() {
  if [[ -n "${SERVER_PID:-}" ]] && kill -0 "${SERVER_PID}" 2>/dev/null; then
    kill "${SERVER_PID}" 2>/dev/null || true
    wait "${SERVER_PID}" 2>/dev/null || true
  fi
  rm -f "${RESPONSE_FILE}"
}
trap cleanup EXIT

# build the server
cd "${ROOT_DIR}"
cargo build >/dev/null

# start the server in the background
coproc SERVER_PROC { exec env QKMS_ADDR="${ADDR}" "${ROOT_DIR}/target/debug/sgx-qkms" 2>&1; }
SERVER_PID="${SERVER_PROC_PID}"

if ! timeout 15s grep -q "Server is active" <&"${SERVER_PROC[0]}"; then
  echo "Server did not print readiness message." >&2
  exit 1
fi
if ! kill -0 "${SERVER_PID}" 2>/dev/null; then
  echo "Server exited unexpectedly before request." >&2
  exit 1
fi

status_code="$(curl -sS -o "${RESPONSE_FILE}" -w "%{http_code}" "http://${ADDR}/api/v1/keys/test-sae/status")"
echo "GET /api/v1/keys/test-sae/status -> HTTP ${status_code}"

if [[ "${status_code}" != "503" ]]; then
  echo "Expected placeholder failure HTTP 503, got ${status_code}" >&2
  echo "Response body:" >&2
  cat "${RESPONSE_FILE}" >&2 || true
  exit 1
fi

echo "Placeholder error response verified."

