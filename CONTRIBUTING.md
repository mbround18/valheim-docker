# Contributing

## Using Make

This package includes a `Makefile` for easy development setup and operations. Ensure you have `make` installed on your system, which is generally available by default on Unix-like operating systems (Linux, macOS). For Windows, you might need to install a tool like `GNU Make`.

### Commands

Below is a list of available `make` commands and their descriptions:

| Command              | What it does                                                                                               |
| -------------------- | ---------------------------------------------------------------------------------------------------------- |
| `make setup`         | Prepares the development environment by creating `docker-compose.dev.yml` if it doesn't exist.             |
| `make member_format` | Formats the code using `cargo fmt`.                                                                        |
| `make member_clippy` | Runs clippy to check the Rust code.                                                                        |
| `make docker-build`  | Builds the Docker images using the development compose file.                                               |
| `make docker-up`     | Starts the Docker containers as defined in `docker-compose.dev.yml`.                                       |
| `make docker-down`   | Stops the Docker containers and removes them.                                                              |
| `make docker-push`   | Pushes the Docker images to a Docker registry.                                                             |
| `make start`         | Starts the development environment, including formatting, clippy checks, and Docker containers.            |
| `make start-dev`     | Stops any running containers, rebuilds them, and starts them up for development.                           |
| `make build-dev`     | Formats code, runs clippy, and builds Docker images.                                                       |
| `make access`        | Provides shell access to the `valheim` container as the `steam` user.                                      |
| `make access-admin`  | Provides shell access to the `valheim` container as the `root` user.                                       |
| `make release`       | Builds release binaries for the projects, intended for production environments (set `PROFILE=production`). |

### Development Workflow

To start working on the project, you can run the following command to set up your environment and bring up the development services:

```bash
make start-dev
```

This command will ensure all code is formatted, clippy checks are passed, Docker images are built, and containers are running. If you want to build the project for production, remember to set the `PROFILE` environment variable:

```bash
make release PROFILE=production
```

This ensures that the release binaries are built according to the production profile settings.
