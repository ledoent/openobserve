// Copyright 2026 OpenObserve Inc.
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU Affero General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU Affero General Public License for more details.
//
// You should have received a copy of the GNU Affero General Public License
// along with this program.  If not, see <http://www.gnu.org/licenses/>.

//! REST endpoints for Sentry issue management.
//!
//! All routes are authenticated via standard OpenObserve token auth and are
//! nested under `/api/{org_id}/sentry/`.

use axum::{
    Json,
    extract::{Path, Query},
    response::Response,
};
use infra::table::sentry_issues::{self, ListIssuesParams};
use serde::Deserialize;

use crate::common::{
    meta::http::HttpResponse as MetaHttpResponse,
    utils::auth::UserEmail,
};
use crate::handler::http::extractors::Headers;

// ---------------------------------------------------------------------------
// Request / response types
// ---------------------------------------------------------------------------

#[derive(Debug, Deserialize)]
pub struct ListIssuesQuery {
    pub status: Option<String>,
    pub environment: Option<String>,
    pub release: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: u64,
    #[serde(default)]
    pub offset: u64,
}

fn default_limit() -> u64 {
    25
}

#[derive(Debug, Deserialize)]
pub struct UpdateIssueBody {
    pub status: Option<String>,
    pub assigned_to: Option<serde_json::Value>,
}

// ---------------------------------------------------------------------------
// Handlers
// ---------------------------------------------------------------------------

/// List issues for an org.
///
/// `GET /api/{org_id}/sentry/issues`
pub async fn list(
    Path(org_id): Path<String>,
    Query(params): Query<ListIssuesQuery>,
    Headers(_user_email): Headers<UserEmail>,
) -> Response {
    let list_params = ListIssuesParams {
        status: params.status.as_deref(),
        environment: params.environment.as_deref(),
        release: params.release.as_deref(),
        limit: params.limit,
        offset: params.offset,
    };

    let count_status = params.status.as_deref();
    let (issues, total) = match tokio::try_join!(
        sentry_issues::list(&org_id, &list_params),
        sentry_issues::count(&org_id, count_status),
    ) {
        Ok(r) => r,
        Err(e) => return MetaHttpResponse::internal_error(e),
    };

    MetaHttpResponse::json(serde_json::json!({
        "issues": issues,
        "total": total,
        "limit": params.limit,
        "offset": params.offset,
    }))
}

/// Get a single issue by ID.
///
/// `GET /api/{org_id}/sentry/issues/{id}`
pub async fn get(
    Path((org_id, id)): Path<(String, String)>,
    Headers(_user_email): Headers<UserEmail>,
) -> Response {
    match sentry_issues::get(&org_id, &id).await {
        Ok(Some(issue)) => MetaHttpResponse::json(issue),
        Ok(None) => MetaHttpResponse::not_found("issue not found"),
        Err(e) => MetaHttpResponse::internal_error(e),
    }
}

/// Update issue status and/or assignee.
///
/// `PATCH /api/{org_id}/sentry/issues/{id}`
pub async fn update(
    Path((org_id, id)): Path<(String, String)>,
    Headers(_user_email): Headers<UserEmail>,
    Json(body): Json<UpdateIssueBody>,
) -> Response {
    // Validate status if provided
    if let Some(s) = body.status.as_deref() {
        if !matches!(s, "open" | "resolved" | "ignored") {
            return MetaHttpResponse::bad_request(format!(
                "invalid status '{s}': must be one of open, resolved, ignored"
            ));
        }
    }

    // assigned_to: null means unassign, string means assign, anything else is invalid
    let assigned_to: Option<Option<&str>> = match body.assigned_to.as_ref() {
        None => None,
        Some(serde_json::Value::Null) => Some(None),
        Some(serde_json::Value::String(s)) => Some(Some(s.as_str())),
        Some(_) => {
            return MetaHttpResponse::bad_request(
                "assigned_to must be a string (email) or null to unassign",
            );
        }
    };

    match sentry_issues::update_issue(&org_id, &id, body.status.as_deref(), assigned_to).await {
        Ok(issue) => MetaHttpResponse::json(issue),
        Err(e) => MetaHttpResponse::internal_error(e),
    }
}

/// Delete an issue.
///
/// `DELETE /api/{org_id}/sentry/issues/{id}`
pub async fn delete(
    Path((org_id, id)): Path<(String, String)>,
    Headers(_user_email): Headers<UserEmail>,
) -> Response {
    match sentry_issues::delete(&org_id, &id).await {
        Ok(()) => MetaHttpResponse::ok(""),
        Err(e) => MetaHttpResponse::internal_error(e),
    }
}
