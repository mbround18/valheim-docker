#!/usr/bin/env bash
# End-to-end test of a built valheim image in a real container.
#
# It downloads the Valheim dedicated server (~2 GB) on the first boot, so it is not part
# of the default CI build. Run it with `make e2e-container IMAGE=...`, or the
# "Container E2E" workflow (manual, or a PR labelled `e2e`).
#
#   A. Boot as the documented user 1000:1000 with sudo blocked -> the server connects.
#   B. Odin's player presence, read through Huginn's /players and /metrics: join, death
#      (0:0), respawn, a second player with a negative peer id, leave, a corrupt
#      player.list (Huginn must keep answering), and the atomic rewrite.
#   C. A restart clears player.list.
#   D. Legacy user 111:1000 on a 111-owned volume still boots. That needs the
#      group-writable steamcmd dirs and the steam-legacy passwd entry; without a passwd
#      entry the Valheim server segfaults at startup.
#
# Usage: .github/scripts/e2e-container.sh <image>
#   WORKDIR=<dir>  use this directory for the volumes (default: a fresh temp dir)
#   KEEP=1         keep WORKDIR afterwards, e.g. to reuse the downloaded server
set -uo pipefail

IMAGE="${1:?usage: $0 <image>}"
NAME="valheim-e2e-$$"
KEEP="${KEEP:-0}"
if [ -n "${WORKDIR:-}" ]; then
  mkdir -p "${WORKDIR}"
else
  WORKDIR="$(mktemp -d)"
fi
SERVER="${WORKDIR}/server" SAVES="${WORKDIR}/saves" BACKUPS="${WORKDIR}/backups"
LOG="${SERVER}/logs/valheim_server.log"
mkdir -p "${SERVER}" "${SAVES}" "${BACKUPS}"

pass=0
fail=0
ok() { echo "PASS $*"; pass=$((pass + 1)); }
bad() { echo "FAIL $*"; fail=$((fail + 1)); }
check() {
  local label="$1"
  shift
  if "$@"; then ok "${label}"; else bad "${label}"; fi
}
quiet() { "$@" >/dev/null 2>&1; }

# File operations the host user may not be allowed to do (the volumes end up owned by
# uid 1000 or 111, and CI runners are neither), done as root in a throwaway container.
as_root() {
  docker run --rm --user 0:0 -v "${WORKDIR}:/w" --entrypoint sh "${IMAGE}" -c "$1"
}

stop() { docker rm -f "${NAME}" >/dev/null 2>&1 || true; }

finish() {
  stop
  if [ "${KEEP}" = "1" ]; then
    echo "Kept ${WORKDIR}"
  else
    as_root 'rm -rf /w/server /w/saves /w/backups' >/dev/null 2>&1
    rmdir "${WORKDIR}" 2>/dev/null || true
  fi
}
trap finish EXIT

start() {
  local user="$1"
  stop
  as_root 'rm -f /w/server/logs/valheim_server.log'
  docker run -d --name "${NAME}" --user "${user}" --security-opt no-new-privileges \
    -e NAME=ValheimE2E -e PASSWORD=valheime2e1 -e PUBLIC=0 -e TYPE=Vanilla -e HTTP_PORT=3000 \
    -p 127.0.0.1::3000 \
    -v "${SERVER}:/home/steam/valheim" \
    -v "${SAVES}:/home/steam/.config/unity3d/IronGate/Valheim" \
    -v "${BACKUPS}:/home/steam/backups" \
    "${IMAGE}" >/dev/null
}

container_logs() { docker logs "${NAME}" 2>&1 | sed 's/\x1b\[[0-9;]*m//g'; }

wait_connected() {
  local deadline=$((SECONDS + $1))
  while [ ${SECONDS} -lt ${deadline} ]; do
    if [ "$(docker inspect -f '{{.State.Status}}' "${NAME}" 2>/dev/null)" != running ]; then
      echo "  container stopped:"
      container_logs | tail -30
      return 1
    fi
    if grep -aq 'Caught fatal signal' "${LOG}" 2>/dev/null; then
      echo "  valheim_server crashed:"
      grep -a -A3 'Caught fatal signal' "${LOG}"
      return 1
    fi
    grep -aq 'Game server connected' "${LOG}" 2>/dev/null && return 0
    sleep 5
  done
  echo "  timed out after $1s:"
  container_logs | tail -30
  return 1
}

url() { echo "http://$(docker port "${NAME}" 3000/tcp | head -1)"; }
players() { curl -fsS "$(url)/players"; }
names() { players | jq -c '.names'; }
metrics() { curl -fsS "$(url)/metrics"; }
in_container() { docker exec "${NAME}" sh -c "$1"; }
# Appends a timestamped line to the server log the way Valheim writes it, then gives
# `odin logs --watch` a moment to process it.
emit() {
  in_container "echo \"\$(date +'%m/%d/%Y %H:%M:%S'): $1\" >> /home/steam/valheim/logs/valheim_server.log"
  sleep 2
}
player_list() { in_container 'cat /home/steam/.config/unity3d/IronGate/Valheim/player.list'; }

echo "Image:   ${IMAGE}"
echo "Workdir: ${WORKDIR}"

echo "=== A. documented user 1000:1000, sudo blocked"
as_root 'chown -R 1000:1000 /w/server /w/saves /w/backups'
start 1000:1000
check "A server connects as 1000:1000" wait_connected 1800
check "A preflight passed" sh -c "! docker logs '${NAME}' 2>&1 | grep -q 'Preflight write check failed'"
# Huginn starts before the server; give it a moment to answer.
for _ in $(seq 1 12); do quiet players && break; sleep 5; done

echo "=== B. player presence through Huginn"
check "B /players answers" quiet players
check "B nobody online yet" test "$(names)" = "[]"

emit "Got character ZDOID from Viking : 2130425389:1"
check "B join -> [Viking]" test "$(names)" = '["Viking"]'
joined_at="$(players | jq '.sessions[0].joined_at')"
check "B session has joined_at" test "${joined_at:-0}" -gt 0
check "B /metrics has valheim_player_online for Viking" \
  sh -c "curl -fsS '$(url)/metrics' | grep -q 'valheim_player_online{player=\"Viking\"} 1'"

emit "Got character ZDOID from Viking : 0:0"
emit "Got character ZDOID from Viking : 2130425389:68"
check "B death + respawn keeps one entry" test "$(names)" = '["Viking"]'
check "B respawn keeps joined_at" test "$(players | jq '.sessions[0].joined_at')" = "${joined_at}"

emit "Got character ZDOID from Sir Lance : -99:3"
check "B second player with a negative peer id" test "$(names)" = '["Viking","Sir Lance"]'

emit "Destroying abandoned non persistent zdo 2130425389:1204 owner 2130425389"
check "B leave removes only Viking" test "$(names)" = '["Sir Lance"]'
check "B /metrics drops Viking" sh -c "! curl -fsS '$(url)/metrics' | grep -q 'player=\"Viking\"'"

in_container "printf '{\"players\": [{\"id\": 1, \"zdo_' > /home/steam/.config/unity3d/IronGate/Valheim/player.list"
check "B corrupt player.list: /players still answers" quiet players
check "B corrupt player.list: /metrics still answers" quiet metrics

emit "Got character ZDOID from Viking : 2130425389:1"
check "B next event rewrites a valid list" test "$(player_list | jq -c '[.players[].name]')" = '["Viking"]'
check "B no temp file left behind" \
  in_container 'test ! -e /home/steam/.config/unity3d/IronGate/Valheim/player.list.tmp'

echo "=== C. restart clears presence"
start 1000:1000
check "C server reconnects" wait_connected 900
check "C player.list cleared by odin start" test "$(player_list | jq -c '.players')" = "[]"
for _ in $(seq 1 12); do quiet players && break; sleep 5; done
check "C /players empty after restart" test "$(names)" = "[]"

echo "=== D. legacy user 111:1000 on a 111-owned volume"
stop
as_root 'chown -R 111:1000 /w/server /w/saves /w/backups'
start 111:1000
check "D server connects as legacy 111:1000" wait_connected 900
check "D runtime uid is 111" sh -c "docker logs '${NAME}' 2>&1 | grep -q 'Runtime uid: 111'"

echo
echo "RESULT: ${pass} passed, ${fail} failed"
[ "${fail}" -eq 0 ]
