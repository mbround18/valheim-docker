#!/usr/bin/env bash
set -euxo pipefail

# This script runs in the steamcmd stage as user steam
# Expects HOME and USER envs already set

STEAMCMD_BOOTSTRAP_ATTEMPTS="${STEAMCMD_BOOTSTRAP_ATTEMPTS:-3}"
STEAMCMD_BOOTSTRAP_BASE_DELAY_SECS="${STEAMCMD_BOOTSTRAP_BASE_DELAY_SECS:-5}"
STEAMCMD_HOME="/home/steam/.local/share/Steam/steamcmd"

mkdir -p /home/steam/.local/share/Steam/steamcmd

setup_steam_symlinks() {
  mkdir -p "$HOME/.steam"
  ln -sfn "$STEAMCMD_HOME/linux32" "$HOME/.steam/sdk32"
  ln -sfn "$STEAMCMD_HOME/linux64" "$HOME/.steam/sdk64"
  ln -sfn "$HOME/.steam/sdk32/steamclient.so" "$HOME/.steam/sdk32/steamservice.so"
}

bootstrap_steamcmd() {
  local attempt=1
  while [ "$attempt" -le "$STEAMCMD_BOOTSTRAP_ATTEMPTS" ]; do
    rm -rf "$STEAMCMD_HOME"
    mkdir -p "$STEAMCMD_HOME"
    tar -xvzf /home/steam/steamcmd.tar.gz -C "$STEAMCMD_HOME/"
    setup_steam_symlinks
    if steamcmd +quit; then
      return 0
    fi
    if [ "$attempt" -lt "$STEAMCMD_BOOTSTRAP_ATTEMPTS" ]; then
      local delay=$((STEAMCMD_BOOTSTRAP_BASE_DELAY_SECS * (2 ** (attempt - 1))))
      echo "steamcmd bootstrap failed on attempt ${attempt}/${STEAMCMD_BOOTSTRAP_ATTEMPTS}; retrying in ${delay}s"
      sleep "$delay"
    fi
    attempt=$((attempt + 1))
  done
  return 1
}

bootstrap_steamcmd
rm /home/steam/steamcmd.tar.gz

# Everything above (~/.local/share/Steam, steamcmd's install and self-update dir) is
# created after setup-system.sh made /home/steam group-writable, so repeat that here. A
# runtime uid that only shares the steam group - `runAsUser: 111` from before steam became
# uid 1000, or an orchestrator-assigned uid - must still pass odin's install preflight.
chmod -R g+rwX "$HOME"
