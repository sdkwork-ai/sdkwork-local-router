//! API assembly bootstrap for sdkwork-local-router.

use axum::Router;

pub struct ApiAssembly {
    pub router: Router,
}

pub fn assemble_api_router() -> ApiAssembly {
    ApiAssembly {
        router: Router::new(),
    }
}
