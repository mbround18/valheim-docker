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
