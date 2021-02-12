# ------------------ #
# -- Odin Builder -- #
# ------------------ #
FROM rust:latest as RustBuilder

WORKDIR /data/odin
COPY . .

RUN cargo install --path . \
    && cargo build --release

ENTRYPOINT ["/data/odin/target/release/odin"]
CMD ["--version"]
