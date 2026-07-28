use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use http::header;
use serde::Deserialize;
use std::{net::SocketAddr, sync::Arc};
use tokio::net::TcpListener;

use crate::server::{ServerContext, resolve_message};

pub fn router(ctx: Arc<ServerContext>) -> Router {
    Router::new()
        .route("/dns-query", post(doh_post))
        .route("/dns-query", get(doh_get))
        .with_state(ctx)
}

async fn doh_post(State(ctx): State<Arc<ServerContext>>, body: Bytes) -> impl IntoResponse {
    let outcome = resolve_message(
        body.as_ref(),
        &ctx.transport,
        &ctx.cache,
        &ctx.singleflight,
        false,
    );
    return match outcome {
        Ok(response) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/dns-message")
            .body(Body::from(response))
            .unwrap(),

        Err(e) => Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(format!("{e:?}")))
            .unwrap(),
    };
}

#[derive(Deserialize)]
struct DohQuery {
    dns: String,
}

async fn doh_get(
    State(ctx): State<Arc<ServerContext>>,
    Query(query): Query<DohQuery>,
) -> impl IntoResponse {
    let request = match URL_SAFE_NO_PAD.decode(&query.dns) {
        Ok(bytes) => bytes,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from(format!("Invalid dns parameter: {e}")))
                .unwrap();
        }
    };

    match resolve_message(
        &request,
        &ctx.transport,
        &ctx.cache,
        &ctx.singleflight,
        false,
    ) {
        Ok(response) => Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/dns-message")
            .body(Body::from(response))
            .unwrap(),

        Err(e) => Response::builder()
            .status(StatusCode::BAD_REQUEST)
            .body(Body::from(format!("{e:?}")))
            .unwrap(),
    }
}

pub async fn run(ctx: Arc<ServerContext>) -> std::io::Result<()> {
    let app = router(ctx);

    let addr = SocketAddr::from(([127, 0, 0, 1], 8443));

    let listener = TcpListener::bind(addr).await?;

    println!("DoH listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
