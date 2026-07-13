//! Previews the prerendered site the way a static host serves it: files off
//! disk, unknown paths answered with the prerendered 404 page.
//!
//! This is a local convenience only — production serves `dist/` directly, with
//! no process of ours involved. Run `make website-build` first.

use axum::Router;
use axum::http::StatusCode;
use tower_http::services::{ServeDir, ServeFile};
use tower_http::set_status::SetStatus;

const DIST: &str = "dist";
const ADDR: &str = "127.0.0.1:3000";

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // ServeFile answers 200 by default; a missing page must look like a missing
    // page here, or the preview would disagree with the real host.
    let not_found = SetStatus::new(
        ServeFile::new(format!("{DIST}/404.html")),
        StatusCode::NOT_FOUND,
    );
    let site = ServeDir::new(DIST).not_found_service(not_found);

    println!("serving {DIST}/ on http://{ADDR}");
    let listener = tokio::net::TcpListener::bind(ADDR).await?;
    axum::serve(listener, Router::new().fallback_service(site)).await?;
    Ok(())
}
