#!/usr/bin/env bash
initialize () {
  echo "###########################################################################"
  echo "# Valheim Server - $(date)"
  echo "###########################################################################"
}

log () {
  echo "[Valheim]: $1"
}

initialize
#
#export TEMP_LD_LIBRARY_PATH=${LD_LIBRARY_PATH}
#export LD_LIBRARY_PATH=/home/steam/valheim/linux64:${LD_LIBRARY_PATH}
export SteamAppId=892970
export PATH="/home/steam/odin:$PATH"


# Setting up server
if [ -f "/home/steam/valheim/valheim_server.x86_64" ]; then
  log "Server installed!"
else
  log "Installing Server..."
  odin install
fi

log "Herding Cats..."

odin start &

#export LD_LIBRARY_PATH=${TEMP_LD_LIBRARY_PATH}

cleanup() {
    log "Halting server! Received interrupt!"
    odin stop
    exit
}

trap cleanup INT TERM
trap cleanup EXIT

while :; do
    sleep 1s
done
