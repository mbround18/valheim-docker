#!/usr/bin/env bash
# Cron uses blank env and does not pick up /usr/local/bin files.
export PATH="/usr/local/bin:$PATH"

log() {
  PREFIX="[Valheim][steam]"
  printf "%-16s: %s\n" "${PREFIX}" "$1"
}
line () {
  log "###########################################################################"
}

line
log "Valheim Server - $(date)"
log "Starting auto update..."
log "
Port: ${PORT}
Name: ${NAME}
World: ${WORLD}
Public: ${PUBLIC}
Password: (REDACTED)
"
line


cd /home/steam/valheim || exit 1
log "Stopping server..."
odin stop || exit 1
log "Installing Updates..."
odin install || exit 1
log "Starting server..."
odin start || exit 1
line
log "
Everything looks happy <3

Check your output.log for 'Game server connected'
"
line


