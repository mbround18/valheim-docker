ARG DEBIAN_VERSION=12
ARG RUST_VERSION=1.78

# ------------------ #
# -- Odin Planner -- #
# ------------------ #
FROM lukemathwalker/cargo-chef:latest-rust-${RUST_VERSION} as planner
WORKDIR /data/odin
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ------------------ #
# -- Odin Cacher  -- #
# ------------------ #
FROM lukemathwalker/cargo-chef:latest-rust-${RUST_VERSION} as cacher
WORKDIR /data/odin
RUN apt-get update && apt-get install -y cmake
COPY --from=planner /data/odin/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json


# ------------------ #
# -- Odin Builder -- #
# ------------------ #
FROM rust:${RUST_VERSION} as builder
WORKDIR /data/odin
RUN apt-get update && apt-get install -y cmake
COPY . .
# Copy over the cached dependencies
COPY --from=cacher /data/odin/target target
COPY --from=cacher /usr/local/cargo/registry /usr/local/cargo/
RUN make release PROFILE=production

# ------------------ #
# -- Odin Runtime -- #
# ------------------ #
FROM debian:${DEBIAN_VERSION}-slim as runtime
WORKDIR /apps
COPY --from=builder /data/odin/target/release/odin /data/odin/target/release/huginn ./
ENTRYPOINT ["/apps/odin"]
CMD ["--version"]
