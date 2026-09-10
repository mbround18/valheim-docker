#!/usr/bin/env bash
set -euxo pipefail

# Expected envs: TZ, PUID, PGID, DEBIAN_FRONTEND

ln -snf "/usr/share/zoneinfo/${TZ}" /etc/localtime
printf "%s\n" "${TZ}" > /etc/timezone

apt-get update
apt-get upgrade -y
apt-get install -y -qq --no-install-recommends \
  build-essential procps htop net-tools nano gcc g++ gdb \
  netcat-traditional curl wget zip unzip sudo gosu dos2unix \
  libsdl2-2.0-0 jq libc6 libc6-dev libpulse-dev libatomic1 \
  tzdata bc ca-certificates
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

# The base image ships an `ubuntu` account on uid 1000. Retire whichever account holds
# the target uid so `steam` (the documented `user: 1000:1000`) can own it.
EXISTING_UID_USER="$(getent passwd "${PUID}" | cut -d: -f1 || true)"
if [ -n "${EXISTING_UID_USER}" ] && [ "${EXISTING_UID_USER}" != "steam" ]; then
  echo "uid ${PUID} is held by '${EXISTING_UID_USER}'; removing it in favour of 'steam'"
  # userdel can delete the account yet still exit non-zero (e.g. no mail spool to remove),
  # and a second userdel would then fail on the missing user. Check the outcome instead.
  userdel -r "${EXISTING_UID_USER}" || true
  if getent passwd "${PUID}" >/dev/null; then
    echo "uid ${PUID} is still held by '$(getent passwd "${PUID}" | cut -d: -f1)'" >&2
    exit 1
  fi
fi

# Create/update steam user
if id -u steam >/dev/null 2>&1; then
  usermod -u "${PUID}" -g "${PGID}" -d /home/steam -s /bin/bash steam || true
else
  useradd -u "${PUID}" -g "${PGID}" -d /home/steam -m -s /bin/bash steam
fi

# steam was uid 111 before it became 1000, and existing deployments still run as
# `runAsUser: 111`. The Valheim server segfaults at startup (in PlayFab's logger) when its
# uid has no passwd entry, so keep one for 111 that shares steam's group and home.
LEGACY_UID=111
if [ "${PUID}" != "${LEGACY_UID}" ] && ! getent passwd "${LEGACY_UID}" >/dev/null; then
  useradd -u "${LEGACY_UID}" -g "${PGID}" -d /home/steam -M -s /bin/bash steam-legacy
fi

# Ensure directories and permissions
mkdir -p /home/steam/.steam/steam/package
mkdir -p /home/steam /home/steam/valheim /home/steam/.steam
mkdir -p /tmp/dumps && chmod ugo+rw /tmp/dumps
chown -R "${PUID}:${PGID}" /home/steam
# Group-writable home so a runtime uid that only shares the gid (arbitrary-uid orchestrators) still works.
chmod -R g+rwX /home/steam
