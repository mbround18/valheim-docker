# ------------------ #
# -- Odin Builder -- #
# ------------------ #
FROM registry.hub.docker.com/library/rust:latest as RustBuilder

WORKDIR /data/odin
COPY . .

RUN cargo install --path . \
    && cargo build --release

# ----------------------- #
# -- Script Formatting -- #
# ----------------------- #

FROM registry.hub.docker.com/library/alpine:latest as ScriptSanitize

WORKDIR /data/scripts
COPY src/scripts/* ./

RUN apk add dos2unix  --update-cache --repository http://dl-3.alpinelinux.org/alpine/edge/testing/ --allow-untrusted \
    && dos2unix /data/scripts/**


# --------------- #
# -- Steam CMD -- #
# --------------- #
FROM registry.hub.docker.com/cm2network/steamcmd:root

RUN apt-get update          \
    && apt-get install -y   \
    htop net-tools nano     \
    netcat curl wget        \
    cron

# Set up timezone information
ENV TZ=America/Los_Angeles
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

# Copy hello-cron file to the cron.d directory
COPY --chown=steam:steam  src/cron/auto-update /etc/cron.d/auto-update

# Give execution rights on the cron job
RUN chmod 0644 /etc/cron.d/auto-update

# Apply cron job
RUN crontab /etc/cron.d/auto-update

# Setup Directories
RUN usermod -d /home/steam steam \
    && mkdir -p /home/steam/valheim \
    && chown -R steam:steam /home/steam/valheim \
    && mkdir -p /home/steam/scripts \
    && chown -R steam:steam /home/steam/scripts

USER steam

# Server Specific env variables.
ENV NAME "Valheim Docker"
ENV WORLD "Dedicated"
ENV PORT "2456"
ENV PASSWORD ""
ENV AUTO_UPDATE "0"

COPY --from=ScriptSanitize --chown=steam:steam --chmod=755  /data/scripts/*.sh /home/steam/scripts/
COPY --from=RustBuilder  --chown=steam:steam --chmod=755 /data/odin/target/release /home/steam/.odin

RUN mkdir -p /home/steam/valheim \
    && echo "export PATH=\"/home/steam/.odin:$PATH\"" >> /home/steam/.bashrc \
    && chown -R steam:steam /home/steam/ \
    && chown -R steam:steam /home/steam/valheim \
    && cp /home/steam/steamcmd/linux64/steamclient.so /home/steam/valheim

WORKDIR /home/steam/valheim

#RUN wget -O /etc/sudoers.d/sudo-lecture-disable https://raw.githubusercontent.com/Whonix/usability-misc/master/etc/sudoers.d/sudo-lecture-disable?raw=True

ENTRYPOINT ["/bin/bash", "/home/steam/scripts/entrypoint.sh"]
