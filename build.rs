//! The web UI (`web/dist`) is embedded into the binary. Release builds refuse
//! to compile without it; debug builds get an empty placeholder so
//! `cargo test`/`cargo run` work and the server shows a "UI not built" page.

use std::path::Path;

fn main() {
    println!("cargo:rerun-if-changed=web/dist");
    println!("cargo:rerun-if-changed=build.rs");
    let dist = Path::new("web/dist");
    if dist.join("index.html").exists() {
        return;
    }
    if std::env::var("PROFILE").as_deref() == Ok("release") {
        panic!(
            "web/dist/index.html is missing: build the web UI first (cd web && npm ci && npm run build)"
        );
    }
    std::fs::create_dir_all(dist).expect("create web/dist placeholder");
}
