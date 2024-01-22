#!/usr/bin/env bash
# Cron uses blank env and does not pick up /usr/local/bin files.
export PATH="/usr/local/bin:$PATH"

if [ "$(whoami)" != "steam" ]; then
  echo "You must run this script as the steam user!"
  exit 1
fi

log() {
  PREFIX="[Valheim][steam]"
  printf "%-16s: %s\n" "${PREFIX}" "$1"
}
line () {
  log "###########################################################################"
}

line
log "Valheim Server - $(date)"
cd /home/steam/valheim || exit 1

if odin update --check; then
    if [ "${PUBLIC:=0}" -eq 0 ] && [ "${AUTO_UPDATE_PAUSE_WITH_PLAYERS:=0}" -eq 1 ]; then
      log "Woah, cannot pause auto update using AUTO_UPDATE_PAUSE_WITH_PLAYERS on your server with PUBLIC=0"
      log "This is because we cannot query your server via the Steam API"
    else
      if [ "${AUTO_UPDATE_PAUSE_WITH_PLAYERS:=0}" -eq 1 ]; then
        export ADDRESS=${ADDRESS:="127.0.0.1:2457"}
        NUMBER_OF_PLAYERS=$(DEBUG_MODE=false odin status --address="${ADDRESS}" --json | jq -r '.players')
        if [ "${NUMBER_OF_PLAYERS:=0}" -gt 0 ]; then
          log "An update is available. Skipping update, while ${NUMBER_OF_PLAYERS} players online...."
          exit 0
        fi
      fi
    fi

    log "An update is available! Beginning update process..."

    # Store if the server is currently running
    ! pidof valheim_server.x86_64 > /dev/null
    SERVER_RUNNING=$?

    # Stop the server if it's running
    if [ "${SERVER_RUNNING}" -eq 1 ]; then
        odin stop || exit 1
    fi

    if [ "${AUTO_BACKUP_ON_UPDATE:=0}" -eq 1 ]; then
        /bin/bash /home/steam/scripts/auto_backup.sh "pre-update-backup"
    fi

    odin update || exit 1

    # Start the server if it was running before
    if [ "${SERVER_RUNNING}" -eq 1 ]; then
        odin start || exit 1
        line
        log "
        Finished updating and everything looks happy <3

        Check your output.log for 'Game server connected'
        "
    fi
else
    log "No update available"
fi

line
