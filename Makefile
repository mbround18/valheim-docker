# Makefile

.PHONY: help build setup .member_format .member_clippy docker-build docker-up docker-down docker-push start start-dev build-dev access access-admin release-odin release-http-server release test docker-dev smoke-rootless


help: ## Show this help and available make targets
	@echo "Usage: make [target]"
	@echo ""
	@echo "Available targets (add '## description' after target to document it):"
	@awk -F'##' '/^[a-zA-Z0-9._-]+:/{name=$$1; sub(/:.*/,"",name); if (name ~ /^\./) next; desc=$$2; gsub(/^ +| +$$/,"",desc); printf "  %-20s %s\n", name, desc}' Makefile | sort


setup: ## Create docker-compose.dev.yml (if missing) and tmp directories
	@if [ ! -f "$$PWD/docker-compose.dev.yml" ]; then \
		echo "Creating docker-compose.dev.yml for development"; \
		cp "$$PWD/docker-compose.yml" "$$PWD/docker-compose.dev.yml"; \
	fi
	mkdir -p ./tmp/saves
	mkdir -p ./tmp/backups
	mkdir -p ./tmp/server

.member_format: ## Run `cargo fmt` to format Rust code
	cargo fmt

.member_clippy: ## Run `cargo clippy` for lint checks
	cargo clippy

lint: .member_format ## Run JS/TS formatting with prettier (depends on .member_format)
	@docker run --rm -t -v "$$PWD:/app" -w /app -e FORCE_COLOR=1 node:lts sh -c 'npx -y prettier --write .'

docker-build: setup ## Build docker images using docker-compose.dev.yml
	docker compose -f ./docker-compose.dev.yml build

docker-up: setup ## Start docker services using docker-compose.dev.yml
	docker compose -f ./docker-compose.dev.yml up

docker-down: setup ## Stop docker services using docker-compose.dev.yml
	docker compose -f ./docker-compose.dev.yml down

docker-push: setup ## Push docker images using docker-compose.dev.yml
	docker compose -f ./docker-compose.dev.yml push

docker-dev: setup ## Build and start docker services (dev)
	docker compose -f ./docker-compose.dev.yml up --build

smoke-rootless: setup ## Smoke test container startup as default user and explicit 1000:1000
	@bash ./.github/scripts/smoke-rootless-users.sh

check-image-permissions: ## Check IMAGE=<valheim image> has accounts for, and is writable by, uid 1000 and legacy 111
	@bash ./.github/scripts/check-image-permissions.sh "$(IMAGE)"

e2e-container: ## Boot IMAGE=<valheim image> for real (downloads ~2 GB): users, player presence via Huginn
	@bash ./.github/scripts/e2e-container.sh "$(IMAGE)"

# The same paws commands CI runs (.github/workflows/ci.yml), for running locally.
paws-ci: ## Rust fmt/clippy/build/test exactly as CI runs it (paws ci)
	paws ci --toolchain rust

paws-docker: ## Build TARGET=odin|valheim exactly as CI does, without pushing (paws docker)
	paws docker --image "mbround18/$(TARGET)" --target "$(TARGET)"

paws-release-dry-run: ## Build and smoke-test the release binaries without uploading (paws release)
	paws release --local-build --no-upload --target x86_64-unknown-linux-gnu --package odin,huginn --binary-name odin,huginn --tag v0.0.0-local

test: ## Run Rust tests (cargo test)
	cargo test

build: lint ## Build Rust
	@cargo build

build-dev: .member_format .member_clippy docker-build ## Format, lint and build images

start: .member_format .member_clippy docker-up ## Format, lint and start services

start-dev: .member_format .member_clippy docker-down docker-build docker-up ## Rebuild and start services for development

access: ## Exec into valheim container as user 'steam'
	docker-compose -f ./docker-compose.dev.yml exec --user steam valheim bash

access-admin: ## Exec into valheim container as admin/root
	docker-compose -f ./docker-compose.dev.yml exec valheim bash

release-odin: ## Build `odin` in release mode when PROFILE=production
	cargo build --release --bin odin

release-http-server: ## Build `huginn` in release mode when PROFILE=production
	cargo build --release --bin huginn

release: release-odin release-http-server ## Build release binaries
