//! sdkwork-routes-local-router-app-api gateway route manifest (materialized from the authored
//! route definitions; business routes use dual-token auth).

use sdkwork_web_core::{HttpMethod, HttpRoute, HttpRouteManifest};

const HTTP_ROUTES: &[HttpRoute] = &[
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/local_router/status",
        "local_router",
        "localRouter.status.retrieve",
    ),
    HttpRoute::dual_token(
        HttpMethod::Get,
        "/app/v3/api/local_router/models",
        "local_router",
        "localRouter.models.list",
    ),
];

pub fn gateway_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(HTTP_ROUTES)
}
