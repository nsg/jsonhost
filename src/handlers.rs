use axum::{
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::proxy::{AppState, forward, forward_auth};
use crate::validate::{is_valid_collection, is_valid_slug};

/// `PUT /{collection}/{slug}` — validate JSON, then store as `{slug}.json` in stathost.
pub async fn put_document(
    State(state): State<Arc<AppState>>,
    Path((collection, slug)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> Response {
    if !is_valid_collection(&collection) || !is_valid_slug(&slug) {
        return (
            StatusCode::BAD_REQUEST,
            "Invalid collection or document name",
        )
            .into_response();
    }

    if serde_json::from_slice::<serde_json::Value>(&body).is_err() {
        return (StatusCode::BAD_REQUEST, "Request body is not valid JSON").into_response();
    }

    let req = state
        .client
        .put(state.doc_url(&collection, &slug))
        .header("content-type", "application/json")
        .body(body);

    forward(forward_auth(req, &headers)).await
}

/// `GET /{collection}/{slug}` — fetch the JSON document.
pub async fn get_document(
    State(state): State<Arc<AppState>>,
    Path((collection, slug)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if !is_valid_collection(&collection) || !is_valid_slug(&slug) {
        return StatusCode::NOT_FOUND.into_response();
    }

    let req = state.client.get(state.doc_url(&collection, &slug));
    forward(forward_auth(req, &headers)).await
}

/// `DELETE /{collection}/{slug}` — remove the JSON document.
pub async fn delete_document(
    State(state): State<Arc<AppState>>,
    Path((collection, slug)): Path<(String, String)>,
    headers: HeaderMap,
) -> Response {
    if !is_valid_collection(&collection) || !is_valid_slug(&slug) {
        return StatusCode::NOT_FOUND.into_response();
    }

    let req = state.client.delete(state.doc_url(&collection, &slug));
    forward(forward_auth(req, &headers)).await
}
