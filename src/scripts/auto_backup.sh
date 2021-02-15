#!/usr/bin/env bash
# Cron uses blank env and does not pick up /usr/local/bin files.
export PATH="/usr/local/bin:$PATH"
cd /home/steam/ || exit 1

log() {
  PREFIX="[Valheim][steam]"
  printf "%-16s: %s\n" "${PREFIX}" "$1"
}

file_name="$(date +"%Y%m%d-%H%M%S")-${1:-"backup"}.tar.gz"

log "Starting auto backup process..."
odin backup /home/steam/.config/unity3d/IronGate/Valheim "/home/steam/backups/${file_name}" || exit 1

if [ "${AUTO_BACKUP_REMOVE_OLD:=0}" -eq 1 ]; then
    find /home/steam/backups/*.tar.gz -mtime +${AUTO_BACKUP_DAYS_TO_LIVE:-5} -exec rm {} \;
fi

log "Backup process complete! Created ${file_name}"
