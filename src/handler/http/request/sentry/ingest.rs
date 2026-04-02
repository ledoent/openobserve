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

//! Sentry SDK ingestion endpoints.
//!
//! Implements two compatible endpoints:
//! - `POST /sentry/{org_id}/api/{project_id}/envelope/` — modern envelope format
//! - `POST /sentry/{org_id}/api/{project_id}/store/`    — legacy JSON store format
//!
//! DSN format for SDK configuration:
//! `https://{api_token}@{host}/sentry/{org_id}/{project_id}`

use std::collections::HashMap;

use axum::{
    body::Bytes,
    extract::Path,
    response::Response,
};
use config::utils::json;
use infra::table::sentry_issues::{self, NewIssueData};
use serde::{Deserialize, Serialize};
use sha256::digest as sha256_digest;
use svix_ksuid::KsuidLike;

use crate::{
    common::{
        meta::{
            http::HttpResponse as MetaHttpResponse,
            ingestion::{IngestUser, IngestionRequest},
        },
        utils::auth::UserEmail,
    },
    handler::http::extractors::Headers,
    service::logs,
};

pub const SENTRY_ERRORS_STREAM: &str = "_sentry_errors";

// ---------------------------------------------------------------------------
// Sentry payload types
// ---------------------------------------------------------------------------

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct SentryEventPayload {
    pub event_id: Option<String>,
    pub timestamp: Option<serde_json::Value>,
    pub level: Option<String>,
    pub platform: Option<String>,
    pub release: Option<String>,
    pub environment: Option<String>,
    pub server_name: Option<String>,
    pub transaction: Option<String>,
    pub message: Option<String>,
    pub exception: Option<SentryException>,
    pub user: Option<SentryUser>,
    pub tags: Option<serde_json::Value>,
    pub extra: Option<serde_json::Value>,
    pub breadcrumbs: Option<serde_json::Value>,
    pub request: Option<serde_json::Value>,
    pub contexts: Option<serde_json::Value>,
    pub sdk: Option<SentrySdk>,
    // SDK may send a custom fingerprint array — honour it for grouping
    pub fingerprint: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct SentryException {
    pub values: Vec<SentryExceptionValue>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct SentryExceptionValue {
    #[serde(rename = "type")]
    pub exc_type: Option<String>,
    pub value: Option<String>,
    pub module: Option<String>,
    pub stacktrace: Option<SentryStacktrace>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct SentryStacktrace {
    pub frames: Vec<SentryFrame>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct SentryFrame {
    pub filename: Option<String>,
    pub function: Option<String>,
    pub module: Option<String>,
    pub lineno: Option<i64>,
    pub colno: Option<i64>,
    pub context_line: Option<String>,
    pub in_app: Option<bool>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct SentryUser {
    pub id: Option<String>,
    pub email: Option<String>,
    pub username: Option<String>,
    pub ip_address: Option<String>,
}

#[derive(Debug, Default, Deserialize, Serialize, Clone)]
pub struct SentrySdk {
    pub name: Option<String>,
    pub version: Option<String>,
}

// ---------------------------------------------------------------------------
// Fingerprint computation
// ---------------------------------------------------------------------------

/// Compute a fingerprint for grouping similar errors.
///
/// If the SDK supplied a custom `fingerprint` array, use that.
/// Otherwise derive from `{exc_type}:{top_frame_function}:{top_frame_module}`,
/// falling back to the event message.
fn compute_fingerprint(payload: &SentryEventPayload) -> String {
    // Honour SDK-supplied fingerprint override
    if let Some(fp) = &payload.fingerprint {
        if !fp.is_empty() && fp != &["{{ default }}"] {
            return sha256_digest(fp.join(":"));
        }
    }

    // Derive from the last exception value's top (last) in-app frame
    if let Some(exc) = &payload.exception {
        if let Some(last_exc) = exc.values.last() {
            let exc_type = last_exc.exc_type.as_deref().unwrap_or("UnknownError");

            let top_frame = last_exc
                .stacktrace
                .as_ref()
                .and_then(|st| {
                    // Prefer last in_app frame; fall back to absolute last frame
                    st.frames
                        .iter()
                        .rev()
                        .find(|f| f.in_app.unwrap_or(false))
                        .or_else(|| st.frames.last())
                });

            let fn_name = top_frame
                .and_then(|f| f.function.as_deref())
                .unwrap_or("");
            let module = top_frame
                .and_then(|f| f.module.as_deref().or(f.filename.as_deref()))
                .unwrap_or("");

            return sha256_digest(format!("{exc_type}:{fn_name}:{module}"));
        }
    }

    // Fallback: hash the message
    let msg = payload.message.as_deref().unwrap_or("unknown");
    sha256_digest(msg)
}

// ---------------------------------------------------------------------------
// Title / culprit helpers
// ---------------------------------------------------------------------------

fn make_title(payload: &SentryEventPayload) -> String {
    if let Some(exc) = &payload.exception {
        if let Some(last_exc) = exc.values.last() {
            let t = last_exc.exc_type.as_deref().unwrap_or("Error");
            let v = last_exc.value.as_deref().unwrap_or("");
            if v.is_empty() {
                return t.to_string();
            }
            return format!("{t}: {v}");
        }
    }
    payload
        .message
        .clone()
        .unwrap_or_else(|| "<no message>".to_string())
}

fn make_culprit(payload: &SentryEventPayload) -> String {
    // Try transaction first
    if let Some(tx) = &payload.transaction {
        if !tx.is_empty() {
            return tx.clone();
        }
    }

    // Then top frame info
    if let Some(exc) = &payload.exception {
        if let Some(last_exc) = exc.values.last() {
            let top_frame = last_exc.stacktrace.as_ref().and_then(|st| {
                st.frames
                    .iter()
                    .rev()
                    .find(|f| f.in_app.unwrap_or(false))
                    .or_else(|| st.frames.last())
            });
            if let Some(frame) = top_frame {
                let module = frame
                    .module
                    .as_deref()
                    .or(frame.filename.as_deref())
                    .unwrap_or("");
                let func = frame.function.as_deref().unwrap_or("");
                if !module.is_empty() && !func.is_empty() {
                    return format!("{module} in {func}");
                }
                if !module.is_empty() {
                    return module.to_string();
                }
            }
        }
    }
    String::new()
}

// ---------------------------------------------------------------------------
// Timestamp parsing
// ---------------------------------------------------------------------------

/// Parse Sentry timestamp (ISO8601 string or float unix seconds) to microseconds.
fn parse_timestamp(ts: &serde_json::Value) -> i64 {
    match ts {
        serde_json::Value::Number(n) => {
            let secs = n.as_f64().unwrap_or(0.0);
            (secs * 1_000_000.0) as i64
        }
        serde_json::Value::String(s) => {
            // Try parsing as float first
            if let Ok(f) = s.parse::<f64>() {
                return (f * 1_000_000.0) as i64;
            }
            // Try ISO8601
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
                return dt.timestamp_micros();
            }
            chrono::Utc::now().timestamp_micros()
        }
        _ => chrono::Utc::now().timestamp_micros(),
    }
}

// ---------------------------------------------------------------------------
// Core processing
// ---------------------------------------------------------------------------

/// Process a parsed Sentry event payload: upsert the issue and ingest the event.
async fn process_sentry_event(
    org_id: &str,
    _project_id: &str,
    payload: SentryEventPayload,
    user_email: &str,
) -> Result<String, String> {
    let event_id = payload
        .event_id
        .clone()
        .unwrap_or_else(|| svix_ksuid::Ksuid::new(None, None).to_string());

    let event_ts = payload
        .timestamp
        .as_ref()
        .map(parse_timestamp)
        .unwrap_or_else(|| chrono::Utc::now().timestamp_micros());

    let fingerprint = compute_fingerprint(&payload);
    let title = make_title(&payload);
    let culprit = make_culprit(&payload);
    let exc_type = payload
        .exception
        .as_ref()
        .and_then(|e| e.values.last())
        .and_then(|v| v.exc_type.clone())
        .unwrap_or_else(|| "Error".to_string());

    let user_id = payload.user.as_ref().and_then(|u| {
        u.id.as_deref()
            .or(u.email.as_deref())
            .or(u.username.as_deref())
    });

    // Upsert the issue record (best-effort; don't fail ingestion if this errors)
    let issue_data = NewIssueData {
        fingerprint: &fingerprint,
        title: &title,
        culprit: &culprit,
        exception_type: &exc_type,
        platform: payload.platform.as_deref(),
        release: payload.release.as_deref(),
        environment: payload.environment.as_deref(),
        event_id: &event_id,
        event_ts,
        user_id,
    };
    // Best-effort: don't fail ingestion if the issues table update fails.
    if let Err(e) = sentry_issues::upsert_issue(org_id, &issue_data).await {
        log::warn!("[sentry::ingest] upsert_issue failed org={org_id}: {e}");
    }

    // Build a flat JSON record for the event stream, including the fingerprint
    // and issue metadata so it can be queried directly.
    let mut record = serde_json::to_value(&payload)
        .map_err(|e| format!("serialize error: {e}"))?;

    if let serde_json::Value::Object(ref mut map) = record {
        map.insert("event_id".to_string(), serde_json::Value::String(event_id.clone()));
        map.insert("fingerprint".to_string(), serde_json::Value::String(fingerprint));
        map.insert("title".to_string(), serde_json::Value::String(title));
        map.insert("culprit".to_string(), serde_json::Value::String(culprit));
        map.insert("exception_type".to_string(), serde_json::Value::String(exc_type));
        // Ensure _timestamp is set so OO indexes it correctly
        if !map.contains_key("_timestamp") {
            map.insert("_timestamp".to_string(), serde_json::Value::Number(
                serde_json::Number::from(event_ts),
            ));
        }
    }

    // Wrap as a JSON array (multi-line format expected by IngestionData::Multi)
    let body = format!("{record}\n").into_bytes();

    match logs::ingest::ingest(
        0,
        org_id,
        SENTRY_ERRORS_STREAM,
        IngestionRequest::Sentry(Bytes::from(body)),
        IngestUser::from_user_email(user_email.to_string()),
        None,
        false,
    )
    .await
    {
        Ok(_) => Ok(event_id),
        Err(e) => Err(format!("ingest error: {e}")),
    }
}

// ---------------------------------------------------------------------------
// Parse helpers
// ---------------------------------------------------------------------------

/// Parse the Sentry envelope format.
///
/// Envelopes are newline-delimited sections:
/// ```text
/// {envelope_header}\n
/// {item_header}\n
/// {item_body}\n
/// ```
/// Multiple items can follow in a single envelope; we process only `type == "event"`.
fn parse_envelope(body: &Bytes) -> Option<SentryEventPayload> {
    let text = std::str::from_utf8(body).ok()?;
    let mut lines = text.lines();

    // Skip envelope header (line 0)
    let _envelope_header = lines.next()?;

    // Walk item pairs until we find an "event" item or exhaust the envelope
    while let Some(item_header_str) = lines.next() {
        let Some(item_payload_str) = lines.next() else {
            break;
        };

        let item_header: HashMap<String, serde_json::Value> =
            json::from_str(item_header_str).unwrap_or_default();

        let item_type = item_header
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("");

        if item_type == "event" {
            return json::from_str(item_payload_str).ok();
        }
        // Skip non-event items (transactions, sessions, metrics, etc.)
    }
    None
}

// ---------------------------------------------------------------------------
// HTTP handlers
// ---------------------------------------------------------------------------

/// Sentry envelope ingestion endpoint (modern SDKs).
///
/// `POST /sentry/{org_id}/api/{project_id}/envelope/`
pub async fn envelope(
    Path((org_id, project_id)): Path<(String, String)>,
    Headers(user_email): Headers<UserEmail>,
    body: Bytes,
) -> Response {
    let user_email = &user_email.user_id;

    let payload = match parse_envelope(&body) {
        Some(p) => p,
        None => {
            // No "event" item found — silently accept (SDK may send session-only envelopes)
            return MetaHttpResponse::ok("{}");
        }
    };

    match process_sentry_event(&org_id, &project_id, payload, user_email).await {
        Ok(event_id) => MetaHttpResponse::json(serde_json::json!({"id": event_id})),
        Err(e) => MetaHttpResponse::internal_error(e),
    }
}

/// Sentry store ingestion endpoint (legacy SDKs).
///
/// `POST /sentry/{org_id}/api/{project_id}/store/`
pub async fn store(
    Path((org_id, project_id)): Path<(String, String)>,
    Headers(user_email): Headers<UserEmail>,
    body: Bytes,
) -> Response {
    let user_email = &user_email.user_id;

    let payload: SentryEventPayload = match json::from_slice(&body) {
        Ok(p) => p,
        Err(e) => return MetaHttpResponse::bad_request(format!("invalid JSON: {e}")),
    };

    match process_sentry_event(&org_id, &project_id, payload, user_email).await {
        Ok(event_id) => MetaHttpResponse::json(serde_json::json!({"id": event_id})),
        Err(e) => MetaHttpResponse::internal_error(e),
    }
}
