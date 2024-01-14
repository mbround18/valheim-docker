#!/usr/bin/env bash

if [ -f "/home/steam/scripts/utils.sh" ]; then
  source "/home/steam/scripts/utils.sh"
fi


# Set up variables
# shellcheck disable=SC2155
export NAME="$(sed -e 's/^"//' -e 's/"$//' <<<"$NAME")"
export WORLD="$(sed -e 's/^"//' -e 's/"$//' <<<"$WORLD")"
export PASSWORD="$(sed -e 's/^"//' -e 's/"$//' <<<"$PASSWORD")"
export ODIN_CONFIG_FILE="${ODIN_CONFIG_FILE:-"${GAME_LOCATION}/config.json"}"
export ODIN_DISCORD_FILE="${ODIN_DISCORD_FILE:-"${GAME_LOCATION}/discord.json"}"

# Set up timezone
sudo ln -snf "/usr/share/zoneinfo/$TZ" /etc/localtime && echo "$TZ" | sudo tee -a /etc/timezone

clean_up() {
  echo "Safely shutting down..." >>/home/steam/output.log
  if [[ -n $CRON_PID ]]; then
    kill "$CRON_PID"
  fi
}

trap 'clean_up' INT TERM

setup_cron_env() {
  echo "Configuring Preset Env"
  echo "
    DEBUG_MODE=${DEBUG_MODE:=0}
    ODIN_CONFIG_FILE=${ODIN_CONFIG_FILE}
    ODIN_DISCORD_FILE=${ODIN_DISCORD_FILE}
    ODIN_WORKING_DIR=${ODIN_WORKING_DIR}
    SAVE_LOCATION=${SAVE_LOCATION}
    MODS_LOCATION=${MODS_LOCATION}
    GAME_LOCATION=${GAME_LOCATION}
    BACKUP_LOCATION=${BACKUP_LOCATION}

    NAME=${NAME}
    ADDRESS=${ADDRESS}
    PORT=${PORT}
    PUBLIC=${PUBLIC}
    ENABLE_CROSSPLAY=${ENABLE_CROSSPLAY:-"0"}
    UPDATE_ON_STARTUP=${UPDATE_ON_STARTUP}
    SERVER_EXTRA_LAUNCH_ARGS=${SERVER_EXTRA_LAUNCH_ARGS}
    PRESET=${PRESET}
    MODIFIERS=$(echo "${MODIFIERS}" | xargs echo -n | tr ' ' ',' | sed 's/,,/,/g')
    SET_KEY=${SET_KEY}

    WEBHOOK_URL=${WEBHOOK_URL:-""}
    WEBHOOK_STATUS_SUCCESSFUL=${WEBHOOK_STATUS_SUCCESSFUL:-"1"}
    WEBHOOK_STATUS_FAILED=${WEBHOOK_STATUS_FAILED:-"1"}
    WEBHOOK_STATUS_RUNNING=${WEBHOOK_STATUS_RUNNING:-"1"}
    WEBHOOK_INCLUDE_PUBLIC_IP=${WEBHOOK_INCLUDE_PUBLIC_IP:-"0"}

    AUTO_UPDATE=${AUTO_UPDATE}
    AUTO_UPDATE_PAUSE_WITH_PLAYERS=${AUTO_UPDATE_PAUSE_WITH_PLAYERS}

    AUTO_BACKUP=${AUTO_BACKUP}
    AUTO_BACKUP_NICE_LEVEL=${AUTO_BACKUP_NICE_LEVEL}
    AUTO_BACKUP_REMOVE_OLD=${AUTO_BACKUP_REMOVE_OLD}
    AUTO_BACKUP_DAYS_TO_LIVE=${AUTO_BACKUP_DAYS_TO_LIVE}
    AUTO_BACKUP_ON_UPDATE=${AUTO_BACKUP_ON_UPDATE}
    AUTO_BACKUP_ON_SHUTDOWN=${AUTO_BACKUP_ON_SHUTDOWN}
    AUTO_BACKUP_PAUSE_WITH_NO_PLAYERS=${AUTO_BACKUP_PAUSE_WITH_NO_PLAYERS}

    VALHEIM_PLUS_RELEASES_URL=\"${VALHEIM_PLUS_RELEASES_URL:-""}\"
    VALHEIM_PLUS_DOWNLOAD_URL=\"${VALHEIM_PLUS_DOWNLOAD_URL}\"

    BEPINEX_RELEASES_URL=\"${BEPINEX_RELEASES_URL:-"https://valheim.thunderstore.io/api/experimental/package/denikson/BepInExPack_Valheim/"}\"
    BEPINEX_DOWNLOAD_URL=\"${BEPINEX_DOWNLOAD_URL}\"

    BEPINEX_FULL_RELEASES_URL=\"${BEPINEX_FULL_RELEASES_URL:-""}\"
    " | \
    while read -r line; do
       if [[ "${line}" == *"="* ]]; then
         CONTENT="export ${line}"
         if ! grep -q -F "${CONTENT}" "/env.sh" && [[ ! "${CONTENT}" =~ =$ ]]; then
            echo "${CONTENT}" | sudo tee -a /env.sh
         fi
       fi
    done
    echo "Preset Env Configured"
}

setup_cron() {
  set -f
  CRON_NAME=$1
  SCRIPT_PATH="/home/steam/scripts/$2"
  CRON_SCHEDULE=$3
  LOG_LOCATION="/home/steam/valheim/logs/$CRON_NAME.out"
  mkdir -p "/home/steam/valheim/logs"
  [ -f "$LOG_LOCATION" ] && rm "$LOG_LOCATION"
  printf "%s %s /usr/sbin/gosu steam /bin/bash %s >> %s 2>&1" \
    "${CRON_SCHEDULE}" \
    "BASH_ENV=/env.sh" \
    "${SCRIPT_PATH}" \
    "${LOG_LOCATION}" \
    | sudo tee "/etc/cron.d/${CRON_NAME}"
  echo "" | sudo tee -a "/etc/cron.d/${CRON_NAME}"
  # Give execution rights on the cron job
  sudo chmod 0644 "/etc/cron.d/${CRON_NAME}"
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
  sudo chown -R "${STEAM_UID}":"${STEAM_GID}" "${GAME_LOCATION}"
  sudo chown -R "${STEAM_UID}":"${STEAM_GID}" "${GAME_LOCATION}"

  # Other
  mkdir -p /home/steam/scripts
  sudo chown -R "${STEAM_UID}":"${STEAM_GID}" /home/steam/scripts
  sudo chown -R "${STEAM_UID}":"${STEAM_GID}" /home/steam/

  # Enforce steam home
  sudo usermod -d /home/steam steam
  cd /home/steam || exit 1
}

check_memory() {
#  MEMORY=$(($(getconf _PHYS_PAGES) * $(getconf PAGE_SIZE) / (1024 * 1024)))
  MESSAGE="Your system has less than 2GB of ram!!\nValheim might not run on your system!!"

  # Get the total memory in GB
  total_memory=$(free -h | grep Mem: | awk '{print $2}' | tr -d GiM)

  # Check if total memory is less than 2GB
  if (( $(echo "$total_memory < 2" | bc -l) )); then
    line
    log "${MESSAGE^^}"
    line
    line
  else
    log "Total memory: ${total_memory}GB"
  fi
}

line
log "Valheim Server - $(date)"
log "Initializing your container..."
#check_version
check_memory
line

# Configure Cron
AUTO_UPDATE="${AUTO_UPDATE:=0}"
AUTO_BACKUP="${AUTO_BACKUP:=0}"
SCHEDULED_RESTART="${SCHEDULED_RESTART:=0}" 

setup_cron_env

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

if [ "${SCHEDULED_RESTART}" -eq 1 ]; then
  log "Scheduled Restarts Enabled..."
  log "Scheduled Restart Schedule: ${SCHEDULED_RESTART_SCHEDULE}"
  SCHEDULED_RESTART_SCHEDULE=$(echo "$SCHEDULED_RESTART_SCHEDULE" | tr -d '"')
  setup_cron \
    "scheduled-restart" \
    "scheduled_restart.sh" \
    "${SCHEDULED_RESTART_SCHEDULE}"
fi

# Apply cron job
if [ "${AUTO_BACKUP}" -eq 1 ] || [ "${AUTO_UPDATE}" -eq 1 ] || [ "${SCHEDULED_RESTART}" -eq 1 ]; then
  cat /etc/cron.d/* | crontab -
  sudo /usr/sbin/cron -f &
  export CRON_PID=$!
fi

# Configure filesystem
setup_filesystem

# Launch as steam user :)
log "Navigating to steam home..."
cd /home/steam/valheim || exit 1

log "Launching as server..."
. /home/steam/scripts/start_valheim.sh
