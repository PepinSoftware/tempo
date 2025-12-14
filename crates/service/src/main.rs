use std::{net::SocketAddr, str::FromStr};

use config::Config;
use prost_types::Timestamp;
use serde::Deserialize;
use tonic::{transport::Server, Request, Response, Status};
use tracing_subscriber::{fmt, EnvFilter};
use uuid::Uuid;

use tempo_core::{Classification, Dataset, LabelDefinition, Ordering, Series};
use tempo_persistence::{
    ClassificationRepo, DbPool, DatasetRepo, LabelRepo, PointsRepo, PostgresRepos, SeriesRepo,
};

pub mod proto {
    tonic::include_proto!("tempo.v1");
}

use proto::tempo_service_server::{TempoService, TempoServiceServer};
use proto::{
    AppendPointsRequest, AppendPointsResponse, CreateDatasetRequest, CreateDatasetResponse,
    DefineLabelRequest, DefineLabelResponse, GetSeriesClassificationRequest,
    GetSeriesClassificationResponse, ListSeriesRequest, ListSeriesResponse,
    RegisterSeriesRequest, RegisterSeriesResponse, SetSeriesClassificationRequest,
    SetSeriesClassificationResponse,
};

#[derive(Debug, Deserialize)]
struct Settings {
    #[serde(default = "default_grpc_addr")]
    grpc_addr: String,
    #[serde(default = "default_database_url")]
    database_url: String,
    #[serde(default = "default_max_db_connections")]
    max_db_connections: u32,
    log_level: Option<String>,
}

fn default_grpc_addr() -> String {
    "0.0.0.0:50051".to_string()
}

fn default_database_url() -> String {
    "postgres://tempo:tempo@localhost:5432/tempo".to_string()
}

fn default_max_db_connections() -> u32 {
    5
}

fn load_settings() -> anyhow::Result<Settings> {
    let builder = Config::builder()
        .add_source(config::Environment::with_prefix("TEMPO").separator("__"))
        .add_source(config::File::with_name("tempo").required(false))
        .set_default("grpc_addr", default_grpc_addr())?
        .set_default("database_url", default_database_url())?
        .set_default("max_db_connections", default_max_db_connections())?;

    let settings = builder.build()?.try_deserialize()?;
    Ok(settings)
}

#[derive(Clone)]
struct TempoGrpc {
    repos: PostgresRepos,
}

#[tonic::async_trait]
impl TempoService for TempoGrpc {
    async fn create_dataset(
        &self,
        request: Request<CreateDatasetRequest>,
    ) -> Result<Response<CreateDatasetResponse>, Status> {
        let req = request.into_inner();
        let dataset = self
            .repos
            .create(&req.name, &req.description)
            .await
            .map_err(internal)?;
        Ok(Response::new(CreateDatasetResponse {
            dataset: Some(to_proto_dataset(&dataset)),
        }))
    }

    async fn register_series(
        &self,
        request: Request<RegisterSeriesRequest>,
    ) -> Result<Response<RegisterSeriesResponse>, Status> {
        let req = request.into_inner();
        let ordering = Ordering::from_str(&req.ordering).map_err(|_| {
            Status::invalid_argument("ordering must be either \"timestamp\" or \"ordinal\"")
        })?;
        let series = self
            .repos
            .create(
                Uuid::parse_str(&req.dataset_id).map_err(|_| Status::invalid_argument("dataset_id"))?,
                &req.name,
                &req.source_uri,
                ordering,
            )
            .await
            .map_err(internal)?;
        Ok(Response::new(RegisterSeriesResponse {
            series: Some(to_proto_series(&series)),
        }))
    }

    async fn define_label(
        &self,
        request: Request<DefineLabelRequest>,
    ) -> Result<Response<DefineLabelResponse>, Status> {
        let req = request.into_inner();
        if req.values.is_empty() {
            return Err(Status::invalid_argument("values must be non-empty"));
        }
        let label = self
            .repos
            .define(&req.name, &req.values)
            .await
            .map_err(internal)?;
        Ok(Response::new(DefineLabelResponse {
            label_definition: Some(to_proto_label(&label)),
        }))
    }

    async fn set_series_classification(
        &self,
        request: Request<SetSeriesClassificationRequest>,
    ) -> Result<Response<SetSeriesClassificationResponse>, Status> {
        let req = request.into_inner();
        let series_id =
            Uuid::parse_str(&req.series_id).map_err(|_| Status::invalid_argument("series_id"))?;
        let label_id = Uuid::parse_str(&req.label_definition_id)
            .map_err(|_| Status::invalid_argument("label_definition_id"))?;
        let classification = self
            .repos
            .set_classification(series_id, label_id, &req.value)
            .await
            .map_err(internal)?;
        Ok(Response::new(SetSeriesClassificationResponse {
            classification: Some(to_proto_classification(&classification)),
        }))
    }

    async fn get_series_classification(
        &self,
        request: Request<GetSeriesClassificationRequest>,
    ) -> Result<Response<GetSeriesClassificationResponse>, Status> {
        let req = request.into_inner();
        let series_id =
            Uuid::parse_str(&req.series_id).map_err(|_| Status::invalid_argument("series_id"))?;
        let label_id = Uuid::parse_str(&req.label_definition_id)
            .map_err(|_| Status::invalid_argument("label_definition_id"))?;
        let classification = self
            .repos
            .get_classification(series_id, label_id)
            .await
            .map_err(internal)?;
        let Some(classification) = classification else {
            return Err(Status::not_found("classification not found"));
        };
        Ok(Response::new(GetSeriesClassificationResponse {
            classification: Some(to_proto_classification(&classification)),
        }))
    }

    async fn list_series(
        &self,
        request: Request<ListSeriesRequest>,
    ) -> Result<Response<ListSeriesResponse>, Status> {
        let req = request.into_inner();
        let dataset_id =
            Uuid::parse_str(&req.dataset_id).map_err(|_| Status::invalid_argument("dataset_id"))?;
        let series = self
            .repos
            .list_by_dataset(dataset_id)
            .await
            .map_err(internal)?;
        let proto_series = series.iter().map(to_proto_series).collect();
        Ok(Response::new(ListSeriesResponse {
            series: proto_series,
        }))
    }

    async fn append_points(
        &self,
        request: Request<AppendPointsRequest>,
    ) -> Result<Response<AppendPointsResponse>, Status> {
        let req = request.into_inner();
        let series_id =
            Uuid::parse_str(&req.series_id).map_err(|_| Status::invalid_argument("series_id"))?;
        let series = self
            .repos
            .get(series_id)
            .await
            .map_err(internal)?
            .ok_or_else(|| Status::not_found("series not found"))?;

        if req.points.is_empty() {
            return Ok(Response::new(AppendPointsResponse { accepted: 0 }));
        }

        match series.ordering {
            Ordering::Timestamp => {
                let mut last = None;
                let mut payload = Vec::with_capacity(req.points.len());
                for p in req.points {
                    let ts = p
                        .timestamp
                        .ok_or_else(|| Status::invalid_argument("timestamp required"))?;
                    let dt = to_chrono(ts)?;
                    if let Some(prev) = last {
                        if dt < prev {
                            return Err(Status::invalid_argument("timestamps must be monotonic"));
                        }
                    }
                    last = Some(dt);
                    let meta = serde_json::to_value(p.meta).map_err(internal)?;
                    payload.push((dt, p.value, meta));
                }
                let accepted = self
                    .repos
                    .insert_timestamp_points(series_id, &payload)
                    .await
                    .map_err(internal)?;
                Ok(Response::new(AppendPointsResponse { accepted }))
            }
            Ordering::Ordinal => {
                let mut last: Option<i64> = None;
                let mut payload = Vec::with_capacity(req.points.len());
                for p in req.points {
                    let ord = p.ordinal.ok_or_else(|| Status::invalid_argument("ordinal required"))? as i64;
                    if let Some(prev) = last {
                        if ord < prev {
                            return Err(Status::invalid_argument("ordinals must be monotonic"));
                        }
                    }
                    last = Some(ord);
                    let meta = serde_json::to_value(p.meta).map_err(internal)?;
                    payload.push((ord, p.value, meta));
                }
                let accepted = self
                    .repos
                    .insert_ordinal_points(series_id, &payload)
                    .await
                    .map_err(internal)?;
                Ok(Response::new(AppendPointsResponse { accepted }))
            }
        }
    }
}

fn to_proto_dataset(d: &Dataset) -> proto::Dataset {
    proto::Dataset {
        id: d.id.to_string(),
        name: d.name.clone(),
        description: d.description.clone(),
        created_at: Some(to_timestamp(d.created_at)),
    }
}

fn to_proto_series(s: &Series) -> proto::Series {
    proto::Series {
        id: s.id.to_string(),
        dataset_id: s.dataset_id.to_string(),
        name: s.name.clone(),
        source_uri: s.source_uri.clone(),
        ordering: s.ordering.as_str().to_string(),
        created_at: Some(to_timestamp(s.created_at)),
    }
}

fn to_proto_label(l: &LabelDefinition) -> proto::LabelDefinition {
    proto::LabelDefinition {
        id: l.id.to_string(),
        name: l.name.clone(),
        values: l.values.clone(),
        created_at: Some(to_timestamp(l.created_at)),
    }
}

fn to_proto_classification(c: &Classification) -> proto::Classification {
    proto::Classification {
        series_id: c.series_id.to_string(),
        label_definition_id: c.label_definition_id.to_string(),
        value: c.value.clone(),
        applied_at: Some(to_timestamp(c.applied_at)),
    }
}

fn to_timestamp(dt: chrono::DateTime<chrono::Utc>) -> Timestamp {
    Timestamp {
        seconds: dt.timestamp(),
        nanos: dt.timestamp_subsec_nanos() as i32,
    }
}

fn to_chrono(ts: Timestamp) -> Result<chrono::DateTime<chrono::Utc>, Status> {
    chrono::DateTime::from_timestamp(ts.seconds, ts.nanos as u32)
        .ok_or_else(|| Status::invalid_argument("invalid timestamp"))
}

fn internal<E: std::error::Error + 'static>(err: E) -> Status {
    Status::internal(err.to_string())
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let settings = load_settings()?;
    let filter = settings
        .log_level
        .clone()
        .unwrap_or_else(|| "info".to_string());
    fmt().with_env_filter(EnvFilter::new(filter)).init();

    let addr: SocketAddr = settings.grpc_addr.parse()?;
    tracing::info!(%addr, "starting gRPC service");

    let pool = DbPool::connect(&settings.database_url, settings.max_db_connections).await?;
    let repos = PostgresRepos::new(pool);
    let svc = TempoGrpc { repos };

    Server::builder()
        .add_service(TempoServiceServer::new(svc))
        .serve(addr)
        .await?;

    Ok(())
}
