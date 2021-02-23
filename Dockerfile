# ------------------ #
# -- Odin Builder -- #
# ------------------ #
ARG ODIN_IMAGE_VERSION=latest
FROM mbround18/valheim-odin:${ODIN_IMAGE_VERSION} as runtime

# --------------- #
# -- Steam CMD -- #
# --------------- #
FROM cm2network/steamcmd:root

RUN apt-get update                     \
    && apt-get install -y              \
    htop net-tools nano gcc g++ gdb    \
    netcat curl wget zip unzip         \
    cron sudo gosu dos2unix            \
    libsdl2-2.0-0  jq   libc6-dev      \
    && rm -rf /var/lib/apt/lists/*     \
    && gosu nobody true                \
    && dos2unix

# Container informaiton
ARG GITHUB_SHA="not-set"
ARG GITHUB_REF="not-set"
ARG GITHUB_REPOSITORY="not-set"

# User config
ENV PUID=1000                           \
    PGID=1000                           \
    # Set up timezone information
    TZ=America/Los_Angeles              \
    # Server Specific env variables.
    PORT="2456"                         \
    NAME="Valheim Docker"               \
    WORLD="Dedicated"                   \
    PUBLIC="1"                          \
    PASSWORD="12345"                    \
    # Auto Update Configs
    AUTO_UPDATE="0"                     \
    AUTO_UPDATE_SCHEDULE="0 1 * * *"    \
    # Auto Backup Configs
    AUTO_BACKUP="0"                     \
    AUTO_BACKUP_SCHEDULE="*/15 * * * *" \
    AUTO_BACKUP_REMOVE_OLD="1"          \
    AUTO_BACKUP_DAYS_TO_LIVE="3"        \
    AUTO_BACKUP_ON_UPDATE="0"           \
    AUTO_BACKUP_ON_SHUTDOWN="0"

COPY ./src/scripts/*.sh /home/steam/scripts/
COPY ./src/scripts/entrypoint.sh /entrypoint.sh
COPY --from=runtime /usr/local/bin/odin /usr/local/bin/odin
COPY ./src/scripts/steam_bashrc.sh /home/steam/.bashrc

RUN usermod -u ${PUID} steam                            \
    && groupmod -g ${PGID} steam                        \
    && chsh -s /bin/bash steam                          \
    && printf "${GITHUB_SHA}\n${GITHUB_REF}\n${GITHUB_REPOSITORY}\n" >/home/steam/.version \
    && chmod 755 -R /home/steam/scripts/                \
    && chmod 755 /entrypoint.sh                         \
    && chmod 755 /usr/local/bin/odin


HEALTHCHECK --interval=1m --timeout=3s \
  CMD pidof valheim_server.x86_64 || exit 1

ENTRYPOINT ["/bin/bash","/entrypoint.sh"]
CMD ["/bin/bash", "/home/steam/scripts/start_valheim.sh"]
