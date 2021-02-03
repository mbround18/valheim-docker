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

export templdpath=$LD_LIBRARY_PATH
export LD_LIBRARY_PATH=/home/steam/valheim/linux64:$LD_LIBRARY_PATH
export SteamAppId=892970
export PATH="/home/steam/odin:$PATH"


# Setting up server
if [ "$(ls -A "/home/steam/valheim")" ]; then
  log "Server installed!"
else
  log "Installing Server..."
  odin install
fi

log "Herding Cats..."
export TERM=linux

odin start

export LD_LIBRARY_PATH=$templdpath

log "Server Started! :)"

cleanup() {
    log "Halting server! Received interrupt!"
    echo 1 > /home/steam/valheim/server_exit.drp
    exit
}

trap cleanup INT TERM

while :; do
    sleep 1s
done
