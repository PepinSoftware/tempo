use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Ordering {
    Timestamp,
    Ordinal,
}

impl Ordering {
    pub fn as_str(&self) -> &'static str {
        match self {
            Ordering::Timestamp => "timestamp",
            Ordering::Ordinal => "ordinal",
        }
    }
}

impl std::str::FromStr for Ordering {
    type Err = DomainError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "timestamp" => Ok(Ordering::Timestamp),
            "ordinal" => Ok(Ordering::Ordinal),
            _ => Err(DomainError::InvalidOrdering),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Dataset {
    pub id: Uuid,
    pub name: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Series {
    pub id: Uuid,
    pub dataset_id: Uuid,
    pub name: String,
    pub source_uri: String,
    pub ordering: Ordering,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LabelDefinition {
    pub id: Uuid,
    pub name: String,
    pub values: Vec<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Classification {
    pub series_id: Uuid,
    pub label_definition_id: Uuid,
    pub value: String,
    pub applied_at: DateTime<Utc>,
}

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("label value invalid for definition")]
    InvalidLabelValue,
    #[error("series ordering mismatch")]
    OrderingMismatch,
    #[error("invalid ordering value")]
    InvalidOrdering,
    #[error("ordering must be monotonic")]
    NonMonotonicPoints,
    #[error("not found")]
    NotFound,
    #[error("conflict")]
    Conflict,
}
