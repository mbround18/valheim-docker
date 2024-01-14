#!/usr/bin/env bash

function log() {
  local prefix="[Valheim]"
  local line=""
  local level="INFO"

  while [ "$#" -gt 0 ]; do
    case "$1" in
      -p|--prefix)
        prefix="$2"
        shift 2
        ;;
      -l|--level)
        level="$2"
        shift 2
        ;;
      *)
        line="$1"
        shift 1
        ;;
     esac
  done

  if [ "${DEBUG_MODE:-0}" -eq "0" ] && [ "${level}" == "DEBUG" ]; then
    return
  fi

  echo "$(date +"%Y-%m-%d %H:%M:%S") - ${prefix}[${level}]: ${line}"
}


line() {
  log -p "#" "###########################################################################"
}

