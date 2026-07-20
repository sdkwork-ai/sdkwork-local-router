use std::collections::HashMap;
use std::sync::Arc;

use parking_lot::RwLock;
use sdkwork_lr_config::RuntimeConfig;
use sdkwork_lr_core::{
    Account, AccountPool, AuditMetadataInterceptor, CircuitBreakerConfig, HealthManager,
    InterceptorChain, LoggingInterceptor, MetricsInterceptor,
};
use sdkwork_lr_plugin::PluginRegistry;
use sdkwork_lr_proxy::ProxyClient;
use sdkwork_lr_store::{Store, DEFAULT_USER_ID};

use crate::rate_limit::RateLimiter;

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

pub fn build_app_state(
    config: &RuntimeConfig,
    store: Store,
    rate_limiter: Arc<RateLimiter>,
) -> (
    AppState,
    Arc<HealthManager>,
    Arc<RwLock<AccountPool>>,
    RoutingStrategyOverrides,
) {
    let accounts: Vec<Account> = config
        .upstreams
        .iter()
        .map(|upstream| Account {
            name: upstream.name.clone(),
            provider: upstream.provider.clone(),
            base_url: upstream.base_url.clone(),
            upstream_api_key: upstream.upstream_api_key.clone(),
            upstream_auth_scheme: upstream.upstream_auth_scheme.clone(),
            models: upstream.models.clone(),
            priority: upstream.priority,
            timeout: upstream.timeout,
            max_retries: upstream.max_retries,
            retry_delay_ms: upstream.retry_delay_ms,
            anthropic_version: upstream.anthropic_version.clone(),
            default_headers: upstream.default_headers.clone(),
            enabled: true,
            model_route_mappings: upstream.model_route_mappings.clone(),
        })
        .collect();

    let health_manager = Arc::new(HealthManager::new(CircuitBreakerConfig {
        failure_threshold: config.circuit_breaker.failure_threshold,
        success_threshold: config.circuit_breaker.success_threshold,
        open_duration: config.circuit_breaker.open_duration,
        half_open_max_requests: config.circuit_breaker.half_open_max_requests,
    }));
    let strategy = config.routing.strategy_for_account_count(accounts.len());
    let pool = Arc::new(RwLock::new(AccountPool::with_health_manager(
        accounts,
        strategy,
        health_manager.clone(),
    )));
    let mut user_pools = HashMap::new();
    user_pools.insert(DEFAULT_USER_ID, pool.clone());
    let routing_strategy_overrides = Arc::new(RwLock::new(HashMap::new()));

    let mut interceptor_chain = InterceptorChain::new();
    interceptor_chain.add(Arc::new(LoggingInterceptor));
    interceptor_chain.add(Arc::new(MetricsInterceptor));
    interceptor_chain.add(Arc::new(AuditMetadataInterceptor));

    let transform_registry = Arc::new(sdkwork_lr_transform::plugins::built_in_plugin_registry());
    let model_catalog = sdkwork_models::load_bundled_catalog()
        .map(Arc::new)
        .map_err(|error| {
            tracing::warn!(%error, "failed to load sdkwork-models catalog");
        })
        .ok();

    let state = AppState {
        config: Arc::new(config.clone()),
        pool: pool.clone(),
        user_pools: Arc::new(RwLock::new(user_pools)),
        routing_strategy_overrides: routing_strategy_overrides.clone(),
        client: sdkwork_lr_proxy::build_proxy_client(),
        store,
        rate_limiter,
        interceptor_chain: Arc::new(interceptor_chain),
        health_manager: health_manager.clone(),
        transform_registry,
        model_catalog,
    };

    (state, health_manager, pool, routing_strategy_overrides)
}
