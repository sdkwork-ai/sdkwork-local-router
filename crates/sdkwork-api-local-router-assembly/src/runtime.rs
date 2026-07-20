use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

use anyhow::Result;
use axum::http::StatusCode;
use parking_lot::RwLock;
use tokio::net::TcpListener;
use tokio::signal;

use crate::rate_limit::RateLimiter;
use crate::router;
use crate::upstream_auth::auth_from_scheme;
use sdkwork_lr_config::RuntimeConfig;
use sdkwork_lr_core::{Account, HealthManager, HealthState};
use sdkwork_lr_proxy::ProxyClient;
use sdkwork_lr_store::Store;

pub async fn serve(config: RuntimeConfig) -> Result<()> {
    let encryption = sdkwork_lr_store::KeyEncryption::new(&config.storage.encryption_secret);
    if encryption.is_enabled() {
        tracing::info!("upstream API key encryption enabled");
    } else {
        tracing::warn!("upstream API key encryption disabled - keys stored in plaintext");
    }

    let store = Store::with_encryption(&config.storage.database_url, encryption)
        .await
        .map_err(anyhow::Error::msg)?;
    store.run_migrations().await.map_err(anyhow::Error::msg)?;

    let rate_limiter = Arc::new(RateLimiter::new(
        config.rate_limit.max_requests,
        config.rate_limit.window_secs,
    ));

    let shutdown_token = Arc::new(AtomicBool::new(false));

    if config.rate_limit.enabled {
        spawn_rate_limit_cleanup(
            rate_limiter.clone(),
            config.rate_limit.window_secs,
            shutdown_token.clone(),
        );
    }

    let cleanup_store = store.clone();
    let cleanup_retention_days = config.recording.retention_days;
    spawn_invocation_cleanup(
        cleanup_store,
        cleanup_retention_days,
        shutdown_token.clone(),
    );

    let (app, health_manager, pool_arc, routing_strategy_overrides) =
        router::build_router(&config, store.clone(), rate_limiter).await?;

    if config.health_probe.enabled {
        let probe_accounts: Vec<Account> = {
            let pool = pool_arc.read();
            pool.enabled_accounts_arc()
                .into_iter()
                .map(|a| (*a).clone())
                .collect()
        };
        let probe_client = sdkwork_lr_proxy::build_proxy_client();
        spawn_health_probe(
            health_manager,
            config.health_probe.interval_secs,
            probe_accounts,
            probe_client,
            shutdown_token.clone(),
        );
    }

    if let Some(ref config_path) = config.config_file_path {
        spawn_config_watcher(
            config_path.clone(),
            pool_arc.clone(),
            routing_strategy_overrides,
            store.clone(),
            shutdown_token.clone(),
        );
    }

    let bind_addr = config.bind_addr();
    let listener = TcpListener::bind(&bind_addr).await?;
    tracing::info!(addr = %bind_addr, "listening");

    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal(shutdown_token.clone()))
        .await?;

    shutdown_token.store(true, Ordering::SeqCst);
    tracing::info!("waiting for background tasks to complete...");
    tokio::time::sleep(Duration::from_millis(500)).await;
    tracing::info!("server shutdown complete");
    Ok(())
}

fn spawn_rate_limit_cleanup(
    limiter: Arc<RateLimiter>,
    window_secs: u64,
    shutdown: Arc<AtomicBool>,
) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(window_secs * 2));
        loop {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }
            interval.tick().await;
            limiter.cleanup();
        }
        tracing::debug!("rate limit cleanup task stopped");
    });
}

fn spawn_invocation_cleanup(store: Store, retention_days: i64, shutdown: Arc<AtomicBool>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(3600));
        loop {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }
            interval.tick().await;
            match store.cleanup_invocations(retention_days).await {
                Ok(deleted) => {
                    if deleted > 0 {
                        tracing::info!(
                            deleted = deleted,
                            retention_days = retention_days,
                            "cleaned up old invocations"
                        );
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to cleanup invocations");
                }
            }
        }
        tracing::debug!("invocation cleanup task stopped");
    });
}

fn spawn_config_watcher(
    config_path: String,
    pool_arc: Arc<RwLock<sdkwork_lr_core::AccountPool>>,
    routing_strategy_overrides: router::RoutingStrategyOverrides,
    store: Store,
    shutdown: Arc<AtomicBool>,
) {
    use std::sync::atomic::AtomicU64;

    const CONFIG_CHECK_INTERVAL_SECS: u64 = 30;

    let last_modified = Arc::new(AtomicU64::new(0));
    let last_modified_clone = last_modified.clone();

    let path = config_path.clone();
    tokio::spawn(async move {
        let init_mtime = std::fs::metadata(&path)
            .ok()
            .and_then(|m| m.modified().ok())
            .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);
        last_modified_clone.store(init_mtime, Ordering::Relaxed);
        tracing::info!(path = %path, "config file watcher started");
    });

    let store_for_reload = store.clone();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(CONFIG_CHECK_INTERVAL_SECS));
        loop {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }
            interval.tick().await;

            let current_mtime = match std::fs::metadata(&config_path) {
                Ok(meta) => match meta.modified() {
                    Ok(t) => t
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0),
                    Err(_) => continue,
                },
                Err(_) => continue,
            };

            let prev = last_modified.load(Ordering::Relaxed);
            if current_mtime <= prev {
                continue;
            }
            last_modified.store(current_mtime, Ordering::Relaxed);

            tracing::info!(path = %config_path, "config file changed, reloading...");

            match sdkwork_lr_config::RuntimeTomlConfig::from_config_file(&config_path) {
                Ok(toml_config) => {
                    match sdkwork_lr_config::RuntimeConfig::from_toml(&toml_config) {
                        Ok(new_config) => {
                            let accounts: Vec<Account> = new_config
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

                            // Persist to store (best-effort, log on failure)
                            for acc in &accounts {
                                let new_account = sdkwork_lr_store::NewAccount {
                                    user_id: sdkwork_lr_store::DEFAULT_USER_ID,
                                    name: acc.name.clone(),
                                    provider: acc.provider.to_string(),
                                    base_url: acc.base_url.clone(),
                                    upstream_api_key: acc.upstream_api_key.clone(),
                                    upstream_auth_scheme: acc.upstream_auth_scheme.clone(),
                                    models: acc.models.clone(),
                                    priority: acc.priority as i32,
                                    timeout_secs: acc.timeout.as_secs() as i64,
                                    max_retries: acc.max_retries as i32,
                                    retry_delay_ms: acc.retry_delay_ms as i64,
                                    anthropic_version: acc.anthropic_version.clone(),
                                    default_headers: acc.default_headers.clone(),
                                    model_route_mappings: acc.model_route_mappings.clone(),
                                    enabled: acc.enabled,
                                };
                                let _ = store_for_reload.upsert_account(&new_account).await;
                            }

                            let health_manager = {
                                let pool = pool_arc.read();
                                pool.health_manager().clone()
                            };
                            let strategy = routing_strategy_overrides
                                .read()
                                .get(&sdkwork_lr_store::DEFAULT_USER_ID)
                                .copied()
                                .unwrap_or_else(|| {
                                    new_config
                                        .routing
                                        .strategy_for_account_count(accounts.len())
                                });
                            let new_pool = sdkwork_lr_core::AccountPool::with_health_manager(
                                accounts,
                                strategy,
                                health_manager,
                            );
                            *pool_arc.write() = new_pool;
                            tracing::info!("account pool hot-reloaded from config file");
                        }
                        Err(e) => {
                            tracing::warn!(error = %e, "failed to parse reloaded config, keeping previous configuration");
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(error = %e, "failed to read config file, keeping previous configuration");
                }
            }
        }
        tracing::debug!("config watcher task stopped");
    });
}

fn spawn_health_probe(
    health_manager: Arc<HealthManager>,
    interval_secs: u64,
    accounts: Vec<Account>,
    client: ProxyClient,
    shutdown: Arc<AtomicBool>,
) {
    use std::collections::HashMap;

    let account_map: HashMap<String, Account> =
        accounts.into_iter().map(|a| (a.name.clone(), a)).collect();

    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs));
        loop {
            if shutdown.load(Ordering::Relaxed) {
                break;
            }
            interval.tick().await;
            let snapshots = health_manager.snapshots();
            let unhealthy: Vec<_> = snapshots
                .iter()
                .filter(|s| s.state != HealthState::Healthy)
                .collect();

            if unhealthy.is_empty() {
                continue;
            }

            for snapshot in &unhealthy {
                let Some(account) = account_map.get(&snapshot.account_name) else {
                    continue;
                };

                if snapshot.state == HealthState::CircuitOpen {
                    if snapshot.opened_ago.unwrap_or(0) < 10 {
                        continue;
                    }
                }

                let probe_result = probe_account(&client, account).await;
                match probe_result {
                    ProbeResult::Reachable => {
                        health_manager.record_success(&snapshot.account_name);
                        tracing::info!(
                            account = %snapshot.account_name,
                            old_state = %snapshot.state,
                            "health probe: account is reachable, recorded success"
                        );
                    }
                    ProbeResult::Unreachable(reason) => {
                        health_manager.record_failure(&snapshot.account_name);
                        tracing::warn!(
                            account = %snapshot.account_name,
                            state = %snapshot.state,
                            reason = %reason,
                            "health probe: account is unreachable, recorded failure"
                        );
                    }
                    ProbeResult::Skipped => {}
                }
            }

            let healthy = health_manager.healthy_count();
            let degraded = health_manager.degraded_count();
            let open = health_manager.circuit_open_count();
            if open > 0 || degraded > 0 {
                tracing::info!(
                    healthy = healthy,
                    degraded = degraded,
                    circuit_open = open,
                    "health probe summary"
                );
            }
        }
        tracing::debug!("health probe task stopped");
    });
}

enum ProbeResult {
    Reachable,
    Unreachable(String),
    #[allow(dead_code)]
    Skipped,
}

fn probe_account<'a>(
    client: &'a ProxyClient,
    account: &'a Account,
) -> std::pin::Pin<Box<dyn std::future::Future<Output = ProbeResult> + Send + 'a>> {
    Box::pin(async move {
        let auth = build_probe_auth(account);
        let probe_path = build_probe_path(account);

        let target = sdkwork_lr_proxy::ForwardTarget {
            base_url: account.base_url.clone(),
            auth,
            default_headers: account
                .default_headers
                .iter()
                .map(|(k, v)| (k.clone(), v.clone()))
                .collect(),
            anthropic_version: effective_probe_anthropic_version(account),
            timeout: Some(account.timeout),
            request_id: "health-probe".to_owned(),
        };

        let target_ref = &target;
        let path_ref = &probe_path;

        let result = sdkwork_lr_proxy::forward_raw(
            client,
            axum::http::Method::GET,
            &axum::http::HeaderMap::new(),
            bytes::Bytes::new(),
            target_ref,
            path_ref,
        )
        .await;

        match result {
            Ok(response) => {
                let status = response.status();
                if status.is_success() {
                    ProbeResult::Reachable
                } else if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
                    ProbeResult::Unreachable(format!("auth error: {}", status))
                } else if status.is_server_error() {
                    ProbeResult::Unreachable(format!("server error: {}", status))
                } else {
                    ProbeResult::Reachable
                }
            }
            Err(e) => ProbeResult::Unreachable(e),
        }
    })
}

fn build_probe_auth(account: &Account) -> Option<sdkwork_lr_proxy::AuthInjection> {
    use sdkwork_lr_core::ProviderKind;
    if account.upstream_api_key.is_empty() {
        return None;
    }
    if let Some(auth) = auth_from_scheme(
        account.upstream_auth_scheme.as_deref(),
        &account.upstream_api_key,
    ) {
        return Some(auth);
    }
    match account.provider {
        ProviderKind::Openai | ProviderKind::Custom(_) => Some(
            sdkwork_lr_proxy::AuthInjection::Bearer(account.upstream_api_key.clone()),
        ),
        ProviderKind::Anthropic => Some(sdkwork_lr_proxy::AuthInjection::Header(
            "x-api-key".to_owned(),
            account.upstream_api_key.clone(),
        )),
        ProviderKind::Google => Some(sdkwork_lr_proxy::AuthInjection::Header(
            "x-goog-api-key".to_owned(),
            account.upstream_api_key.clone(),
        )),
    }
}

fn build_probe_path(account: &Account) -> String {
    use sdkwork_lr_core::ProviderKind;
    match account.provider {
        ProviderKind::Openai | ProviderKind::Custom(_) => "/v1/models".to_owned(),
        ProviderKind::Anthropic => "/v1/models".to_owned(),
        ProviderKind::Google => "/v1/models".to_owned(),
    }
}

fn effective_probe_anthropic_version(account: &Account) -> Option<String> {
    match account.provider {
        sdkwork_lr_core::ProviderKind::Anthropic => account
            .anthropic_version
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_owned)
            .or_else(|| Some("2023-06-01".to_owned())),
        _ => account.anthropic_version.clone(),
    }
}

async fn shutdown_signal(_shutdown_token: Arc<AtomicBool>) {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(windows)]
    let terminate = async {
        signal::windows::ctrl_close()
            .expect("failed to install Windows ctrl_close handler")
            .recv()
            .await
            .expect("failed to receive Windows ctrl_close signal");
    };

    #[cfg(not(windows))]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install SIGTERM handler")
            .recv()
            .await
    };

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("received Ctrl+C, shutting down gracefully");
        },
        _ = terminate => {
            tracing::info!("received terminate signal, shutting down gracefully");
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn anthropic_probe_uses_default_messages_api_version_when_unset() {
        let account = Account {
            name: "claude".to_owned(),
            provider: sdkwork_lr_core::ProviderKind::Anthropic,
            base_url: "https://api.anthropic.com".to_owned(),
            upstream_api_key: "sk-ant".to_owned(),
            upstream_auth_scheme: None,
            models: vec!["claude-*".to_owned()],
            priority: 10,
            timeout: Duration::from_secs(30),
            max_retries: 0,
            retry_delay_ms: 500,
            anthropic_version: None,
            default_headers: BTreeMap::new(),
            enabled: true,
            model_route_mappings: BTreeMap::new(),
        };

        assert_eq!(
            effective_probe_anthropic_version(&account).as_deref(),
            Some("2023-06-01")
        );
    }
}
