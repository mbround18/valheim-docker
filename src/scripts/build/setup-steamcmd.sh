#!/usr/bin/env bash
set -euxo pipefail

# This script runs in the steamcmd stage as user steam
# Expects HOME and USER envs already set

mkdir -p /home/steam/.local/share/Steam/steamcmd

tar -xvzf /home/steam/steamcmd.tar.gz -C /home/steam/.local/share/Steam/steamcmd/
rm /home/steam/steamcmd.tar.gz

mkdir -p "$HOME/.steam"
ln -s "$HOME/.local/share/Steam/steamcmd/linux32" "$HOME/.steam/sdk32"
ln -s "$HOME/.local/share/Steam/steamcmd/linux64" "$HOME/.steam/sdk64"
ln -s "$HOME/.steam/sdk32/steamclient.so" "$HOME/.steam/sdk32/steamservice.so"

steamcmd +quit
