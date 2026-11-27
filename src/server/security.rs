//! Localhost guards: exact Host check and a CSRF header for mutating methods.

use axum::Router;
use axum::extract::{Request, State};
use axum::http::{Method, StatusCode, header};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};

use super::AppState;
use super::routes::error_response;

pub(super) fn guard(router: Router<AppState>, state: AppState) -> Router<AppState> {
    router.layer(middleware::from_fn_with_state(state, check))
}

fn host_allowed(host: &str, port: u16) -> bool {
    let Some((name, p)) = host.rsplit_once(':') else {
        return false;
    };
    (name.eq_ignore_ascii_case("127.0.0.1") || name.eq_ignore_ascii_case("localhost"))
        && p == port.to_string()
}

async fn check(State(state): State<AppState>, req: Request, next: Next) -> Response {
    let host_ok = req
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .is_some_and(|h| host_allowed(h, state.port));
    if !host_ok {
        return error_response(StatusCode::FORBIDDEN, "forbidden host").into_response();
    }
    let safe = matches!(*req.method(), Method::GET | Method::HEAD);
    if !safe && req.headers().get("x-plan-atlas").map(|v| v.as_bytes()) != Some(b"1") {
        return error_response(StatusCode::FORBIDDEN, "missing X-Plan-Atlas header")
            .into_response();
    }
    next.run(req).await
}
