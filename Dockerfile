FROM rust as RustBuilder

WORKDIR /data/odin
COPY . .

RUN cargo install --path . \
    && cargo build --release

FROM alpine as ScriptSanitize

WORKDIR /data/scripts
COPY ./scripts/* .

RUN apk add dos2unix  --update-cache --repository http://dl-3.alpinelinux.org/alpine/edge/testing/ --allow-untrusted \
    && dos2unix /data/scripts/**


# --------------------------------------------------------------------------------- #
# --------------------------------------------------------------------------------- #
# --------------------------------------------------------------------------------- #
FROM cm2network/steamcmd

RUN mkdir -p /home/steam/valheim \
    && mkdir -p /home/steam/scripts

ENV NAME "Valheim Docker"
ENV WORLD "Dedicated"
ENV PORT "2456"
ENV PASSWORD ""

COPY --from=ScriptSanitize --chown=steam:steam  /data/scripts/entrypoint.sh /home/steam/scripts/

WORKDIR /home/steam/valheim

COPY --from=RustBuilder --chown=steam:steam /data/odin/target/release /home/steam/odin

ENTRYPOINT ["/bin/bash", "/home/steam/scripts/entrypoint.sh"]
