#!/usr/bin/env bash

# Exit immediately if a command exits with a non-zero status
set -e

# Logging via odin
log() { odin log --message "$*"; }
log_debug() { odin log --level debug --message "$*"; }

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

# ------------------------------------------------------------
# ValheimPlus (Grantapher) installer (DLL + CFG assets only)
# ------------------------------------------------------------
# Env:
#   VALHEIMPLUS=latest | <tag>            # required to enable install
#   VALHEIMPLUS_DLL_URL=<direct-url>      # optional override (must be .dll)
#   VALHEIMPLUS_CFG_URL=<direct-url>      # optional override (must be .cfg)
#   VALHEIMPLUS_UPDATE=1                   # optional: force re-install each start
#
install_valheimplus_from_github() {
  local tag="${VALHEIMPLUS:-}"
  local dll_override="${VALHEIMPLUS_DLL_URL:-}"
  local cfg_override="${VALHEIMPLUS_CFG_URL:-}"
  local vhome="${VH_HOME:-/home/steam/valheim}"
  local plugins_dir="${vhome}/BepInEx/plugins"
  local config_dir="${vhome}/BepInEx/config"
  local marker_dir="${vhome}/.vplus-installed"
  local api dll_url cfg_url marker_key marker force_update tmp dll_path cfg_path

  # Only run if user requested V+
  [[ -z "$tag" && -z "$dll_override" && -z "$cfg_override" ]] && return 0

  mkdir -p "$plugins_dir" "$config_dir" "$marker_dir"

  # Marker / idempotency
  if [[ -n "$dll_override" || -n "$cfg_override" ]]; then
    marker_key="$(printf '%s|%s' "${dll_override:-none}" "${cfg_override:-none}" | sha1sum | awk '{print $1}')"
  else
    marker_key="${tag:-latest}"
  fi
  marker="${marker_dir}/${marker_key}"
  force_update=$([[ "${VALHEIMPLUS_UPDATE:-0}" -eq 1 ]] && echo 1 || echo 0)

  if [[ -f "$marker" && "$force_update" -eq 0 ]]; then
    echo "[ValheimPlus] Already installed (${marker_key}); skipping."
    return 0
  fi

  # Determine URLs for DLL + CFG
  if [[ -n "$dll_override" ]]; then
    dll_url="$dll_override"
  fi
  if [[ -n "$cfg_override" ]]; then
    cfg_url="$cfg_override"
  fi

  if [[ -z "$dll_url" || -z "$cfg_url" ]]; then
    # Resolve via GitHub API
    if [[ -z "$tag" || "${tag,,}" == "latest" ]]; then
      api="https://api.github.com/repos/Grantapher/ValheimPlus/releases/latest"
    else
      api="https://api.github.com/repos/Grantapher/ValheimPlus/releases/tags/${tag}"
    fi
    echo "[ValheimPlus] Resolving asset URLs via GitHub API (${api})"
    # Pull assets block once
    assets_json="$(
      curl -fsSL -H "Accept: application/vnd.github+json" "$api" \
      | tr -d '\r' \
      | awk '/"assets": \[/,/\]/{print}'
    )" || {
      echo "[ValheimPlus] ERROR: failed to fetch release metadata."
      return 1
    }

    # Extract DLL URL
    if [[ -z "$dll_url" ]]; then
      dll_url="$(printf '%s' "$assets_json" | awk -v IGNORECASE=1 '
        /"name":/ {
          name=$0
          gsub(/.*"name": *"/,"",name); gsub(/".*/,"",name)
        }
        /"browser_download_url":/ {
          url=$0
          gsub(/.*"browser_download_url": *"/,"",url); gsub(/".*/,"",url)
          if (name ~ /ValheimPlus.*\.dll$/) { print url; exit }
        }'
      )"
    fi

    # Extract CFG URL (accept common names)
    if [[ -z "$cfg_url" ]]; then
      cfg_url="$(printf '%s' "$assets_json" | awk -v IGNORECASE=1 '
        /"name":/ {
          name=$0
          gsub(/.*"name": *"/,"",name); gsub(/".*/,"",name)
        }
        /"browser_download_url":/ {
          url=$0
          gsub(/.*"browser_download_url": *"/,"",url); gsub(/".*/,"",url)
          if (name ~ /(ValheimPlus\.cfg|valheim_plus\.cfg)$/) { print url; exit }
        }'
      )"
    fi
  fi

  if [[ -z "$dll_url" ]]; then
    echo "[ValheimPlus] ERROR: Could not find ValheimPlus.dll asset URL."
    return 1
  fi
  if [[ -z "$cfg_url" ]]; then
    echo "[ValheimPlus] NOTE: CFG asset not found; server will generate defaults on first run."
  fi

  echo "[ValheimPlus] Downloading DLL: $dll_url"
  tmp="$(mktemp -d)"
  dll_path="${tmp}/ValheimPlus.dll"
  if ! curl -fL --retry 3 --retry-delay 2 -o "$dll_path" "$dll_url"; then
    echo "[ValheimPlus] ERROR: Failed to download DLL."
    rm -rf "$tmp"
    return 1
  fi
  install -m 0644 "$dll_path" "${plugins_dir}/ValheimPlus.dll"

  if [[ -n "$cfg_url" ]]; then
    echo "[ValheimPlus] Downloading CFG: $cfg_url"
    cfg_path="${tmp}/valheim_plus.cfg"
    if curl -fL --retry 3 --retry-delay 2 -o "$cfg_path" "$cfg_url"; then
      install -m 0644 "$cfg_path" "${config_dir}/valheim_plus.cfg"
    else
      echo "[ValheimPlus] WARNING: Failed to download CFG; continuing."
    fi
  fi

  touch "$marker"
  rm -rf "$tmp"
  echo "[ValheimPlus] Installed (${marker_key})."
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
log "Update On Startup: ${VALHEIMPLUS_UPDATE}"
log "Mods: ${MODS}"
log "-------------------------------------------------------------"

# Export Steam App ID
export SteamAppId=${APPID:-896660}

# Setting up server
log "Running Install..."
log_debug "Current Directory: $(pwd)"
log_debug "Current User: $(whoami)"
log_debug "Current UID: ${UID}"
log_debug "Current GID: ${PGID}"
log_debug "Home Directory: ${HOME}"

# Install or update the server if necessary
if [ ! -f "./valheim_server.x86_64" ] || [ "${FORCE_INSTALL:-0}" -eq 1 ]; then
  odin install || exit 1
elif [ "${VALHEIMPLUS_UPDATE:-1}" -eq 1 ]; then
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
  if [ ! -d "${GAME_LOCATION}/BepInEx" ] || [ ! -f "${GAME_LOCATION}/BepInEx/core/BepInEx.dll" ] || [ "${VALHEIMPLUS_UPDATE:-0}" -eq 1 ] || [ "${FORCE_INSTALL:-0}" -eq 1 ]; then
    install_bepinex
  fi
  
  # --- ValheimPlus hook ---
  if [ -n "${VALHEIMPLUS:-}" ] || [ -n "${VALHEIMPLUS_DLL_URL:-}" ] || [ -n "${VALHEIMPLUS_CFG_URL:-}" ]; then
    vplus_dll="${GAME_LOCATION}/BepInEx/plugins/ValheimPlus.dll"
    vplus_cfg_a="${GAME_LOCATION}/BepInEx/config/ValheimPlus.cfg"
    vplus_cfg_b="${GAME_LOCATION}/BepInEx/config/valheim_plus.cfg"

    need_install=0
    if [ "${VALHEIMPLUS_UPDATE:-0}" -eq 1 ]; then
      need_install=1
    else
      if [ ! -f "$vplus_dll" ] || { [ ! -f "$vplus_cfg_a" ] && [ ! -f "$vplus_cfg_b" ]; }; then
        need_install=1
      fi
    fi

    if [ "$need_install" -eq 1 ]; then
      echo "[ValheimPlus] Installing (DLL+CFG assets)…"
      install_valheimplus_from_github
    else
      echo "[ValheimPlus] DLL and cfg present; skipping install."
    fi
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
