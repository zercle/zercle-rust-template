.PHONY: build run test lint clean migrate-up migrate-down

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

docker-build:
	docker build -t zercle-rust-template .

docker-run:
	docker run -p 3000:3000 --env-file .env zercle-rust-template
