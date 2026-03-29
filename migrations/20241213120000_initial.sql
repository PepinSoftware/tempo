-- Enable extensions
CREATE EXTENSION IF NOT EXISTS timescaledb;

-- Core entities
CREATE TABLE datasets (
    id          uuid PRIMARY KEY,
    name        text NOT NULL UNIQUE,
    description text NOT NULL DEFAULT '',
    created_at  timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE series (
    id          uuid PRIMARY KEY,
    dataset_id  uuid NOT NULL REFERENCES datasets(id) ON DELETE CASCADE,
    name        text NOT NULL,
    source_uri  text NOT NULL,
    ordering    text NOT NULL CHECK (ordering IN ('timestamp', 'ordinal')),
    created_at  timestamptz NOT NULL DEFAULT now(),
    UNIQUE (dataset_id, name)
);

CREATE TABLE label_definitions (
    id         uuid PRIMARY KEY,
    name       text NOT NULL UNIQUE,
    values     text[] NOT NULL CHECK (cardinality(values) > 0),
    created_at timestamptz NOT NULL DEFAULT now()
);

CREATE TABLE series_classifications (
    series_id           uuid NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    label_definition_id uuid NOT NULL REFERENCES label_definitions(id) ON DELETE CASCADE,
    value               text NOT NULL,
    applied_at          timestamptz NOT NULL DEFAULT now(),
    PRIMARY KEY (series_id, label_definition_id)
);

-- Points stored separately by ordering semantics
CREATE TABLE points_ts (
    series_id  uuid NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    ts         timestamptz NOT NULL,
    value      double precision NOT NULL,
    meta       jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (series_id, ts)
);

CREATE TABLE points_ord (
    series_id  uuid NOT NULL REFERENCES series(id) ON DELETE CASCADE,
    ordinal    bigint NOT NULL,
    value      double precision NOT NULL,
    meta       jsonb NOT NULL DEFAULT '{}'::jsonb,
    created_at timestamptz NOT NULL DEFAULT now(),
    UNIQUE (series_id, ordinal)
);

CREATE TABLE event_log (
    id         bigserial PRIMARY KEY,
    event_type text NOT NULL,
    payload    jsonb NOT NULL,
    created_at timestamptz NOT NULL DEFAULT now()
);

-- Hypertables
SELECT create_hypertable('points_ts', 'ts', if_not_exists => TRUE);
SELECT create_hypertable('points_ord', 'ordinal', if_not_exists => TRUE);
SELECT create_hypertable('series_classifications', 'applied_at', if_not_exists => TRUE);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_series_dataset_name ON series(dataset_id, name);
CREATE INDEX IF NOT EXISTS idx_classifications_label ON series_classifications(label_definition_id);
CREATE INDEX IF NOT EXISTS idx_points_ts_series_ts ON points_ts(series_id, ts DESC);
CREATE INDEX IF NOT EXISTS idx_points_ord_series_ord ON points_ord(series_id, ordinal DESC);
