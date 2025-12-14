# Tempo — Time Series Annotation Core

Tempo is a Rust-based backend for time-series annotation, built on TimescaleDB for ingesting, indexing, and labeling high-volume signals.

## Scope
- **Domain**: Manage labeled intervals and points over time-series data with provenance, audit, and versioning for label sets.
- **Storage**: TimescaleDB for schema-managed hypertables, retaining raw signal references plus derived annotations; migrations owned in-repo.
- **APIs**: Rust gRPC service exposing CRUD for datasets, spans, events, label definitions, and review workflows; batch ingest and export endpoints.
- **Processing**: Background jobs for validation (overlaps, gaps), deduplication, and rollups; optional queue for long-running tasks.
- **Auth & Access**: Token-based service auth first; user/workspace model added before UI launch.
- **Quality**: Strong typing, property-based tests for temporal logic, integration tests against TimescaleDB, and migration checks in CI.
- **Frontend**: Deferred; future web UI will consume the API for interactive annotation, review, and analytics.
- **Out of Scope** (initial): Realtime collaboration, ML-assisted labeling, and multi-tenant billing.

## Getting Started (backend core)
- Bring up TimescaleDB: `docker compose up -d timescaledb`.
- Apply migrations: `DATABASE_URL=postgres://tempo:tempo@localhost:5432/tempo sqlx migrate run`.
- Generate/build gRPC service: `make proto` or `cargo run -p tempo-service`.
- gRPC schema lives in `proto/tempo/v1/tempo.proto`; server binds to `0.0.0.0:50051` by default and serves dataset/series/label/classification plus point ingest RPCs.
