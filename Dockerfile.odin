# ---------------------------- #
# -- Odin Project Mangement -- #
# ---------------------------- #
FROM mbround18/cargo-make:latest as cargo-make

# ------------------ #
# -- Odin Planner -- #
# ------------------ #
FROM registry.hub.docker.com/lukemathwalker/cargo-chef:latest-rust-1.58 as planner
WORKDIR /data/odin
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ------------------ #
# -- Odin Builder -- #
# ------------------ #
FROM registry.hub.docker.com/lukemathwalker/cargo-chef:latest-rust-1.58 as builder
# Restrict Cargo
# COPY ./config/config.toml /.cargo/config.toml

# Setup Project Files
WORKDIR /data/odin
COPY . .
COPY --from=planner /data/odin/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json

# Build for production
COPY --from=cargo-make /usr/local/bin/cargo-make /usr/local/cargo/bin
RUN /usr/local/cargo/bin/cargo make -p production release

# ------------------ #
# -- Odin Runtime -- #
# ------------------ #
FROM registry.hub.docker.com/library/debian:11-slim as runtime
WORKDIR /apps
COPY --from=builder /data/odin/target/release/odin /data/odin/target/release/huginn ./
ENTRYPOINT ["/apps/odin"]
CMD ["--version"]
