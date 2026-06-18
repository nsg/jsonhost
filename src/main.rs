use axum::{extract::Request, middleware::Next, response::Response};
use jsonhost::AppState;
use jsonhost::config::AppConfig;
use std::{path::PathBuf, sync::Arc, time::Instant};

async fn access_log(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let uri = request.uri().clone();
    let start = Instant::now();

    let response = next.run(request).await;

    let elapsed = start.elapsed();
    println!(
        "{} {} {} {}ms",
        method,
        uri,
        response.status().as_u16(),
        elapsed.as_millis()
    );

    response
}

#[tokio::main]
async fn main() {
    let config_path = std::env::args()
        .skip_while(|a| a != "--config")
        .nth(1)
        .map(PathBuf::from);

    let config = AppConfig::load(config_path.as_deref()).unwrap_or_else(|e| {
        eprintln!("Failed to load config: {}", e);
        std::process::exit(1);
    });

    let state = Arc::new(AppState::new(config.server.stathost_url.clone()));

    let app = jsonhost::router(state).layer(axum::middleware::from_fn(access_log));

    let addr = format!("{}:{}", config.server.host, config.server.port);
    println!(
        "jsonhost listening on {} -> stathost {}",
        addr, config.server.stathost_url
    );

    let listener = match tokio::net::TcpListener::bind(&addr).await {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    if let Err(e) = axum::serve(listener, app).await {
        eprintln!("Server error: {}", e);
        std::process::exit(1);
    }
}
