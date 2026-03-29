SHELL := /bin/sh

PROTO_SRC := proto/tempo/v1/tempo.proto

.PHONY: proto
proto:
	cd crates/service && cargo build -p tempo-service

.PHONY: db-up
db-up:
	docker compose up -d timescaledb

.PHONY: db-down
db-down:
	docker compose down

.PHONY: migrate
migrate:
	sqlx migrate run

.PHONY: fmt
fmt:
	cargo fmt --all

.PHONY: lint
lint:
	cargo clippy --all-targets --all-features -- -D warnings

.PHONY: test
test:
	cargo test --all

.PHONY: run
run:
	cargo run -p tempo-service
