# ------------------ #
# -- Odin Planner -- #
# ------------------ #
FROM lukemathwalker/cargo-chef as planner
WORKDIR /data/odin
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ------------------ #
# -- Odin Cacher  -- #
# ------------------ #
FROM lukemathwalker/cargo-chef as cacher
ARG CARGO_MAKE_VERSION="0.32.14"
WORKDIR /data/odin
COPY --from=planner /data/odin/recipe.json recipe.json
RUN cargo install cargo-make --version "${CARGO_MAKE_VERSION}"
RUN cargo chef cook --release --recipe-path recipe.json

# ------------------ #
# -- Odin Builder -- #
# ------------------ #
FROM rust as builder
WORKDIR /data/odin
COPY . .
# Copy over the cached dependencies
COPY --from=cacher /data/odin/target target
COPY --from=cacher /usr/local/cargo /usr/local/cargo
RUN makers -p production release
# ------------------ #
# -- Odin Runtime -- #
# ------------------ #
FROM debian:buster-slim as runtime
WORKDIR /data/odin
COPY --from=builder /data/odin/target/release/odin /usr/local/bin
COPY --from=builder /data/odin/target/release/odin-http-server /usr/local/bin
ENTRYPOINT ["/usr/local/bin/odin"]
CMD ["--version"]
