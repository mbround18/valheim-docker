#!/usr/bin/env bash

# Function to log messages with a customizable prefix and log level
log() {
  local prefix="[Valheim]"
  local line=""
  local level="INFO"

  # Parse arguments
  while [ "$#" -gt 0 ]; do
    case "$1" in
    -p | --prefix)
      prefix="$2"
      shift 2
      ;;
    -l | --level)
      level="$2"
      shift 2
      ;;
    *)
      line="$1"
      shift 1
      ;;
    esac
  done

  # Skip debug messages if DEBUG_MODE is not enabled
  if [ "${DEBUG_MODE:-0}" -eq 0 ] && [ "${level}" == "DEBUG" ]; then
    return
  fi

  # Log the message with timestamp, prefix, and log level
  echo "$(date +"%Y-%m-%d %H:%M:%S") - ${prefix}[${level}]: ${line}"
}

# Function to log a separator line
line() {
  log -p "#" "###########################################################################"
}
