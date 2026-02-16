#!/usr/bin/env bash
set -euo pipefail

COMPOSE_FILE="${COMPOSE_FILE:-./docker-compose.dev.yml}"
SERVICE="${SERVICE:-valheim}"
WAIT_SECS="${WAIT_SECS:-120}"
POLL_SECS="${POLL_SECS:-2}"

cleanup() {
  docker compose -f "${COMPOSE_FILE}" down --remove-orphans >/dev/null 2>&1 || true
}
trap cleanup EXIT

compose_up() {
  local user_override="${1:-}"
  if [ -n "${user_override}" ]; then
    CONTAINER_USER="${user_override}" docker compose -f "${COMPOSE_FILE}" up -d --build
  else
    docker compose -f "${COMPOSE_FILE}" up -d --build
  fi
}

container_id() {
  docker compose -f "${COMPOSE_FILE}" ps -q "${SERVICE}"
}

tail_logs() {
  docker compose -f "${COMPOSE_FILE}" logs --no-color --tail=120 "${SERVICE}" || true
}

wait_for_running() {
  local label="$1"
  local deadline=$((SECONDS + WAIT_SECS))

  while [ "${SECONDS}" -lt "${deadline}" ]; do
    local cid
    cid="$(container_id)"
    if [ -z "${cid}" ]; then
      sleep "${POLL_SECS}"
      continue
    fi

    local status
    status="$(docker inspect -f '{{.State.Status}}' "${cid}")"

    if [ "${status}" = "running" ]; then
      local logs
      logs="$(tail_logs)"
      if echo "${logs}" | grep -q "sudo: a password is required"; then
        echo "FAIL (${label}): passworded sudo error detected"
        echo "${logs}"
        return 1
      fi
      if echo "${logs}" | grep -q "Launching server..."; then
        echo "PASS (${label}): container is running and startup reached launch"
        return 0
      fi
      # Still running and healthy enough for smoke; keep waiting briefly for startup marker.
    elif [ "${status}" = "exited" ] || [ "${status}" = "dead" ]; then
      echo "FAIL (${label}): container status=${status}"
      tail_logs
      return 1
    fi

    sleep "${POLL_SECS}"
  done

  echo "FAIL (${label}): timed out after ${WAIT_SECS}s"
  tail_logs
  return 1
}

run_case() {
  local label="$1"
  local user_override="${2:-}"

  echo "=== Smoke case: ${label} (CONTAINER_USER=${user_override:-default}) ==="
  docker compose -f "${COMPOSE_FILE}" down --remove-orphans >/dev/null 2>&1 || true
  compose_up "${user_override}"
  wait_for_running "${label}"
}

run_case "default-steam" ""
run_case "explicit-uid-gid" "1000:1000"

echo "All rootless smoke cases passed."
