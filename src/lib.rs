use axum::{Router, routing::get};
use std::sync::Arc;

pub mod config;
mod handlers;
mod meta;
mod proxy;
mod validate;

pub use proxy::AppState;

/// Build the jsonhost router. Shared by `main` and the integration tests.
pub fn router(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/", get(meta::root_index))
        .route("/openapi.json", get(meta::openapi))
        .route("/{collection}", get(meta::list_documents))
        .route(
            "/{collection}/{*slug}",
            get(handlers::get_document)
                .put(handlers::put_document)
                .delete(handlers::delete_document),
        )
        .with_state(state)
}
