use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

use crate::server::ServerContext;

pub fn router(ctx: Arc<ServerContext>) -> Router {
    Router::new()
        .route("/dns-query", post(doh_post))
        .route("/dns-query", get(doh_get))
        .with_state(ctx)
}

async fn doh_post(State(ctx): State<Arc<ServerContext>>, body: Bytes) -> impl IntoResponse {
    println!("Received {} bytes", body.len());



    StatusCode::OK
}

async fn doh_get() -> impl IntoResponse {
    StatusCode::NOT_IMPLEMENTED
}

pub async fn run(ctx: Arc<ServerContext>) -> std::io::Result<()> {
    let app = router(ctx);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8000));

    let listener = TcpListener::bind(addr).await?;

    println!("DoH listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
