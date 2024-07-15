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

odin stop
sleep 5
odin start

line
