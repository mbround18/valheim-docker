#!/usr/bin/env bash

# Set up variables
# shellcheck disable=SC2155
export NAME="$(sed -e 's/^"//' -e 's/"$//' <<<"$NAME")"
export WORLD="$(sed -e 's/^"//' -e 's/"$//' <<<"$WORLD")"
export PASSWORD="$(sed -e 's/^"//' -e 's/"$//' <<<"$PASSWORD")"
export ODIN_CONFIG_FILE="${ODIN_CONFIG_FILE:-"${GAME_LOCATION}/config.json"}"
export ODIN_DISCORD_FILE="${ODIN_DISCORD_FILE:-"${GAME_LOCATION}/discord.json"}"

# Set up timezone
ln -snf "/usr/share/zoneinfo/$TZ" /etc/localtime && echo "$TZ" >/etc/timezone

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

check_version() {
  file="/home/steam/.version"
  sha="$(tail -n+1 $file | head -n1)"
  branch="$(tail -n+2 $file | head -n1)"
  repository="$(tail -n+3 $file | head -n1)"
  github_version="$(curl -s "https://api.github.com/repos/${repository}/branches/${branch//refs\/heads\//}" | jq '.commit.sha')"
  if [ -z "$github_version" ] || [ "$github_version" == "null" ]; then
    log "You must be in development. Good luck!"
  elif [ "${github_version//\"/}" != "${sha//\"/}" ]; then
    log "Hey you! It looks like there is an update on $repository for $branch"
    log "Please consider running \`docker-compose pull valheim\` or pull the image based on your use case"
  fi
}

clean_up() {
  echo "Safely shutting down..." >>/home/steam/output.log
  if [[ -n $CRON_PID ]]; then
    kill "$CRON_PID"
  fi
}

trap 'clean_up' INT TERM

setup_cron() {
  set -f
  CRON_NAME=$1
  SCRIPT_PATH="/home/steam/scripts/$2"
  CRON_SCHEDULE=$3

  PRESET_ENV="
  DEBUG_MODE=${DEBUG_MODE:=0}
  ODIN_CONFIG_FILE=${ODIN_CONFIG_FILE}
  ODIN_DISCORD_FILE=${ODIN_DISCORD_FILE}
  ODIN_WORKING_DIR=${ODIN_WORKING_DIR}
  SAVE_LOCATION=${SAVE_LOCATION}
  MODS_LOCATION=${MODS_LOCATION}
  GAME_LOCATION=${GAME_LOCATION}
  BACKUP_LOCATION=${BACKUP_LOCATION}

  ADDRESS=${ADDRESS}
  PORT=${PORT}
  PUBLIC=${PUBLIC}
  UPDATE_ON_STARTUP=${UPDATE_ON_STARTUP}

  WEBHOOK_URL=${WEBHOOK_URL:-""}
  WEBHOOK_STATUS_SUCCESSFUL=${WEBHOOK_STATUS_SUCCESSFUL:-"1"}
  WEBHOOK_STATUS_FAILED=${WEBHOOK_STATUS_FAILED:-"1"}

  AUTO_UPDATE=${AUTO_UPDATE}
  AUTO_UPDATE_PAUSE_WITH_PLAYERS=${AUTO_UPDATE_PAUSE_WITH_PLAYERS}

  AUTO_BACKUP=${AUTO_BACKUP}
  AUTO_BACKUP_REMOVE_OLD=${AUTO_BACKUP_REMOVE_OLD}
  AUTO_BACKUP_DAYS_TO_LIVE=${AUTO_BACKUP_DAYS_TO_LIVE}
  AUTO_BACKUP_ON_UPDATE=${AUTO_BACKUP_ON_UPDATE}
  AUTO_BACKUP_ON_SHUTDOWN=${AUTO_BACKUP_ON_SHUTDOWN}
  AUTO_BACKUP_PAUSE_WITH_NO_PLAYERS=${AUTO_BACKUP_PAUSE_WITH_NO_PLAYERS}
  "
  CRON_ENV="${PRESET_ENV} ${4}"
  CRON_ENV="$(echo "${CRON_ENV}" | tr '\n' " " )"
  LOG_LOCATION="/home/steam/valheim/logs/$CRON_NAME.out"
  mkdir -p "/home/steam/valheim/logs"
  [ -f "$LOG_LOCATION" ] && rm "$LOG_LOCATION"
  printf "%s %s /usr/sbin/gosu steam /bin/bash %s >> %s 2>&1" \
    "${CRON_SCHEDULE}" \
    "${CRON_ENV}" \
    "${SCRIPT_PATH}" \
    "${LOG_LOCATION}" \
    > "/etc/cron.d/${CRON_NAME}"
  echo "" >> "/etc/cron.d/${CRON_NAME}"
  # Give execution rights on the cron job
  chmod 0644 "/etc/cron.d/${CRON_NAME}"
  set +f
}

setup_filesystem() {
  log "Setting up file systems"
  STEAM_UID=${PUID:=1000}
  STEAM_GID=${PGID:=1000}

  # Save Files
  mkdir -p "${SAVE_LOCATION}"

  # Mod staging location
  mkdir -p "${MODS_LOCATION}"

  # Backups
  mkdir -p "${BACKUP_LOCATION}"

  # Valheim Server
  mkdir -p "${GAME_LOCATION}"
  mkdir -p "${GAME_LOCATION}/logs"
  chown -R ${STEAM_UID}:${STEAM_GID} "${GAME_LOCATION}"

  # Other
  mkdir -p /home/steam/scripts
  chown -R ${STEAM_UID}:${STEAM_GID} /home/steam/scripts
  chown -R ${STEAM_UID}:${STEAM_GID} /home/steam/
}

check_memory() {
  MEMORY=$(($(getconf _PHYS_PAGES) * $(getconf PAGE_SIZE) / (1024 * 1024)))
  MESSAGE="Your system has less than 2GB of ram!!\nValheim might not run on your system!!"
  if [ $MEMORY -lt 2000 ]; then
    line
    log "${MESSAGE^^}"
    line
    line
  fi
}

line
log "Valheim Server - $(date)"
log "Initializing your container..."
check_version
line
check_memory

log "Switching UID and GID"
# shellcheck disable=SC2086
log "$(usermod -u ${PUID} steam)"
# shellcheck disable=SC2086
log "$(groupmod -g ${PGID} steam)"

# Configure Cron
AUTO_UPDATE="${AUTO_UPDATE:=0}"
AUTO_BACKUP="${AUTO_BACKUP:=0}"

if [ "${AUTO_UPDATE}" -eq 1 ]; then
  log "Auto Update Enabled..."
  log "Auto Update Schedule: ${AUTO_UPDATE_SCHEDULE}"
  AUTO_UPDATE_SCHEDULE=$(echo "$AUTO_UPDATE_SCHEDULE" | tr -d '"')
  setup_cron \
    "auto-update" \
    "auto_update.sh" \
    "${AUTO_UPDATE_SCHEDULE}" \
    "AUTO_BACKUP_ON_UPDATE=${AUTO_BACKUP_ON_UPDATE:=0}"
fi

if [ "${AUTO_BACKUP}" -eq 1 ]; then
  log "Auto Backup Enabled..."
  log "Auto Backup Schedule: ${AUTO_BACKUP_SCHEDULE}"
  AUTO_BACKUP_SCHEDULE=$(echo "$AUTO_BACKUP_SCHEDULE" | tr -d '"')
  setup_cron \
    "auto-backup" \
    "auto_backup.sh" \
    "${AUTO_BACKUP_SCHEDULE}" \
    "AUTO_BACKUP_REMOVE_OLD=${AUTO_BACKUP_REMOVE_OLD} AUTO_BACKUP_DAYS_TO_LIVE=${AUTO_BACKUP_DAYS_TO_LIVE}"
fi

# Apply cron job
if [ "${AUTO_BACKUP}" -eq 1 ] || [ "${AUTO_UPDATE}" -eq 1 ]; then
  cat /etc/cron.d/* | crontab -
  /usr/sbin/cron -f &
  export CRON_PID=$!
fi

# Configure filesystem
setup_filesystem

# Launch as steam user :)
log "Navigating to steam home..."
cd /home/steam/valheim || exit 1

log "Launching as steam..."
exec gosu steam "$@"
