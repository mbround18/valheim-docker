# ------------------ #
# -- Build Args    -- #
# ------------------ #
ARG DEBIAN_VERSION=12
ARG RUST_VERSION=1.81
ARG GITHUB_SHA="not-set"
ARG GITHUB_REF="not-set"
ARG GITHUB_REPOSITORY="not-set"
ARG PUID=111
ARG PGID=1000

# ------------------ #
# -- Odin Builder -- #
# ------------------ #
FROM rust:${RUST_VERSION}-slim-bullseye AS odin_base

# Set up necessary packages for building Odin
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    cmake \
    git \
    make && \
    rm -rf /var/lib/apt/lists/*

FROM odin_base AS odin_chef

RUN cargo install cargo-chef

FROM odin_chef AS odin_planner
WORKDIR /data/odin
# Copy Odin source code
COPY . . 
RUN cargo chef prepare --recipe-path recipe.json

FROM odin_chef AS odin_cacher
WORKDIR /data/odin
COPY --from=odin_planner /data/odin/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

FROM odin_base AS odin_builder
WORKDIR /data/odin
COPY . .  
COPY --from=odin_cacher /data/odin/target target
COPY --from=odin_cacher /usr/local/cargo/registry /usr/local/cargo/registry
RUN make release PROFILE=production

# ------------------ #
# -- Odin Runtime  -- #
# ------------------ #
FROM debian:${DEBIAN_VERSION}-slim AS odin_runtime
WORKDIR /apps
COPY --from=odin_builder /data/odin/target/release/odin /data/odin/target/release/huginn ./
ENTRYPOINT ["/apps/odin"]
CMD ["--version"]

# ------------------ #
# -- Valheim Base  -- #
# ------------------ #
FROM debian:${DEBIAN_VERSION}-slim AS valheim_base

# Define arguments and environment variables
ARG GITHUB_SHA
ARG GITHUB_REF
ARG GITHUB_REPOSITORY
ARG PUID
ARG PGID

ENV TZ=America/Los_Angeles \
    HOME=/home/steam \
    USER=steam

# Install necessary packages
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    bc \
    build-essential \
    ca-certificates \
    cron \
    curl \
    dos2unix \
    g++ \
    gcc \
    gdb \
    gosu \
    htop \
    jq \
    lib32gcc-s1 \
    libatomic1 \
    libc6 \
    libc6-dev \
    libpulse-dev \
    libpulse0 \
    libsdl2-2.0-0 \
    nano \
    net-tools \
    netcat-traditional \
    procps \
    sudo \
    tzdata \
    unzip \
    wget \
    zip && \    
    rm -rf /var/lib/apt/lists/*

# Set timezone
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

# Create a non-root user
RUN groupadd -g ${PGID} steam && \
    useradd -m -u ${PUID} -g steam -s /bin/bash steam

# Set version information
RUN echo -e "${GITHUB_SHA}\n${GITHUB_REF}\n${GITHUB_REPOSITORY}" > /home/steam/.version && \
    chown steam:steam /home/steam/.version

# ------------------ #
# -- SteamCMD Setup -- #
# ------------------ #
FROM valheim_base AS steamcmd

USER steam

# Install SteamCMD
ADD --chown=steam https://steamcdn-a.akamaihd.net/client/installer/steamcmd_linux.tar.gz $HOME/steamcmd_linux.tar.gz
RUN mkdir -p $HOME/steamcmd && \
    tar zxvf $HOME/steamcmd_linux.tar.gz -C $HOME/steamcmd && \
    mkdir -p $HOME/.steam/sdk32 && \
    ln -s $HOME/steamcmd/linux32/steamclient.so $HOME/.steam/sdk32/steamclient.so && \
    rm $HOME/steamcmd_linux.tar.gz

# Copy version file
COPY --chown=steam:steam --from=valheim_base /home/steam/.version $HOME/.version

# ------------------ #
# -- Valheim Setup -- #
# ------------------ #
FROM steamcmd AS valheim

USER steam

# Environment variables for Valheim server
ENV PORT=2456 \
    NAME="Valheim Docker" \
    WORLD="Dedicated" \
    PUBLIC=1 \
    UPDATE_ON_STARTUP=1 \
    GAME_LOCATION="$HOME/valheim"

# Set working directory
WORKDIR $GAME_LOCATION

# Copy scripts
COPY --chown=steam:steam --chmod=0755 ./src/scripts/*.sh $HOME/scripts/
COPY --chown=steam:steam --chmod=0755 ./src/scripts/entrypoint.sh /entrypoint.sh
COPY --chown=steam:steam --chmod=0755 ./src/scripts/env.sh /env.sh
COPY --chown=steam:steam --chmod=0755 ./src/scripts/steam_bashrc.sh $HOME/.bashrc

# ------------------ #
# -- Valheim Final -- #
# ------------------ #
FROM valheim AS valheim_final

USER root

# Copy Odin binaries into Valheim image (if needed)
COPY --from=odin_builder /data/odin/target/release/odin /usr/local/bin/
COPY --from=odin_builder /data/odin/target/release/huginn /usr/local/bin/
RUN chown root:root /usr/local/bin/odin /usr/local/bin/huginn && \
    chmod 0755 /usr/local/bin/odin /usr/local/bin/huginn

# Ensure scripts have correct permissions and Unix line endings
RUN dos2unix /entrypoint.sh $HOME/.bashrc $HOME/scripts/*.sh && \
    chmod 0755 /entrypoint.sh $HOME/.bashrc $HOME/scripts/*.sh && \
    echo "steam ALL=(ALL) NOPASSWD:ALL" | sudo tee /etc/sudoers.d/steam

USER steam

# Expose necessary ports
EXPOSE 2456-2458/udp 2456-2458/tcp 3000/tcp

# Healthcheck to ensure the Valheim server is running
HEALTHCHECK --interval=1m --timeout=3s \
    CMD pidof valheim_server.x86_64 || exit 1

# Set entrypoint and command
ENTRYPOINT ["/bin/bash", "/entrypoint.sh"]
CMD ["/bin/bash", "$HOME/scripts/start_valheim.sh"]
