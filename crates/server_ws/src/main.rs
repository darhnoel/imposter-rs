use anyhow::Context;
use server_ws::run_server;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env().add_directive("server_ws=info".parse()?))
        .init();

    let bind = std::env::var("IMPOSTER_WS_BIND").unwrap_or_else(|_| "127.0.0.1:4000".to_string());
    run_server(&bind)
        .await
        .with_context(|| format!("failed to run ws server on {bind}"))
}
