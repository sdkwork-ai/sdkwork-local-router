use sdkwork_api_local_router_assembly::assemble_api_router;
use sdkwork_web_bootstrap::{service_router, ServiceRouterConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let assembly = assemble_api_router().await?;
    let bind_address = assembly.bind_address.clone();
    let runtime = assembly.runtime;
    let app = service_router(
        assembly.router,
        ServiceRouterConfig::default().with_always_ready(),
    );
    let listener = tokio::net::TcpListener::bind(&bind_address).await?;
    eprintln!("sdkwork-api-local-router-standalone-gateway listening on {bind_address}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    runtime.shutdown().await;
    Ok(())
}

async fn shutdown_signal() {
    #[cfg(windows)]
    {
        let ctrl_c = async {
            tokio::signal::ctrl_c()
                .await
                .expect("failed to install Ctrl+C handler");
        };
        let ctrl_close = async {
            tokio::signal::windows::ctrl_close()
                .expect("failed to install Windows ctrl-close handler")
                .recv()
                .await;
        };
        tokio::select! { _ = ctrl_c => {}, _ = ctrl_close => {} }
    }
    #[cfg(not(windows))]
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
}
