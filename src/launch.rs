//! The default command: one instance per machine, start the server, open the
//! browser. The initial scan and import run in the background so the browser
//! opens right away; `LiveEvent::IndexReady` tells the UI when the scan (and
//! again when a non-empty import) finishes.

use std::net::{Ipv4Addr, SocketAddr};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::broadcast;

use crate::config::{Config, Paths};
use crate::events::LiveEvent;
use crate::events::live::{EVENT_RETENTION_DAYS, spawn_live_poller};
use crate::index::Indexer;
use crate::server::{AppState, serve_with_shutdown};
use crate::store::Store;
use crate::taxonomy::Taxonomy;
use crate::{import, setup};

/// Last port tried when starting from the default range.
const MAX_PORT: u16 = 7430;
/// Ports tried past a preferred port that is already beyond [`MAX_PORT`].
const EXTRA_PORTS: u16 = 10;
const PROBE_TIMEOUT: Duration = Duration::from_millis(500);
const SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(2);
const MAX_PROBE_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, Default)]
pub struct ServeOpts {
    pub port: Option<u16>,
    pub no_open: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PortChoice {
    /// Nothing listens here; bind it.
    Free(u16),
    /// Another Plan Atlas answers `/api/health` here.
    OursRunning(u16),
}

/// Try `preferred..=7430` (or `preferred..=preferred+10` when `preferred` is
/// past 7430). A busy port is probed: our own server means `OursRunning`,
/// anything else is skipped. If every port is taken by something else,
/// returns `Free(preferred)` and binding it reports the error.
pub async fn pick_port(preferred: u16) -> PortChoice {
    let last = if preferred > MAX_PORT {
        preferred.saturating_add(EXTRA_PORTS)
    } else {
        MAX_PORT
    };
    for port in preferred..=last {
        if can_bind(port) {
            return PortChoice::Free(port);
        }
        if is_plan_atlas(port).await {
            return PortChoice::OursRunning(port);
        }
    }
    PortChoice::Free(preferred)
}

/// Bind with the same options the real listener uses (`SO_REUSEADDR` on
/// Unix), so a port left in TIME_WAIT by a previous run still counts as free.
fn can_bind(port: u16) -> bool {
    let Ok(sock) = tokio::net::TcpSocket::new_v4() else {
        return false;
    };
    #[cfg(unix)]
    if sock.set_reuseaddr(true).is_err() {
        return false;
    }
    sock.bind(SocketAddr::from((Ipv4Addr::LOCALHOST, port)))
        .is_ok()
}

async fn is_plan_atlas(port: u16) -> bool {
    tokio::time::timeout(PROBE_TIMEOUT, probe_health(port))
        .await
        .unwrap_or(false)
}

/// Minimal HTTP/1.1 `GET /api/health`; true when the JSON body says
/// `"app": "plan-atlas"`.
async fn probe_health(port: u16) -> bool {
    let Ok(mut stream) = tokio::net::TcpStream::connect((Ipv4Addr::LOCALHOST, port)).await else {
        return false;
    };
    let req = format!(
        "GET /api/health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nAccept: application/json\r\nConnection: close\r\n\r\n"
    );
    if stream.write_all(req.as_bytes()).await.is_err() {
        return false;
    }
    let mut buf = Vec::new();
    if (&mut stream)
        .take(MAX_PROBE_BYTES)
        .read_to_end(&mut buf)
        .await
        .is_err()
    {
        return false;
    }
    let text = String::from_utf8_lossy(&buf);
    let Some((head, body)) = text.split_once("\r\n\r\n") else {
        return false;
    };
    if !head.starts_with("HTTP/1.1 200") {
        return false;
    }
    serde_json::from_str::<serde_json::Value>(body)
        .ok()
        .and_then(|v| {
            v.get("app")
                .and_then(|a| a.as_str())
                .map(|a| a == "plan-atlas")
        })
        .unwrap_or(false)
}

fn open_browser(url: &str) {
    if webbrowser::open(url).is_err() {
        println!("Could not open a browser; visit {url}");
    }
}

/// First launch on a terminal: offer to connect Claude Code. Without a TTY
/// nothing is asked (and `setup_done` stays unset).
fn first_run(paths: &Paths, config: &Config) {
    use std::io::{BufRead, IsTerminal, Write};
    if config.setup_done || !std::io::stdin().is_terminal() {
        return;
    }
    if let Err(hint) = setup::setup_binary() {
        // Not recorded as done: a later global install still gets asked.
        println!("Skipping Claude Code setup: {hint}");
        return;
    }
    print!("Connect Plan Atlas to Claude Code? [Y/n] ");
    let _ = std::io::stdout().flush();
    let mut answer = String::new();
    if std::io::stdin().lock().read_line(&mut answer).is_err() {
        return;
    }
    let a = answer.trim().to_lowercase();
    if a.is_empty() || a == "y" || a == "yes" {
        if let Err(e) = setup::run_setup(true) {
            // Not recorded as done, so the next launch asks again.
            eprintln!("plan-atlas: setup failed: {e}");
            return;
        }
    } else {
        println!("Skipped. Run `plan-atlas setup` any time.");
    }
    if let Err(e) = setup::set_setup_done(paths, true) {
        eprintln!("plan-atlas: warning: {e}");
    }
}

/// Prune old events, scan everything and announce [`LiveEvent::IndexReady`],
/// then import `~/.claude/plans` (rescanning and announcing again when
/// anything was imported). Runs on a blocking thread.
fn initial_index(
    paths: Paths,
    store: Arc<Store>,
    tax: Arc<Taxonomy>,
    live: broadcast::Sender<LiveEvent>,
) {
    if let Err(e) = store.prune_events(EVENT_RETENTION_DAYS) {
        eprintln!("plan-atlas: warning: pruning old events failed: {e:#}");
    }
    // Existing plans first, so the UI is populated before any import work.
    let indexer = Indexer::new(paths.clone(), store, tax);
    let scan = |indexer: &Indexer| {
        if let Err(e) = indexer.full_scan() {
            eprintln!("plan-atlas: warning: initial scan failed: {e:#}");
        }
        let _ = live.send(LiveEvent::IndexReady);
    };
    scan(&indexer);
    let Some(claude_home) = setup::claude_home() else {
        return;
    };
    match import::import_claude_plans(&claude_home, &paths) {
        Ok(r) => {
            if r.imported > 0 || r.failed > 0 {
                eprintln!(
                    "plan-atlas: imported {} plan(s) from {} ({} failed)",
                    r.imported,
                    claude_home.join("plans").display(),
                    r.failed
                );
            }
            // The watcher also sees the new files; rescanning keeps the
            // index right even when watching is unavailable.
            if r.imported > 0 {
                scan(&indexer);
            }
        }
        Err(e) => eprintln!("plan-atlas: warning: import failed: {e:#}"),
    }
}

/// Run the launcher until Ctrl+C.
pub async fn run(opts: ServeOpts) -> Result<()> {
    let paths = Paths::resolve();
    paths
        .ensure()
        .with_context(|| format!("cannot create {}", paths.home.display()))?;
    let config = Config::load(&paths);
    {
        let (p, c) = (paths.clone(), config.clone());
        tokio::task::spawn_blocking(move || first_run(&p, &c))
            .await
            .context("first-run prompt")?;
    }
    let tax = Arc::new(Taxonomy::load(&paths));
    let store = Arc::new(Store::open(&paths.db, tax.clone())?);

    let port = match pick_port(opts.port.unwrap_or(config.port)).await {
        PortChoice::OursRunning(port) => {
            let url = format!("http://127.0.0.1:{port}");
            if !opts.no_open {
                open_browser(&url);
            }
            println!("Plan Atlas is already running at {url}");
            return Ok(());
        }
        PortChoice::Free(port) => port,
    };
    let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, port))
        .await
        .with_context(|| format!("cannot listen on 127.0.0.1:{port}"))?;
    let port = listener.local_addr()?.port();

    let (live, _) = broadcast::channel(256);
    let state = AppState {
        store: store.clone(),
        paths: Arc::new(paths.clone()),
        tax: tax.clone(),
        live: live.clone(),
        port,
    };
    let (stop_tx, stop_rx) = tokio::sync::oneshot::channel::<()>();
    let mut server = tokio::spawn(serve_with_shutdown(listener, state, async {
        let _ = stop_rx.await;
    }));
    let _watch = match Indexer::new(paths.clone(), store.clone(), tax.clone()).watch(live.clone()) {
        Ok(w) => Some(w),
        Err(e) => {
            eprintln!("plan-atlas: warning: live reload disabled: {e:#}");
            None
        }
    };
    let poller = spawn_live_poller(store.clone(), live.clone());

    let url = format!("http://127.0.0.1:{port}");
    if !opts.no_open {
        let u = url.clone();
        let _ = tokio::task::spawn_blocking(move || open_browser(&u)).await;
    }
    println!("Plan Atlas is running at {url} \u{2014} press Ctrl+C to stop");

    tokio::task::spawn_blocking(move || initial_index(paths, store, tax, live));

    let result = tokio::select! {
        r = tokio::signal::ctrl_c() => r.context("waiting for Ctrl+C"),
        r = &mut server => {
            // The server stopped on its own: report why.
            poller.abort();
            return match r {
                Ok(Ok(())) => Ok(()),
                Ok(Err(e)) => Err(e),
                Err(e) => Err(e.into()),
            };
        }
    };
    poller.abort();
    let _ = stop_tx.send(());
    // Open SSE streams never end on their own; do not let them hold exit.
    let _ = tokio::time::timeout(SHUTDOWN_TIMEOUT, server).await;
    result
}
