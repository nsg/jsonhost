use axum::{
    Json,
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
};
use std::sync::Arc;

use crate::proxy::{AppState, forward_auth, relay};
use crate::validate::is_valid_collection;

/// `GET /{collection}` — list document slugs in the collection.
///
/// Backed by stathost's `_meta/list`; the `.json` suffix is stripped so clients
/// see `["volvo", "volkswagen"]`. Non-success responses are relayed unchanged.
pub async fn list_documents(
    State(state): State<Arc<AppState>>,
    Path(collection): Path<String>,
    headers: HeaderMap,
) -> Response {
    if !is_valid_collection(&collection) {
        return StatusCode::NOT_FOUND.into_response();
    }

    let req = forward_auth(state.client.get(state.list_url(&collection)), &headers);
    let resp = match req.send().await {
        Ok(r) => r,
        Err(_) => return (StatusCode::BAD_GATEWAY, "stathost backend unreachable").into_response(),
    };

    if !resp.status().is_success() {
        return relay(resp).await;
    }

    let files: Vec<String> = match resp.json().await {
        Ok(f) => f,
        Err(_) => return StatusCode::BAD_GATEWAY.into_response(),
    };

    let docs: Vec<String> = files
        .into_iter()
        .filter_map(|f| f.strip_suffix(".json").map(str::to_string))
        .collect();

    Json(docs).into_response()
}

/// `GET /` — service banner.
pub async fn root_index() -> Response {
    Json(serde_json::json!({
        "service": "jsonhost",
        "openapi": "/openapi.json"
    }))
    .into_response()
}

/// `GET /openapi.json` — API specification.
pub async fn openapi() -> Response {
    let spec = serde_json::json!({
        "openapi": "3.0.3",
        "info": {
            "title": "jsonhost API",
            "version": "1.0.0",
            "description": "A JSON document store backed by stathost"
        },
        "paths": {
            "/{collection}/{slug}": {
                "get": {
                    "summary": "Fetch a JSON document",
                    "parameters": [
                        {"name": "collection", "in": "path", "required": true, "schema": {"type": "string"}},
                        {"name": "slug", "in": "path", "required": true, "schema": {"type": "string"}}
                    ],
                    "responses": {
                        "200": {"description": "Document content", "content": {"application/json": {}}},
                        "401": {"description": "Unauthorized"},
                        "404": {"description": "Document not found"}
                    }
                },
                "put": {
                    "summary": "Create or replace a JSON document",
                    "security": [{"bearerAuth": []}],
                    "parameters": [
                        {"name": "collection", "in": "path", "required": true, "schema": {"type": "string"}},
                        {"name": "slug", "in": "path", "required": true, "schema": {"type": "string"}}
                    ],
                    "requestBody": {
                        "required": true,
                        "content": {"application/json": {"schema": {"type": "object"}}}
                    },
                    "responses": {
                        "201": {"description": "Document stored"},
                        "400": {"description": "Invalid JSON or document name"},
                        "401": {"description": "Unauthorized"},
                        "403": {"description": "Forbidden"}
                    }
                },
                "delete": {
                    "summary": "Delete a JSON document",
                    "security": [{"bearerAuth": []}],
                    "parameters": [
                        {"name": "collection", "in": "path", "required": true, "schema": {"type": "string"}},
                        {"name": "slug", "in": "path", "required": true, "schema": {"type": "string"}}
                    ],
                    "responses": {
                        "204": {"description": "Document deleted"},
                        "401": {"description": "Unauthorized"},
                        "403": {"description": "Forbidden"},
                        "404": {"description": "Document not found"}
                    }
                }
            },
            "/{collection}": {
                "get": {
                    "summary": "List document slugs in a collection",
                    "parameters": [
                        {"name": "collection", "in": "path", "required": true, "schema": {"type": "string"}}
                    ],
                    "responses": {
                        "200": {
                            "description": "Document slugs",
                            "content": {"application/json": {"schema": {"type": "array", "items": {"type": "string"}}}}
                        },
                        "401": {"description": "Unauthorized"},
                        "403": {"description": "Forbidden"}
                    }
                }
            }
        },
        "components": {
            "securitySchemes": {
                "bearerAuth": {"type": "http", "scheme": "bearer"}
            }
        }
    });

    Json(spec).into_response()
}
