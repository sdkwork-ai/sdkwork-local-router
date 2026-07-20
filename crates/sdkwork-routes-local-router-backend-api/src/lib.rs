use axum::routing::{delete, get, post, put};
use axum::Router;
use sdkwork_routes_local_router_support::{api_groups, integration, AppState};

pub fn routes() -> Router<AppState> {
    let prefix = api_groups::LOCAL_ROUTER_BACKEND_API_PREFIX;
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
            delete(integration::delete_client_api_key),
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
            delete(integration::delete_model_route_mapping),
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

pub fn gateway_mount(state: AppState) -> Router {
    routes().with_state(state)
}
