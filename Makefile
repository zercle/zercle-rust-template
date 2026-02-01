.PHONY: build run test lint clean migrate-up migrate-down docker-build docker-run docker-compose-up docker-compose-down docker-compose-logs docker-migrate docker-clean docker-up docker-down

build:
	cargo build --release

run: build
	./target/release/zercle-rust-template

run-dev:
	APP_ENV=local cargo run --bin zercle-rust-template

test:
	cargo test --all

test-unit:
	cargo test --lib

test-integration:
	cargo test --test integration

lint:
	cargo clippy --all-targets --all-features -- -D warnings

check:
	cargo check --all-targets --all-features

fmt:
	cargo fmt --all

fmt-check:
	cargo fmt --all -- --check

clean:
	cargo clean

migrate-up:
	@echo "Running migrations up..."
	@MIGRATIONS_PATH=./migrations cargo run --bin zercle-rust-template -- migrate up 2>/dev/null || echo "Migration command not implemented - use sqlx CLI or migrate tool"

migrate-down:
	@echo "Running migrations down..."
	@MIGRATIONS_PATH=./migrations cargo run --bin zercle-rust-template -- migrate down 2>/dev/null || echo "Migration command not implemented - use sqlx CLI or migrate tool"

install-deps:
	cargo install cargo-watch
	cargo install sqlx-cli --no-default-features --features postgres

watch:
	cargo watch -x run

# Docker commands
docker-build:
	docker build -t zercle-rust-template:latest -f deployments/docker/Dockerfile .

docker-run:
	docker run -p 3000:3000 --env-file .env zercle-rust-template:latest

docker-compose-up:
	docker-compose -f deployments/docker/docker-compose.yml up -d

docker-compose-down:
	docker-compose -f deployments/docker/docker-compose.yml down

docker-compose-logs:
	docker-compose -f deployments/docker/docker-compose.yml logs -f

docker-migrate:
	docker-compose -f deployments/docker/docker-compose.yml --profile migrate run --rm migrate

docker-clean:
	docker-compose -f deployments/docker/docker-compose.yml down -v
	docker rmi zercle-rust-template:latest || true

docker-up: docker-compose-up

docker-down: docker-compose-down
