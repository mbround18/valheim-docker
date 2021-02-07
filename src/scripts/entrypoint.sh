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
odin install

log "Herding Cats..."

odin start

cleanup() {
    log "Halting server! Received interrupt!"
    odin stop
    exit
}

trap cleanup INT TERM EXIT

tail -f /home/steam/valheim/output.log

while :; do
  sleep 1s
done
