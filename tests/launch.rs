use std::io::Read;
use std::process::{Child, Command, Stdio};
use std::sync::Arc;
use std::time::{Duration, Instant};

use plan_atlas::config::Paths;
use plan_atlas::launch::{PortChoice, pick_port};
use plan_atlas::server::{AppState, serve};
use plan_atlas::store::Store;
use plan_atlas::taxonomy::Taxonomy;
use tokio::sync::broadcast;

#[tokio::test]
async fn second_instance_detected() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::from_home(dir.path().to_path_buf());
    paths.ensure().unwrap();
    let tax = Arc::new(Taxonomy::embedded_default());
    let store = Arc::new(Store::open(&paths.db, tax.clone()).unwrap());
    let listener = tokio::net::TcpListener::bind(("127.0.0.1", 0))
        .await
        .unwrap();
    let port = listener.local_addr().unwrap().port();
    let (live, _) = broadcast::channel(16);
    let state = AppState {
        store,
        paths: Arc::new(paths),
        tax,
        live,
        port,
    };
    tokio::spawn(serve(listener, state));
    assert_eq!(pick_port(port).await, PortChoice::OursRunning(port));
}

#[tokio::test]
async fn foreign_port_skipped() {
    // Accepts connections but never answers: the probe must time out.
    let foreign = std::net::TcpListener::bind(("127.0.0.1", 0)).unwrap();
    let port = foreign.local_addr().unwrap().port();
    let started = Instant::now();
    assert_eq!(pick_port(port).await, PortChoice::Free(port + 1));
    assert!(started.elapsed() < Duration::from_secs(2));
}

struct Kill(Child);

impl Drop for Kill {
    fn drop(&mut self) {
        let _ = self.0.kill();
        let _ = self.0.wait();
    }
}

fn health_ok(port: u16) -> bool {
    use std::io::Write;
    let Ok(mut s) = std::net::TcpStream::connect(("127.0.0.1", port)) else {
        return false;
    };
    let _ = s.set_read_timeout(Some(Duration::from_millis(500)));
    let req =
        format!("GET /api/health HTTP/1.1\r\nHost: 127.0.0.1:{port}\r\nConnection: close\r\n\r\n");
    if s.write_all(req.as_bytes()).is_err() {
        return false;
    }
    let mut resp = String::new();
    let _ = s.read_to_string(&mut resp);
    resp.starts_with("HTTP/1.1 200") && resp.contains("\"app\":\"plan-atlas\"")
}

#[test]
fn run_serves_health_with_no_open() {
    let data = tempfile::tempdir().unwrap();
    let home = tempfile::tempdir().unwrap();
    let port = std::net::TcpListener::bind(("127.0.0.1", 0))
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    let child = Command::new(env!("CARGO_BIN_EXE_plan-atlas"))
        .args(["--no-open", "--port", &port.to_string()])
        .env("PLAN_ATLAS_HOME", data.path())
        .env("HOME", home.path())
        .env_remove("CLAUDE_CONFIG_DIR")
        .env_remove("PLAN_ATLAS_PORT")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let _guard = Kill(child);
    let deadline = Instant::now() + Duration::from_secs(5);
    while !health_ok(port) {
        assert!(Instant::now() < deadline, "no healthy server within 5 s");
        std::thread::sleep(Duration::from_millis(50));
    }
}
