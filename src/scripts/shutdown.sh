#!/usr/bin/env bash
echo "
Shutting down the server.........
"
cd /home/steam/valheim || exit 1
odin stop || exit 1
