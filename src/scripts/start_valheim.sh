#!/usr/bin/env bash
cd /home/steam/valheim || exit 1
STEAM_UID=${PUID:=1000}
STEAM_GID=${PGID:=1000}

log() {
  PREFIX="[Valheim][steam]"
  printf "%-16s: %s\n" "${PREFIX}" "$1"
}
line () {
  log "###########################################################################"
}

initialize () {
  line
  log "Valheim Server - $(date)"
  log "STEAM_UID ${STEAM_UID} - STEAM_GUID ${STEAM_GID}"
  log "$1"
  line
}

initialize "Installing Valheim via Odin..."
export PATH="/home/steam/.odin:$PATH"


log "Variables loaded....."
log "
Port: ${PORT}
Name: ${NAME}
World: ${WORLD}
Public: ${PUBLIC}
Password: (REDACTED)
"
export SteamAppId=892970

# Setting up server
log "Running Install..."
odin install || exit 1

log "Herding Cats..."
log "Starting server..."

odin start || exit 1

cleanup() {
    log "Halting server! Received interrupt!"
    odin stop
    if [[ -n $TAIL_PID ]];then
      kill $TAIL_PID
    fi
}

trap 'cleanup' INT TERM


initialize "
Valheim Server Started...

Keep an eye out for 'Game server connected' in the log!
(this indicates its online without any errors.)
" >> /home/steam/valheim/output.log

tail -f /home/steam/valheim/output.log &
export TAIL_PID=$!
wait $TAIL_PID
