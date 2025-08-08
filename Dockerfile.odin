ARG DEBIAN_VERSION=12
ARG RUST_VERSION=1.89

FROM rust:${RUST_VERSION} AS base

RUN --mount=type=cache,target=/var/cache/apt \
    --mount=type=cache,target=/var/lib/apt \
    apt-get update && apt-get install -y cmake


FROM base AS chef

RUN cargo install cargo-chef


# ------------------ #
# -- Odin Planner -- #
# ------------------ #
FROM chef AS planner
WORKDIR /data/odin
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# ------------------ #
# -- Odin Cacher  -- #
# ------------------ #
FROM chef AS cacher

WORKDIR /data/odin

COPY --from=planner /data/odin/recipe.json recipe.json
RUN cargo chef cook --release --recipe-path recipe.json


# ------------------ #
# -- Odin Builder -- #
# ------------------ #
FROM base AS builder
WORKDIR /data/odin

COPY . .

COPY --from=cacher /data/odin/target target
COPY --from=cacher /usr/local/cargo/registry /usr/local/cargo/
RUN make release PROFILE=production

# ------------------ #
# -- Odin Runtime -- #
# ------------------ #
FROM debian:${DEBIAN_VERSION}-slim AS runtime
WORKDIR /apps
COPY --from=builder /data/odin/target/release/odin /data/odin/target/release/huginn ./
ENTRYPOINT ["/apps/odin"]
CMD ["--version"]
