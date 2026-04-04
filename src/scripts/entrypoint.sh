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

# Return a best-effort runtime user label, even for numeric-only UIDs.
runtime_user_label() {
  if id -un >/dev/null 2>&1; then
    id -un
  else
    printf "uid:%s" "$(id -u)"
  fi
}

# Function to check and log the current user and steam user's ID and group ID
check_user_and_group() {
  log "Runtime user: $(runtime_user_label)"
  log "Runtime uid: $(id -u)"
  log "Runtime gid: $(id -g)"
  if id -u steam >/dev/null 2>&1; then
    log "Steam uid: $(id -u steam)"
    log "Steam gid: $(id -g steam)"
  else
    log "Steam user entry not found in /etc/passwd"
  fi
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

  # Rootless mode: do not attempt to mutate /etc. The TZ env var is still exported.
  if [ -f "/usr/share/zoneinfo/$TZ" ]; then
    log "Rootless mode: skipping /etc/localtime update (TZ=${TZ})"
  else
    log "Timezone '$TZ' not found under /usr/share/zoneinfo; leaving timezone files unchanged"
  fi
  log "Rootless mode: skipping /etc/timezone update"
}

# Function to safely shut down the server
clean_up() {
  log "Safely shutting down..."
}

# Trap signals for safe shutdown
trap clean_up INT TERM

# Main script execution
log "Valheim Server - $(date)"
log "Initializing your container..."

# Check current user and steam user details
check_user_and_group

# Set up environment
setup_environment

# Validate system/runtime requirements in Odin (creates required dirs, checks memory + path writability)
log "Running system checks..."
odin system --check || exit 1

# Navigate to the Valheim game directory
log "Navigating to steam home..."
cd /home/steam/valheim || exit 1

# Launch the Valheim server
log "Launching server..."
exec /home/steam/scripts/start_valheim.sh
