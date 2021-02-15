#!/usr/bin/env bash

# Set up variables
# shellcheck disable=SC2155
export NAME="$(sed -e 's/^"//' -e 's/"$//' <<<"$NAME")"
export WORLD="$(sed -e 's/^"//' -e 's/"$//' <<<"$WORLD")"
export PASSWORD="$(sed -e 's/^"//' -e 's/"$//' <<<"$PASSWORD")"

# Set up timezone
ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ >/etc/timezone

# shellcheck disable=SC2039
if [ "${EUID}" -ne 0 ]; then
  log "Please run as root"
  exit
fi

log() {
  PREFIX="[Valheim][root]"
  printf "%-16s: %s\n" "${PREFIX}" "$1"
}

line() {
  log "###########################################################################"
}

clean_up() {
  echo "Safely shutting down..." >>/home/steam/output.log
  if [[ -n $CRON_PID ]]; then
    kill $CRON_PID
  fi
}

trap 'clean_up' INT TERM

setup_cron() {
  set -f
  log "Auto Update Enabled..."
  log "Schedule: ${AUTO_UPDATE_SCHEDULE}"
  AUTO_UPDATE_SCHEDULE=$(echo "$AUTO_UPDATE_SCHEDULE" | tr -d '"')
  printf "%s /usr/sbin/gosu steam /bin/bash /home/steam/scripts/auto_update.sh  2>&1 | tee -a /home/steam/valheim/valheim_server.out" "${AUTO_UPDATE_SCHEDULE}" >/etc/cron.d/auto-update
  echo "" >>/etc/cron.d/auto-update
  # Give execution rights on the cron job
  chmod 0644 /etc/cron.d/auto-update
  # Apply cron job
  crontab /etc/cron.d/auto-update
  set +f
  /usr/sbin/cron -f &
  export CRON_PID=$!
}

setup_filesystem() {
  log "Setting up file systems"
  STEAM_UID=${PUID:=1000}
  STEAM_GID=${PGID:=1000}
  mkdir -p /home/steam/valheim
  chown -R ${STEAM_UID}:${STEAM_GID} /home/steam/valheim
  mkdir -p /home/steam/scripts
  chown -R ${STEAM_UID}:${STEAM_GID} /home/steam/scripts
  mkdir -p /home/steam/valheim
  cp /home/steam/steamcmd/linux64/steamclient.so /home/steam/valheim
  chown -R ${STEAM_UID}:${STEAM_GID} /home/steam/
  chown -R ${STEAM_UID}:${STEAM_GID} /home/steam/valheim
}

line
log "Valheim Server - $(date)"
log "Initializing your container..."
line

log "Switching UID and GID"
# shellcheck disable=SC2086
log "$(usermod -u ${PUID} steam)"
# shellcheck disable=SC2086
log "$(groupmod -g ${PGID} steam)"

# Configure Cron
[ "${AUTO_UPDATE:=0}" -eq 1 ] && setup_cron

# Configure filesystem
setup_filesystem

# Launch as steam user :)
log "Launching as steam..."
cd /home/steam/valheim || exit 1
exec gosu steam "$@"
