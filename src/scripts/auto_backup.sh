#!/usr/bin/env bash
# Cron uses blank env and does not pick up /usr/local/bin odin.files.
export PATH="/usr/local/bin:$PATH"
cd /home/steam/ || exit 1

log() {
  PREFIX="[Valheim][steam]"
  printf "%-16s: %s\n" "${PREFIX}" "$1"
}

file_name="$(date +"%Y%m%d-%H%M%S")-${1:-"backup"}.tar.gz"

if [ "${AUTO_BACKUP_PAUSE_WITH_NO_PLAYERS:=0}" -eq 1 ]; then
  export ADDRESS=${ADDRESS:="127.0.0.1:2457"}
  NUMBER_OF_PLAYERS=$(DEBUG_MODE=false odin status --address="${ADDRESS}" --json | jq -r '.players')
  if [ "${NUMBER_OF_PLAYERS}" -eq 0 ]; then
    log "Skipping backup, no players are online."
    exit 0
  fi
fi

log "Starting auto backup process..."
odin backup /home/steam/.config/unity3d/IronGate/Valheim "/home/steam/backups/${file_name}" || exit 1

if [ "${AUTO_BACKUP_REMOVE_OLD:=0}" -eq 1 ]; then
    find /home/steam/backups -mtime +${AUTO_BACKUP_DAYS_TO_LIVE:-5} -exec rm {} \;
fi

log "Backup process complete! Created ${file_name}"
