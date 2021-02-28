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

initialize "Installing Valheim via $(odin --version)..."
log "Variables loaded....."
log "Port: ${PORT}"
log "Name: ${NAME}"
log "World: ${WORLD}"
log "Public: ${PUBLIC}"
log "Password: (REDACTED)"
export SteamAppId=${APPID:-892970}

# Setting up server
log "Running Install..."
if [ ! -f "./valheim_server.x86_64" ] || \
    [ "${FORCE_INSTALL:-0}" -eq 1 ]; then
    odin install || exit 1
elif [ "${UPDATE_ON_STARTUP:-1}" -eq 1 ]; then
    log "Attempting to update before launching the server!"
    if [ "${AUTO_BACKUP_ON_UPDATE:=0}" -eq 1 ]; then
        /bin/bash /home/steam/scripts/auto_backup.sh "pre-update-backup"
    fi

    log "Installing Updates..."
    odin install || exit 1
else
    log "Skipping install process, looks like valheim_server is already installed :)"
fi
cp /home/steam/steamcmd/linux64/steamclient.so /home/steam/valheim/linux64/


# Setting up server
log "Initializing Variables...."
odin configure || exit 1


# Setting up script traps
trap 'cleanup' INT TERM

# Starting server
log "Starting server..."
odin start || exit 1

sleep 2

# Initializing all logs
log "Herding Graydwarfs..."
log_names=("valheim_server.log" "valheim_server.err" "output.log" "auto-update.out" "auto-backup.out")
log_files=("${log_names[@]/#/\/home\/steam\/valheim\/logs/}")
touch "${log_files[@]}" # Destroy logs on start up, this can be changed later to roll logs or archive them.
tail -F ${log_files[*]} &
export TAIL_PID=$!
# Waiting for logs.
wait $TAIL_PID
