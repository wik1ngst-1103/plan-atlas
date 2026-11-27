//! Web UI assets. Until the SPA is embedded, every non-API GET gets a short
//! page explaining how to build the UI.

use axum::http::{Method, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};

use super::routes::error_response;

const NOT_BUILT: &str = "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>Plan Atlas</title></head>\n<body><p>Plan Atlas \u{2014} the web UI has not been built yet. Run <code>npm run build</code> in web/ and rebuild.</p></body></html>\n";

/// Router fallback: unknown `/api` paths and non-GET requests are 404.
pub(super) async fn fallback(method: Method, uri: Uri) -> Response {
    let api = uri.path() == "/api" || uri.path().starts_with("/api/");
    if api || !matches!(method, Method::GET | Method::HEAD) {
        return error_response(StatusCode::NOT_FOUND, "not found");
    }
    Html(NOT_BUILT).into_response()
}
