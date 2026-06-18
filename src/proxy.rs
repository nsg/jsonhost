use axum::{
    body::Body,
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
};

/// Shared state: an HTTP client and the stathost backend base URL.
pub struct AppState {
    pub client: reqwest::Client,
    pub stathost_url: String,
}

impl AppState {
    pub fn new(stathost_url: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            stathost_url: stathost_url.trim_end_matches('/').to_string(),
        }
    }

    /// A document `cars/volvo` is stored in stathost as `cars/volvo.json`.
    pub fn doc_url(&self, collection: &str, slug: &str) -> String {
        format!("{}/{}/{}.json", self.stathost_url, collection, slug)
    }

    pub fn list_url(&self, collection: &str) -> String {
        format!("{}/{}/_meta/list", self.stathost_url, collection)
    }
}

/// Forward the client's `Authorization` header verbatim — including its absence.
/// jsonhost makes no auth decisions; stathost is the sole authority.
pub fn forward_auth(
    builder: reqwest::RequestBuilder,
    headers: &HeaderMap,
) -> reqwest::RequestBuilder {
    match headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
    {
        Some(value) => builder.header("authorization", value),
        None => builder,
    }
}

/// Send a request to stathost and relay its response transparently.
pub async fn forward(builder: reqwest::RequestBuilder) -> Response {
    match builder.send().await {
        Ok(resp) => relay(resp).await,
        Err(_) => (StatusCode::BAD_GATEWAY, "stathost backend unreachable").into_response(),
    }
}

/// Translate a stathost response into an axum response, preserving status,
/// content type, and body so backend auth errors reach the client unchanged.
pub async fn relay(resp: reqwest::Response) -> Response {
    let status = StatusCode::from_u16(resp.status().as_u16()).unwrap_or(StatusCode::BAD_GATEWAY);
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .map(str::to_string);

    let body = match resp.bytes().await {
        Ok(b) => b,
        Err(_) => return StatusCode::BAD_GATEWAY.into_response(),
    };

    let mut builder = Response::builder().status(status);
    if let Some(ct) = content_type {
        builder = builder.header(header::CONTENT_TYPE, ct);
    }
    builder
        .body(Body::from(body))
        .unwrap_or_else(|_| StatusCode::INTERNAL_SERVER_ERROR.into_response())
}
