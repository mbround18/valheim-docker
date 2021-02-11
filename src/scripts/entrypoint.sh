#!/usr/bin/env bash
ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ >/etc/timezone

echo "
###########################################################################
Valheim Server - $(date)

Initializing your container...
###########################################################################
"

log() {
  echo "[Valheim][root]: $1"
}

# shellcheck disable=SC2039
if [ "${EUID}" -ne 0 ]; then
  log "Please run as root"
  exit
fi

log "Switching UID and GID"
# shellcheck disable=SC2086
usermod -u ${PUID} steam || echo "Looks like no changes were needed to the user!"
# shellcheck disable=SC2086
groupmod -g ${PGID} steam || echo "Looks like no changes were needed to the user!"

log "Setting up file systems"
STEAM_UID=${PUID:=1000}
STEAM_GID=${PGID:=1000}
mkdir -p /home/steam/valheim

echo "
# Load Valheim base directory,
cd /home/steam/valheim
" > /home/steam/.bashrc

chown -R ${STEAM_UID}:${STEAM_GID} /home/steam/valheim
mkdir -p /home/steam/scripts
chown -R ${STEAM_UID}:${STEAM_GID} /home/steam/scripts
mkdir -p /home/steam/valheim
echo "export PATH=\"/home/steam/.odin:$PATH\"" >>/home/steam/.bashrc
cp /home/steam/steamcmd/linux64/steamclient.so /home/steam/valheim
chown -R ${STEAM_UID}:${STEAM_GID} /home/steam/
chown -R ${STEAM_UID}:${STEAM_GID} /home/steam/valheim

# Launch run.sh with user steam (-p allow to keep env variables)
log "Launching as steam..."
cd /home/steam/valheim || exit 1

trap 'exec goso steam cd /home/steam/valheim && odin stop' INT TERM EXIT

exec gosu steam "$@"
