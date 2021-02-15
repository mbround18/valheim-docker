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

cleanup() {
    log "Halting server! Received interrupt!"
    odin stop
    if [ "${AUTO_BACKUP_ON_SHUTDOWN:=0}" -eq 1 ]; then
        log "Backup on shutdown triggered! Running backup tool..."
        /bin/bash /home/steam/scripts/auto_backup.sh "shutdown"
    fi
    if [[ -n $TAIL_PID ]];then
      kill $TAIL_PID
    fi
}

initialize "Installing Valheim via Odin..."

log "Variables loaded....."
log "
Port: ${PORT}
Name: ${NAME}
World: ${WORLD}
Public: ${PUBLIC}
Password: (REDACTED)
"

export SteamAppId=${APPID:-892970}

# Setting up server
log "Running Install..."
odin install || exit 1

log "Initializing Variables...."
odin configure || exit 1

trap 'cleanup' INT TERM

log "Herding Cats..."
log "Starting server..."

odin start || exit 1

initialize "
Valheim Server Started...

Keep an eye out for 'Game server connected' in the log!
(this indicates its online without any errors.)
" >> /home/steam/valheim/logs/output.log


log_names=("valheim_server.log" "valheim_server.err" "output.log" "auto-update.out" "auto-backup.out")
log_files=("${log_names[@]/#/\/home\/steam\/valheim\/logs/}")
touch "${log_files[@]}"
tail -F ${log_files[*]} &
export TAIL_PID=$!
wait $TAIL_PID
