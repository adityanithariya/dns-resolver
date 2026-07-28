use std::sync::Arc;

use dns_resolver::{doh, server::ServerContext};

#[tokio::main]
async fn main() -> Result<(), Box<(dyn std::error::Error + 'static)>> {
    let workers = std::thread::available_parallelism()?.get() * 4;
    let ctx = Arc::new(ServerContext::new(workers));

    doh::run(ctx).await?;

    Ok(())
}
