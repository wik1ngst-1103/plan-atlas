use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode, header};
use http_body_util::BodyExt;
use plan_atlas::config::Paths;
use plan_atlas::server::{AppState, build_router};
use plan_atlas::store::Store;
use plan_atlas::taxonomy::Taxonomy;
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

const HOST: &str = "127.0.0.1:7420";

fn app() -> (TempDir, Router) {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::from_home(dir.path().to_path_buf());
    paths.ensure().unwrap();
    let tax = Arc::new(Taxonomy::embedded_default());
    let store = Arc::new(Store::open(&paths.db, tax.clone()).unwrap());
    let (live, _) = broadcast::channel(16);
    let state = AppState {
        store,
        paths: Arc::new(paths),
        tax,
        live,
        port: 7420,
    };
    (dir, build_router(state))
}

async fn get(
    app: &Router,
    uri: &str,
    enc: Option<&str>,
) -> (StatusCode, axum::http::HeaderMap, Vec<u8>) {
    let mut req = Request::get(uri).header(header::HOST, HOST);
    if let Some(e) = enc {
        req = req.header(header::ACCEPT_ENCODING, e);
    }
    let res = app
        .clone()
        .oneshot(req.body(Body::empty()).unwrap())
        .await
        .unwrap();
    let (parts, body) = res.into_parts();
    (
        parts.status,
        parts.headers,
        body.collect().await.unwrap().to_bytes().to_vec(),
    )
}

fn asset_path() -> String {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("web/dist/assets");
    let name = std::fs::read_dir(dir)
        .unwrap()
        .map(|e| e.unwrap().file_name().into_string().unwrap())
        .find(|n| n.ends_with(".js"))
        .expect("a built js asset");
    format!("/assets/{name}")
}

#[tokio::test]
async fn serves_index_and_spa_fallback() {
    let (_d, app) = app();
    let (st, h, index) = get(&app, "/", None).await;
    assert_eq!(st, StatusCode::OK);
    assert!(String::from_utf8_lossy(&index).contains("<div id=\"app\""));
    assert_eq!(h[header::CACHE_CONTROL], "no-cache");
    assert!(
        h[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .starts_with("text/html")
    );
    let (st, _, body) = get(&app, "/plan/x", None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body, index);
    let (st, _, _) = get(&app, "/api/nope", None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn missing_files_are_404_not_index() {
    let (_d, app) = app();
    for uri in [
        "/assets/index-deadbeef.js",
        "/assets/nope",
        "/favicon.ico",
        "/sub/dir/missing.css",
        "/x.gz",
    ] {
        let (st, _, _) = get(&app, uri, None).await;
        assert_eq!(st, StatusCode::NOT_FOUND, "{uri}");
    }
    let (st, _, _) = get(&app, "/board", None).await;
    assert_eq!(st, StatusCode::OK, "extension-less paths still get the SPA");
}

#[tokio::test]
async fn assets_are_immutable_and_gzip_is_negotiated() {
    let (_d, app) = app();
    let path = asset_path();
    let (st, h, raw) = get(&app, &path, None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(
        h[header::CACHE_CONTROL],
        "public, max-age=31536000, immutable"
    );
    assert!(h.get(header::CONTENT_ENCODING).is_none());
    assert!(
        h[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .contains("javascript")
    );

    let (st, h, gz) = get(&app, &path, Some("br, gzip;q=0.8")).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(h[header::CONTENT_ENCODING], "gzip");
    assert_eq!(h[header::VARY], "Accept-Encoding");
    assert!(
        h[header::CONTENT_TYPE]
            .to_str()
            .unwrap()
            .contains("javascript")
    );
    assert!(gz.len() < raw.len());
    assert_eq!(&gz[..2], &[0x1f, 0x8b]);

    let (_, h, _) = get(&app, &path, Some("gzip;q=0")).await;
    assert!(h.get(header::CONTENT_ENCODING).is_none());
}
