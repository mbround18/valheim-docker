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
    cron sudo

# Set up timezone information
ENV TZ=America/Los_Angeles
RUN ln -snf /usr/share/zoneinfo/$TZ /etc/localtime && echo $TZ > /etc/timezone

# Copy hello-cron file to the cron.d directory
COPY --chown=steam:steam  src/cron/auto-update /etc/cron.d/auto-update

# Give execution rights on the cron job
RUN chmod 0644 /etc/cron.d/auto-update

# Apply cron job
RUN crontab /etc/cron.d/auto-update

# Server Specific env variables.
ENV NAME "Valheim Docker"
ENV WORLD "Dedicated"
ENV PORT "2456"
ENV PASSWORD ""
ENV AUTO_UPDATE "0"

COPY --from=ScriptSanitize --chmod=755  /data/scripts/*.sh /home/steam/scripts/
COPY --from=ScriptSanitize --chmod=755  /data/scripts/init.sh /init.sh
COPY --from=RustBuilder  --chmod=755 /data/odin/target/release /home/steam/.odin


#WORKDIR /home/steam/valheim

ENV PUID=1000
ENV PGID=1000
RUN usermod -u ${PUID} steam \
    && groupmod -g ${PGID} steam

ENTRYPOINT ["/bin/bash", "/init.sh"]
