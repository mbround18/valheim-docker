#!/usr/bin/env bash

# Exit immediately if a command exits with a non-zero status
set -e

export HOME="/home/steam"
export GAME_LOCATION=${GAME_LOCATION:-"${HOME}/valheim"}
export SAVE_LOCATION=${SAVE_LOCATION:-"${GAME_LOCATION}/saves"}
export MODS_LOCATION=${MODS_LOCATION:-"${GAME_LOCATION}/BepInEx/plugins"}
export BACKUP_LOCATION=${BACKUP_LOCATION:-"${GAME_LOCATION}/backups"}
export LOG_LOCATION="${GAME_LOCATION}/logs"

 # Logging via odin
log() {
  odin log --message "$*"
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

# Function to safely shut down the server
clean_up() {
  log "Safely shutting down..."
}

# Trap signals for safe shutdown
trap clean_up INT TERM

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
  # Ensure /tmp remains writable for steamcmd/work files even when host-mounted.
  sudo mkdir -p /tmp
  sudo chown root:root /tmp
  sudo chmod 1777 /tmp

  create_dir_with_ownership "$PUID" "$PGID" "$SAVE_LOCATION"
  create_dir_with_ownership "$PUID" "$PGID" "$MODS_LOCATION"
  create_dir_with_ownership "$PUID" "$PGID" "$BACKUP_LOCATION"
  create_dir_with_ownership "$PUID" "$PGID" "$GAME_LOCATION"
  create_dir_with_ownership "$PUID" "$PGID" "$GAME_LOCATION/logs"
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

# Navigate to the Valheim game directory
log "Navigating to steam home..."
cd /home/steam/valheim || exit 1

# Launch the Valheim server
log "Launching server..."
exec /home/steam/scripts/start_valheim.sh
