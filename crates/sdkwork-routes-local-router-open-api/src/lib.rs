mod passthrough;

use axum::Router;
use sdkwork_routes_local_router_support::AppState;

pub fn routes(base_paths: &sdkwork_lr_config::BasePathConfig) -> Router<AppState> {
    passthrough::routes(
        &base_paths.openai,
        &base_paths.anthropic,
        &base_paths.google,
    )
}

pub fn gateway_mount(base_paths: &sdkwork_lr_config::BasePathConfig, state: AppState) -> Router {
    routes(base_paths).with_state(state)
}
