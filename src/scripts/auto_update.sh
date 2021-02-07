#!/usr/bin/env bash
cd "$(dirname "$0")" || exit 1

if [[ "${AUTO_UPDATE}" = "1" ]]; then
  cd /home/steam/valhaim || exit 1
  odin install
  odin stop
  odin start > /dev/null 2>&1
fi
