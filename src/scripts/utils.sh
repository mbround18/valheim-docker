#!/usr/bin/env bash

# Function to log messages via `odin log` with optional prefix and level
log() {
  local prefix="[Valheim]"
  local line=""
  local level="info"

  # Parse args: -p/--prefix, -l/--level, and message
  while [ "$#" -gt 0 ]; do
    case "$1" in
      -p|--prefix)
        prefix="$2"; shift 2 ;;
      -l|--level)
        level="$2"; shift 2 ;;
      *)
        line="$1"; shift 1 ;;
    esac
  done

  # Normalize level to lowercase for odin CLI
  level="${level,,}"

  # Delegate to odin; odin respects DEBUG_MODE/RUST_LOG for filtering
  odin log --level "${level}" --message "${prefix} ${line}"
}

# Function to log a separator line
line() {
  log -p "#" "###########################################################################"
}

# --- TYPE --------------------------------------------------------------------
#
# The value of TYPE as the container receives it. Compose's list-form `environment:`
# passes `- TYPE="Vanilla"` with the quotes in the value, and `- TYPE=""` as two quote
# characters, neither of which is a server type. Strip surrounding whitespace and one pair
# of matching quotes, lowercase, and read an empty value as vanilla.
normalize_type() {
  local type="${1-}"
  type="${type#"${type%%[![:space:]]*}"}"
  type="${type%"${type##*[![:space:]]}"}"
  case "${type}" in
    \"*\") type="${type#\"}"; type="${type%\"}" ;;
    \'*\') type="${type#\'}"; type="${type%\'}" ;;
  esac
  type="${type#"${type%%[![:space:]]*}"}"
  type="${type%"${type##*[![:space:]]}"}"
  type="${type,,}"
  printf '%s' "${type:-vanilla}"
}

# --- ValheimPlus (TYPE=ValheimPlus) ----------------------------------------
#
# ValheimPlus is a BepInEx plugin, so TYPE=ValheimPlus is TYPE=BepInEx plus one
# known plugin. The plugin is installed through the normal MODS pipeline, which
# means it is version-reconciled and removed again if TYPE changes.

# Where to pull ValheimPlus.dll from.
#
# Defaults to the latest release of Grantapher's maintained fork. That URL is a
# redirect GitHub serves without the API, so it costs no rate limit, and each
# restart picks up the current release the same way BepInEx itself does. Pin a
# release with VALHEIM_PLUS_RELEASE, or bypass all of this with
# VALHEIM_PLUS_DOWNLOAD_URL.
valheim_plus_download_url() {
  if [ -n "${VALHEIM_PLUS_DOWNLOAD_URL:-}" ]; then
    printf '%s' "${VALHEIM_PLUS_DOWNLOAD_URL}"
    return 0
  fi

  local repository="${VALHEIM_PLUS_REPOSITORY:-Grantapher/ValheimPlus}"
  local release="${VALHEIM_PLUS_RELEASE:-latest}"

  if [ "${release,,}" = "latest" ]; then
    printf 'https://github.com/%s/releases/latest/download/ValheimPlus.dll' "${repository}"
  else
    printf 'https://github.com/%s/releases/download/%s/ValheimPlus.dll' "${repository}" "${release}"
  fi
}

# True when a MODS list already names ValheimPlus, in whatever form: a release
# URL, a Thunderstore package, a pinned version. Someone who listed it by hand
# gets their entry, not ours.
mods_include_valheim_plus() {
  local mods="${1:-}"
  [[ "${mods,,}" == *valheimplus* || "${mods,,}" == *valheim_plus* ]]
}

# Echoes the MODS list with the ValheimPlus plugin appended. Odin treats commas,
# newlines and whitespace alike as separators, so a space is always a safe join.
mods_with_valheim_plus() {
  local mods="${1:-}"
  local url="${2:?valheim plus url is required}"

  if mods_include_valheim_plus "${mods}"; then
    printf '%s' "${mods}"
  elif [ -z "${mods//[[:space:],]/}" ]; then
    printf '%s' "${url}"
  else
    printf '%s %s' "${mods}" "${url}"
  fi
}
