#!/usr/bin/env bash
. /home/steam/scripts/load_env.sh
echo "
Shutting down the server.........
"
cd /home/steam/valheim || exit 1
odin stop || exit 1
