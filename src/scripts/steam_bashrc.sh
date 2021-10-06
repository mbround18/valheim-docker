#!/usr/bin/env bash

# Load Valheim base directory,
cd /home/steam/valheim || exit 1

set -o allexport
source /home/steam/.config/odin || exit 1
set +o allexport
