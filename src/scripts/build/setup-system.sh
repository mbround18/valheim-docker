#!/usr/bin/env bash
set -euxo pipefail

# Expected envs: TZ, PUID, PGID, DEBIAN_FRONTEND

ln -snf "/usr/share/zoneinfo/${TZ}" /etc/localtime
printf "%s\n" "${TZ}" > /etc/timezone

apt-get update
apt-get upgrade -y
apt-get install -y -qq --no-install-recommends \
  build-essential procps htop net-tools nano gcc g++ gdb \
  netcat-traditional curl wget zip unzip cron sudo gosu dos2unix \
  libsdl2-2.0-0 jq libc6 libc6-dev libpulse-dev libatomic1 \
  tzdata bc
rm -rf /var/lib/apt/lists/*

# Validate gosu install
gosu nobody true

# Create group steam with desired GID, adjusting if exists
if getent group steam >/dev/null; then
  CURRENT_GID="$(getent group steam | cut -d: -f3)"
  if [ "${CURRENT_GID}" != "${PGID}" ]; then
    if getent group "${PGID}" >/dev/null; then
      echo "GID ${PGID} already in use; leaving existing 'steam' group (${CURRENT_GID})"
    else
      groupmod -g "${PGID}" steam
    fi
  fi
else
  if getent group "${PGID}" >/dev/null; then
    groupadd -o -g "${PGID}" steam || true
  else
    groupadd -g "${PGID}" steam
  fi
fi

# Create/update steam user
if id -u steam >/dev/null 2>&1; then
  usermod -u "${PUID}" -g "${PGID}" -d /home/steam -s /bin/bash steam || true
else
  useradd -u "${PUID}" -g "${PGID}" -d /home/steam -m -s /bin/bash steam
fi

# Ensure directories and permissions
mkdir -p /home/steam/.steam/steam/package
mkdir -p /home/steam /home/steam/valheim /home/steam/.steam
mkdir -p /tmp/dumps && chmod ugo+rw /tmp/dumps
chown -R "${PUID}:${PGID}" /home/steam
