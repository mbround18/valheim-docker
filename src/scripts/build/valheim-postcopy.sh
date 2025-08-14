#!/usr/bin/env bash
set -euxo pipefail

# Post-copy conversions and ownership adjustments for valheim stage

dos2unix /entrypoint.sh /home/steam/.bashrc /home/steam/scripts/*.sh || true
chmod +x /entrypoint.sh /home/steam/.bashrc /home/steam/scripts/*.sh
chmod ug+rw /home/steam/
# Use numeric GID via env PGID; fallback to steam:steam if not present
if getent group "$PGID" >/dev/null 2>&1; then
  chown -R steam:"${PGID}" /home/steam/.steam || true
else
  chown -R steam:steam /home/steam/.steam || true
fi
steamcmd +quit
