use async_trait::async_trait;
use serde_json::Value;
use sqlx::{postgres::PgPoolOptions, PgPool};
use tempo-core::{Classification, Dataset, DomainError, LabelDefinition, Ordering, Series};
use uuid::Uuid;

#[derive(Clone)]
pub struct DbPool(pub PgPool);

impl DbPool {
    pub async fn connect(database_url: &str, max_connections: u32) -> anyhow::Result<Self> {
        let pool = PgPoolOptions::new()
            .max_connections(max_connections)
            .connect(database_url)
            .await?;
        Ok(Self(pool))
    }
}

#[async_trait]
pub trait DatasetRepo {
    async fn create(&self, name: &str, description: &str) -> anyhow::Result<Dataset>;
}

#[async_trait]
pub trait SeriesRepo {
    async fn create(
        &self,
        dataset_id: Uuid,
        name: &str,
        source_uri: &str,
        ordering: Ordering,
    ) -> anyhow::Result<Series>;
    async fn get(&self, id: Uuid) -> anyhow::Result<Option<Series>>;
    async fn list_by_dataset(&self, dataset_id: Uuid) -> anyhow::Result<Vec<Series>>;
}

#[async_trait]
pub trait LabelRepo {
    async fn define(&self, name: &str, values: &[String]) -> anyhow::Result<LabelDefinition>;
}

#[async_trait]
pub trait ClassificationRepo {
    async fn set_classification(
        &self,
        series_id: Uuid,
        label_definition_id: Uuid,
        value: &str,
    ) -> anyhow::Result<Classification>;
    async fn get_classification(
        &self,
        series_id: Uuid,
        label_definition_id: Uuid,
    ) -> anyhow::Result<Option<Classification>>;
}

#[async_trait]
pub trait PointsRepo {
    async fn insert_timestamp_points(
        &self,
        series_id: Uuid,
        points: &[(chrono::DateTime<chrono::Utc>, f64, Value)],
    ) -> anyhow::Result<u64>;

    async fn insert_ordinal_points(
        &self,
        series_id: Uuid,
        points: &[(i64, f64, Value)],
    ) -> anyhow::Result<u64>;
}

#[derive(Clone)]
pub struct PostgresRepos {
    pool: DbPool,
}

impl PostgresRepos {
    pub fn new(pool: DbPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl DatasetRepo for PostgresRepos {
    async fn create(&self, name: &str, description: &str) -> anyhow::Result<Dataset> {
        let rec = sqlx::query_as!(
            Dataset,
            r#"
            INSERT INTO datasets (id, name, description)
            VALUES ($1, $2, $3)
            RETURNING id, name, description, created_at
            "#,
            Uuid::new_v4(),
            name,
            description
        )
        .fetch_one(&self.pool.0)
        .await?;
        Ok(rec)
    }
}

#[async_trait]
impl SeriesRepo for PostgresRepos {
    async fn create(
        &self,
        dataset_id: Uuid,
        name: &str,
        source_uri: &str,
        ordering: Ordering,
    ) -> anyhow::Result<Series> {
        let rec = sqlx::query!(
            r#"
            INSERT INTO series (id, dataset_id, name, source_uri, ordering)
            VALUES ($1, $2, $3, $4, $5)
            RETURNING id, dataset_id, name, source_uri, ordering, created_at
            "#,
            Uuid::new_v4(),
            dataset_id,
            name,
            source_uri,
            ordering.as_str()
        )
        .fetch_one(&self.pool.0)
        .await?;

        let ordering = rec.ordering.parse::<Ordering>()?;
        Ok(Series {
            id: rec.id,
            dataset_id: rec.dataset_id,
            name: rec.name,
            source_uri: rec.source_uri,
            ordering,
            created_at: rec.created_at,
        })
    }

    async fn get(&self, id: Uuid) -> anyhow::Result<Option<Series>> {
        let rec = sqlx::query!(
            r#"
            SELECT id, dataset_id, name, source_uri, ordering, created_at
            FROM series
            WHERE id = $1
            "#,
            id
        )
        .fetch_optional(&self.pool.0)
        .await?;

        if let Some(rec) = rec {
            let ordering = rec.ordering.parse::<Ordering>()?;
            Ok(Some(Series {
                id: rec.id,
                dataset_id: rec.dataset_id,
                name: rec.name,
                source_uri: rec.source_uri,
                ordering,
                created_at: rec.created_at,
            }))
        } else {
            Ok(None)
        }
    }

    async fn list_by_dataset(&self, dataset_id: Uuid) -> anyhow::Result<Vec<Series>> {
        let rows = sqlx::query!(
            r#"
            SELECT id, dataset_id, name, source_uri, ordering, created_at
            FROM series
            WHERE dataset_id = $1
            ORDER BY name
            "#,
            dataset_id
        )
        .fetch_all(&self.pool.0)
        .await?;

        let mut series = Vec::with_capacity(rows.len());
        for rec in rows {
            let ordering = rec.ordering.parse::<Ordering>()?;
            series.push(Series {
                id: rec.id,
                dataset_id: rec.dataset_id,
                name: rec.name,
                source_uri: rec.source_uri,
                ordering,
                created_at: rec.created_at,
            });
        }
        Ok(series)
    }
}

#[async_trait]
impl LabelRepo for PostgresRepos {
    async fn define(&self, name: &str, values: &[String]) -> anyhow::Result<LabelDefinition> {
        let rec = sqlx::query_as!(
            LabelDefinition,
            r#"
            INSERT INTO label_definitions (id, name, values)
            VALUES ($1, $2, $3)
            RETURNING id, name, values, created_at
            "#,
            Uuid::new_v4(),
            name,
            values
        )
        .fetch_one(&self.pool.0)
        .await?;
        Ok(rec)
    }
}

#[async_trait]
impl ClassificationRepo for PostgresRepos {
    async fn set_classification(
        &self,
        series_id: Uuid,
        label_definition_id: Uuid,
        value: &str,
    ) -> anyhow::Result<Classification> {
        let rec = sqlx::query_as!(
            Classification,
            r#"
            INSERT INTO series_classifications (series_id, label_definition_id, value)
            VALUES ($1, $2, $3)
            ON CONFLICT (series_id, label_definition_id)
            DO UPDATE SET value = EXCLUDED.value, applied_at = now()
            RETURNING series_id, label_definition_id, value, applied_at
            "#,
            series_id,
            label_definition_id,
            value
        )
        .fetch_one(&self.pool.0)
        .await?;
        Ok(rec)
    }

    async fn get_classification(
        &self,
        series_id: Uuid,
        label_definition_id: Uuid,
    ) -> anyhow::Result<Option<Classification>> {
        let rec = sqlx::query_as!(
            Classification,
            r#"
            SELECT series_id, label_definition_id, value, applied_at
            FROM series_classifications
            WHERE series_id = $1 AND label_definition_id = $2
            "#,
            series_id,
            label_definition_id
        )
        .fetch_optional(&self.pool.0)
        .await?;

        Ok(rec)
    }
}

#[async_trait]
impl PointsRepo for PostgresRepos {
    async fn insert_timestamp_points(
        &self,
        series_id: Uuid,
        points: &[(chrono::DateTime<chrono::Utc>, f64, Value)],
    ) -> anyhow::Result<u64> {
        let mut accepted = 0u64;
        let mut tx = self.pool.0.begin().await?;
        for (ts, value, meta) in points {
            let res = sqlx::query!(
                r#"
                INSERT INTO points_ts (series_id, ts, value, meta)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (series_id, ts) DO NOTHING
                "#,
                series_id,
                ts,
                value,
                meta
            )
            .execute(&mut *tx)
            .await?;
            accepted += res.rows_affected() as u64;
        }
        tx.commit().await?;
        Ok(accepted)
    }

    async fn insert_ordinal_points(
        &self,
        series_id: Uuid,
        points: &[(i64, f64, Value)],
    ) -> anyhow::Result<u64> {
        let mut accepted = 0u64;
        let mut tx = self.pool.0.begin().await?;
        for (ordinal, value, meta) in points {
            let res = sqlx::query!(
                r#"
                INSERT INTO points_ord (series_id, ordinal, value, meta)
                VALUES ($1, $2, $3, $4)
                ON CONFLICT (series_id, ordinal) DO NOTHING
                "#,
                series_id,
                ordinal,
                value,
                meta
            )
            .execute(&mut *tx)
            .await?;
            accepted += res.rows_affected() as u64;
        }
        tx.commit().await?;
        Ok(accepted)
    }
}
