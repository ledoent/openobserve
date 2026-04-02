// Copyright 2026 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! Sentry Issues Table Operations
//!
//! CRUD operations for error issues grouped by fingerprint.

use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder, Set,
    sea_query::{Expr, OnConflict},
};
use svix_ksuid::KsuidLike;

use super::entity::sentry_issues;
use crate::{
    db::{ORM_CLIENT, connect_to_orm},
    errors::{self, DbError, Error},
};

/// Get an issue by ID
pub async fn get(org_id: &str, id: &str) -> Result<Option<sentry_issues::Model>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    sentry_issues::Entity::find_by_id(id)
        .filter(sentry_issues::Column::OrgId.eq(org_id))
        .one(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))
}

/// Get an issue by fingerprint
pub async fn get_by_fingerprint(
    org_id: &str,
    fingerprint: &str,
) -> Result<Option<sentry_issues::Model>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    sentry_issues::Entity::find()
        .filter(sentry_issues::Column::OrgId.eq(org_id))
        .filter(sentry_issues::Column::Fingerprint.eq(fingerprint))
        .one(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))
}

pub struct NewIssueData<'a> {
    pub fingerprint: &'a str,
    pub title: &'a str,
    pub culprit: &'a str,
    pub exception_type: &'a str,
    pub platform: Option<&'a str>,
    pub release: Option<&'a str>,
    pub environment: Option<&'a str>,
    pub event_id: &'a str,
    pub event_ts: i64,
    pub user_id: Option<&'a str>,
}

/// Upsert an issue: insert on first occurrence, atomically increment counters on repeat.
///
/// Uses `INSERT … ON CONFLICT (org_id, fingerprint) DO UPDATE` so concurrent ingestion
/// of the same fingerprint is race-free.
pub async fn upsert_issue(org_id: &str, data: &NewIssueData<'_>) -> Result<(), errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let now = chrono::Utc::now().timestamp_micros();
    let id = svix_ksuid::Ksuid::new(None, None).to_string();
    let users_delta: i64 = if data.user_id.is_some() { 1 } else { 0 };

    let model = sentry_issues::ActiveModel {
        id: Set(id),
        org_id: Set(org_id.to_string()),
        fingerprint: Set(data.fingerprint.to_string()),
        status: Set("open".to_string()),
        title: Set(data.title.chars().take(500).collect()),
        culprit: Set(data.culprit.chars().take(500).collect()),
        exception_type: Set(data.exception_type.to_string()),
        platform: Set(data.platform.map(str::to_string)),
        release: Set(data.release.map(str::to_string)),
        environment: Set(data.environment.map(str::to_string)),
        times_seen: Set(1),
        users_seen: Set(users_delta),
        first_seen: Set(data.event_ts),
        last_seen: Set(data.event_ts),
        last_event_id: Set(data.event_id.to_string()),
        assigned_to: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
    };

    sentry_issues::Entity::insert(model)
        .on_conflict(
            OnConflict::columns([sentry_issues::Column::OrgId, sentry_issues::Column::Fingerprint])
                .value(
                    sentry_issues::Column::TimesSeen,
                    Expr::col((sentry_issues::Entity, sentry_issues::Column::TimesSeen)).add(1i64),
                )
                .value(
                    sentry_issues::Column::UsersSeen,
                    Expr::col((sentry_issues::Entity, sentry_issues::Column::UsersSeen))
                        .add(users_delta),
                )
                .value(sentry_issues::Column::LastSeen, Expr::value(data.event_ts))
                .value(
                    sentry_issues::Column::LastEventId,
                    Expr::value(data.event_id.to_string()),
                )
                .value(sentry_issues::Column::UpdatedAt, Expr::value(now))
                .to_owned(),
        )
        .exec(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(())
}

pub struct ListIssuesParams<'a> {
    pub status: Option<&'a str>,
    pub environment: Option<&'a str>,
    pub release: Option<&'a str>,
    pub limit: u64,
    pub offset: u64,
}

/// List issues for an org with optional filters
pub async fn list(
    org_id: &str,
    params: &ListIssuesParams<'_>,
) -> Result<Vec<sentry_issues::Model>, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    let mut query = sentry_issues::Entity::find()
        .filter(sentry_issues::Column::OrgId.eq(org_id))
        .order_by_desc(sentry_issues::Column::LastSeen);

    if let Some(s) = params.status {
        query = query.filter(sentry_issues::Column::Status.eq(s));
    }
    if let Some(env) = params.environment {
        query = query.filter(sentry_issues::Column::Environment.eq(env));
    }
    if let Some(rel) = params.release {
        query = query.filter(sentry_issues::Column::Release.eq(rel));
    }

    let limit = params.limit.max(1);
    let page = params.offset / limit;

    query
        .paginate(client, limit)
        .fetch_page(page)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))
}

/// Count issues for an org with optional status filter
pub async fn count(org_id: &str, status: Option<&str>) -> Result<u64, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    let mut query =
        sentry_issues::Entity::find().filter(sentry_issues::Column::OrgId.eq(org_id));

    if let Some(s) = status {
        query = query.filter(sentry_issues::Column::Status.eq(s));
    }

    query
        .count(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))
}

/// Update issue status and/or assignee
pub async fn update_issue(
    org_id: &str,
    id: &str,
    status: Option<&str>,
    assigned_to: Option<Option<&str>>,
) -> Result<sentry_issues::Model, errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;
    let now = chrono::Utc::now().timestamp_micros();

    let issue = get(org_id, id)
        .await?
        .ok_or_else(|| Error::DbError(DbError::SeaORMError("Issue not found".to_string())))?;

    let mut active: sentry_issues::ActiveModel = issue.into();
    active.updated_at = Set(now);

    if let Some(s) = status {
        active.status = Set(s.to_string());
    }
    if let Some(a) = assigned_to {
        active.assigned_to = Set(a.map(str::to_string));
    }

    active
        .update(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))
}

/// Delete an issue
pub async fn delete(org_id: &str, id: &str) -> Result<(), errors::Error> {
    let client = ORM_CLIENT.get_or_init(connect_to_orm).await;

    sentry_issues::Entity::delete_many()
        .filter(sentry_issues::Column::OrgId.eq(org_id))
        .filter(sentry_issues::Column::Id.eq(id))
        .exec(client)
        .await
        .map_err(|e| Error::DbError(DbError::SeaORMError(e.to_string())))?;

    Ok(())
}
