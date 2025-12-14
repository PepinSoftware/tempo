# Tempo Backend Plan (gRPC)

This document captures the overall execution plan for Tempo and a detailed Phase 1 scope, including the initial gRPC surface for inter-pod communication.

## gRPC Mock Schema (v0)
```proto
syntax = "proto3";
package tempo.v1;

import "google/protobuf/timestamp.proto";

message Dataset {
  string id = 1;
  string name = 2;
  string description = 3;
  google.protobuf.Timestamp created_at = 4;
}

message Series {
  string id = 1;
  string dataset_id = 2;
  string name = 3;
  string source_uri = 4; // pointer to where points live (object store, topic, etc.)
  string ordering = 5;   // e.g., "timestamp" or "ordinal" for point ordering semantics
  google.protobuf.Timestamp created_at = 6;
}

message LabelDefinition {
  string id = 1;
  string name = 2;          // e.g., "quality"
  repeated string values = 3; // allowed classifications
  google.protobuf.Timestamp created_at = 4;
}

message Classification {
  string series_id = 1;
  string label_definition_id = 2;
  string value = 3; // must be one of LabelDefinition.values
  google.protobuf.Timestamp applied_at = 4;
}

message Point {
  oneof ordering {
    google.protobuf.Timestamp timestamp = 1;
    uint64 ordinal = 2;
  }
  double value = 3;
  map<string, string> meta = 4; // optional per-point metadata (e.g., quality flags, source)
}

message CreateDatasetRequest { string name = 1; string description = 2; }
message CreateDatasetResponse { Dataset dataset = 1; }

message RegisterSeriesRequest {
  string dataset_id = 1;
  string name = 2;
  string source_uri = 3;
  string ordering = 4;
}
message RegisterSeriesResponse { Series series = 1; }

message DefineLabelRequest { string name = 1; repeated string values = 2; }
message DefineLabelResponse { LabelDefinition label_definition = 1; }

message SetSeriesClassificationRequest {
  string series_id = 1;
  string label_definition_id = 2;
  string value = 3;
}
message SetSeriesClassificationResponse { Classification classification = 1; }

message GetSeriesClassificationRequest {
  string series_id = 1;
  string label_definition_id = 2;
}
message GetSeriesClassificationResponse { Classification classification = 1; }

message ListSeriesRequest { string dataset_id = 1; }
message ListSeriesResponse { repeated Series series = 1; }

message AppendPointsRequest {
  string series_id = 1;
  repeated Point points = 2;
}
message AppendPointsResponse {
  uint64 accepted = 1; // number of points written
}

service TempoService {
  rpc CreateDataset(CreateDatasetRequest) returns (CreateDatasetResponse);
  rpc RegisterSeries(RegisterSeriesRequest) returns (RegisterSeriesResponse);
  rpc DefineLabel(DefineLabelRequest) returns (DefineLabelResponse);
  rpc SetSeriesClassification(SetSeriesClassificationRequest) returns (SetSeriesClassificationResponse);
  rpc GetSeriesClassification(GetSeriesClassificationRequest) returns (GetSeriesClassificationResponse);
  rpc ListSeries(ListSeriesRequest) returns (ListSeriesResponse);
  rpc AppendPoints(AppendPointsRequest) returns (AppendPointsResponse);
}
```

## Overall Phases (brief)
- Phase 1: Foundations — schema, migrations, Rust workspace, gRPC proto, repositories, core services, integration tests.
- Phase 2: Application layer expansion — invariants, batching, background jobs, metrics, auth stub hardening.
- Phase 3: Advanced annotations — intervals/spans, alignment/broadcasting, richer label semantics, review flows.
- Phase 4: Frontend + UX — web UI for annotation/review, session management, visualizations.

## Phase 1 Detailed Plan
- **Data model & constraints**
  - Tables: `datasets`, `series`, `label_definitions`, `series_classifications`, `points_ts`, `points_ord`, `event_log`.
  - Constraints: FK integrity, unique `(series_id, label_definition_id)` in `series_classifications`, non-empty `values` array for label definitions, created/updated timestamps.
  - Indexes: PK per table; composite index on `series(dataset_id, name)`; index on `series_classifications(label_definition_id)`; unique `(series_id, ts)` for `points_ts`; unique `(series_id, ordinal)` for `points_ord`.
  - Hypertables: `points_ts` partitioned on `ts`; `points_ord` partitioned on `ordinal`; `series_classifications` hypertable on `applied_at`.
- **Migrations**
  - Tool: `sqlx migrate` (offline mode) with checked-in SQL.
  - Add migration tests that apply/rollback against a throwaway Timescale container.
- **Workspace layout**
  - `proto/tempo/v1/tempo.proto` (from mock schema above).
  - `crates/core` (domain types, errors, validation helpers).
  - `crates/persistence` (SQLx models, query functions, repositories).
  - `crates/service` (gRPC server, wiring, config, logging).
  - `scripts/` for local tooling (DB bootstrap, lint/test runners).
- **Config & ops**
  - Config via env + `config` crate: DB URL, gRPC bind address, log level.
  - Logging with `tracing` + JSON option; healthz endpoint via gRPC reflection or a minimal TCP health check.
  - Docker Compose for TimescaleDB; optional `sqlx` offline cache baked in.
- **Service behavior (initial)**
  - gRPC handlers map 1:1 to services, performing validation then repository calls; `AppendPoints` enforces monotonic ordering within a batch, rejects ordering type mismatches, and deduplicates identical points (same series, ordering key) when re-sent.
  - Error mapping: not found, invalid argument (bad label value, unordered points), conflict (duplicate classification), internal.
- **Testing**
  - Unit tests in `core` for validation logic.
  - Integration tests in `service` hitting a test TimescaleDB (via `docker compose` or `testcontainers`).
  - Proto contract test: ensure generated Rust matches committed proto (CI check).
- **Developer workflow**
  - Make targets: `make proto` (generate Rust stubs via `tonic-build`), `make db-up` / `db-down`, `make migrate`, `make test`, `make fmt`, `make lint`, `make run` (launch server).
  - Sample seed script inserting one dataset, one series, one label definition, one classification for smoketests.
- **Deliverables for Phase 1**
  - Checked-in proto, migrations, Rust workspace scaffolding, gRPC server with core endpoints, repository implementations, initial tests, CI config for lint/format/test/migrate-check.
