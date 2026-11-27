//! Embedded web UI. Files come pre-compressed (`.gz` siblings written by
//! `web/scripts/compress.mjs`) and are served by `Accept-Encoding`. Without an
//! embedded `index.html` a short "UI not built" page is shown instead.

use axum::http::header::{
    ACCEPT_ENCODING, CACHE_CONTROL, CONTENT_ENCODING, CONTENT_TYPE, HeaderMap, HeaderValue, VARY,
};
use axum::http::{Method, StatusCode, Uri};
use axum::response::{Html, IntoResponse, Response};
use rust_embed::RustEmbed;

use super::routes::error_response;

const NOT_BUILT: &str = "<!doctype html>\n<html lang=\"en\"><head><meta charset=\"utf-8\"><title>Plan Atlas</title></head>\n<body><p>Plan Atlas \u{2014} the web UI has not been built yet. Run <code>npm run build</code> in web/ and rebuild.</p></body></html>\n";

#[derive(RustEmbed)]
#[folder = "web/dist"]
struct Dist;

fn accepts_gzip(headers: &HeaderMap) -> bool {
    headers
        .get_all(ACCEPT_ENCODING)
        .iter()
        .filter_map(|v| v.to_str().ok())
        .flat_map(|v| v.split(','))
        .any(|part| {
            let mut it = part.split(';');
            let coding = it.next().unwrap_or("").trim();
            let q0 = it.any(|p| matches!(p.trim().replace(' ', "").as_str(), "q=0" | "q=0.0"));
            coding.eq_ignore_ascii_case("gzip") && !q0
        })
}

/// Serve one embedded file, gzip variant when accepted and available.
fn serve_file(path: &str, headers: &HeaderMap) -> Option<Response> {
    let raw = Dist::get(path)?;
    let mime = mime_guess::from_path(path).first_or_octet_stream();
    let gz = if accepts_gzip(headers) {
        Dist::get(&format!("{path}.gz"))
    } else {
        None
    };
    let (data, gzipped) = match gz {
        Some(g) => (g.data, true),
        None => (raw.data, false),
    };
    let cache = if path.starts_with("assets/") {
        "public, max-age=31536000, immutable"
    } else {
        "no-cache"
    };
    let mut res = Response::new(axum::body::Body::from(data.into_owned()));
    let h = res.headers_mut();
    h.insert(
        CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref()).unwrap_or(HeaderValue::from_static("text/plain")),
    );
    h.insert(CACHE_CONTROL, HeaderValue::from_static(cache));
    h.insert(VARY, HeaderValue::from_static("Accept-Encoding"));
    if gzipped {
        h.insert(CONTENT_ENCODING, HeaderValue::from_static("gzip"));
    }
    Some(res)
}

/// Router fallback: unknown `/api` paths and non-GET requests are 404; other
/// GETs get the matching embedded file, else the SPA `index.html`.
pub(super) async fn fallback(method: Method, uri: Uri, headers: HeaderMap) -> Response {
    let api = uri.path() == "/api" || uri.path().starts_with("/api/");
    if api || !matches!(method, Method::GET | Method::HEAD) {
        return error_response(StatusCode::NOT_FOUND, "not found");
    }
    let path = uri.path().trim_start_matches('/');
    let direct = if path.is_empty() || path.ends_with(".gz") {
        None
    } else {
        serve_file(path, &headers)
    };
    direct
        .or_else(|| serve_file("index.html", &headers))
        .unwrap_or_else(|| Html(NOT_BUILT).into_response())
}
