# Makefile

.PHONY: setup member_format member_clippy docker-build docker-up docker-down docker-push start start-dev build-dev access access-admin release-odin release-http-server release

setup:
	@if [ ! -f "$$PWD/docker-compose.dev.yml" ]; then \
		echo "Creating docker-compose.dev.yml for development"; \
		cp "$$PWD/docker-compose.yml" "$$PWD/docker-compose.dev.yml"; \
	fi

lint: member_format
		docker run --rm -v "$$PWD:/app" -w /app node:lts  sh -c 'npx -y prettier --write .'

member_format:
	cargo fmt

member_clippy:
	cargo clippy

docker-build: setup
	docker compose -f ./docker-compose.dev.yml build

docker-up: setup
	docker compose -f ./docker-compose.dev.yml up

docker-down: setup
	docker compose -f ./docker-compose.dev.yml down

docker-push: setup
	docker compose -f ./docker-compose.dev.yml push

start: member_format member_clippy docker-up

start-dev: member_format member_clippy docker-down docker-build docker-up

build-dev: member_format member_clippy docker-build

access:
	docker-compose -f ./docker-compose.dev.yml exec --user steam valheim bash

access-admin:
	docker-compose -f ./docker-compose.dev.yml exec valheim bash

release-odin:
ifeq ($(PROFILE),production)
	cargo build --release --bin odin
endif

release-http-server:
ifeq ($(PROFILE),production)
	cargo build --release --bin huginn
endif

release: release-odin release-http-server
