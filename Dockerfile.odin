# ------------------ #
# -- Odin Builder -- #
# ------------------ #
FROM rust:latest as RustBuilder

WORKDIR /data/odin
COPY . .

RUN cargo build --release && find . ! -name 'odin' -type f -exec rm -f {} +


ENTRYPOINT ["/data/odin/target/release/odin"]
CMD ["--version"]
