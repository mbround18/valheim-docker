#!/usr/bin/env bash
# Cron uses blank env and does not pick up /usr/local/bin files.
export PATH="/usr/local/bin:$PATH"
cd /home/steam/ || exit 1

if [ "$(whoami)" != "steam" ]; then
  echo "You must run this script as the steam user!"
  exit 1
fi

log() {
  PREFIX="[Valheim][steam]"
  printf "%-16s: %s\n" "${PREFIX}" "$1"
}

line() {
  log "###########################################################################"
}

line
log "Valheim Server - $(date) Backup Process"

if { [ "${PUBLIC:=0}" -eq 0 ] || [ "${ENABLE_CROSSPLAY:=0}" -eq 1 ]; } && [ "${AUTO_BACKUP_PAUSE_WITH_NO_PLAYERS:=0}" -eq 1 ]; then
  log "Woah, cannot pause backup process on a private or crossplay enabled server"
  log "This is because we cannot query your server via the Steam API"
else
  if [ "${AUTO_BACKUP_PAUSE_WITH_NO_PLAYERS:=0}" -eq 1 ]; then
    export ADDRESS=${ADDRESS:="127.0.0.1:2457"}
    NUMBER_OF_PLAYERS=$(DEBUG_MODE=false odin status --address="${ADDRESS}" --json | jq -r '.players')
    if [ "${NUMBER_OF_PLAYERS:=0}" -eq 0 ]; then
      log "Skipping backup, no players are online."
      exit 0
    fi
  fi
fi

log "Starting auto backup process..."

if [ "${AUTO_BACKUP_REMOVE_OLD:=0}" -eq 1 ]; then
  log "Removing old backups..."
  find /home/steam/backups -mtime +$((${AUTO_BACKUP_DAYS_TO_LIVE:-5} - 1)) -exec rm {} \;
fi

log "Creating backup..."
file_name="$(date +"%Y%m%d-%H%M%S")-${1:-"backup"}.tar.gz"

if [ -x "$(command -v nice)" ] && [ "${AUTO_BACKUP_NICE_LEVEL:=0}" -ge "1" ] && [ "${AUTO_BACKUP_NICE_LEVEL:=0}" -le "19" ]; then
  nice -n "${AUTO_BACKUP_NICE_LEVEL}" \
    odin backup \
    /home/steam/.config/unity3d/IronGate/Valheim \
    "/home/steam/backups/${file_name}" ||
    exit 1
else
  odin backup \
    /home/steam/.config/unity3d/IronGate/Valheim \
    "/home/steam/backups/${file_name}" ||
    exit 1
fi

log "Backup process complete! Created ${file_name}"
line
