# Contributing

## Using Make

This repository includes a `Makefile` to simplify common development tasks. If you have `make` installed, the quickest way to see available commands and short descriptions is:

```bash
make help
```

That prints all documented targets and short descriptions derived from `##` comments in the [Makefile](Makefile).

Below are the most commonly used targets and concise guidance.

- **`make setup`**: Create `docker-compose.dev.yml` if missing and prepare `./tmp` directories.
- **`make help`**: Show documented make targets (preferred way to discover commands).
- **`make lint`**: Run formatting and JS/TS prettier. Internally depends on `member_format` which runs `cargo fmt` for Rust.
- **`make docker-build`**: Build Docker images using `docker-compose.dev.yml`.
- **`make docker-up`**: Start services via `docker-compose.dev.yml`.
- **`make docker-down`**: Stop services defined in `docker-compose.dev.yml`.
- **`make docker-push`**: Push built images to a registry.
- **`make docker-dev`**: Build images and start services (development flow).
- **`make build`**: Build Rust binaries (`cargo build`).
- **`make build-dev`**: Build Docker images and run formatting/linting steps.
- **`make start`**: Format, lint and start services.
- **`make start-dev`**: Full dev start: stop, rebuild, and start services (recommended for local development).
- **`make test`**: Run Rust tests (`cargo test`).
- **`make access`**: Get a shell in the `valheim` container as user `steam`.
- **`make access-admin`**: Get a shell in the `valheim` container as root/admin.
- **`make release`**: Build release binaries

## Recommended workflow

- Fetch dependencies and set up the workspace:

```bash
make setup
```

- For iterative development:

```bash
make start-dev
```

- To run tests:

```bash
make test
```

- To produce production artifacts (note `PROFILE`):

```bash
make release PROFILE=production
```

If you want to add or update make target documentation, edit the corresponding target in the [Makefile](Makefile) and append a `## short description` to that target line; then run `make help` to see the updated list.
