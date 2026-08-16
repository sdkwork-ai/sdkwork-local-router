use sdkwork_api_local_router_assembly::assemble_api_router;
use sdkwork_iam_web_adapter::{
    build_web_framework_builder, iam_web_request_context_resolver_from_env,
};
use sdkwork_web_bootstrap::{infra_public_path_prefixes, ComposedApiAssembly};

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
    let contribution = assembly.contribution;
    let framework = build_web_framework_builder(
        iam_web_request_context_resolver_from_env().await,
        contribution.route_manifest.clone(),
        infra_public_path_prefixes(),
    );
    let app = ComposedApiAssembly::try_compose("SDKWork Local Router API", vec![contribution])?
        .into_hosted(framework)
        .router;
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
