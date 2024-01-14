ARG DEBIAN_VERSION=12
ARG RUST_VERSION=1.75

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

FROM rust:${RUST_VERSION} as cargo-make

ARG CARGO_MAKE_VERSION=0.37.7

ADD https://github.com/sagiegurari/cargo-make/releases/download/${CARGO_MAKE_VERSION}/cargo-make-v${CARGO_MAKE_VERSION}-x86_64-unknown-linux-gnu.zip /tmp/cargo-make.zip
RUN unzip /tmp/cargo-make.zip -d /tmp \
    && mv /tmp/cargo-make-v${CARGO_MAKE_VERSION}-x86_64-unknown-linux-gnu/cargo-make /usr/local/bin/cargo-make \
    && rm -rf /tmp/cargo-make* \
    && chmod +x /usr/local/bin/cargo-make


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
