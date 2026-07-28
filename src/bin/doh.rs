use std::sync::Arc;

use dns_resolver::{doh, server::ServerContext};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let ctx = Arc::new(ServerContext::new_without_pool());

    doh::run(ctx).await?;

    Ok(())
}
