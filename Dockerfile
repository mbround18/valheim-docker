# ------------------------------------ #
# Odin (Rust) build and runtime stages #
# ------------------------------------ #
ARG DEBIAN_VERSION=12
ARG RUST_VERSION=1.90
ARG UBUNTU_VERSION=24
ARG EXPECTED_OCTAL=775

FROM rust:${RUST_VERSION} AS odin-base
ENV DEBIAN_FRONTEND=noninteractive
RUN --mount=type=cache,target=/var/cache/apt \
    --mount=type=cache,target=/var/lib/apt \
    apt-get update && \
    apt-get install -y --no-install-recommends cmake && \
    rm -rf /var/lib/apt/lists/*

FROM odin-base AS odin-chef
RUN cargo install cargo-chef

FROM odin-chef AS odin-planner
WORKDIR /data/odin
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

FROM odin-chef AS odin-cacher
WORKDIR /data/odin
COPY --from=odin-planner /data/odin/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM odin-base AS odin-builder
WORKDIR /data/odin
COPY . .
COPY --from=odin-cacher /data/odin/target target
COPY --from=odin-cacher /usr/local/cargo/registry /usr/local/cargo/
RUN make release PROFILE=production

FROM debian:${DEBIAN_VERSION}-slim AS odin
WORKDIR /apps
COPY --from=odin-builder /data/odin/target/release/odin /data/odin/target/release/huginn ./
ENTRYPOINT ["/apps/odin"]
CMD ["--version"]


# --------------------------- #
# Valheim server build image  #
# --------------------------- #

# Root setup (installs deps, creates steam user/group, etc.)
FROM steamcmd/steamcmd:ubuntu-${UBUNTU_VERSION} AS valheim-root

USER root

ENV TZ=America/Los_Angeles \
    DEBIAN_FRONTEND=noninteractive \
    PUID=111 \
    PGID=1000

RUN --mount=type=cache,target=/var/cache/apt,sharing=locked \
    --mount=type=cache,target=/var/lib/apt,sharing=locked \
    --mount=type=bind,source=src/scripts/build/setup-system.sh,target=/tmp/setup-system.sh \
    bash /tmp/setup-system.sh

# Container information
ARG GITHUB_SHA="not-set"
ARG GITHUB_REF="not-set"
ARG GITHUB_REPOSITORY="not-set"

# Pull Odin binaries from odin-runtime stage
COPY --from=odin --chmod=755 /apps/odin /apps/huginn /usr/local/bin/

# Set version information and configure sudoers
RUN printf "${GITHUB_SHA}\n${GITHUB_REF}\n${GITHUB_REPOSITORY}\n" >/home/steam/.version && \
    echo "root ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers && \
    echo "steam ALL=(ALL) NOPASSWD: ALL" >> /etc/sudoers


# SteamCMD setup
FROM valheim-root AS valheim-steamcmd

USER steam
ENV HOME=/home/steam
ENV USER=steam
ARG EXPECTED_OCTAL

ADD --chown=steam:${PGID} https://steamcdn-a.akamaihd.net/client/installer/steamcmd_linux.tar.gz /home/steam/steamcmd.tar.gz
COPY --chmod=${EXPECTED_OCTAL} --chown=steam:${PGID} --from=valheim-root /home/steam/.version /home/steam/.version

RUN --mount=type=bind,source=src/scripts/build/setup-steamcmd.sh,target=/tmp/setup-steamcmd.sh \
    bash /tmp/setup-steamcmd.sh


# Final Valheim runtime image
FROM valheim-steamcmd AS valheim

USER steam
ENV HOME=/home/steam
ENV USER=steam
ARG EXPECTED_OCTAL

# Set environment variables for Valheim server
ENV PORT="2456" \
    NAME="Valheim Docker" \
    WORLD="Dedicated" \
    PUBLIC="1" \
    TYPE="Vanilla" \
    UPDATE_ON_STARTUP="1" \
    AUTO_UPDATE="0" \
    AUTO_UPDATE_SCHEDULE="0 1 * * *" \
    AUTO_BACKUP="0" \
    AUTO_BACKUP_SCHEDULE="*/15 * * * *" \
    AUTO_BACKUP_REMOVE_OLD="1" \
    AUTO_BACKUP_DAYS_TO_LIVE="3" \
    AUTO_BACKUP_ON_UPDATE="0" \
    AUTO_BACKUP_ON_SHUTDOWN="0" \
    AUTO_BACKUP_PAUSE_WITH_NO_PLAYERS="0" \
    SCHEDULED_RESTART="0" \
    SCHEDULED_RESTART_SCHEDULE="0 2 * * *" \
    SAVE_LOCATION="/home/steam/.config/unity3d/IronGate/Valheim" \
    MODS_LOCATION="/home/steam/staging/mods" \
    GAME_LOCATION="/home/steam/valheim" \
    BACKUP_LOCATION="/home/steam/backups" \
    WEBHOOK_STATUS_SUCCESSFUL="1" \
    WEBHOOK_STATUS_FAILED="1" \
    BEPINEX_RELEASES_URL="https://thunderstore.io/api/experimental/package/denikson/BepInExPack_Valheim/" \
    WEBHOOK_STATUS_JOINED="1" \
    WEBHOOK_STATUS_LEFT="1"

# Copy scripts and set permissions
COPY --chmod=${EXPECTED_OCTAL} --chown=steam:${PGID} ./src/scripts/*.sh /home/steam/scripts/
COPY --chmod=${EXPECTED_OCTAL} --chown=steam:${PGID} ./src/scripts/entrypoint.sh /entrypoint.sh
COPY --chmod=${EXPECTED_OCTAL} --chown=steam:${PGID} ./src/scripts/env.sh /env.sh
COPY --chmod=${EXPECTED_OCTAL} --chown=steam:${PGID} ./src/scripts/steam_bashrc.sh /home/steam/.bashrc

# Convert scripts to Unix format
RUN --mount=type=bind,source=src/scripts/build/valheim-postcopy.sh,target=/tmp/valheim-postcopy.sh \
    bash /tmp/valheim-postcopy.sh

# Set the working directory to the game directory
WORKDIR /home/steam/valheim

# Expose the necessary ports
EXPOSE 2456/udp 2457/udp 2458/udp
EXPOSE 2456/tcp 2457/tcp 2458/tcp
EXPOSE 3000/tcp

# Healthcheck to ensure the Valheim server is running
HEALTHCHECK --interval=1m --timeout=3s CMD pidof valheim_server.x86_64 || exit 1

# Define the entrypoint and command
ENTRYPOINT ["/bin/bash", "/entrypoint.sh"]
CMD ["/bin/bash", "/home/steam/scripts/start_valheim.sh"]
