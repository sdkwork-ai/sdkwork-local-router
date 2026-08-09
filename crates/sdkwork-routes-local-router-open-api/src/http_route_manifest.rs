//! sdkwork-routes-local-router-open-api gateway route manifest (materialized from the authored
//! route definitions; business routes use dual-token auth).

use sdkwork_web_core::{HttpMethod, HttpRoute, HttpRouteManifest};

const HTTP_ROUTES: &[HttpRoute] = &[
    HttpRoute::dual_token(HttpMethod::Get, "/local-router/v1/{*path}", "local_router", "localRouter.openai.proxy"),
    HttpRoute::dual_token(HttpMethod::Get, "/local-router/anthropic/{*path}", "local_router", "localRouter.anthropic.proxy"),
    HttpRoute::dual_token(HttpMethod::Get, "/local-router/google/{*path}", "local_router", "localRouter.google.proxy"),
];

pub fn gateway_route_manifest() -> HttpRouteManifest {
    HttpRouteManifest::new(HTTP_ROUTES)
}
