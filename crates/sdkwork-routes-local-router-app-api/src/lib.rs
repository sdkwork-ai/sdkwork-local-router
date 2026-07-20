use axum::routing::get;
use axum::Router;
use sdkwork_routes_local_router_support::{api_groups, integration, AppState};

pub fn routes() -> Router<AppState> {
    let prefix = api_groups::LOCAL_ROUTER_APP_API_PREFIX;
    Router::new()
        .route(&format!("{prefix}/status"), get(integration::router_status))
        .route(&format!("{prefix}/models"), get(integration::list_models))
}

pub fn gateway_mount(state: AppState) -> Router {
    routes().with_state(state)
}
