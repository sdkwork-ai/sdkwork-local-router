//! API assembly bootstrap for sdkwork-local-router.

use axum::http::header;
use axum::middleware;
use axum::Router;
use sdkwork_routes_local_router_support::{auth, rate_limit::rate_limit_middleware};
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;

#[path = "runtime.rs"]
mod runtime;

use runtime::{bootstrap, RuntimeHandle};

pub struct ApiAssembly {
    pub router: Router,
    pub bind_address: String,
    pub runtime: RuntimeHandle,
}

pub async fn assemble_api_router() -> Result<ApiAssembly, String> {
    let config = load_config()?;
    let bind_address = config.bind_addr();
    let runtime = bootstrap(config.clone())
        .await
        .map_err(|error| error.to_string())?;
    let state = runtime.state;

    let open_routes = sdkwork_routes_local_router_open_api::routes(&config.base_paths)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::proxy_auth_middleware,
        ));
    let app_routes = sdkwork_routes_local_router_app_api::routes().layer(
        middleware::from_fn_with_state(state.clone(), auth::app_auth_middleware),
    );
    let backend_routes = sdkwork_routes_local_router_backend_api::routes().layer(
        middleware::from_fn_with_state(state.clone(), auth::admin_auth_middleware),
    );

    let router = Router::new()
        .merge(open_routes)
        .merge(app_routes)
        .merge(backend_routes)
        .layer(RequestBodyLimitLayer::new(
            config.server.max_body_size_bytes(),
        ))
        .layer(build_cors_layer(&config.cors))
        .with_state(state);

    Ok(ApiAssembly {
        router,
        bind_address,
        runtime: runtime.handle,
    })
}

fn load_config() -> Result<sdkwork_lr_config::RuntimeConfig, String> {
    let config_path = std::env::var("SDKWORK_LR_CONFIG_FILE")
        .ok()
        .filter(|value| !value.trim().is_empty());
    let mut config = sdkwork_lr_config::RuntimeConfig::from_file_or_env(config_path.as_deref())?;
    if let Ok(port) = std::env::var("SDKWORK_LR_PORT") {
        if let Ok(port) = port.trim().parse::<u16>() {
            config = config.with_port(port);
        }
    }
    if let Ok(bind) = std::env::var("SDKWORK_LOCAL_ROUTER_APPLICATION_PUBLIC_INGRESS_BIND") {
        if !bind.trim().is_empty() {
            config = config.with_bind(bind.trim());
        }
    }
    Ok(config)
}

fn build_cors_layer(cors_config: &sdkwork_lr_config::CorsConfig) -> CorsLayer {
    let use_private_network_policy = cors_config.is_allow_any()
        || cors_config
            .allowed_origins
            .iter()
            .any(|origin| origin.contains(":*"));
    let mut policy = if use_private_network_policy {
        sdkwork_web_core::CorsPolicy::development_private_network()
    } else {
        sdkwork_web_core::CorsPolicy::default()
    };
    for origin in cors_config
        .allowed_origins
        .iter()
        .filter(|origin| !origin.contains(":*") && origin.as_str() != "*")
    {
        if !policy.allowed_origins.contains(origin) {
            policy.allowed_origins.push(origin.clone());
        }
    }
    for name in [
        "accept",
        "x-goog-api-key",
        "x-sdkwork-client-api-key-id",
        "x-request-id",
        "x-sdkwork-client-api",
        "anthropic-version",
    ] {
        if !policy.allowed_headers.iter().any(|allowed| allowed == name) {
            policy.allowed_headers.push(name.to_owned());
        }
    }
    sdkwork_web_axum::cors_layer_from_policy(policy).expose_headers([
        header::HeaderName::from_static("x-request-id"),
        header::HeaderName::from_static("x-sdkwork-plugin-policy"),
        header::HeaderName::from_static("x-sdkwork-plugin-source"),
        header::HeaderName::from_static("x-sdkwork-plugin-target"),
        header::HeaderName::from_static("x-sdkwork-plugin-id"),
        header::HeaderName::from_static("x-sdkwork-model-vendor"),
        header::HeaderName::from_static("x-sdkwork-client-api"),
        header::HeaderName::from_static("access-token"),
    ])
}
