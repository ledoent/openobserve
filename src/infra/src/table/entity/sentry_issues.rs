//! `SeaORM` Entity for sentry_issues table

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Eq, Serialize, Deserialize)]
#[sea_orm(table_name = "sentry_issues")]
pub struct Model {
    /// KSUID (27 chars)
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: String,
    pub org_id: String,

    /// SHA256 hex fingerprint (64 chars), unique per org
    pub fingerprint: String,

    /// open, resolved, ignored
    pub status: String,

    /// "{exc_type}: {value}" truncated to 500 chars
    pub title: String,

    /// "{module} in {function}" or "filename:lineno"
    pub culprit: String,

    pub exception_type: String,
    pub platform: Option<String>,
    pub release: Option<String>,
    pub environment: Option<String>,

    /// Running count of events matched to this issue
    pub times_seen: i64,

    /// Approximate distinct user count
    pub users_seen: i64,

    /// Timestamps in microseconds
    pub first_seen: i64,
    pub last_seen: i64,

    /// UUID of the most recent event (for quick detail lookup)
    pub last_event_id: String,

    pub assigned_to: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
