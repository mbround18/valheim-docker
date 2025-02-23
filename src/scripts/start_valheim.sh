#!/usr/bin/env bash

# Exit immediately if a command exits with a non-zero status
set -e

# Logging function to prepend timestamps to log messages
log() {
  echo "$(date) - $*"
}

# Function to display a deprecation notice
deprecation_notice() {
  log "-------------------------------------------------------------------------"
  log "WARNING: ${1}"
  log "-------------------------------------------------------------------------"
}

# Function to initialize the server with a log message
initialize() {
  log "-------------------------------------------------------------"
  log "Valheim Server - $(date)"
  log "STEAM_UID ${STEAM_UID} - STEAM_GID ${STEAM_GID}"
  log "$1"
  log "-------------------------------------------------------------"
}

# Function to clean up on exit
cleanup() {
  log "Halting server! Received interrupt!"
  odin stop
  if [ "${AUTO_BACKUP_ON_SHUTDOWN:=0}" -eq 1 ]; then
    log "Backup on shutdown triggered! Running backup tool..."
    /bin/bash /home/steam/scripts/auto_backup.sh "shutdown"
  fi
  [[ -n $TAIL_PID ]] && kill "$TAIL_PID"
  [[ -n $ODIN_HTTP_SERVER_PID ]] && kill "$ODIN_HTTP_SERVER_PID"
}

# Function to handle BepInEx installation
install_bepinex() {
  log "Installing BepInEx"
  if [ -z "${BEPINEX_DOWNLOAD_URL}" ]; then
    log "Fetching BepInEx download URL..."
    log "BepInEx Releases URL: ${BEPINEX_RELEASES_URL}"
    BEPINEX_DOWNLOAD_URL="$(curl -L "${BEPINEX_RELEASES_URL}" | jq -r '.latest.download_url')"
  fi
  log "Pulling BepInEx from ${BEPINEX_DOWNLOAD_URL}"
  odin mod:install "${BEPINEX_DOWNLOAD_URL}"
}

# Navigate to the Valheim directory or exit if it fails
cd /home/steam/valheim

# Set default values for Steam user and group IDs if not provided
STEAM_UID=${PUID:=1000}
STEAM_GID=${PGID:=1000}

# Source utility scripts if they exist
[ -f "/home/steam/scripts/utils.sh" ] && source "/home/steam/scripts/utils.sh"

# Source environment variables if env.sh exists
[ -f "/env.sh" ] && source "/env.sh"

# Warn if running as root
if [ "${STEAM_UID}" -eq 0 ] || [ "${STEAM_GID}" -eq 0 ]; then
  log "WARNING: You should not run the server as root! Please use a non-root user!"
fi

# Warn if .bashrc is missing
if [ ! -f "/home/steam/.bashrc" ]; then
  log "WARNING: You should not run the server without a .bashrc! Please use a non-root user!"
fi

# Check if webhook URL is provided
has_webhook="true"
[ -z "$WEBHOOK_URL" ] && has_webhook="false"

# Initialize server with a message
initialize "Installing Valheim via $(odin --version)..."
log "Variables loaded....."
log "Port: ${PORT}"
log "Name: ${NAME}"
log "World: ${WORLD}"
log "Public: ${PUBLIC}"
log "With Crossplay: ${ENABLE_CROSSPLAY}"
log "Password: (REDACTED)"
log "Preset: ${PRESET}"
log "Modifiers: ${MODIFIERS}"
log "Set Key: ${SET_KEY}"
log "Has Webhook: ${has_webhook}"
log "Auto Update: ${AUTO_UPDATE}"
log "Auto Backup: ${AUTO_BACKUP}"
log "Auto Backup On Update: ${AUTO_BACKUP_ON_UPDATE}"
log "Auto Backup On Shutdown: ${AUTO_BACKUP_ON_SHUTDOWN}"
log "Auto Backup Pause With No Players: ${AUTO_BACKUP_PAUSE_WITH_NO_PLAYERS}"
log "Auto Backup Pause With Players: ${AUTO_BACKUP_PAUSE_WITH_PLAYERS}"
log "Auto Backup Remove Old: ${AUTO_BACKUP_REMOVE_OLD}"
log "Auto Backup Days To Live: ${AUTO_BACKUP_DAYS_TO_LIVE}"
log "Auto Backup Nice Level: ${AUTO_BACKUP_NICE_LEVEL}"
log "Update On Startup: ${UPDATE_ON_STARTUP}"
log "Mods: ${MODS}"
log "-------------------------------------------------------------"

# Export Steam App ID
export SteamAppId=${APPID:-896660}

# Setting up server
log "Running Install..."
log -l "DEBUG" "Current Directory: $(pwd)"
log -l "DEBUG" "Current User: $(whoami)"
log -l "DEBUG" "Current UID: ${UID}"
log -l "DEBUG" "Current GID: ${PGID}"
log -l "DEBUG" "Home Directory: ${HOME}"

# Install or update the server if necessary
if [ ! -f "./valheim_server.x86_64" ] || [ "${FORCE_INSTALL:-0}" -eq 1 ]; then
  odin install || exit 1
elif [ "${UPDATE_ON_STARTUP:-1}" -eq 1 ]; then
  log "Attempting to update before launching the server!"
  [ "${AUTO_BACKUP_ON_UPDATE:=0}" -eq 1 ] && /bin/bash /home/steam/scripts/auto_backup.sh "pre-update-backup"
  log "Installing Updates..."
  odin install || exit 1
else
  log "Skipping install process, looks like valheim_server is already installed :)"
fi

# Copy steamclient.so if it exists
[ -f "/home/steam/steamcmd/linux64/steamclient.so" ] && cp /home/steam/steamcmd/linux64/steamclient.so /home/steam/valheim/linux64/

# Configure the server
log "Initializing Variables...."
odin configure || exit 1

# Check the server type and handle mod installations
log "Checking for TYPE flag"
export TYPE="${TYPE:="vanilla"}"
log "Found Type ${TYPE}"
log "Running with ${TYPE} Valheim <3"
export TYPE="${TYPE,,}"
export GAME_LOCATION="${GAME_LOCATION:="/home/steam/valheim"}"

case "${TYPE}" in
"vanilla")
  if [ -n "${MODS:=""}" ]; then
    log "Mods supplied but you are running with Vanilla!!!"
    log "Mods will NOT be installed!."
  fi
  ;;
"bepinex")
  if [ ! -d "${GAME_LOCATION}/BepInEx" ] || [ ! -f "${GAME_LOCATION}/BepInEx/core/BepInEx.dll" ] || [ "${UPDATE_ON_STARTUP:-0}" -eq 1 ] || [ "${FORCE_INSTALL:-0}" -eq 1 ]; then
    install_bepinex
  fi
  ;;
*)
  log "Unknown type: ${TYPE}"
  exit 1
  ;;
esac

# Install additional mods if not running vanilla
if [ "${TYPE}" != "vanilla" ]; then
  # Replace commas and newlines with spaces
  MODS=$(echo "${MODS}" | tr ',\n' ' ')

  # Convert the MODS string into an array
  # shellcheck disable=SC2206
  MODS=(${MODS})

  for mod in "${MODS[@]}"; do
    log "Installing Mod ${mod}"
    odin mod:install "${mod}"
  done
fi


# Execute post-install scripts if they exist
if [ -d "/valheim-post-install.d/" ]; then
  log "Executing post-install scripts"
  find /valheim-post-install.d/ -type f -executable -exec {} \;
fi

# Start HTTP server if HTTP_PORT is specified
if [ -n "${HTTP_PORT}" ]; then
  huginn &
  export ODIN_HTTP_SERVER_PID=$!
fi

# Set up traps for cleaning up on exit
trap cleanup INT TERM

# Start the Valheim server
log "Starting server..."
odin start

sleep 2

# Initialize log files and start tailing them
log "Herding Graydwarfs..."
odin logs --watch &
export TAIL_PID=$!

# Wait for the tail process to exit
wait $TAIL_PID
