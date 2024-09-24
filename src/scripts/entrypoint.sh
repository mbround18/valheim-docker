#!/usr/bin/env bash

# Exit immediately if a command exits with a non-zero status
set -e

export HOME="/home/steam"
export GAME_LOCATION=${GAME_LOCATION:-"${HOME}/valheim"}
export SAVE_LOCATION=${SAVE_LOCATION:-"${GAME_LOCATION}/saves"}
export MODS_LOCATION=${MODS_LOCATION:-"${GAME_LOCATION}/BepInEx/plugins"}
export BACKUP_LOCATION=${BACKUP_LOCATION:-"${GAME_LOCATION}/backups"}
export CRON_LOCATION="${HOME}/cron.d"
export LOG_LOCATION="${GAME_LOCATION}/logs"

# Logging function to prepend timestamps to log messages
log() {
  echo "$(date) - $*"
}

# Function to check and log the current user and steam user's ID and group ID
check_user_and_group() {
  whoami
  id -u steam
  id -g steam
}

# Function to set up the environment, including sourcing utility scripts and setting environment variables
setup_environment() {
  if [ -f "/home/steam/scripts/utils.sh" ]; then
    source "/home/steam/scripts/utils.sh"
  fi

  export NAME
  NAME=$(sed -e 's/^"//' -e 's/"$//' <<<"$NAME")
  export WORLD
  WORLD=$(sed -e 's/^"//' -e 's/"$//' <<<"$WORLD")
  export PASSWORD
  PASSWORD=$(sed -e 's/^"//' -e 's/"$//' <<<"$PASSWORD")
  export ODIN_CONFIG_FILE="${ODIN_CONFIG_FILE:-"${GAME_LOCATION}/config.json"}"
  export ODIN_DISCORD_FILE="${ODIN_DISCORD_FILE:-"${GAME_LOCATION}/discord.json"}"

  # Set timezone
  sudo ln -snf "/usr/share/zoneinfo/$TZ" /etc/localtime
  echo "$TZ" | sudo tee -a /etc/timezone
}

# Function to safely shut down the server and terminate cron jobs
clean_up() {
  log "Safely shutting down..."
  if [[ -n $CRON_PID ]]; then
    kill "$CRON_PID"
  fi
}

# Trap signals for safe shutdown
trap clean_up INT TERM

# Function to set up environment variables for cron jobs
setup_cron_env() {
  log "Configuring Preset Env"
  # shellcheck disable=SC2054
  env_vars=(
    "DEBUG_MODE"
    "ODIN_CONFIG_FILE"
    "ODIN_DISCORD_FILE"
    "ODIN_WORKING_DIR"
    "SAVE_LOCATION"
    "MODS_LOCATION"
    "GAME_LOCATION"
    "BACKUP_LOCATION"
    "NAME"
    "ADDRESS"
    "PORT"
    "PUBLIC"
    "ENABLE_CROSSPLAY"
    "UPDATE_ON_STARTUP"
    "SERVER_EXTRA_LAUNCH_ARGS"
    "PRESET"
    "MODIFIERS"
    "SET_KEY"
    "WEBHOOK_URL"
    "WEBHOOK_STATUS_SUCCESSFUL"
    "WEBHOOK_STATUS_FAILED"
    "WEBHOOK_STATUS_RUNNING"
    "WEBHOOK_INCLUDE_PUBLIC_IP"
    "AUTO_UPDATE"
    "AUTO_UPDATE_PAUSE_WITH_PLAYERS"
    "AUTO_BACKUP"
    "AUTO_BACKUP_NICE_LEVEL"
    "AUTO_BACKUP_REMOVE_OLD"
    "AUTO_BACKUP_DAYS_TO_LIVE"
    "AUTO_BACKUP_ON_UPDATE"
    "AUTO_BACKUP_ON_SHUTDOWN"
    "AUTO_BACKUP_PAUSE_WITH_NO_PLAYERS"
    "AUTO_BACKUP_SCHEDULE"
    "VALHEIM_PLUS_RELEASES_URL"
    "VALHEIM_PLUS_DOWNLOAD_URL"
    "BEPINEX_RELEASES_URL"
    "BEPINEX_DOWNLOAD_URL"
    "BEPINEX_FULL_RELEASES_URL"
    "BETA_BRANCH"
    "BETA_BRANCH_PASSWORD"
    "HTTP_PORT"
    "SCHEDULED_RESTART_SCHEDULE"
    "WORLD"
    "PASSWORD"
    "TYPE"
    "MODS"
    "ADDITIONAL_STEAMCMD_ARGS"
    "TZ"
    "PUID"
    "PGID"
  )

  for var in "${env_vars[@]}"; do
    value="${!var//\"/}"
    [[ -n "$value" ]] && echo "export ${var}=\"$value\"" | sudo tee -a /env.sh
  done

  log "Preset Env Configured"
}

# Function to set up cron jobs
setup_cron() {
  local name=$1
  local script=$2
  local schedule=$3

  echo "Setting up cron job: $name"

  local cron_folder="$CRON_LOCATION"
  local log_folder="$LOG_LOCATION"
  local log_location="$log_folder/$name.out"

  # Create necessary directories
  mkdir -p "$log_folder" "$cron_folder"
  rm -f "$log_location"

  # Create the cron job
  # shellcheck disable=SC2086
  echo "${schedule//\"/} BASH_ENV=/env.sh /bin/bash $HOME/scripts/$script >> $log_location 2>&1" | tee "$cron_folder/$name"
  chmod 0644 "$cron_folder/$name"
}

# Function to create directories with specified ownership and permissions
create_dir_with_ownership() {
  local user=$1
  local group=$2
  local dir=$3

  mkdir -p "${dir}"
  sudo chown -R "${user}:${group}" "${dir}"
  sudo chmod -R ug+rw "${dir}"
}

# Function to set up the filesystem, ensuring correct ownership and permissions
setup_filesystem() {
  log "Setting up file systems"

  sudo chown -R "$PUID:$PGID" "$HOME" /home/steam/.*
  sudo chmod -R ug+rwx "$HOME"

  create_dir_with_ownership "$PUID" "$PGID" "$SAVE_LOCATION"
  create_dir_with_ownership "$PUID" "$PGID" "$MODS_LOCATION"
  create_dir_with_ownership "$PUID" "$PGID" "$BACKUP_LOCATION"
  create_dir_with_ownership "$PUID" "$PGID" "$GAME_LOCATION"
  create_dir_with_ownership "$PUID" "$PGID" "$GAME_LOCATION/logs"
  create_dir_with_ownership "$PUID" "$PGID" "$HOME/cron.d"
  mkdir -p /home/steam/scripts

  sudo usermod -d /home/steam steam
}

# Function to check if the system has sufficient memory
check_memory() {
  local total_memory
  total_memory=$(free -h | awk '/^Mem:/ {print $2}' | tr -d 'G')
  if (($(echo "$total_memory < 2" | bc -l))); then
    log "Your system has less than 2GB of RAM! Valheim might not run on your system."
  else
    log "Total memory: ${total_memory} GB"
  fi
}

# Main script execution
log "Valheim Server - $(date)"
log "Initializing your container..."

# Check current user and steam user details
check_user_and_group

# Set up environment
setup_environment

# Check system memory
check_memory

# Set up the filesystem
setup_filesystem

# Configure environment variables for cron jobs
setup_cron_env

# Set up cron jobs if enabled
[[ "$AUTO_UPDATE" -eq 1 ]] && setup_cron "auto-update" "auto_update.sh" "$AUTO_UPDATE_SCHEDULE"
[[ "$AUTO_BACKUP" -eq 1 ]] && setup_cron "auto-backup" "auto_backup.sh" "$AUTO_BACKUP_SCHEDULE"
[[ "$SCHEDULED_RESTART" -eq 1 ]] && setup_cron "scheduled-restart" "scheduled_restart.sh" "$SCHEDULED_RESTART_SCHEDULE"

# Verify the cron directory and its contents
if [[ "$AUTO_BACKUP" -eq 1 || "$AUTO_UPDATE" -eq 1 || "$SCHEDULED_RESTART" -eq 1 ]]; then
  log "Checking if cron directory and files exist..."
  if [[ -d "$CRON_LOCATION" && $(ls -A "$CRON_LOCATION") ]]; then
    touch /tmp/master-cron
    for file in "$CRON_LOCATION"/*; do
      cat "$file" >>/tmp/master-cron
    done
    crontab /tmp/master-cron
    rm -f /tmp/master-cron
    sudo cron -f &
    export CRON_PID=$!
  else
    log "Error: Cron directory or files are missing."
    exit 1
  fi
fi

# Navigate to the Valheim game directory
log "Navigating to steam home..."
cd /home/steam/valheim || exit 1

# Launch the Valheim server
log "Launching server..."
exec /home/steam/scripts/start_valheim.sh
