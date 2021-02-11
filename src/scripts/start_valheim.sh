#!/usr/bin/env bash
cd /home/steam/valheim || exit 1
STEAM_UID=${PUID:=1000}
STEAM_GID=${PGID:=1000}

initialize () {
  echo "
###########################################################################
Valheim Server - $(date)
STEAM_UID ${STEAM_UID} - STEAM_GUID ${STEAM_GID}

$1

###########################################################################
  "
}

log () {
  echo "[Valheim][steam]: $1"
}

initialize "Installing Valheim via Odin..."

echo "
Variables loaded.....

Port: ${PORT}
Name: ${NAME}
World: ${WORLD}
Public: ${PUBLIC}
Password: (REDACTED)
Auto Update: ${AUTO_UPDATE}
"

export SteamAppId=892970
export PATH="/home/steam/.odin:$PATH"

# Setting up server
log "Running Install..."
odin install || exit 1

log "Herding Cats..."
log "Starting server..."

odin start || exit 1

trap 'cleanup' INT TERM EXIT

cleanup() {
    log "Halting server! Received interrupt!"
    odin stop
    exit
}

initialize "
Valheim Server Started...

Keep an eye out for 'Game server connected' in the log!
(this indicates its online without any errors.)
" >> /home/steam/valheim/output.log
tail -f /home/steam/valheim/output.log

while :; do
  sleep 1s
done
