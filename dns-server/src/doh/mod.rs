use axum::{
    Router,
    body::{Body, Bytes},
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    routing::{get, post},
};
use base64::{Engine as _, engine::general_purpose::URL_SAFE_NO_PAD};
use http::{HeaderValue, Method, header};
use serde::Deserialize;
use std::sync::Arc;
use tokio::net::TcpListener;
use tower_http::cors::CorsLayer;

use crate::server::{ServerContext, resolve_message};
use std::env;

pub fn router(ctx: Arc<ServerContext>) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/dns-query", post(doh_post))
        .route("/dns-query", get(doh_get))
        .with_state(ctx)
}

async fn health() -> impl IntoResponse {
    Response::builder()
        .status(StatusCode::OK)
        .body(Body::from("OK"))
        .unwrap()
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
    if cfg!(debug_assertions) {
        let manifest_dir = env!("CARGO_MANIFEST_DIR");
        dotenvy::from_filename(format!("{}/.env", manifest_dir)).ok();
    }

    let client_url = env::var("CLIENT_URL").expect("CLIENT_URL must be set");

    let cors = CorsLayer::new()
        .allow_origin(
            client_url
                .parse::<HeaderValue>()
                .expect("Invalid CLIENT_URL"),
        )
        .allow_methods([Method::GET, Method::POST])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT]);

    let app = router(ctx).layer(cors);

    let port = std::env::var("PORT").unwrap_or_else(|_| "8443".to_string());

    let addr = format!("0.0.0.0:{}", port);

    let listener = TcpListener::bind(&addr).await?;

    println!("DoH listening on http://{}", addr);

    axum::serve(listener, app).await?;

    Ok(())
}
