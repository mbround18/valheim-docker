# ------------------ #
# -- Odin Builder -- #
# ------------------ #
FROM mbround18/valheim-odin:latest as RustBuilder

# --------------- #
# -- Steam CMD -- #
# --------------- #
FROM cm2network/steamcmd:root

RUN apt-get update                  \
    && apt-get install -y           \
    htop net-tools nano             \
    netcat curl wget                \
    cron sudo gosu dos2unix         \
    libsdl2-2.0-0  jq  multitail    \
    && rm -rf /var/lib/apt/lists/*  \
    && gosu nobody true             \
    && dos2unix

# Container informaiton
ARG GITHUB_SHA="not-set"
ARG GITHUB_REF="not-set"
ARG GITHUB_REPOSITORY="not-set"

# User config
ENV PUID=1000
ENV PGID=1000

# Set up timezone information
ENV TZ=America/Los_Angeles

# Server Specific env variables.
ENV PORT "2456"
ENV NAME "Valheim Docker"
ENV WORLD "Dedicated"
ENV PUBLIC "1"
ENV PASSWORD "12345"

# Auto Update Configs
ENV AUTO_UPDATE "0"
ENV AUTO_UPDATE_SCHEDULE "0 1 * * *"

# Auto Update Configs
ENV AUTO_BACKUP "0"
ENV AUTO_BACKUP_SCHEDULE "*/15 * * * *"
ENV AUTO_BACKUP_REMOVE_OLD "1"
ENV AUTO_BACKUP_DAYS_TO_LIVE "3"


COPY --chmod=755 ./src/scripts/*.sh /home/steam/scripts/
COPY --chmod=755  ./src/scripts/entrypoint.sh /entrypoint.sh
COPY --from=RustBuilder  --chmod=755 /data/odin/target/release /usr/local/odin
COPY --chown=steam:steam ./src/scripts/steam_bashrc.sh /home/steam/.bashrc

RUN usermod -u ${PUID} steam                            \
    && groupmod -g ${PGID} steam                        \
    && chsh -s /bin/bash steam                          \
    && ln -s /usr/local/odin/odin /usr/local/bin/odin   \
    && printf "${GITHUB_SHA}\n${GITHUB_REF}\n${GITHUB_REPOSITORY}\n" >/home/steam/.version


HEALTHCHECK --interval=1m --timeout=3s \
  CMD gosu steam pidof valheim_server.x86_64 || exit 1

ENTRYPOINT ["/bin/bash","/entrypoint.sh"]
CMD ["/bin/bash", "/home/steam/scripts/start_valheim.sh"]
