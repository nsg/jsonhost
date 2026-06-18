use axum::{
    Json, Router,
    body::Bytes,
    extract::{Path, State},
    http::{HeaderMap, StatusCode, header},
    response::{IntoResponse, Response},
    routing::get,
};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::{Mutex, oneshot};
use tokio::time::sleep;

const TOKEN: &str = "token1";

// ---- A minimal in-memory stand-in for stathost ----
// Mirrors the behaviour jsonhost relies on: bearer-gated writes/list, public reads.

type Store = Arc<Mutex<HashMap<String, Vec<u8>>>>;

enum Auth {
    Missing,
    Wrong,
    Ok,
}

fn check_auth(headers: &HeaderMap) -> Auth {
    match headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
    {
        None => Auth::Missing,
        Some(t) if t == TOKEN => Auth::Ok,
        Some(_) => Auth::Wrong,
    }
}

fn deny(headers: &HeaderMap) -> Option<StatusCode> {
    match check_auth(headers) {
        Auth::Missing => Some(StatusCode::UNAUTHORIZED),
        Auth::Wrong => Some(StatusCode::FORBIDDEN),
        Auth::Ok => None,
    }
}

async fn mock_put(
    State(store): State<Store>,
    Path((bucket, path)): Path<(String, String)>,
    headers: HeaderMap,
    body: Bytes,
) -> StatusCode {
    if let Some(code) = deny(&headers) {
        return code;
    }
    store
        .lock()
        .await
        .insert(format!("{bucket}/{path}"), body.to_vec());
    StatusCode::CREATED
}

async fn mock_get(
    State(store): State<Store>,
    Path((bucket, path)): Path<(String, String)>,
) -> Response {
    let key = format!("{bucket}/{path}");
    match store.lock().await.get(&key) {
        Some(bytes) => {
            let ct = if key.ends_with(".json") {
                "application/json"
            } else {
                "application/octet-stream"
            };
            ([(header::CONTENT_TYPE, ct)], bytes.clone()).into_response()
        }
        None => StatusCode::NOT_FOUND.into_response(),
    }
}

async fn mock_delete(
    State(store): State<Store>,
    Path((bucket, path)): Path<(String, String)>,
    headers: HeaderMap,
) -> StatusCode {
    if let Some(code) = deny(&headers) {
        return code;
    }
    match store.lock().await.remove(&format!("{bucket}/{path}")) {
        Some(_) => StatusCode::NO_CONTENT,
        None => StatusCode::NOT_FOUND,
    }
}

async fn mock_list(
    State(store): State<Store>,
    Path(bucket): Path<String>,
    headers: HeaderMap,
) -> Response {
    if let Some(code) = deny(&headers) {
        return code.into_response();
    }
    let prefix = format!("{bucket}/");
    let files: Vec<String> = store
        .lock()
        .await
        .keys()
        .filter_map(|k| k.strip_prefix(&prefix).map(str::to_string))
        .collect();
    Json(files).into_response()
}

async fn spawn<F>(app: Router, ready: F) -> (SocketAddr, oneshot::Sender<()>)
where
    F: FnOnce(),
{
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let (tx, rx) = oneshot::channel();
    tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                rx.await.ok();
            })
            .await
            .unwrap();
    });
    ready();
    (addr, tx)
}

#[tokio::test]
async fn test_full_workflow() {
    // Backend mock
    let store: Store = Arc::new(Mutex::new(HashMap::new()));
    let mock = Router::new()
        .route("/{bucket}/_meta/list", get(mock_list))
        .route(
            "/{bucket}/{*path}",
            get(mock_get).put(mock_put).delete(mock_delete),
        )
        .with_state(store.clone());
    let (mock_addr, mock_shutdown) = spawn(mock, || {}).await;

    // jsonhost pointed at the mock
    let state = Arc::new(jsonhost::AppState::new(format!("http://{mock_addr}")));
    let (jh_addr, jh_shutdown) = spawn(jsonhost::router(state), || {}).await;

    sleep(Duration::from_millis(50)).await;

    let client = reqwest::Client::new();
    let url = |p: &str| format!("http://{jh_addr}{p}");

    // PUT valid JSON with token -> 201, stored as cars/volvo.json
    let resp = client
        .put(url("/cars/volvo"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .body(r#"{"brand":"Volvo"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    assert!(store.lock().await.contains_key("cars/volvo.json"));

    // GET public read (no token) -> 200, JSON content type
    let resp = client.get(url("/cars/volvo")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    assert_eq!(resp.headers()[header::CONTENT_TYPE], "application/json");
    assert_eq!(resp.text().await.unwrap(), r#"{"brand":"Volvo"}"#);

    // PUT without token -> 401 (relayed from backend)
    let resp = client
        .put(url("/cars/bmw"))
        .body(r#"{"brand":"BMW"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 401);

    // PUT wrong token -> 403
    let resp = client
        .put(url("/cars/bmw"))
        .header("Authorization", "Bearer wrong")
        .body(r#"{"brand":"BMW"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 403);

    // PUT invalid JSON -> 400, never reaches backend
    let resp = client
        .put(url("/cars/broken"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .body("not json{")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);
    assert!(!store.lock().await.contains_key("cars/broken.json"));

    // Invalid document name -> 400
    let resp = client
        .put(url("/cars/has space"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .body("{}")
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 400);

    // Nested slug round-trips
    let resp = client
        .put(url("/cars/eu/polestar"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .body(r#"{"brand":"Polestar"}"#)
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 201);
    assert!(store.lock().await.contains_key("cars/eu/polestar.json"));

    // List with token -> slugs with .json stripped, nesting kept
    let resp = client
        .get(url("/cars"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 200);
    let mut slugs: Vec<String> = resp.json().await.unwrap();
    slugs.sort();
    assert_eq!(slugs, vec!["eu/polestar".to_string(), "volvo".to_string()]);

    // List without token -> 401 relayed
    let resp = client.get(url("/cars")).send().await.unwrap();
    assert_eq!(resp.status(), 401);

    // DELETE with token -> 204, then gone
    let resp = client
        .delete(url("/cars/volvo"))
        .header("Authorization", format!("Bearer {TOKEN}"))
        .send()
        .await
        .unwrap();
    assert_eq!(resp.status(), 204);
    let resp = client.get(url("/cars/volvo")).send().await.unwrap();
    assert_eq!(resp.status(), 404);

    // OpenAPI spec
    let resp = client.get(url("/openapi.json")).send().await.unwrap();
    assert_eq!(resp.status(), 200);
    let spec: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(spec["info"]["title"], "jsonhost API");

    let _ = jh_shutdown.send(());
    let _ = mock_shutdown.send(());
}
