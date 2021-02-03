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
COPY ./scripts/* .

RUN apk add dos2unix  --update-cache --repository http://dl-3.alpinelinux.org/alpine/edge/testing/ --allow-untrusted \
    && dos2unix /data/scripts/**


# --------------- #
# -- Steam CMD -- #
# --------------- #
FROM registry.hub.docker.com/cm2network/steamcmd

RUN mkdir -p /home/steam/valheim \
    && mkdir -p /home/steam/scripts

ENV NAME "Valheim Docker"
ENV WORLD "Dedicated"
ENV PORT "2456"
ENV PASSWORD ""

COPY --from=ScriptSanitize --chown=steam:steam  /data/scripts/entrypoint.sh /home/steam/scripts/

WORKDIR /home/steam/valheim

RUN chown steam:steam -R /home/steam/valheim \
    && echo "export PATH=\"/home/steam/odin:$PATH\"" >> /home/steam/.bashrc

COPY --from=RustBuilder --chown=steam:steam /data/odin/target/release /home/steam/odin

ENTRYPOINT ["/bin/bash", "/home/steam/scripts/entrypoint.sh"]
