#!/usr/bin/env bash
set -euxo pipefail

# This script runs in the steamcmd stage as user steam
# Expects HOME and USER envs already set

STEAMCMD_BOOTSTRAP_ATTEMPTS="${STEAMCMD_BOOTSTRAP_ATTEMPTS:-3}"
STEAMCMD_BOOTSTRAP_BASE_DELAY_SECS="${STEAMCMD_BOOTSTRAP_BASE_DELAY_SECS:-5}"
STEAMCMD_HOME="/home/steam/.local/share/Steam/steamcmd"
BOOTSTRAP_STEAMCMD_ON_BUILD="${BOOTSTRAP_STEAMCMD_ON_BUILD:-0}"
STEAMCMD_64BIT_BINARIES="${STEAMCMD_64BIT_BINARIES:-0}"

setup_steam_symlinks() {
  mkdir -p "$HOME/.steam"
  ln -sfn "$STEAMCMD_HOME/linux32" "$HOME/.steam/sdk32"
  ln -sfn "$STEAMCMD_HOME/linux64" "$HOME/.steam/sdk64"
  if [ -e "$HOME/.steam/sdk32/steamclient.so" ]; then
    ln -sfn "$HOME/.steam/sdk32/steamclient.so" "$HOME/.steam/sdk32/steamservice.so"
  fi
}

bootstrap_steamcmd() {
  local attempt=1
  while [ "$attempt" -le "$STEAMCMD_BOOTSTRAP_ATTEMPTS" ]; do
    rm -rf "$STEAMCMD_HOME"
    if steamcmd +quit; then
      setup_steam_symlinks
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

# Seed steamcmd from the packaged copy, the same way /usr/bin/steamcmd does on first run.
# The wrapper only seeds when $STEAMCMD_HOME is missing, so an empty directory here makes
# every runtime call exec a steamcmd.sh that isn't there (exit 127). steamcmd replaces
# these files with newer ones itself on first run.
seed_steamcmd() {
  mkdir -p "$STEAMCMD_HOME/linux32"
  cp /usr/lib/games/steam/steamcmd.sh "$STEAMCMD_HOME/"
  cp /usr/lib/games/steam/steamcmd "$STEAMCMD_HOME/linux32/"
}

seed_steamcmd
setup_steam_symlinks

# Same accepted values as odin's parse_truthy. This script runs in the build stage without
# src/scripts/utils.sh, so it carries its own copy.
is_truthy() {
  case "${1,,}" in
    1|true|yes|y|on) return 0 ;;
    *) return 1 ;;
  esac
}

if is_truthy "$BOOTSTRAP_STEAMCMD_ON_BUILD" || is_truthy "$STEAMCMD_64BIT_BINARIES"; then
  bootstrap_steamcmd
  # The bootstrap's self-update also installs the steamrt64 binaries into linux64/.
  if is_truthy "$STEAMCMD_64BIT_BINARIES" && [ ! -x "$STEAMCMD_HOME/linux64/steamcmd" ]; then
    echo "STEAMCMD_64BIT_BINARIES=1 but the bootstrap did not provide $STEAMCMD_HOME/linux64/steamcmd" >&2
    exit 1
  fi
else
  echo "Skipping steamcmd +quit bootstrap during image build"
fi

# Everything above (~/.local/share/Steam, steamcmd's install and self-update dir) is
# created after setup-system.sh made /home/steam group-writable, so repeat that here. A
# runtime uid that only shares the steam group - `runAsUser: 111` from before steam became
# uid 1000, or an orchestrator-assigned uid - must still pass odin's install preflight.
chmod -R g+rwX "$HOME"
