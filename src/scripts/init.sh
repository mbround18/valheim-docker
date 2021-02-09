#!/bin/sh

ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

echo "
###########################################################################
Valheim Server - $(date)

Initializing your container...
###########################################################################
"

log () {
  echo "[Valheim][root]: $1"
}


# shellcheck disable=SC2039
if [ "${EUID}" -ne 0 ]
  then log "Please run as root"
  exit
fi

log "Switching UID and GID"
# shellcheck disable=SC2086
usermod -u ${PUID} steam
# shellcheck disable=SC2086
groupmod -g ${PGID} steam


log "Setting up file systems"
STEAM_UID=${PUID:=1000}
STEAM_GID=${PGID:=1000}
mkdir -p /home/steam/valheim
chown -R ${STEAM_UID}:${STEAM_GID} /home/steam/valheim
mkdir -p /home/steam/scripts
chown -R ${STEAM_UID}:${STEAM_GID} /home/steam/scripts
mkdir -p /home/steam/valheim
echo "export PATH=\"/home/steam/.odin:$PATH\"" >> /home/steam/.bashrc
cp /home/steam/steamcmd/linux64/steamclient.so /home/steam/valheim
chown -R ${STEAM_UID}:${STEAM_GID} /home/steam/
chown -R ${STEAM_UID}:${STEAM_GID} /home/steam/valheim

# Launch run.sh with user steam (-p allow to keep env variables)
log "Launching as steam..."
cd /home/steam/valheim || exit 1;

PORT=$(echo "${PORT}" | tr -d '"')
NAME=$(echo "${NAME}" | tr -d '"')
WORLD=$(echo "${WORLD}" | tr -d '"')
PASSWORD=$(echo "${PASSWORD}" | tr -d '"')
printf "
PORT=%s
NAME=\"%s\"
WORLD=\"%s\"
PASSWORD=\"%s\"
" "${PORT}" "${NAME}" "${WORLD}" "${PASSWORD}" > /home/steam/.env

su -ps /bin/bash --login steam -c "/bin/bash /home/steam/scripts/entrypoint.sh"
