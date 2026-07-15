use std::sync::Arc;

use axum::extract::State;
use axum::http::header;
use axum::middleware;
use axum::response::IntoResponse;
use axum::routing::{get, post, put};
use axum::{Json, Router};
use parking_lot::RwLock;
use serde_json::json;
use std::collections::HashMap;
use tower_http::cors::CorsLayer;
use tower_http::limit::RequestBodyLimitLayer;

use crate::auth;
use crate::integration;
use crate::passthrough;
use crate::rate_limit::{rate_limit_middleware, RateLimiter};
use sdkwork_lr_config::RuntimeConfig;
use sdkwork_lr_core::Account;
use sdkwork_lr_core::AccountPool;
use sdkwork_lr_core::AuditMetadataInterceptor;
use sdkwork_lr_core::CircuitBreakerConfig;
use sdkwork_lr_core::HealthManager;
use sdkwork_lr_core::InterceptorChain;
use sdkwork_lr_core::LoggingInterceptor;
use sdkwork_lr_core::MetricsInterceptor;
use sdkwork_lr_plugin::PluginRegistry;
use sdkwork_lr_proxy::ProxyClient;
use sdkwork_lr_store::{Store, DEFAULT_USER_ID};

pub type RoutingStrategyOverrides = Arc<RwLock<HashMap<i64, sdkwork_lr_core::RoutingStrategy>>>;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<RuntimeConfig>,
    pub pool: Arc<RwLock<AccountPool>>,
    pub user_pools: Arc<RwLock<HashMap<i64, Arc<RwLock<AccountPool>>>>>,
    pub routing_strategy_overrides: RoutingStrategyOverrides,
    pub client: ProxyClient,
    pub store: Store,
    pub rate_limiter: Arc<RateLimiter>,
    pub interceptor_chain: Arc<InterceptorChain>,
    pub health_manager: Arc<HealthManager>,
    pub transform_registry: Arc<PluginRegistry>,
    pub model_catalog: Option<Arc<sdkwork_models::ModelCatalog>>,
}

pub async fn build_router(
    config: &RuntimeConfig,
    store: Store,
    rate_limiter: Arc<RateLimiter>,
) -> anyhow::Result<(
    Router,
    Arc<HealthManager>,
    Arc<RwLock<AccountPool>>,
    RoutingStrategyOverrides,
)> {
    let accounts: Vec<Account> = config
        .upstreams
        .iter()
        .map(|u| Account {
            name: u.name.clone(),
            provider: u.provider.clone(),
            base_url: u.base_url.clone(),
            upstream_api_key: u.upstream_api_key.clone(),
            upstream_auth_scheme: u.upstream_auth_scheme.clone(),
            models: u.models.clone(),
            priority: u.priority,
            timeout: u.timeout,
            max_retries: u.max_retries,
            retry_delay_ms: u.retry_delay_ms,
            anthropic_version: u.anthropic_version.clone(),
            default_headers: u.default_headers.clone(),
            enabled: true,
            model_route_mappings: u.model_route_mappings.clone(),
        })
        .collect();

    let cb_config = CircuitBreakerConfig {
        failure_threshold: config.circuit_breaker.failure_threshold,
        success_threshold: config.circuit_breaker.success_threshold,
        open_duration: config.circuit_breaker.open_duration,
        half_open_max_requests: config.circuit_breaker.half_open_max_requests,
    };
    let health_manager = Arc::new(HealthManager::new(cb_config));
    let strategy = config.routing.strategy_for_account_count(accounts.len());
    let pool = sdkwork_lr_core::AccountPool::with_health_manager(
        accounts,
        strategy,
        health_manager.clone(),
    );
    let pool_arc = Arc::new(RwLock::new(pool));
    let mut user_pools = HashMap::new();
    user_pools.insert(DEFAULT_USER_ID, pool_arc.clone());
    let routing_strategy_overrides: RoutingStrategyOverrides =
        Arc::new(RwLock::new(HashMap::new()));
    let client = sdkwork_lr_proxy::build_proxy_client();

    let mut interceptor_chain = InterceptorChain::new();
    interceptor_chain.add(Arc::new(LoggingInterceptor));
    interceptor_chain.add(Arc::new(MetricsInterceptor));
    interceptor_chain.add(Arc::new(AuditMetadataInterceptor));

    let transform_registry = Arc::new(sdkwork_lr_transform::plugins::built_in_plugin_registry());
    let model_catalog = match sdkwork_models::load_bundled_catalog() {
        Ok(catalog) => {
            tracing::info!(
                catalog_version = %catalog.manifest.catalog_version,
                vendors = catalog.vendors.len(),
                "sdkwork-models catalog loaded"
            );
            Some(Arc::new(catalog))
        }
        Err(error) => {
            tracing::warn!(error = %error, "failed to load sdkwork-models catalog; falling back to provider-based plugin routing");
            None
        }
    };

    let state = AppState {
        config: Arc::new(config.clone()),
        pool: pool_arc.clone(),
        user_pools: Arc::new(RwLock::new(user_pools)),
        routing_strategy_overrides: routing_strategy_overrides.clone(),
        client,
        store,
        rate_limiter,
        interceptor_chain: Arc::new(interceptor_chain),
        health_manager: health_manager.clone(),
        transform_registry,
        model_catalog,
    };

    let bp = &config.base_paths;
    let cors = build_cors_layer(&config.cors);

    let health_routes = Router::new()
        .route("/healthz", get(healthz))
        .route("/readyz", get(readyz))
        .route("/metrics", get(integration::metrics_endpoint));

    let local_router_open_api_routes = local_router_open_api_routes(bp)
        .layer(middleware::from_fn_with_state(
            state.clone(),
            rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth::proxy_auth_middleware,
        ));

    let local_router_app_api_routes = local_router_app_api_routes().layer(
        middleware::from_fn_with_state(state.clone(), auth::app_auth_middleware),
    );

    let local_router_backend_api_routes = local_router_backend_api_routes().layer(
        middleware::from_fn_with_state(state.clone(), auth::admin_auth_middleware),
    );

    let body_limit_bytes = config.server.max_body_size_bytes();

    let router = Router::new()
        .merge(health_routes)
        .merge(local_router_open_api_routes)
        .merge(local_router_app_api_routes)
        .merge(local_router_backend_api_routes)
        .layer(RequestBodyLimitLayer::new(body_limit_bytes))
        .layer(cors)
        .with_state(state);

    Ok((router, health_manager, pool_arc, routing_strategy_overrides))
}

fn build_cors_layer(cors_config: &sdkwork_lr_config::CorsConfig) -> CorsLayer {
    let use_private_network_policy = cors_config.is_allow_any()
        || cors_config.allowed_origins.iter().any(|origin| origin.contains(":*"));
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

    let exposed_headers = [
        header::HeaderName::from_static("x-request-id"),
        header::HeaderName::from_static("x-sdkwork-plugin-policy"),
        header::HeaderName::from_static("x-sdkwork-plugin-source"),
        header::HeaderName::from_static("x-sdkwork-plugin-target"),
        header::HeaderName::from_static("x-sdkwork-plugin-id"),
        header::HeaderName::from_static("x-sdkwork-model-vendor"),
        header::HeaderName::from_static("x-sdkwork-client-api"),
        header::HeaderName::from_static("access-token"),
    ];

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
    sdkwork_web_axum::cors_layer_from_policy(policy).expose_headers(exposed_headers)
}

fn local_router_open_api_routes(
    base_paths: &sdkwork_lr_config::BasePathConfig,
) -> Router<AppState> {
    passthrough::routes(
        &base_paths.openai,
        &base_paths.anthropic,
        &base_paths.google,
    )
}

fn local_router_app_api_routes() -> Router<AppState> {
    let canonical_prefix = crate::api_groups::LOCAL_ROUTER_APP_API_PREFIX;
    local_router_app_api_routes_for_prefix(canonical_prefix)
}

fn local_router_app_api_routes_for_prefix(prefix: &str) -> Router<AppState> {
    Router::new()
        .route(&format!("{prefix}/status"), get(integration::router_status))
        .route(&format!("{prefix}/models"), get(integration::list_models))
}

fn local_router_backend_api_routes() -> Router<AppState> {
    let canonical_prefix = crate::api_groups::LOCAL_ROUTER_BACKEND_API_PREFIX;
    local_router_backend_api_routes_for_prefix(canonical_prefix)
}

fn local_router_backend_api_routes_for_prefix(prefix: &str) -> Router<AppState> {
    Router::new()
        .route(
            &format!("{prefix}/upstreams"),
            get(integration::list_upstreams),
        )
        .route(&format!("{prefix}/logs"), get(integration::list_logs))
        .route(&format!("{prefix}/plugins"), get(integration::list_plugins))
        .route(
            &format!("{prefix}/plugins/decision"),
            get(integration::plugin_decision),
        )
        .route(
            &format!("{prefix}/api_groups"),
            get(integration::list_api_groups),
        )
        .route(
            &format!("{prefix}/accounts"),
            get(integration::list_accounts).post(integration::create_account),
        )
        .route(
            &format!("{prefix}/client_api_keys"),
            get(integration::list_client_api_keys).post(integration::create_client_api_key),
        )
        .route(
            &format!("{prefix}/client_api_keys/{{clientApiKeyId}}"),
            axum::routing::delete(integration::delete_client_api_key),
        )
        .route(
            &format!("{prefix}/accounts/{{accountId}}"),
            put(integration::update_account).delete(integration::delete_account),
        )
        .route(
            &format!("{prefix}/accounts/{{accountId}}/toggle"),
            post(integration::toggle_account),
        )
        .route(
            &format!("{prefix}/model_route_mappings"),
            get(integration::list_model_route_mappings)
                .post(integration::upsert_model_route_mapping),
        )
        .route(
            &format!("{prefix}/model_route_mappings/{{modelRouteMappingId}}"),
            axum::routing::delete(integration::delete_model_route_mapping),
        )
        .route(&format!("{prefix}/usages"), get(integration::list_usages))
        .route(
            &format!("{prefix}/usages/summary"),
            get(integration::usage_summary),
        )
        .route(&format!("{prefix}/health"), get(integration::list_health))
        .route(
            &format!("{prefix}/health/{{accountName}}/open"),
            post(integration::force_open_account),
        )
        .route(
            &format!("{prefix}/health/{{accountName}}/close"),
            post(integration::force_close_account),
        )
        .route(
            &format!("{prefix}/strategy"),
            post(integration::set_routing_strategy),
        )
}

async fn healthz() -> impl IntoResponse {
    Json(json!({"status": "ok", "service": "sdkwork-local-router"}))
}

async fn readyz(State(state): State<AppState>) -> impl IntoResponse {
    let pool = state.pool.read();
    let mut response = json!({
        "status": "ok",
        "service": "sdkwork-local-router",
        "upstream_count": pool.len(),
        "routing_strategy": format!("{:?}", pool.strategy()),
    });
    response["version"] = json!(env!("CARGO_PKG_VERSION"));
    Json(response)
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::{Method, Request, StatusCode};
    use axum::response::Response;
    use std::path::PathBuf;
    use std::task::{Context, Poll};
    use tower::{Layer, Service, ServiceExt};

    fn temp_sqlite_url(name: &str) -> (String, PathBuf) {
        let mut path = std::env::temp_dir();
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!("sdkwork-local-router-service-{name}-{nanos}.db"));
        let url = format!("sqlite:{}?mode=rwc", path.to_string_lossy());
        (url, path)
    }

    #[tokio::test]
    async fn cors_preflight_allows_explicit_client_api_header() {
        let config = sdkwork_lr_config::CorsConfig {
            allowed_origins: vec!["https://tools.example".to_owned()],
        };
        let layer = build_cors_layer(&config);
        let mut service = layer.layer(tower::service_fn(|_req| async {
            Ok::<_, std::convert::Infallible>(Response::new(Body::empty()))
        }));

        let mut cx = Context::from_waker(std::task::Waker::noop());
        assert!(matches!(service.poll_ready(&mut cx), Poll::Ready(Ok(()))));

        let request = Request::builder()
            .method(Method::OPTIONS)
            .uri("/v1/responses")
            .header(header::ORIGIN, "https://tools.example")
            .header(header::ACCESS_CONTROL_REQUEST_METHOD, "POST")
            .header(
                header::ACCESS_CONTROL_REQUEST_HEADERS,
                "content-type,x-sdkwork-client-api",
            )
            .body(Body::empty())
            .unwrap();

        let response = service.call(request).await.unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn server_router_does_not_expose_embedded_frontend_routes() {
        let (url, path) = temp_sqlite_url("server-only-router");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();
        let config =
            RuntimeConfig::from_toml(&sdkwork_lr_config::RuntimeTomlConfig::default()).unwrap();
        let rate_limiter = Arc::new(RateLimiter::new(60, 60));
        let (app, _health_manager, _pool, _strategy_overrides) =
            build_router(&config, store.clone(), rate_limiter)
                .await
                .unwrap();

        for uri in ["/", "/dashboard"] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(Method::GET)
                        .uri(uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn backend_api_exposes_local_router_api_group_manifest() {
        let (url, path) = temp_sqlite_url("api-groups");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();
        let config =
            RuntimeConfig::from_toml(&sdkwork_lr_config::RuntimeTomlConfig::default()).unwrap();
        let rate_limiter = Arc::new(RateLimiter::new(60, 60));
        let (app, _health_manager, _pool, _strategy_overrides) =
            build_router(&config, store.clone(), rate_limiter)
                .await
                .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/backend/v3/api/local_router/api_groups")
                    .header(crate::auth::X_SDKWORK_SUBJECT_USER_ID_HEADER, "42")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let groups = payload["api_groups"].as_array().expect("api groups");
        assert_eq!(groups.len(), 3);
        assert!(groups
            .iter()
            .any(|group| group["code"] == "local-router-open-api"
                && group["auth_scheme"] == "client_api_key"));
        assert!(groups
            .iter()
            .any(|group| group["code"] == "local-router-app-api"
                && group["canonical_prefix"] == "/app/v3/api/local_router"));
        assert!(groups
            .iter()
            .any(|group| group["code"] == "local-router-backend-api"
                && group["canonical_prefix"] == "/backend/v3/api/local_router"));

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn app_and_backend_router_paths_only_expose_local_router_prefixes() {
        let (url, path) = temp_sqlite_url("api-prefixes");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();
        let config =
            RuntimeConfig::from_toml(&sdkwork_lr_config::RuntimeTomlConfig::default()).unwrap();
        let rate_limiter = Arc::new(RateLimiter::new(60, 60));
        let (app, _health_manager, _pool, _strategy_overrides) =
            build_router(&config, store.clone(), rate_limiter)
                .await
                .unwrap();

        for uri in [
            "/app/v3/api/local_router/status",
            "/backend/v3/api/local_router/api_groups",
            "/backend/v3/api/local_router/model_route_mappings",
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(Method::GET)
                        .uri(uri)
                        .header(crate::auth::X_SDKWORK_SUBJECT_USER_ID_HEADER, "42")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::OK, "{uri}");
        }

        let removed_app_prefix = format!("/app/v3/api/{}", "router");
        let removed_backend_prefix = format!("/backend/v3/api/{}", "router");
        let removed_mapping_path = format!("/backend/v3/api/local_router/model_{}", "aliases");
        for uri in [
            format!("{removed_app_prefix}/status"),
            format!("{removed_backend_prefix}/api_groups"),
            removed_mapping_path,
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(Method::GET)
                        .uri(&uri)
                        .header(crate::auth::X_SDKWORK_SUBJECT_USER_ID_HEADER, "42")
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();

            assert_eq!(response.status(), StatusCode::NOT_FOUND, "{uri}");
        }

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn backend_accounts_response_uses_model_route_mapping_names() {
        let (url, path) = temp_sqlite_url("accounts-route-mappings");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();
        let mut route_mappings = std::collections::BTreeMap::new();
        route_mappings.insert("gpt-5.5".to_owned(), "gemini-2.5-pro".to_owned());
        let account_id = store
            .insert_account(&sdkwork_lr_store::NewAccount {
                user_id: 42,
                name: "gemini".to_owned(),
                provider: "google".to_owned(),
                base_url: "https://generativelanguage.googleapis.com".to_owned(),
                upstream_api_key: "sk-upstream".to_owned(),
                upstream_auth_scheme: None,
                models: vec!["gemini-*".to_owned()],
                priority: 10,
                timeout_secs: 120,
                max_retries: 0,
                retry_delay_ms: 500,
                anthropic_version: None,
                default_headers: std::collections::BTreeMap::new(),
                model_route_mappings: route_mappings,
                enabled: true,
            })
            .await
            .unwrap();
        store
            .upsert_model_route_mapping(&sdkwork_lr_store::NewModelRouteMapping {
                user_id: 42,
                account_id,
                account_name: "gemini".to_owned(),
                client_model: "codex-gemini".to_owned(),
                upstream_model: "gemini-2.5-flash".to_owned(),
                enabled: true,
                notes: None,
            })
            .await
            .unwrap();

        let config =
            RuntimeConfig::from_toml(&sdkwork_lr_config::RuntimeTomlConfig::default()).unwrap();
        let rate_limiter = Arc::new(RateLimiter::new(60, 60));
        let (app, _health_manager, _pool, _strategy_overrides) =
            build_router(&config, store.clone(), rate_limiter)
                .await
                .unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::GET)
                    .uri("/backend/v3/api/local_router/accounts")
                    .header(crate::auth::X_SDKWORK_SUBJECT_USER_ID_HEADER, "42")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let account = &payload["accounts"][0];
        assert!(account.get("model_route_mappings").is_some());
        assert!(account.get("model_route_mapping_rows").is_some());
        let removed_field = format!("model_{}", "aliases");
        assert!(account.get(&removed_field).is_none());
        assert_eq!(account["model_route_mappings"]["gpt-5.5"], "gemini-2.5-pro");
        assert_eq!(
            account["model_route_mappings"]["codex-gemini"],
            "gemini-2.5-flash"
        );

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[tokio::test]
    async fn backend_strategy_auto_clears_user_override_and_restores_multi_account_default() {
        let (url, path) = temp_sqlite_url("strategy-auto");
        let store = Store::new(&url).await.unwrap();
        store.run_migrations().await.unwrap();
        let mut toml_config = sdkwork_lr_config::RuntimeTomlConfig::default();
        toml_config.upstreams = vec![
            sdkwork_lr_config::UpstreamSectionConfig {
                name: "primary".to_owned(),
                provider: "openai".to_owned(),
                base_url: "https://primary.example/v1".to_owned(),
                upstream_api_key: Some("sk-primary".to_owned()),
                upstream_auth_scheme: None,
                models: vec!["*".to_owned()],
                priority: Some(10),
                timeout_secs: None,
                max_retries: None,
                retry_delay_ms: None,
                anthropic_version: None,
                default_headers: std::collections::BTreeMap::new(),
                model_route_mappings: std::collections::BTreeMap::new(),
            },
            sdkwork_lr_config::UpstreamSectionConfig {
                name: "secondary".to_owned(),
                provider: "openai".to_owned(),
                base_url: "https://secondary.example/v1".to_owned(),
                upstream_api_key: Some("sk-secondary".to_owned()),
                upstream_auth_scheme: None,
                models: vec!["*".to_owned()],
                priority: Some(20),
                timeout_secs: None,
                max_retries: None,
                retry_delay_ms: None,
                anthropic_version: None,
                default_headers: std::collections::BTreeMap::new(),
                model_route_mappings: std::collections::BTreeMap::new(),
            },
        ];
        let config = RuntimeConfig::from_toml(&toml_config).unwrap();
        let rate_limiter = Arc::new(RateLimiter::new(60, 60));
        let (app, _health_manager, pool, strategy_overrides) =
            build_router(&config, store.clone(), rate_limiter)
                .await
                .unwrap();

        assert_eq!(
            pool.read().strategy(),
            sdkwork_lr_core::RoutingStrategy::RoundRobin
        );

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/backend/v3/api/local_router/strategy")
                    .header(crate::auth::X_SDKWORK_SUBJECT_USER_ID_HEADER, "0")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"strategy":"priority"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            pool.read().strategy(),
            sdkwork_lr_core::RoutingStrategy::Priority
        );
        assert_eq!(
            strategy_overrides
                .read()
                .get(&sdkwork_lr_store::DEFAULT_USER_ID)
                .copied(),
            Some(sdkwork_lr_core::RoutingStrategy::Priority)
        );

        let response = app
            .oneshot(
                Request::builder()
                    .method(Method::POST)
                    .uri("/backend/v3/api/local_router/strategy")
                    .header(crate::auth::X_SDKWORK_SUBJECT_USER_ID_HEADER, "0")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"strategy":"auto"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let payload: serde_json::Value = serde_json::from_slice(&body).unwrap();
        assert_eq!(payload["strategy_mode"], "auto");
        assert_eq!(payload["new_strategy"], "RoundRobin");
        assert_eq!(
            pool.read().strategy(),
            sdkwork_lr_core::RoutingStrategy::RoundRobin
        );
        assert!(!strategy_overrides
            .read()
            .contains_key(&sdkwork_lr_store::DEFAULT_USER_ID));

        drop(store);
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn local_router_api_groups_are_declared_as_three_service_surfaces() {
        let base_paths = sdkwork_lr_config::BasePathConfig {
            openai: "/v1".to_owned(),
            anthropic: "/anthropic".to_owned(),
            google: "/google".to_owned(),
        };

        let groups = crate::api_groups::local_router_api_groups(&base_paths);
        let codes: Vec<_> = groups.iter().map(|group| group.code).collect();

        assert_eq!(
            codes,
            vec![
                "local-router-open-api",
                "local-router-app-api",
                "local-router-backend-api",
            ]
        );

        let open_api = groups
            .iter()
            .find(|group| group.code == "local-router-open-api")
            .expect("open api group");
        assert_eq!(
            open_api.user_id_source,
            "local_router_client_api_keys.user_id"
        );
        assert_eq!(open_api.auth_scheme, "client_api_key");
        assert!(open_api
            .path_prefixes
            .iter()
            .any(|path| path == "/v1/{*path}"));

        let app_api = groups
            .iter()
            .find(|group| group.code == "local-router-app-api")
            .expect("app api group");
        assert_eq!(app_api.canonical_prefix, "/app/v3/api/local_router");
        assert_eq!(app_api.auth_scheme, "sdkwork_subject_projection");

        let backend_api = groups
            .iter()
            .find(|group| group.code == "local-router-backend-api")
            .expect("backend api group");
        assert_eq!(backend_api.canonical_prefix, "/backend/v3/api/local_router");
        assert!(backend_api
            .capabilities
            .iter()
            .any(|capability| *capability == "account_management"));
    }
}
