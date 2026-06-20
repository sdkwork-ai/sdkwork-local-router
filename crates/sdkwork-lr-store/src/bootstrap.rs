//! SDKWork Local Router database pool bootstrap via `sdkwork-database`.

use sdkwork_database_config::DatabaseConfig;
use sdkwork_database_sqlx::{create_pool_from_config, DatabasePool, PoolError};

pub use sdkwork_local_router_database_host::{
    bootstrap_local_router_database, bootstrap_local_router_database_from_env,
    LocalRouterDatabaseHost,
};

pub type LocalRouterDatabasePool = DatabasePool;

pub async fn connect_local_router_database_pool_from_env(
) -> Result<LocalRouterDatabasePool, PoolError> {
    let config = DatabaseConfig::from_env("LOCAL_ROUTER")?;
    create_pool_from_config(config).await
}

pub async fn connect_and_bootstrap_local_router_database_from_env(
) -> Result<LocalRouterDatabaseHost, String> {
    let pool = connect_local_router_database_pool_from_env()
        .await
        .map_err(|error| error.to_string())?;
    bootstrap_local_router_database(pool).await
}
