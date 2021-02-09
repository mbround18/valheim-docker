#!/bin/sh

  echo "###########################################################################"
  echo "# Valheim Server - $(date)"
  echo "# STEAM_UID ${STEAM_UID} - STEAM_GID ${STEAM_GID}"
  echo "# Running setup as $(whoami) ...."
  echo "###########################################################################"

# Change the STEAM_UID if needed
if [ ! "$(id -u steam)" -eq "${STEAM_UID}" ]; then
	echo "Changing steam STEAM_UID to ${STEAM_UID}."
	usermod -o -u "${STEAM_UID}" steam ;
fi
# Change STEAM_GID if needed
if [ ! "$(id -g steam)" -eq "$STEAM_GID" ]; then
	echo "Changing steam STEAM_GID to $STEAM_GID."
	groupmod -o -g "$STEAM_GID" steam ;
fi

# Put steam owner of directories (if the STEAM_UID changed, then it's needed)
chown -R steam:steam /home/steam

# avoid error message when su -p (we need to read the /root/.bash_rc )
chmod -R +r /root

# Launch run.sh with user steam (-p allow to keep env variables)
sudo \
  --preserve-env=NAME \
  --preserve-env=WORLD \
  --preserve-env=PORT \
  --preserve-env=PASSWORD \
  --preserve-env=AUTO_UPDATE \
  -u steam bash -c /home/steam/scripts/entrypoint.sh
