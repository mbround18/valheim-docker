# Contributing

## Cargo Make

This package includes a make file for easy development.
You can get use the make file by installing cargo make via `cargo install cargo-make`

### Commands

| Command             | What it does                                                                   |
| ------------------- | ------------------------------------------------------------------------------ |
| makers format       | Formats the `http-server` and `odin`                                           |
| makers clippy       | Builds and runs clippy on `http-server` and `odin`                             |
| makers build        | Builds the two projects                                                        |
| makers start:dev    | Formats, Clippy, docker-compose build, and docker-compose up                   |
| makers docker:build | Runs docker-compose build for the file `docker-compose.dev.yml`                |
| makers docker:up    | Runs docker-compose up for `docker-compose.dev.yml`                            |
| makers access       | Runs `docker-compose -f docker-compose.dev.yml exec --user steam valheim bash` |
| makers access:admin | Runs `docker-compose -f docker-compose.dev.yml exec valheim bash`              |
| makers release      | Builds a release binary for `odin` and `http-server`                           |
