ARG DEBIAN_VERSION=12
ARG RUST_VERSION=1.73

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
COPY --from=planner /data/odin/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# ---------------------------- #
# -- Odin Project Mangement -- #
# ---------------------------- #

FROM mbround18/cargo-make:latest as cargo-make

# ------------------ #
# -- Odin Builder -- #
# ------------------ #
FROM rust:${RUST_VERSION} as builder
WORKDIR /data/odin
COPY . .
# Copy over the cached dependencies
COPY --from=cacher /data/odin/target target
COPY --from=cacher /usr/local/cargo/registry /usr/local/cargo/
COPY --from=cargo-make /usr/local/bin/cargo-make /usr/local/cargo/bin
RUN /usr/local/cargo/bin/cargo make -p production release

# ------------------ #
# -- Odin Runtime -- #
# ------------------ #
FROM debian:${DEBIAN_VERSION}-slim as runtime
WORKDIR /apps
COPY --from=builder /data/odin/target/release/odin /data/odin/target/release/huginn ./
ENTRYPOINT ["/apps/odin"]
CMD ["--version"]
