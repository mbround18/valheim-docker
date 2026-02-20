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

# Run a command with elevated privileges when possible.
# If elevation is not available (common in explicit rootless user mode),
# return non-zero and let the caller decide whether to skip.
run_privileged() {
  if [ "$(id -u)" -eq 0 ]; then
    "$@"
    return $?
  fi

  if command -v sudo >/dev/null 2>&1 && sudo -n true >/dev/null 2>&1; then
    sudo "$@"
    return $?
  fi

  return 1
}

# Best-effort wrapper for privileged commands.
run_privileged_or_warn() {
  if ! run_privileged "$@"; then
    log "Skipping privileged command (no sudo/root): $*"
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

  # Set timezone (best effort). In rootless mode, /etc may be read-only.
  if [ -f "/usr/share/zoneinfo/$TZ" ]; then
    run_privileged_or_warn ln -snf "/usr/share/zoneinfo/$TZ" /etc/localtime
  else
    log "Timezone '$TZ' not found under /usr/share/zoneinfo; skipping /etc/localtime update"
  fi
  run_privileged_or_warn sh -c 'printf "%s\n" "$1" > /etc/timezone' sh "$TZ"
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
  run_privileged_or_warn chown -R "${user}:${group}" "${dir}"
  run_privileged_or_warn chmod -R ug+rw "${dir}"
}

# Function to set up the filesystem, ensuring correct ownership and permissions
setup_filesystem() {
  log "Setting up file systems"

  run_privileged_or_warn chown -R "$PUID:$PGID" "$HOME"
  run_privileged_or_warn chmod -R ug+rwx "$HOME"
  # Ensure /tmp remains writable for steamcmd/work files even when host-mounted.
  run_privileged_or_warn mkdir -p /tmp
  run_privileged_or_warn chown root:root /tmp
  run_privileged_or_warn chmod 1777 /tmp

  create_dir_with_ownership "$PUID" "$PGID" "$SAVE_LOCATION"
  create_dir_with_ownership "$PUID" "$PGID" "$MODS_LOCATION"
  create_dir_with_ownership "$PUID" "$PGID" "$BACKUP_LOCATION"
  create_dir_with_ownership "$PUID" "$PGID" "$GAME_LOCATION"
  create_dir_with_ownership "$PUID" "$PGID" "$GAME_LOCATION/logs"
  mkdir -p /home/steam/scripts || log "Could not create /home/steam/scripts without elevated permissions"

  run_privileged_or_warn usermod -d /home/steam steam
}

# Validate that key runtime paths are writable for the current user.
validate_runtime_paths() {
  local path
  for path in "$HOME" "$GAME_LOCATION" "$SAVE_LOCATION" "$MODS_LOCATION" "$BACKUP_LOCATION" /tmp; do
    mkdir -p "$path" 2>/dev/null || true
    if [ ! -w "$path" ]; then
      log "Path is not writable by runtime user: $path"
      log "If you run with a custom user, ensure mounted volumes and /home/steam are writable by uid:gid $(id -u):$(id -g)"
    fi
  done
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

# Default ownership targets to current runtime UID/GID when not explicitly set.
export PUID="${PUID:-$(id -u)}"
export PGID="${PGID:-$(id -g)}"

# Set up environment
setup_environment

# Check system memory
check_memory

# Set up the filesystem
setup_filesystem

# Validate runtime write access in rootless mode
validate_runtime_paths

# Navigate to the Valheim game directory
log "Navigating to steam home..."
cd /home/steam/valheim || exit 1

# Launch the Valheim server
log "Launching server..."
exec /home/steam/scripts/start_valheim.sh
