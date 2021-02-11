#!/usr/bin/env bash
cd "$(dirname "$0")" || exit 1

if [[ "${AUTO_UPDATE}" = "1" ]]; then
  cd /home/steam/valheim || exit 1
  odin install
  odin stop
  sleep 15
  odin start
fi
